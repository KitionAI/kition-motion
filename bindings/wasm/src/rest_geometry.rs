//! Stored tracker targets, release matrices, and nested precomposition pixels.
use kition_kit::{Premise, Under};
use kition_motion_engine::{Session, commands::resolve_layer};
use kition_motion_project::{Comp, ItemId, ItemKind, Layer, LayerId, LayerSource};
use serde_json::{Value, json};
use std::collections::HashSet;

fn field(id: Value, names: impl IntoIterator<Item = impl Into<String>>) -> Premise {
    Premise::on(id).fields(names).positions(Vec::<String>::new())
}
fn aspect(session: &Session, layer: &Layer, answer: &mut Under) {
    if let Some(item) = layer.source.item() {
        let path = if session.project.comp(item).is_some() { "comp.attrs.pixel_aspect" } else { "data.pixel_aspect" };
        answer.premises.push(field(json!(item.0), ["kind", path]));
    }
}
pub(crate) fn matrix(session: &Session, cid: ItemId, comp: &Comp, start: LayerId, answer: &mut Under) {
    let mut next = Some(start);
    let mut seen = HashSet::new();
    while let Some(id) = next {
        if !seen.insert(id) || seen.len() > 256 {
            break;
        }
        let Some(layer) = comp.layer(id) else { break };
        answer.premises.push(
            Premise::on(json!(id.0))
                .fields([
                    "attrs.source",
                    "sourceItem",
                    "attrs.stretch",
                    "switches.three_d",
                    "attrs.auto_orient",
                    "parent",
                    "transform/anchor",
                    "transform/position",
                    "transform/positionX",
                    "transform/positionY",
                    "transform/positionZ",
                    "transform/rotation",
                    "transform/rotationX",
                    "transform/rotationY",
                    "transform/rotationZ",
                    "transform/orientation",
                    "transform/scale",
                    "transform/skew",
                    "transform/skewAxis",
                    "transform/pointOfInterest",
                ])
                .positions(["startTime"]),
        );
        aspect(session, layer, answer);
        next = layer.parent;
    }
    answer.premises.push(field(json!(cid.0), ["comp.attrs.pixel_aspect"]));
}
fn source_items(session: &Session, source: ItemId) -> Result<Vec<ItemId>, String> {
    let mut pending = vec![source];
    let mut queued = HashSet::from([source]);
    let mut found = Vec::new();
    let mut layers = 0usize;
    while let Some(id) = pending.pop() {
        found.push(id);
        if let Some(comp) = session.project.comp(id) {
            for layer in &comp.layers {
                layers = layers.saturating_add(1);
                if layers > 65_536 {
                    return Err("auto trace has too many source layers to guard; simplify the precomposition".to_owned());
                }
                if let Some(item) = layer.source.item()
                    && !queued.contains(&item)
                {
                    if queued.len() >= 4096 {
                        return Err("auto trace has too many source items to guard; simplify the precomposition".to_owned());
                    }
                    queued.insert(item);
                    pending.push(item);
                }
            }
        }
    }
    Ok(found)
}
pub(crate) fn pixels(session: &Session, source: ItemId, answer: &mut Under) -> Result<(), String> {
    for id in source_items(session, source)? {
        let Some(item) = session.project.item(id) else { continue };
        match &item.kind {
            ItemKind::Comp(comp) => {
                answer.premises.push(field(
                    json!(id.0),
                    [
                        "kind",
                        "comp.attrs.width",
                        "comp.attrs.height",
                        "comp.attrs.pixel_aspect",
                        "comp.attrs.background",
                        "comp.attrs.frame_rate",
                        "comp.attrs.duration",
                        "comp.attrs.renderer",
                        "~layerMembers",
                    ],
                ));
                for layer in &comp.layers {
                    answer.premises.push(
                        Premise::on(json!(layer.id.0))
                            .fields([
                                "attrs.source",
                                "sourceItem",
                                "switches",
                                "attrs.stretch",
                                "attrs.auto_orient",
                                "parent",
                                "matte",
                                "matteLayer",
                                "props",
                                "transform/opacity",
                                "transform/position",
                                "transform/anchor",
                                "transform/scale",
                                "transform/rotation",
                            ])
                            .positions(["startTime", "inPoint", "outPoint"]),
                    );
                }
            }
            ItemKind::Solid(_) => answer.premises.push(field(json!(id.0), ["kind", "data.color", "data.width", "data.height", "data.pixel_aspect"])),
            ItemKind::Footage(_) => answer.premises.push(field(json!(id.0), ["kind", "data", "attrs.proxy"])),
            ItemKind::Folder => {}
        }
    }
    Ok(())
}

