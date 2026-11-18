//! Source-dependent layer writes, including defaults copied from project items.
use std::collections::{BTreeMap, BTreeSet};

use kition_kit::{Premise, Under, Within};
use kition_motion_engine::{Session, commands::resolve_layer, effects, keyframe::Value as KV};
use kition_motion_project::{Comp, FootageKind, ItemId, ItemKind, Layer, LayerId, LayerSource, ParamUi};
use serde_json::{Value, json};

#[derive(Default)]
struct Reads {
    answer: Under,
    fields: BTreeMap<u64, BTreeSet<String>>,
    positions: BTreeMap<u64, BTreeSet<String>>,
    all_positions: BTreeSet<u64>,
    identities: BTreeMap<u64, BTreeMap<String, String>>,
}

impl Reads {
    fn field(&mut self, id: u64, field: impl Into<String>) {
        self.fields.entry(id).or_default().insert(field.into());
    }

    fn timing(&mut self, id: u64) {
        self.field(id, "attrs.stretch");
        self.positions.entry(id).or_default().extend(["startTime", "inPoint", "outPoint"].map(str::to_owned));
    }

    fn window(&mut self, parent: impl Into<Value>, field: String, from: usize, to: usize) {
        self.answer.within.push(Within { parent: parent.into(), field, from, to, ..Default::default() });
    }

    fn identity(&mut self, id: u64, field: impl Into<String>) {
        self.identities.entry(id).or_default().insert(field.into(), "uid".to_owned());
    }

    fn finish(mut self) -> Under {
        for (id, fields) in self.fields {
            self.answer.premises.push(Premise {
                id: json!(id),
                fields: Some(fields.into_iter().collect()),
                positions: if self.all_positions.contains(&id) { None } else { Some(self.positions.remove(&id).unwrap_or_default().into_iter().collect()) },
                identities: self.identities.remove(&id).unwrap_or_default(),
            });
        }
        self.answer
    }
}

