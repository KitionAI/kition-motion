//! Property expressions and direct key edits, beyond generic target identity.
use std::collections::{BTreeMap, BTreeSet};

use kition_kit::{Covered, CoveredAsk, CoveredAt, CoveredContent, Covers, DerivedValues, Premise, Under, Within};
use kition_motion_engine::{Session, commands::resolve_layer, keyframe::Interp};
use kition_motion_project::props::{Seg, parse_path};
use kition_motion_project::{Comp, ItemId, Layer, Node, PropGroup, Property, Uid};
use kition_motion_time::Tick;
use serde_json::{Value, json};

#[derive(Default)]
struct Reads {
    answer: Under,
    needs: BTreeMap<u64, (BTreeSet<String>, BTreeSet<String>)>,
}

impl Reads {
    fn field(&mut self, id: u64, field: impl Into<String>) {
        self.needs.entry(id).or_default().0.insert(field.into());
    }

    fn window(&mut self, parent: u64, field: String, from: usize, to: usize) {
        self.answer.within.push(Within { parent: json!(parent), field, from, to, ..Default::default() });
    }

    fn finish(mut self) -> Under {
        self.answer.premises.extend(self.needs.into_iter().map(|(id, (fields, positions))| Premise::on(json!(id)).fields(fields).positions(positions)));
        self.answer
    }
}

fn composition<'a>(session: &'a Session, params: &Value, reads: &mut Reads) -> Option<(ItemId, &'a Comp)> {
    let id = match params.get("comp") {
        Some(Value::Number(number)) => ItemId(number.as_u64()?),
        Some(Value::String(text)) => match text.parse().ok().map(ItemId).filter(|id| session.project.comp(*id).is_some()) {
            Some(id) => id,
            None => {
                let mut found = None;
                for item in session.project.items.values() {
                    reads.field(item.id.0, "name");
                    if item.name == *text {
                        reads.field(item.id.0, "kind");
                        if item.as_comp().is_some() {
                            found = Some(item.id);
                            break;
                        }
                    }
                }
                reads.answer.within.push(Within {
                    parent: json!("project"),
                    field: "items".to_owned(),
                    from: 0,
                    to: session.project.items.len().saturating_add(1),
                    ..Default::default()
                });
                found?
            }
        },
        Some(_) => return None,
        None => session.active_comp_id()?,
    };
    Some((id, session.project.comp(id)?))
}

struct Address<'a> {
    cid: ItemId,
    comp: &'a Comp,
    layer: &'a Layer,
    property: &'a Property,
    path: String,
    keys: String,
    chain: Vec<(String, &'a Node)>,
}

fn locate(root: &PropGroup, uid: Uid) -> Option<(String, String, Vec<(String, &Node)>)> {
    let mut pending = vec![(String::new(), "props".to_owned(), root, Vec::new())];
    let mut visited = 0usize;
    while let Some((prefix, physical, group, chain)) = pending.pop() {
        if chain.len() >= 64 {
            return None;
        }
        let mut counts = BTreeMap::<&str, usize>::new();
        for (index, node) in group.children.iter().enumerate() {
            visited = visited.saturating_add(1);
            if visited > 65_536 {
                return None;
            }
            let occurrence = counts.entry(node.match_id()).or_default();
            *occurrence = occurrence.saturating_add(1);
            let segment = if *occurrence == 1 { node.match_id().to_owned() } else { format!("{}#{occurrence}", node.match_id()) };
            let path = if prefix.is_empty() { segment } else { format!("{prefix}/{segment}") };
            let physical = format!("{physical}.children.{index}");
            let mut next = chain.clone();
            next.push((path.clone(), node));
            if node.uid() == uid {
                return Some((path, format!("{physical}.keys"), next));
            }
            if let Some(group) = node.as_group() {
                pending.push((path, physical, group, next));
            }
        }
    }
    None
}

