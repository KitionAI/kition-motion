//! Tracker point defaults copied from source dimensions.
use std::collections::{BTreeMap, BTreeSet};

use kition_kit::{Premise, Under, Within};
use kition_motion_engine::{Session, commands::resolve_layer};
use kition_motion_project::{Comp, ItemId, ItemKind, Layer, LayerId, tracking::TrackKind};
use serde_json::{Value, json};

#[derive(Default)]
struct Reads {
    answer: Under,
    fields: BTreeMap<u64, BTreeSet<String>>,
}

impl Reads {
    fn field(&mut self, id: u64, field: impl Into<String>) {
        self.fields.entry(id).or_default().insert(field.into());
    }

    fn window(&mut self, parent: u64, field: impl Into<String>, from: usize, to: usize) {
        self.answer.within.push(Within { parent: json!(parent), field: field.into(), from, to, ..Default::default() });
    }

    fn finish(mut self) -> Under {
        for (id, fields) in self.fields {
            self.answer.premises.push(Premise { id: json!(id), fields: Some(fields.into_iter().collect()), positions: Some(Vec::new()), ..Default::default() });
        }
        self.answer
    }
}

fn composition<'a>(session: &'a Session, params: &Value, reads: &mut Reads) -> Option<(ItemId, &'a Comp)> {
    let id = match params.get("comp") {
        Some(Value::Number(number)) => Some(ItemId(number.as_u64()?)),
        Some(Value::String(name)) => name
            .parse::<u64>()
            .ok()
            .map(ItemId)
            .filter(|id| session.project.comp(*id).is_some())
            .or_else(|| session.project.items.values().find(|item| item.name == *name && item.as_comp().is_some()).map(|item| item.id)),
        _ => session.active_comp_id(),
    }?;
    if params.get("comp").is_some() {
        reads.fields.entry(id.0).or_default();
    }
    Some((id, session.project.comp(id)?))
}

fn address(reads: &mut Reads, cid: ItemId, comp: &Comp, value: &Value) -> Option<LayerId> {
    let id = resolve_layer(comp, value)?;
    reads.fields.entry(id.0).or_default();
    let index = crate::place::index_of(value).or_else(|| {
        let number = value.as_u64()?;
        (!comp.layers.iter().any(|layer| layer.id.0 == number)).then(|| usize::try_from(number).ok()?.checked_sub(1)).flatten()
    });
    if let Some(index) = index {
        reads.answer.objects.push(json!(id.0));
        reads.window(cid.0, "comp.layers", index, index.saturating_add(1));
    } else if value.as_str().is_some_and(|name| name.parse::<u64>().is_err()) {
        let index = comp.layers.iter().position(|layer| layer.id == id)?;
        for layer in comp.layers.iter().take(index.saturating_add(1)) {
            reads.field(layer.id.0, "name");
        }
        reads.window(cid.0, "comp.layers", 0, index.saturating_add(1));
    }
    Some(id)
}

fn source_size(session: &Session, reads: &mut Reads, cid: ItemId, layer: &Layer) {
    reads.field(layer.id.0, "attrs.source");
    reads.field(layer.id.0, "sourceItem");
    if let Some(id) = layer.source.item() {
        reads.field(id.0, "kind");
        if let Some(item) = session.project.item(id) {
            match &item.kind {
                ItemKind::Comp(comp) => {
                    reads.field(id.0, "comp.attrs.width");
                    if comp.width != 0 {
                        reads.field(id.0, "comp.attrs.height");
                    }
                }
                ItemKind::Footage(footage) => {
                    reads.field(id.0, "data.has_video");
                    if footage.has_video {
                        reads.field(id.0, "data.width");
                        if footage.width != 0 {
                            reads.field(id.0, "data.height");
                        }
                    }
                }
                ItemKind::Solid(solid) => {
                    reads.field(id.0, "data.width");
                    if solid.width != 0 {
                        reads.field(id.0, "data.height");
                    }
                }
                ItemKind::Folder => {}
            }
        }
    }
    if kition_motion_engine::render::source_size(&session.project, layer).0 == 0 {
        reads.field(cid.0, "comp.attrs.width");
        reads.field(cid.0, "comp.attrs.height");
    }
}

fn create(session: &Session, command: &str, params: &Value) -> Option<Under> {
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    let id = match params.get("layer") {
        Some(value) => address(&mut reads, cid, comp, value)?,
        None => *session.state.selected_layers.first()?,
    };
    let layer = comp.layer(id)?;
    source_size(session, &mut reads, cid, layer);
    reads.field(id.0, "attrs.switches.video");
    if let Some(root) = layer.motion_trackers() {
        let index = layer.props.children.iter().position(|node| node.uid() == root.uid)?;
        let prefix = format!("props.children.{index}");
        for field in ["uid", "node", "meta.match"] {
            reads.field(id.0, format!("{prefix}.{field}"));
        }
        reads.window(id.0, "props.children", index, index.saturating_add(1));
        for (index, _) in root.children.iter().enumerate() {
            let member = format!("{prefix}.children.{index}");
            reads.field(id.0, format!("{member}.node"));
            reads.field(id.0, format!("{member}.meta.kind.kind"));
        }
        // The generated name counts existing tracker groups; appending names the end gap.
        reads.window(id.0, format!("{prefix}.children"), 0, root.children.len().saturating_add(1));
    } else {
        for (index, _) in layer.props.children.iter().enumerate() {
            reads.field(id.0, format!("props.children.{index}.meta.match"));
            reads.field(id.0, format!("props.children.{index}.meta.name"));
        }
        reads.window(id.0, "props.children", 0, layer.props.children.len().saturating_add(1));
    }
    if let Some(value) = params.get("target").filter(|value| !value.is_null()) {
        address(&mut reads, cid, comp, value)?;
    } else if params.get("target").is_none()
        && command != "track.stabilize"
        && (command != "track.new" || params.get("kind").and_then(Value::as_str).and_then(TrackKind::from_name) != Some(TrackKind::Stabilize))
    {
        let index = comp.layers.iter().position(|layer| layer.id == id)?;
        reads.window(cid.0, "comp.layers", index.saturating_sub(1), index.saturating_add(1));
        if let Some(above) = index.checked_sub(1).and_then(|index| comp.layers.get(index)) {
            reads.fields.entry(above.id.0).or_default();
        }
    }
    Some(reads.finish())
}