fn composition<'a>(session: &'a Session, params: &Value, reads: &mut Reads) -> Option<(ItemId, &'a Comp)> {
    let id = match params.get("comp") {
        Some(Value::Number(number)) => number.as_u64().map(ItemId),
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

fn layer_address(reads: &mut Reads, cid: ItemId, comp: &Comp, value: &Value) -> Option<LayerId> {
    let id = resolve_layer(comp, value)?;
    reads.fields.entry(id.0).or_default();
    let index = crate::place::index_of(value).or_else(|| {
        let number = value.as_u64()?;
        (!comp.layers.iter().any(|layer| layer.id.0 == number)).then(|| usize::try_from(number).ok()?.checked_sub(1)).flatten()
    });
    if let Some(index) = index {
        reads.answer.objects.push(json!(id.0));
        reads.window(cid.0, "comp.layers".to_owned(), index, index.saturating_add(1));
    } else if value.as_str().is_some_and(|name| name.parse::<u64>().is_err()) {
        let index = comp.layers.iter().position(|layer| layer.id == id)?;
        for layer in comp.layers.iter().take(index.saturating_add(1)) {
            reads.field(layer.id.0, "name");
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
    let ids: Vec<_> = session.state.selected_layers.iter().copied().filter(|id| comp.layer(*id).is_some()).collect();
    for id in &ids {
        reads.fields.entry(id.0).or_default();
    }
    Some(ids)
}

fn source_dimensions(session: &Session, reads: &mut Reads, cid: ItemId, layer: &Layer) {
    reads.field(layer.id.0, "attrs.source");
    reads.field(layer.id.0, "sourceItem");
    let mut fallback = !matches!(layer.source, LayerSource::Null);
    if let Some(id) = layer.source.item() {
        reads.field(id.0, "kind");
        if let Some(item) = session.project.item(id) {
            match &item.kind {
                ItemKind::Comp(_) => {
                    reads.field(id.0, "comp.attrs.width");
                    reads.field(id.0, "comp.attrs.height");
                }
                ItemKind::Footage(footage) => {
                    reads.field(id.0, "data.has_video");
                    if footage.has_video {
                        reads.field(id.0, "data.width");
                        reads.field(id.0, "data.height");
                    }
                }
                ItemKind::Solid(_) => {
                    reads.field(id.0, "data.width");
                    reads.field(id.0, "data.height");
                }
                ItemKind::Folder => {}
            }
            fallback = item.dimensions().is_none_or(|(width, _)| width == 0);
        }
    }
    if fallback {
        reads.field(cid.0, "comp.attrs.width");
        reads.field(cid.0, "comp.attrs.height");
    }
}

fn effect_apply(session: &Session, params: &Value) -> Option<Under> {
    let spec = effects::lookup(params.get("effect")?.as_str()?)?;
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    let ids = layers(session, params, cid, comp, &mut reads)?;
    // default_value scales point defaults by the source size; explicit exact-id values replace
    // those defaults. Other parameter aliases remain conservatively dependent on source size.
    let defaults_use_size = spec.params.iter().any(|parameter| {
        matches!((&parameter.ui, &parameter.default), (ParamUi::Point, KV::Vec2(point)) if point.iter().any(|value| *value != 0.0))
            && params.get("params").and_then(|params| params.get(parameter.id)).is_none()
    });
    for id in ids {
        let layer = comp.layer(id)?;
        if defaults_use_size {
            source_dimensions(session, &mut reads, cid, layer);
        }
        let effects = layer.props.sub("effects")?;
        let path = format!("props.children[uid={}]", effects.uid);
        reads.identity(id.0, "props.children");
        reads.identity(id.0, format!("{path}.children"));
        reads.field(id.0, format!("{path}.uid"));
        reads.field(id.0, format!("{path}.node"));
        reads.field(id.0, format!("{path}.meta.match"));
        for node in &effects.children {
            let member = format!("{path}.children[uid={}]", node.uid());
            reads.field(id.0, format!("{member}.node"));
            if node.as_group().is_some() {
                reads.field(id.0, format!("{member}.meta.match"));
            }
        }
        // Appending depends on the end gap and the existing groups' match ids, not their values.
        reads.window(id.0, format!("{path}.children"), effects.children.len(), effects.children.len().saturating_add(1));
    }
    Some(reads.finish())
}

fn environment(session: &Session, params: &Value) -> Option<Under> {
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    let ids = layers(session, params, cid, comp, &mut reads)?;
    let first = comp.layer(*ids.first()?)?;
    let on = params.get("on").and_then(Value::as_bool).unwrap_or(!first.environment);
    for id in ids {
        reads.field(id.0, "attrs.source");
        reads.field(id.0, "attrs.environment");
        if on {
            reads.field(id.0, "attrs.switches.three_d");
        }
    }
    Some(reads.finish())
}

fn layer_settings(session: &Session, params: &Value) -> Option<Under> {
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    // layer_p reads only the singular parameter, otherwise the first selected layer.
    let id = match params.get("layer") {
        Some(value) => layer_address(&mut reads, cid, comp, value)?,
        None => *session.state.selected_layers.first()?,
    };
    let layer = comp.layer(id)?;
    if matches!(layer.source, LayerSource::Camera | LayerSource::Light { .. }) {
        return None;
    }
    reads.field(id.0, "attrs.source");
    reads.field(id.0, "sourceItem");
    // Explicit-id calls currently ignore SDK list windows. Keep the generic position guard so
    // narrowing content does not newly accept source-user insertions that move this layer.
    reads.all_positions.insert(id.0);
    let renames = params.get("name").and_then(Value::as_str).is_some_and(|name| !name.trim().is_empty());
    if renames {
        reads.field(id.0, "name");
    }
    if let LayerSource::Solid { item } = layer.source {
        let affect_all = params.get("affectAll").and_then(Value::as_bool).unwrap_or(true);
        let mut shared = false;
        if !affect_all {
            // The shared-source branch scans every composition and every layer's source.
            // Include list membership so removed users and newly created compositions count.
            reads.window("project", "items".to_owned(), 0, session.project.items.len().saturating_add(1));
            let mut users = 0usize;
            for (iid, entry) in &session.project.items {
                reads.field(iid.0, "kind");
                if let Some(comp) = entry.as_comp() {
                    reads.window(iid.0, "comp.layers".to_owned(), 0, comp.layers.len().saturating_add(1));
                    for user in &comp.layers {
                        reads.field(user.id.0, "attrs.source");
                        reads.field(user.id.0, "sourceItem");
                        users = users.saturating_add(usize::from(user.source == layer.source));
                    }
                }
            }
            shared = users > 1;
        }
        reads.field(item.0, "kind");
        if shared {
            reads.field(item.0, "data");
            reads.field(item.0, "attrs.label");
            reads.field(item.0, "parent");
            if !renames {
                reads.field(item.0, "name");
            }
        } else {
            for (parameter, field) in [("color", "data.color"), ("width", "data.width"), ("height", "data.height"), ("pixelAspect", "data.pixel_aspect")] {
                if params.get(parameter).is_some() {
                    reads.field(item.0, field);
                }
            }
            if renames {
                reads.field(item.0, "name");
            }
        }
    }
    Some(reads.finish())
}

fn source_duration(session: &Session, reads: &mut Reads, layer: &Layer, loops: bool) {
    let Some(id) = layer.source.item() else { return };
    reads.field(id.0, "kind");
    match session.project.item(id).map(|item| &item.kind) {
        Some(ItemKind::Comp(_)) => reads.field(id.0, "comp.attrs.duration"),
        Some(ItemKind::Footage(footage)) => {
            reads.field(id.0, "data.kind");
            if footage.kind != FootageKind::Still {
                reads.field(id.0, "data.duration");
                if loops {
                    reads.field(id.0, "data.loop_count");
                }
            }
        }
        _ => {}
    }
}

fn slip(session: &Session, params: &Value) -> Option<Under> {
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    reads.field(cid.0, "comp.attrs.frame_rate");
    for id in layers(session, params, cid, comp, &mut reads)? {
        let layer = comp.layer(id)?;
        // The branch reads presence, so existing remap key values do not matter.
        reads.field(id.0, "timeRemap.meta.match");
        if layer.props.get("timeRemap").is_some() {
            continue;
        }
        reads.field(id.0, "attrs.source");
        reads.field(id.0, "sourceItem");
        if matches!(layer.source, LayerSource::Footage { .. } | LayerSource::Comp { .. }) {
            source_duration(session, &mut reads, layer, true);
            reads.timing(id.0);
        }
    }
    Some(reads.finish())
}

fn enable_time_remap(session: &Session, params: &Value) -> Option<Under> {
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    for id in layers(session, params, cid, comp, &mut reads)? {
        let layer = comp.layer(id)?;
        reads.field(id.0, "attrs.source");
        reads.field(id.0, "sourceItem");
        if !matches!(layer.source, LayerSource::Footage { .. } | LayerSource::Comp { .. }) {
            continue;
        }
        let has = layer.props.get("timeRemap").is_some();
        let target = params.get("value").and_then(Value::as_bool).unwrap_or(!has);
        reads.field(id.0, "timeRemap.meta.match");
        if target && !has {
            reads.timing(id.0);
            if layer.layer_time(layer.in_point) > layer.layer_time(layer.out_point) {
                reads.field(cid.0, "comp.attrs.frame_rate");
            } else {
                source_duration(session, &mut reads, layer, false);
            }
        } else if !target && has {
            reads.field(id.0, "timeRemap");
        }
    }
    Some(reads.finish())
}

// Marker bulk operations read the lock flag, and deletion consumes the marker array.
// Keep their existing position sensitivity while narrowing unrelated layer content.
fn marker_fields(session: &Session, command: &str, params: &Value) -> Option<Under> {
    let mut reads = Reads::default();
    let (cid, comp) = composition(session, params, &mut reads)?;
    for id in layers(session, params, cid, comp, &mut reads)? {
        reads.field(id.0, "attrs.markers_locked");
        if command == "layer.deleteAllMarkers" && !comp.layer(id)?.markers_locked {
            reads.field(id.0, "attrs.markers");
        }
        reads.all_positions.insert(id.0);
    }
    Some(reads.finish())
}

pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Option<Under> {
    match command {
        "layer.markersLock" | "layer.deleteAllMarkers" => marker_fields(session, command, params),
        "effect.apply" => effect_apply(session, params),
        "layer.environment" => environment(session, params),
        "layer.settings" => layer_settings(session, params),
        "layer.slip" | "layer.slipBack" | "layer.slipForward" => slip(session, params),
        "layer.enableTimeRemap" => enable_time_remap(session, params),
        _ => None,
    }
}