fn named_address<'a>(reads: &mut Reads, layer: &'a Layer, path: &str) -> Option<&'a Property> {
    if path.split(['/', '.']).count() > 64 {
        return None;
    }
    let segments = parse_path(path);
    let mut group = &layer.props;
    let mut physical = "props".to_owned();
    for (depth, segment) in segments.iter().enumerate() {
        let selected = match segment {
            Seg::Uid(uid) => group.children.iter().enumerate().find(|(_, node)| node.uid() == *uid),
            Seg::Index(index) => {
                let index = index.checked_sub(1)?;
                reads.window(layer.id.0, format!("{physical}.children"), index, index.saturating_add(1));
                group.children.get(index).map(|node| (index, node))
            }
            Seg::Match(name, occurrence) => {
                let mut remaining = occurrence.saturating_sub(1);
                let mut selected = None;
                for (index, node) in group.children.iter().enumerate() {
                    let prefix = format!("{physical}.children.{index}");
                    reads.field(layer.id.0, format!("{prefix}.meta.match"));
                    if node.match_id() != name {
                        reads.field(layer.id.0, format!("{prefix}.meta.name"));
                    }
                    if node.match_id() == name || node.name().eq_ignore_ascii_case(name) {
                        if remaining == 0 {
                            selected = Some((index, node));
                            break;
                        }
                        remaining = remaining.saturating_sub(1);
                    }
                }
                let count = selected.map_or(group.children.len().saturating_add(1), |(index, _)| index.saturating_add(1));
                reads.window(layer.id.0, format!("{physical}.children"), 0, count);
                selected
            }
        };
        let (index, node) = selected?;
        if depth.saturating_add(1) == segments.len() {
            return node.as_prop();
        }
        // Membership windows protect these indexes. Selector replay treats intermediate
        // list summary fields as opaque, including unrelated key insertions under a child.
        physical = format!("{physical}.children.{index}");
        group = node.as_group()?;
    }
    None
}

fn address<'a>(session: &'a Session, params: &Value, reads: &mut Reads) -> Option<Address<'a>> {
    let (cid, comp) = composition(session, params, reads)?;
    if params.get("comp").is_some() {
        reads.needs.entry(cid.0).or_default();
    }
    let id = match params.get("layer") {
        Some(value) => resolve_layer(comp, value)?,
        None => *session.state.selected_layers.first().filter(|id| comp.layer(**id).is_some())?,
    };
    let layer = comp.layer(id)?;
    reads.needs.entry(id.0).or_default();
    if let Some(value) = params.get("layer") {
        let index = crate::place::index_of(value).or_else(|| {
            let number = value.as_u64()?;
            (!comp.layers.iter().any(|layer| layer.id.0 == number)).then(|| usize::try_from(number).ok()?.checked_sub(1)).flatten()
        });
        if let Some(index) = index {
            reads.answer.objects.push(json!(id.0));
            reads.window(cid.0, "comp.layers".to_owned(), index, index.saturating_add(1));
        } else if value.as_str().is_some_and(|text| text.parse::<u64>().is_err()) {
            let count = comp.layers.iter().position(|layer| layer.id == id)?.saturating_add(1);
            for candidate in comp.layers.iter().take(count) {
                reads.field(candidate.id.0, "name");
            }
            reads.window(cid.0, "comp.layers".to_owned(), 0, count);
        }
    }
    let path = params.get("path").or_else(|| params.get("prop")).and_then(Value::as_str);
    let uid = params.get("prop").and_then(Value::as_u64).or_else(|| path?.trim().strip_prefix('@')?.parse().ok());
    let property = if let Some(uid) = uid {
        let (_, _, chain) = locate(&layer.props, uid)?;
        chain.last()?.1.as_prop()?
    } else {
        let path = path?;
        named_address(reads, layer, path).or_else(|| (!path.contains('/')).then(|| named_address(reads, layer, &format!("transform/{path}"))).flatten())?
    };
    let (path, keys, chain) = locate(&layer.props, property.uid)?;
    Some(Address { cid, comp, layer, property, path, keys, chain })
}