// Execute checks the same bound, including for direct callers without a premise host.
pub(crate) fn check(session: &Session, command: &str, params: &Value) -> Result<(), String> {
    if command == "light.controlWithCamera" && params.get("layers").and_then(Value::as_array).map_or(session.state.selected_layers.len(), Vec::len) > 4096 {
        return Err("too many lights to guard; split the selection".to_owned());
    }
    if command != "layer.autoTrace" {
        return Ok(());
    }
    let cid = params
        .get("comp")
        .filter(|value| !value.is_null())
        .map(|value| {
            value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()).or_else(|| {
                session.project.items.values().find(|item| value.as_str() == Some(item.name.as_str()) && item.as_comp().is_some()).map(|item| item.id)
            })
        })
        .unwrap_or_else(|| session.active_comp_id());
    let Some(comp) = cid.and_then(|cid| session.project.comp(cid)) else {
        return Ok(());
    };
    let Some(layer) = params
        .get("layer")
        .and_then(|value| resolve_layer(comp, value))
        .or_else(|| session.state.selected_layers.first().copied())
        .and_then(|id| comp.layer(id))
    else {
        return Ok(());
    };
    if let LayerSource::Comp { item } = layer.source {
        source_items(session, item)?;
    }
    Ok(())
}

/// The shape tools (shape_tool.rs:331-376, viewer_cmds.rs:221-282): a default position at the
/// frame centre, a new shape layer sized and spanned by the composition, and comp-space input
/// mapped through the target's transform chain.
fn shape_tools(session: &Session, command: &str, params: &Value, under: &mut Option<Under>) {
    if !matches!(command, "shape.newShape" | "shape.newPath") {
        return;
    }
    let cid =
        match params.get("comp").filter(|value| !value.is_null()) {
            Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()).or_else(|| {
                session.project.items.values().find(|item| value.as_str() == Some(item.name.as_str()) && item.as_comp().is_some()).map(|item| item.id)
            }),
            None => session.active_comp_id(),
        };
    let Some((cid, comp)) = cid.and_then(|cid| Some((cid, session.project.comp(cid)?))) else { return };
    let named_layer = command == "shape.newPath" && params.get("layer").is_none() && params.get("name").and_then(Value::as_str).is_some();
    let target = if named_layer {
        None
    } else {
        match params.get("layer") {
            Some(value) => resolve_layer(comp, value),
            None => kition_motion_engine::commands::shape_tool::selected_shape_target(session, comp),
        }
    };
    let space = params.get("space").and_then(Value::as_str);
    let mut sizes: Vec<&str> = Vec::new();
    if command == "shape.newShape" && params.get("position").is_none_or(Value::is_null) {
        sizes.extend(["comp.attrs.width", "comp.attrs.height"]);
    }
    let answer = under.get_or_insert_with(Under::default);
    match target {
        None => sizes.extend(["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration"]),
        Some(layer) => {
            let comp_space = if command == "shape.newShape" { space != Some("layer") } else { space == Some("comp") };
            if comp_space {
                matrix(session, cid, comp, layer, answer);
            }
        }
    }
    if !sizes.is_empty() {
        sizes.sort_unstable();
        sizes.dedup();
        answer.premises.push(field(json!(cid.0), sizes));
    }
}