fn options(session: &Session, params: &Value) -> Option<Under> {
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    let id = match params.get("layer") {
        Some(value) => address(&mut reads, cid, comp, value)?,
        None => session
            .state
            .current_track
            .filter(|(id, _)| comp.layer(*id).is_some())
            .map(|(id, _)| id)
            .or_else(|| session.state.selected_layers.iter().copied().find(|id| comp.layer(*id).is_some_and(|layer| layer.trackers().next().is_some())))?,
    };
    let layer = comp.layer(id)?;
    let tracker = match params.get("tracker") {
        Some(Value::Number(number)) => {
            let number = number.as_u64()?;
            layer
                .tracker(number)
                .or_else(|| usize::try_from(number).ok().and_then(|index| layer.trackers().nth(index.saturating_sub(1))))
                .map(|(group, _)| group)
        }
        Some(Value::String(name)) => match name.strip_prefix('#').and_then(|index| index.parse::<usize>().ok()) {
            Some(index) => layer.trackers().nth(index.saturating_sub(1)).map(|(group, _)| group),
            None => layer.trackers().find(|(group, _)| group.name == *name).map(|(group, _)| group),
        },
        _ => session
            .state
            .current_track
            .filter(|(owner, _)| *owner == id)
            .and_then(|(_, uid)| layer.tracker(uid))
            .or_else(|| layer.trackers().next())
            .map(|(group, _)| group),
    }?;
    let root = layer.motion_trackers()?;
    let root_index = layer.props.children.iter().position(|node| node.uid() == root.uid)?;
    let prefix = format!("props.children.{root_index}");
    reads.field(id.0, format!("{prefix}.uid"));
    reads.field(id.0, format!("{prefix}.node"));
    reads.field(id.0, format!("{prefix}.meta.match"));
    reads.window(id.0, "props.children", root_index, root_index.saturating_add(1));
    let explicit_uid = params.get("tracker").and_then(Value::as_u64).is_some_and(|uid| layer.tracker(uid).is_some());
    let current_uid = params.get("tracker").is_none_or(|value| !value.is_number() && !value.is_string())
        && session.state.current_track.is_some_and(|(owner, uid)| owner == id && uid == tracker.uid);
    if !explicit_uid && !current_uid {
        // Track indexes count only tracker groups; names choose the first matching tracker.
        for (index, node) in root.children.iter().enumerate() {
            let member = format!("{prefix}.children.{index}");
            reads.field(id.0, format!("{member}.node"));
            reads.field(id.0, format!("{member}.meta.kind.kind"));
            if params.get("tracker").and_then(Value::as_str).is_some_and(|name| !name.starts_with('#')) {
                reads.field(id.0, format!("{member}.meta.name"));
            }
            if node.uid() == tracker.uid {
                break;
            }
        }
        let index = root.children.iter().position(|node| node.uid() == tracker.uid)?;
        reads.window(id.0, format!("{prefix}.children"), 0, index.saturating_add(1));
    }
    let tracker_index = root.children.iter().position(|node| node.uid() == tracker.uid)?;
    let member = format!("{prefix}.children.{tracker_index}");
    reads.field(id.0, format!("{member}.uid"));
    reads.window(id.0, format!("{prefix}.children"), tracker_index, tracker_index.saturating_add(1));
    reads.field(id.0, format!("{member}.node"));
    reads.field(id.0, format!("{member}.meta.kind.kind"));
    if params.get("name").and_then(Value::as_str).is_some() {
        reads.field(id.0, format!("{member}.meta.name"));
    }
    for (parameter, numeric) in [
        ("channel", None),
        ("action", None),
        ("blur", Some(true)),
        ("threshold", Some(true)),
        ("enhance", Some(false)),
        ("subpixel", Some(false)),
        ("adaptEveryFrame", Some(false)),
        ("trackShape", Some(false)),
    ] {
        let assigned = params.get(parameter).is_some_and(|value| match numeric {
            Some(true) => value.as_f64().is_some(),
            Some(false) => value.as_bool().is_some(),
            None => value.as_str().is_some(),
        });
        if assigned {
            reads.field(id.0, format!("{member}.meta.kind.settings.options.{parameter}"));
        }
    }
    Some(reads.finish())
}

pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Option<Under> {
    match command {
        "track.motion" | "track.stabilize" | "track.new" => create(session, command, params),
        "track.options" => options(session, params),
        _ => None,
    }
}