fn reference(reads: &mut Reads, target: &Address<'_>, same_layer: bool, compact: bool) {
    if !same_layer {
        reads.field(target.layer.id.0, "name");
    }
    let mut pending = false;
    let mut in_effect = false;
    for (path, node) in &target.chain {
        reads.field(target.layer.id.0, format!("{path}.meta.match"));
        if !compact {
            continue;
        }
        if pending {
            reads.field(target.layer.id.0, format!("{path}.meta.name"));
            pending = false;
            continue;
        }
        match (node.as_group().is_some(), node.match_id()) {
            (true, "masks" | "effects" | "contents") => {
                pending = true;
                in_effect |= node.match_id() == "effects";
            }
            (true, "transform" | "text") if !in_effect => {}
            (false, "sourceText") => {}
            _ => reads.field(target.layer.id.0, format!("{path}.meta.name")),
        }
    }
}

fn dimensions(reads: &mut Reads, target: &Address<'_>) {
    reads.field(target.layer.id.0, format!("{}.value", target.path));
    if matches!(target.property.value, kition_motion_engine::keyframe::Value::Vec3(_)) {
        reads.field(target.layer.id.0, format!("{}.meta.shown_dims", target.path));
        if target.property.shown_dims == 2 {
            reads.field(target.layer.id.0, "attrs.source");
            reads.field(target.layer.id.0, "switches.three_d");
        }
    }
}

fn nearest(property: &Property, time: Tick) -> Option<usize> {
    property
        .keys
        .iter()
        .position(|key| key.time == time)
        .or_else(|| property.keys.iter().enumerate().min_by_key(|(_, key)| key.time.0.abs_diff(time.0)).map(|(i, _)| i))
}

fn key_times(layer: &Layer) -> Value {
    let mut result = serde_json::Map::new();
    let mut pending = vec![(&layer.props, Vec::<usize>::new())];
    let mut visited = 0usize;
    while let Some((group, prefix)) = pending.pop() {
        if prefix.len() >= 64 {
            return Value::Null;
        }
        for (index, node) in group.children.iter().enumerate() {
            visited = visited.saturating_add(1);
            if visited > 65_536 {
                return Value::Null;
            }
            let mut path = prefix.clone();
            path.push(index);
            if let Some(group) = node.as_group() {
                pending.push((group, path));
            } else if let Some(property) = node.as_prop()
                && !property.keys.is_empty()
            {
                visited = visited.saturating_add(property.keys.len());
                if visited > 65_536 || result.contains_key(&property.uid.to_string()) {
                    return Value::Null;
                }
                let times: Vec<_> = property.keys.iter().map(|key| key.time.0.to_string()).collect();
                let roving: Vec<_> = property.keys.iter().filter(|key| key.roving).map(|key| key.time.0.to_string()).collect();
                result.insert(property.uid.to_string(), json!([path, times, roving]));
            }
        }
    }
    Value::Object(result)
}

pub(crate) fn derived(session: &Session, ids: &[Value], names: &[String]) -> Vec<DerivedValues> {
    if !names.is_empty() && !names.iter().any(|name| name == "keyTimes") {
        return Vec::new();
    }
    session
        .project
        .items
        .values()
        .filter_map(|item| item.as_comp())
        .flat_map(|comp| &comp.layers)
        .filter(|layer| ids.is_empty() || ids.iter().any(|id| numeric_id(id) == Some(layer.id.0)))
        .map(|layer| DerivedValues { id: json!(layer.id.0), values: serde_json::Map::from_iter([("keyTimes".to_owned(), key_times(layer))]) })
        .collect()
}