/// Path nulls (path_nulls.rs:173-300): each null takes a name unique among the comp's layers
/// (its expressions find layers by name), at a point mapped to comp space through the target's
/// parent chain.
fn path_nulls(session: &Session, command: &str, params: &Value, under: &mut Option<Under>) {
    if !matches!(command, "path.nullsFollowPoints" | "path.pointsFollowNulls" | "path.tracePath") {
        return;
    }
    let cid = match params.get("comp").filter(|value| !value.is_null()) {
        Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()),
        None => session.active_comp_id(),
    };
    let Some((cid, comp)) = cid.and_then(|cid| Some((cid, session.project.comp(cid)?))) else { return };
    let target = params.get("layer").and_then(|value| resolve_layer(comp, value)).or_else(|| session.state.selected_layers.first().copied());
    let answer = under.get_or_insert_with(Under::default);
    answer.premises.push(field(json!(cid.0), ["~layerMembers"]));
    for layer in comp.layers.iter().filter(|layer| Some(layer.id) != target) {
        answer.premises.push(field(json!(layer.id.0), ["name"]));
    }
    if let Some(parent) = target.and_then(|id| comp.layer(id)).and_then(|layer| layer.parent) {
        matrix(session, cid, comp, parent, answer);
    }
}

pub(crate) fn extend(session: &Session, command: &str, params: &Value, under: &mut Option<Under>) -> Result<(), String> {
    shape_tools(session, command, params, under);
    path_nulls(session, command, params, under);
    if !matches!(command, "track.apply" | "light.controlWithCamera" | "layer.autoTrace") {
        return Ok(());
    }
    check(session, command, params)?;
    let cid = params
        .get("comp")
        .filter(|value| !value.is_null())
        .map(|value| {
            value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()).or_else(|| {
                session.project.items.values().find(|item| value.as_str() == Some(item.name.as_str()) && item.as_comp().is_some()).map(|item| item.id)
            })
        })
        .unwrap_or_else(|| session.active_comp_id());
    let Some((cid, comp)) = cid.and_then(|id| Some((id, session.project.comp(id)?))) else { return Ok(()) };
    let answer = under.get_or_insert_with(Under::default);
    if command == "track.apply" {
        let source = params
            .get("layer")
            .and_then(|value| resolve_layer(comp, value))
            .or_else(|| session.state.current_track.map(|(layer, _)| layer))
            .and_then(|id| comp.layer(id));
        let Some(source) = source else { return Ok(()) };
        let tracker = match params.get("tracker") {
            Some(Value::Number(n)) => n
                .as_u64()
                .and_then(|uid| source.tracker(uid).or_else(|| usize::try_from(uid).ok()?.checked_sub(1).and_then(|index| source.trackers().nth(index)))),
            Some(Value::String(name)) => name
                .strip_prefix('#')
                .and_then(|index| index.parse::<usize>().ok()?.checked_sub(1))
                .and_then(|index| source.trackers().nth(index))
                .or_else(|| source.trackers().find(|(group, _)| group.name == *name)),
            _ => session.state.current_track.filter(|(id, _)| *id == source.id).and_then(|(_, uid)| source.tracker(uid)).or_else(|| source.trackers().next()),
        };
        if let Some((_, settings)) = tracker {
            let target = if settings.kind == kition_motion_project::tracking::TrackKind::Stabilize { Some(source.id) } else { settings.target };
            if let Some(target) = target {
                matrix(session, cid, comp, target, answer);
            }
            matrix(session, cid, comp, source.id, answer);
        }
    } else if command == "light.controlWithCamera" && params.get("on").and_then(Value::as_bool) == Some(false) {
        let targets: Vec<_> = params
            .get("layers")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(|value| resolve_layer(comp, value)).collect())
            .unwrap_or_else(|| session.state.selected_layers.clone());
        for target in targets {
            matrix(session, cid, comp, target, answer);
        }
    } else if command == "layer.autoTrace"
        && let Some(layer) = params.get("layer").and_then(|value| resolve_layer(comp, value)).and_then(|id| comp.layer(id))
        && let LayerSource::Comp { item } = layer.source
    {
        pixels(session, item, answer)?;
    }
    Ok(())
}
