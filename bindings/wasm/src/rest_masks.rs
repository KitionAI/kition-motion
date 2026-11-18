//! Scalar mask assignments: selectors, kind checks and the assigned register, without geometry.
use std::collections::BTreeMap;

use kition_kit::{ContentAsk, ContentPremise, Premise, Under, Within};
use kition_motion_engine::{Session, commands::resolve_layer};
use kition_motion_project::{Comp, GroupKind, ItemId, Layer, LayerId, PropGroup, Uid};
use serde_json::{Value, json};

// kdm-model's reader rejects property trees beyond its MAX_DEPTH (256).
const MAX_DEPTH: usize = 256;
const MAX_MEMBERS: usize = 65_536;

fn members(layer: &Layer) -> Option<Vec<Uid>> {
    let mut members = Vec::new();
    for mask in layer.props.groups().find(|group| group.match_id == "masks").into_iter().flat_map(|masks| masks.groups()) {
        if matches!(mask.kind, GroupKind::Mask { .. }) {
            if members.len() >= MAX_MEMBERS {
                return None;
            }
            members.push(mask.uid);
        }
    }
    members.sort_unstable();
    Some(members)
}

pub(crate) fn snapshot(comp: &Comp, budget: &mut usize) -> Value {
    let mut values = serde_json::Map::new();
    for layer in &comp.layers {
        if *budget == 0 {
            return Value::Null;
        }
        let Some(members) = members(layer) else { return Value::Null };
        let cost = members.len().saturating_add(1);
        let Some(remaining) = budget.checked_sub(cost) else {
            *budget = 0;
            return Value::Null;
        };
        *budget = remaining;
        values.insert(layer.id.0.to_string(), json!(members));
    }
    json!(values)
}

pub(crate) fn content(session: &Session, ask: &ContentAsk) -> Option<bool> {
    if ask.premise.field != "~maskMembers" {
        return None;
    }
    let comp = ask.premise.id.as_u64().or_else(|| ask.premise.id.as_str()?.parse().ok())?;
    let layer = ask.premise.region.get("layer")?.as_u64()?;
    let before = ask.before.get(layer.to_string())?.as_array()?;
    if before.len() > MAX_MEMBERS {
        return None;
    }
    let mut before: Vec<Uid> = before.iter().map(Value::as_u64).collect::<Option<_>>()?;
    before.sort_unstable();
    let now = session.project.comp(ItemId(comp))?.layer(LayerId(layer))?;
    Some(before != members(now)?)
}

#[derive(Default)]
struct Reads {
    answer: Under,
    fields: BTreeMap<LayerId, Vec<String>>,
}

impl Reads {
    fn field(&mut self, layer: LayerId, field: impl Into<String>) {
        let field = field.into();
        let fields = self.fields.entry(layer).or_default();
        if !fields.contains(&field) {
            fields.push(field);
        }
    }

    fn window(&mut self, parent: u64, field: String, from: usize, to: usize) {
        self.answer.within.push(Within { parent: json!(parent), field, from, to, ..Default::default() });
    }

    fn finish(mut self) -> Under {
        self.answer.premises.extend(self.fields.into_iter().map(|(id, fields)| Premise::on(json!(id.0)).fields(fields).positions(Vec::<String>::new())));
        self.answer
    }
}

fn composition<'a>(session: &'a Session, params: &Value) -> Option<(ItemId, &'a Comp)> {
    let id = match params.get("comp") {
        Some(Value::Number(n)) => n.as_u64().map(ItemId).filter(|id| session.project.comp(*id).is_some()),
        Some(Value::String(name)) => name
            .parse::<u64>()
            .ok()
            .map(ItemId)
            .filter(|id| session.project.comp(*id).is_some())
            .or_else(|| session.project.items.values().find(|item| item.name == *name && item.as_comp().is_some()).map(|item| item.id)),
        _ => session.active_comp_id(),
    }?;
    Some((id, session.project.comp(id)?))
}

fn layer_address(reads: &mut Reads, cid: ItemId, comp: &Comp, value: &Value) -> Option<LayerId> {
    let id = resolve_layer(comp, value)?;
    reads.fields.entry(id).or_default();
    let index = crate::place::index_of(value).or_else(|| {
        let n = value.as_u64()?;
        (!comp.layers.iter().any(|layer| layer.id.0 == n)).then(|| usize::try_from(n).ok()?.checked_sub(1)).flatten()
    });
    if let Some(index) = index {
        reads.answer.objects.push(json!(id.0));
        reads.window(cid.0, "comp.layers".to_owned(), index, index.saturating_add(1));
    } else if value.as_str().is_some_and(|name| name.parse::<u64>().is_err()) {
        // A name resolves to its first match, so earlier candidates' names matter too.
        let index = comp.layers.iter().position(|layer| layer.id == id)?;
        for layer in comp.layers.iter().take(index.saturating_add(1)) {
            reads.field(layer.id, "name");
        }
        reads.window(cid.0, "comp.layers".to_owned(), 0, index.saturating_add(1));
    }
    Some(id)
}