fn numeric_id(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn stable_layer_address(params: &Value, target: &Address<'_>) -> bool {
    params.get("layer").is_none_or(|layer| numeric_id(layer) == Some(target.layer.id.0))
        && params.get("comp").is_none_or(|comp| numeric_id(comp) == Some(target.cid.0))
}

fn nearest_time(snapshot: &Value, property: Uid, time: Tick) -> Option<i64> {
    snapshot.get(property.to_string())?.get(1)?.as_array()?.iter().filter_map(|time| time.as_str()?.parse::<i64>().ok()).min_by_key(|key| key.abs_diff(time.0))
}

fn sampled_times(snapshot: &Value, property: Uid, time: Tick) -> Option<Vec<i64>> {
    let times = snapshot.get(property.to_string())?.get(1)?.as_array()?;
    let times: Option<Vec<i64>> = times.iter().map(|value| value.as_str()?.parse().ok()).collect();
    let times = times?;
    if times.contains(&time.0) {
        return Some(vec![time.0]);
    }
    let left = times.iter().copied().filter(|key| *key <= time.0).max();
    let right = times.iter().copied().filter(|key| *key >= time.0).min();
    Some(left.into_iter().chain(right).collect())
}

pub(crate) fn covered(session: &Session, command: &str, params: &Value, objects: &[CoveredAsk]) -> Option<CoveredAt> {
    let sampled = matches!(command, "prop.set" | "prop.addKey");
    if !sampled && !matches!(command, "keys.set" | "keys.setEase") {
        return None;
    }
    let target = address(session, params, &mut Reads::default())?;
    if !stable_layer_address(params, &target) {
        return None;
    }
    let time = if command == "keys.setEase" {
        Tick::from_seconds_f64(params.get("time")?.as_f64().filter(|time| time.is_finite())?)
    } else {
        kition_motion_engine::commands::written_key_time(session, params, command)?
    };
    let current = key_times(target.layer);
    let property = target.property.uid.to_string();
    let mut answer = CoveredAt { under: vec![json!(target.layer.id.0)], ..Default::default() };
    for object in objects {
        let mut content = None;
        let covered = if numeric_id(&object.id) != Some(target.layer.id.0) {
            Some(false)
        } else if object.created {
            answer.under.clear();
            Some(false)
        } else if object.deleted || object.unknown.iter().any(|field| field == "~keyTimes") {
            None
        } else {
            let before = object.before.get("~keyTimes").unwrap_or(&current);
            let same_locator = uid_address(params) || before.get(&property)?.get(0) == current.get(&property)?.get(0);
            let same_roving = command != "keys.set" || before.get(&property)?.get(2) == current.get(&property)?.get(2);
            let same = if sampled {
                sampled_times(before, target.property.uid, time).zip(sampled_times(&current, target.property.uid, time)).map(|(before, now)| before == now)
            } else {
                nearest_time(before, target.property.uid, time).zip(nearest_time(&current, target.property.uid, time)).map(|(before, now)| before == now)
            };
            // The layer remains the recipient. A redirected nested key changes the consumed
            // content, rather than moving the layer to a different place.
            content = Some(match same {
                Some(true) if same_locator && same_roving => CoveredContent::Same,
                Some(_) => CoveredContent::Changed,
                None => CoveredContent::Unknown,
            });
            Some(true)
        };
        answer.covered.push(Covered { id: object.id.clone(), covered, content, ..Default::default() });
    }
    Some(answer)
}

fn nearest_membership(reads: &mut Reads, target: &Address<'_>, params: &Value, time: Tick) {
    // The requested point itself stays held even if deletion makes the resolver choose a
    // different nearest key. An exact stored key wins independently of every other key.
    reads.field(target.layer.id.0, format!("{}.keys[time={}].time", target.path, time.0));
    if target.property.keys.iter().any(|key| key.time == time) {
        return;
    }
    if stable_layer_address(params, target) && key_times(target.layer).is_object() {
        reads.answer.objects.push(json!(target.layer.id.0));
        reads.answer.covers = Some(Covers { fields: vec!["~keyTimes".to_owned()] });
    } else {
        // Retain the list windows for a layer/comp named by name or index. A tree beyond
        // the bounded snapshot also falls back to its property's full key membership.
        reads.field(target.layer.id.0, format!("{}.keys", target.path));
    }
}

fn uid_address(params: &Value) -> bool {
    params.get("prop").and_then(Value::as_u64).is_some()
        || params
            .get("path")
            .or_else(|| params.get("prop"))
            .and_then(Value::as_str)
            .and_then(|path| path.trim().strip_prefix('@'))
            .is_some_and(|uid| uid.parse::<u64>().is_ok())
}

/// Adds address membership without replacing the named-path guard's whole edited-key policy.
pub(crate) fn supplemental(session: &Session, command: &str, params: &Value) -> Option<Under> {
    if matches!(command, "prop.set" | "prop.addKey") {
        let target = address(session, params, &mut Reads::default())?;
        if command == "prop.set" && target.property.keys.is_empty() {
            return None;
        }
        let mut reads = Reads::default();
        if params.get("time").is_some_and(|time| !time.is_null()) {
            reads.field(target.cid.0, "comp.attrs.frame_rate");
        }
        if !uid_address(params)
            && params.get("selected").and_then(Value::as_bool) != Some(true)
            && !target.property.keys.is_empty()
            && (command != "prop.addKey" || params.get("value").is_none_or(Value::is_null))
        {
            let time = kition_motion_engine::commands::written_key_time(session, params, command)?;
            nearest_membership(&mut reads, &target, params, time);
        }
        if reads.needs.is_empty() && reads.answer.covers.is_none() {
            return None;
        }
        return Some(reads.finish());
    }
    if command != "keys.set" || uid_address(params) {
        return None;
    }
    let mut reads = Reads::default();
    let target = address(session, params, &mut reads)?;
    let time = kition_motion_engine::commands::written_key_time(session, params, command)?;
    nearest_membership(&mut reads, &target, params, time);
    if params.get("newTime").is_none() {
        return Some(reads.finish());
    }
    let index = nearest(target.property, time)?;
    let mut destination_params = params.clone();
    destination_params["time"] = params.get("newTime")?.clone();
    let destination = kition_motion_engine::commands::written_key_time(session, &destination_params, "prop.addKey")?;
    reads.field(target.cid.0, "comp.attrs.frame_rate");
    for (neighbor, lower) in [(index.checked_sub(1), true), (index.checked_add(1), false)] {
        let Some(neighbor) = neighbor.and_then(|index| target.property.keys.get(index)) else { continue };
        let clamps = if lower { neighbor.time.0.saturating_add(1) > destination.0 } else { neighbor.time.0.saturating_sub(1) < destination.0 };
        if clamps {
            reads.field(target.layer.id.0, format!("{}.keys[time={}].time", target.path, neighbor.time.0));
        }
    }
    Some(reads.finish())
}

fn set_key(reads: &mut Reads, target: &Address<'_>, session: &Session, params: &Value) -> Option<()> {
    let time = kition_motion_engine::commands::written_key_time(session, params, "keys.set")?;
    let index = nearest(target.property, time)?;
    let key = target.property.keys.get(index)?;
    let prefix = format!("{}.keys[time={}]", target.path, key.time.0);
    let id = target.layer.id.0;
    reads.field(id, "attrs.stretch");
    // In/Out affect only key_timing's reply warning, not the saved key edit (time5 policy).
    reads.needs.entry(id).or_default().1.insert("startTime".to_owned());
    reads.field(id, format!("{prefix}.time"));
    if params.get("value").is_some() {
        reads.field(id, format!("{prefix}.value"));
    }
    if params.get("newTime").is_some() {
        reads.field(target.cid.0, "comp.attrs.frame_rate");
        for neighbor in [index.checked_sub(1), index.checked_add(1)].into_iter().flatten().filter_map(|index| target.property.keys.get(index)) {
            reads.field(id, format!("{}.keys[time={}].time", target.path, neighbor.time.0));
        }
    }
    if target.property.keys.len() >= 3 && target.property.keys.iter().skip(1).take(target.property.keys.len().saturating_sub(2)).any(|key| key.roving) {
        reads.field(id, format!("{}.keys", target.path));
        reads.field(id, format!("{}.meta.spatial", target.path));
    }
    for key in target.property.keys.first().into_iter().chain(target.property.keys.last()) {
        reads.field(id, format!("{}.keys[time={}].data.roving", target.path, key.time.0));
    }
    nearest_membership(reads, target, params, time);
    Some(())
}

fn set_ease(reads: &mut Reads, target: &Address<'_>, params: &Value) -> Option<()> {
    let time = Tick::from_seconds_f64(params.get("time")?.as_f64().filter(|time| time.is_finite())?);
    let index = nearest(target.property, time)?;
    let key = target.property.keys.get(index)?;
    let out = match params.get("side")?.as_str()? {
        "out" => true,
        "in" => false,
        _ => return None,
    };
    let side = if out { "out" } else { "in" };
    let interp = if out { key.out_interp } else { key.in_interp };
    let id = target.layer.id.0;
    let prefix = format!("{}.keys[time={}]", target.path, key.time.0);
    nearest_membership(reads, target, params, time);
    reads.field(id, format!("{}.meta.spatial", target.path));
    let stored = if out { &key.out_ease } else { &key.in_ease };
    let stored_ready = !target.property.spatial && !key.continuous && stored.len() == key.value.dims().max(1);
    // Spatial/auto/continuous easing can consume both adjacent curves and stored handles.
    // Keep that full curve until a narrower declaration covers those branches independently.
    if !stored_ready && (target.property.spatial || key.continuous || interp != Interp::Linear) {
        reads.field(id, format!("{}.keys", target.path));
        return Some(());
    }
    for field in ["time", "value", "data.auto_bezier", "data.continuous"] {
        reads.field(id, format!("{prefix}.{field}"));
    }
    reads.field(id, format!("{prefix}.data.{side}_interp"));
    reads.field(id, format!("{prefix}.data.{side}_ease"));
    if stored_ready {
        return Some(());
    }
    let neighbor = if out { index.checked_add(1) } else { index.checked_sub(1) };
    if let Some((index, neighbor)) = neighbor.and_then(|index| target.property.keys.get(index).map(|key| (index, key))) {
        let neighbor_prefix = format!("{}.keys[time={}]", target.path, neighbor.time.0);
        reads.field(id, format!("{neighbor_prefix}.time"));
        reads.field(id, format!("{neighbor_prefix}.value"));
        if !out {
            reads.field(id, format!("{neighbor_prefix}.data.out_interp"));
        }
        reads.window(id, target.keys.clone(), index.min(nearest(target.property, time)?), index.max(nearest(target.property, time)?).saturating_add(1));
    }
    Some(())
}

pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Option<Under> {
    if !matches!(command, "prop.setExpression" | "prop.pickWhip" | "keys.set" | "keys.setEase") {
        return None;
    }
    if command == "keys.set" && !uid_address(params) {
        return None;
    }
    let mut reads = Reads::default();
    let target = address(session, params, &mut reads)?;
    match command {
        "keys.set" => set_key(&mut reads, &target, session, params)?,
        "keys.setEase" => set_ease(&mut reads, &target, params)?,
        "prop.setExpression" => {
            reads.field(target.layer.id.0, format!("{}.expr", target.path));
            if params.get("expression").and_then(Value::as_str).is_none()
                && target.property.expr.is_none()
                && params.get("enabled").and_then(Value::as_bool) != Some(false)
            {
                reference(&mut reads, &target, true, true);
            }
        }
        "prop.pickWhip" => {
            let linked = address(session, params.get("target")?, &mut reads)?;
            // link::pick_whip resolves target.comp, but looks the resulting layer id up in
            // the source comp. Cross-comp references currently fail before the expression write.
            if target.cid != linked.cid || target.comp.layer(linked.layer.id).is_none() {
                return None;
            }
            reads.field(target.layer.id.0, format!("{}.expr", target.path));
            reference(&mut reads, &linked, target.layer.id == linked.layer.id, session.prefs.general.expression_pick_whip_compact);
            dimensions(&mut reads, &target);
            dimensions(&mut reads, &linked);
        }
        _ => return None,
    }
    Some(reads.finish())
}