fn layers(session: &Session, params: &Value, cid: ItemId, comp: &Comp, reads: &mut Reads) -> Option<Vec<LayerId>> {
    if let Some(Value::Array(values)) = params.get("layers") {
        return values.iter().map(|value| layer_address(reads, cid, comp, value)).collect();
    }
    if let Some(value) = params.get("layer").filter(|value| !value.is_null()) {
        return Some(vec![layer_address(reads, cid, comp, value)?]);
    }
    Some(session.state.selected_layers.iter().copied().filter(|id| comp.layer(*id).is_some()).collect())
}

fn group_path(root: &PropGroup, uid: Uid) -> Option<(String, &PropGroup)> {
    let mut pending = vec![("props".to_owned(), root, 0usize)];
    while let Some((path, group, depth)) = pending.pop() {
        if group.uid == uid {
            return Some((path, group));
        }
        if depth >= MAX_DEPTH {
            continue;
        }
        for (index, node) in group.children.iter().enumerate().rev() {
            if let Some(child) = node.as_group() {
                pending.push((format!("{path}.children.{index}"), child, depth.saturating_add(1)));
            }
        }
    }
    None
}

fn selected_group<'a>(root: &'a PropGroup, uid: Uid, prefix: &str) -> Option<(String, &'a PropGroup)> {
    let mut pending = vec![(prefix.to_owned(), root, 0usize)];
    while let Some((path, group, depth)) = pending.pop() {
        if group.uid == uid {
            return Some((path, group));
        }
        if depth < MAX_DEPTH {
            for child in group.groups() {
                pending.push((format!("{path}.children[uid={}]", child.uid), child, depth.saturating_add(1)));
            }
        }
    }
    None
}

fn selected_property(root: &PropGroup, uid: Uid, prefix: &str) -> Option<String> {
    let mut pending = vec![(prefix.to_owned(), root, 0usize)];
    while let Some((path, group, depth)) = pending.pop() {
        if depth >= MAX_DEPTH {
            continue;
        }
        for node in &group.children {
            let path = format!("{path}.children[uid={}]", node.uid());
            if node.as_prop().is_some_and(|property| property.uid == uid) {
                return Some(path);
            }
            if let Some(group) = node.as_group() {
                pending.push((path, group, depth.saturating_add(1)));
            }
        }
    }
    None
}

fn masks<'a>(reads: &mut Reads, layer: &'a Layer) -> Option<(String, &'a PropGroup)> {
    for (index, node) in layer.props.children.iter().enumerate() {
        if let Some(group) = node.as_group() {
            reads.field(layer.id, format!("props.children.{index}.meta.match"));
            if group.match_id == "masks" {
                return Some((format!("props.children.{index}"), group));
            }
        }
    }
    reads.window(layer.id.0, "props.children".to_owned(), 0, layer.props.children.len().saturating_add(1));
    None
}

fn target(reads: &mut Reads, layer: &Layer, masks_path: &str, masks: &PropGroup, uid: Uid, assigned: &str) {
    let Some((path, group)) = selected_group(masks, uid, masks_path) else { return };
    reads.field(layer.id, format!("{path}.uid"));
    reads.field(layer.id, format!("{path}.meta.kind.kind"));
    if matches!(group.kind, GroupKind::Mask { .. }) {
        reads.field(layer.id, format!("{path}.meta.kind.{assigned}"));
    }
}

/// Mirrors paths::masks_for / layer_menu::target_masks for the three scalar closures in
/// paths::edit_mask_groups. Shape-path targets are resolved but skipped by that editor.
pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Option<Under> {
    let assigned = match command {
        "path.convertToBezier" => "roto_bezier",
        "layer.mask.motionBlur" => "motion_blur",
        "layer.mask.featherFalloff" => "feather_falloff",
        _ => return None,
    };
    let mut reads = Reads::default();
    if params.get("mask").is_none() && params.get("layer").is_none() && !session.state.selected_vertices.is_empty() {
        let comp = session.project.comp(session.active_comp_id()?)?;
        let mut selected: Vec<_> = session.state.selected_vertices.iter().map(|vertex| (vertex.layer, vertex.mask)).collect();
        selected.sort();
        selected.dedup();
        for (id, uid) in selected {
            let layer = comp.layer(id)?;
            if let Some((path, masks)) = masks(&mut reads, layer) {
                target(&mut reads, layer, &path, masks, uid, assigned);
            }
        }
        return Some(reads.finish());
    }
    let (cid, comp) = composition(session, params)?;
    if let Some(uid) = params.get("mask").and_then(Value::as_u64) {
        // This is layer_p, which intentionally does not read the `layers` array.
        let id = match params.get("layer") {
            Some(value) => layer_address(&mut reads, cid, comp, value)?,
            None => *session.state.selected_layers.first().filter(|id| comp.layer(**id).is_some())?,
        };
        let layer = comp.layer(id)?;
        if let Some((path, group)) = group_path(&layer.props, uid) {
            if group.match_id == "path" {
                reads.field(id, format!("{path}.uid"));
                reads.field(id, format!("{path}.meta.match"));
                return Some(reads.finish());
            }
            let path = if let Some((masks_path, masks)) = masks(&mut reads, layer)
                && masks.groups().any(|group| group.uid == uid)
            {
                format!("{masks_path}.children[uid={uid}]")
            } else {
                path
            };
            reads.field(id, format!("{path}.uid"));
            reads.field(id, format!("{path}.meta.match"));
        }
    }
    let ids = layers(session, params, cid, comp, &mut reads)?;
    if let Some(key) = params.get("mask") {
        for id in ids {
            let layer = comp.layer(id)?;
            let Some((path, masks)) = masks(&mut reads, layer) else { continue };
            let selected = match key {
                Value::Number(number) => {
                    let number = number.as_u64().unwrap_or(1);
                    if let Some(node) = masks.children.iter().find(|node| node.uid() == number) {
                        Some(node.uid())
                    } else {
                        let index = usize::try_from(number.saturating_sub(1)).ok()?;
                        reads.window(id.0, format!("{path}.children"), index, index.saturating_add(1));
                        masks.children.get(index).map(|node| node.uid())
                    }
                }
                Value::String(name) => {
                    let index = masks.children.iter().position(|node| node.name() == name);
                    let count = index.map_or(masks.children.len(), |index| index.saturating_add(1));
                    for node in masks.children.iter().take(count) {
                        reads.field(id, format!("{path}.children[uid={}].meta.name", node.uid()));
                    }
                    reads.window(id.0, format!("{path}.children"), 0, count.saturating_add(usize::from(index.is_none())));
                    index.and_then(|index| masks.children.get(index)).map(|node| node.uid())
                }
                _ => None,
            };
            if let Some(uid) = selected {
                target(&mut reads, layer, &path, masks, uid, assigned);
            }
        }
        return Some(reads.finish());
    }
    let mut selected = Vec::new();
    for (id, property) in &session.state.selected_props {
        let Some(layer) = comp.layer(*id) else { continue };
        let Some((path, masks)) = masks(&mut reads, layer) else { continue };
        for group in masks.groups() {
            let group_path = format!("{path}.children[uid={}]", group.uid);
            let property_path = selected_property(group, *property, &group_path);
            if group.uid == *property || property_path.is_some() {
                selected.push((*id, group.uid));
                reads.field(*id, format!("{group_path}.uid"));
                if let Some(path) = property_path {
                    reads.field(*id, format!("{path}.uid"));
                }
            }
        }
    }
    if selected.is_empty() {
        for id in ids {
            let layer = comp.layer(id)?;
            if let Some((path, masks)) = masks(&mut reads, layer) {
                // Includes the end gap: appending a mask changes an all-mask assignment.
                reads.window(id.0, format!("{path}.children"), 0, masks.children.len().saturating_add(1));
                reads.answer.premises.push(Premise::on(json!(cid.0)).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
                reads.answer.content.push(ContentPremise { id: json!(cid.0), field: "~maskMembers".to_owned(), region: json!({"layer":id.0}) });
                selected.extend(masks.groups().map(|group| (id, group.uid)));
            }
        }
    }
    for (id, uid) in selected {
        let layer = comp.layer(id)?;
        if let Some((path, masks)) = masks(&mut reads, layer) {
            target(&mut reads, layer, &path, masks, uid, assigned);
        }
    }
    Some(reads.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kition_motion_project::Node;

    fn fixture() -> (Session, LayerId, LayerId, Uid, Uid) {
        let mut session = Session::default();
        session.execute("comp.new", json!({"name":"Masks","width":32,"height":32,"duration":2})).unwrap();
        let first = LayerId(session.execute("layer.newSolid", json!({"name":"First"})).unwrap()["layer"].as_u64().unwrap());
        let other = LayerId(session.execute("layer.newSolid", json!({"name":"Other"})).unwrap()["layer"].as_u64().unwrap());
        let mut made = Vec::new();
        for offset in [0, 10] {
            made.push(
                session.execute("mask.new", json!({"layer":first.0,"vertices":[[offset,0],[offset+5,0],[offset+5,5]],"closed":true})).unwrap()["mask"]
                    .as_u64()
                    .unwrap(),
            );
        }
        session.state.selected_layers = vec![first];
        session.state.selected_vertices.clear();
        session.state.selected_props.clear();
        (session, first, other, made[0], made[1])
    }

    fn fields(answer: &Under, layer: LayerId) -> &[String] {
        answer.premises.iter().find(|premise| premise.id == json!(layer.0)).unwrap().fields.as_ref().unwrap()
    }

    #[test]
    fn scalar_uid_index_name_and_all_masks_keep_distinct_selector_dependencies() {
        let (session, first, _, one, two) = fixture();
        let uid = under(&session, "layer.mask.motionBlur", &json!({"layer":first.0,"mask":one,"mode":"off"})).unwrap();
        assert!(uid.within.is_empty());
        assert_eq!(fields(&uid, first).len(), 5);
        assert!(fields(&uid, first).contains(&format!("props.children.0.children[uid={one}].meta.kind.motion_blur")));
        for key in [json!(1), json!("Mask 1")] {
            let indexed = under(&session, "layer.mask.motionBlur", &json!({"layer":first.0,"mask":key,"mode":"off"})).unwrap();
            assert_eq!(indexed.within.len(), 1);
            assert_eq!(indexed.within[0].field, "props.children.0.children");
            assert_eq!((indexed.within[0].from, indexed.within[0].to), (0, 1));
            assert!(fields(&indexed, first).contains(&format!("props.children.0.children[uid={one}].meta.kind.motion_blur")));
        }
        let all = under(&session, "layer.mask.motionBlur", &json!({"layer":first.0,"mode":"off"})).unwrap();
        assert_eq!((all.within[0].from, all.within[0].to), (0, 3));
        for uid in [one, two] {
            assert!(fields(&all, first).contains(&format!("props.children.0.children[uid={uid}].meta.kind.motion_blur")));
        }
        assert!(fields(&all, first).iter().all(|field| !field.contains("opacity") && field != "props.children.0.children"));
    }

    #[test]
    fn selected_vertices_override_a_layers_array_and_selected_properties() {
        let (mut session, first, other, one, two) = fixture();
        session.state.selected_vertices = vec![kition_motion_engine::VertexRef { layer: first, mask: two, index: 0 }];
        session.state.selected_props = vec![(first, one)];
        let got = under(&session, "path.convertToBezier", &json!({"layers":[other.0]})).unwrap();
        assert_eq!(got.premises.len(), 1);
        assert!(fields(&got, first).contains(&format!("props.children.0.children[uid={two}].meta.kind.roto_bezier")));
        assert!(!fields(&got, first).iter().any(|field| field.contains(&format!("uid={one}"))));
    }

    #[test]
    fn selected_property_masks_override_explicit_layers() {
        let (mut session, first, other, one, _) = fixture();
        let comp = session.active_comp_id().unwrap();
        let path = session
            .project
            .comp(comp)
            .unwrap()
            .layer(first)
            .unwrap()
            .masks()
            .unwrap()
            .groups()
            .find(|group| group.uid == one)
            .unwrap()
            .get("path")
            .unwrap()
            .uid;
        session.state.selected_props = vec![(first, path)];
        let got = under(&session, "layer.mask.featherFalloff", &json!({"layer":other.0,"mode":"linear"})).unwrap();
        assert!(fields(&got, other).is_empty());
        assert!(fields(&got, first).contains(&format!("props.children.0.children[uid={one}].meta.kind.feather_falloff")));
        assert!(fields(&got, first).iter().any(|field| field.contains(&format!("uid={path}"))));
    }

    #[test]
    fn numeric_shape_path_probe_is_resolved_but_not_assigned() {
        let (mut session, first, _, _, _) = fixture();
        let uid = session.project.next_id;
        let comp = session.active_comp_id().unwrap();
        let project = std::sync::Arc::make_mut(&mut session.project);
        project.next_id = uid.saturating_add(1);
        project.comp_mut(comp).unwrap().layer_mut(first).unwrap().props.children.push(Node::Group(PropGroup::new(uid, "path", "Shape Path")));
        let got = under(&session, "path.convertToBezier", &json!({"layer":first.0,"mask":uid})).unwrap();
        assert_eq!(fields(&got, first).len(), 2);
        assert!(fields(&got, first).iter().all(|field| field.ends_with(".uid") || field.ends_with(".meta.match")));
        assert_eq!(session.execute("path.convertToBezier", json!({"layer":first.0,"mask":uid})).unwrap()["masks"], 1);
    }
}
