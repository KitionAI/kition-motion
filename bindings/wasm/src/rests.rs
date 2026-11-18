//! Handler dependencies beyond generic target identity, including fields assigned by a write.
use kition_kit::{Premise, Under, Within};
use kition_motion_engine::{Session, commands::resolve_layer};
use kition_motion_project::{Comp, ItemId, Layer};
use serde_json::{Value, json};

fn composition<'a>(session: &'a Session, params: &Value) -> Option<(ItemId, &'a Comp)> {
    let id = match params.get("comp") {
        Some(Value::Number(n)) => n.as_u64().map(ItemId),
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

fn fields(id: impl Into<Value>, names: impl IntoIterator<Item = impl Into<String>>) -> Premise {
    Premise::on(id).fields(names).positions(Vec::<String>::new())
}

fn layer_address(answer: &mut Under, comp_id: ItemId, comp: &Comp, value: &Value, layer: &Layer) {
    let index = crate::place::index_of(value).or_else(|| {
        let number = value.as_u64()?;
        if comp.layers.iter().any(|layer| layer.id.0 == number) { None } else { usize::try_from(number).ok()?.checked_sub(1) }
    });
    if let Some(index) = index {
        answer.objects.push(json!(layer.id.0));
        answer.within.push(Within {
            parent: json!(comp_id.0),
            field: "comp.layers".to_owned(),
            from: index,
            to: index.saturating_add(1),
            ..Default::default()
        });
    }
}

/// `None` leaves a command with its existing, general declaration.
pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Option<Under> {
    let render_preference = match command {
        "renderQueue.setNotify" => Some("attrs.render_prefs.notify"),
        "renderQueue.setOverflowFolders" => Some("attrs.render_prefs.overflow_folders"),
        _ => None,
    };
    if let Some(field) = render_preference {
        return Some(Under { premises: vec![fields("project", [field])], ..Default::default() });
    }
    if let Some(answer) = crate::rest_comp::under(session, command, params) {
        return Some(answer);
    }
    if let Some(answer) = crate::rest_visual::under(session, command, params) {
        return Some(answer);
    }
    if let Some(answer) = crate::rest_mask_path::under(session, command, params) {
        return Some(answer);
    }
    if let Some(answer) = crate::rest_animation::under(session, command, params) {
        return Some(answer);
    }
    if let Some(answer) = crate::rest_layer::under(session, command, params) {
        return Some(answer);
    }
    if let Some(answer) = crate::rest_masks::under(session, command, params) {
        return Some(answer);
    }
    if command == "project.setProjectComment" {
        return Some(Under { premises: vec![fields("project", ["attrs.settings.comment"])], ..Default::default() });
    }
    if matches!(command, "view.addGuide" | "view.clearGuides" | "view.importGuides" | "view.moveGuide" | "view.removeGuide") {
        let (id, comp) = composition(session, params)?;
        let mut names = match command {
            "view.moveGuide" => vec![format!("comp.attrs.guides.{}", params.get("index")?.as_u64()?)],
            "view.removeGuide" => vec![format!("comp.attrs.guides.{}", params.get("index")?.as_u64()?)],
            // `commands/view.rs` `add_guide` pushes after the last guide and reads none of them:
            // the insertion names the gap at the end of the list as it was observed.
            "view.addGuide" => vec![format!("comp.attrs.guides.{}", comp.guides.len())],
            _ => vec!["comp.attrs.guides".to_owned()],
        };
        if command == "view.addGuide" && params.get("position").and_then(Value::as_f64).is_none() {
            let horizontal = matches!(params.get("orientation").and_then(Value::as_str), Some("horizontal" | "h"));
            names.push(if horizontal { "comp.attrs.height" } else { "comp.attrs.width" }.to_owned());
        }
        return Some(Under { premises: vec![fields(json!(id.0), names)], ..Default::default() });
    }
    if matches!(command, "camera.linkFocusToPoi" | "camera.linkFocusToLayer") {
        let (cid, comp) = composition(session, params)?;
        let selected: Vec<_> = session.state.selected_layers.iter().filter_map(|id| comp.layer(*id)).collect();
        let camera = match params.get("camera") {
            Some(value) => comp.layer(resolve_layer(comp, value)?)?,
            None => selected.iter().copied().find(|layer| layer.is_camera())?,
        };
        let mut answer = Under::default();
        if params.get("camera").is_none() {
            for layer in &selected {
                answer.premises.push(fields(json!(layer.id.0), ["attrs.source"]));
                if layer.id == camera.id {
                    break;
                }
            }
        }
        let mut camera_fields = vec!["attrs.source", "cameraOptions/focusDistance.expr"];
        if let Some(value) = params.get("camera") {
            layer_address(&mut answer, cid, comp, value, camera);
            if value.as_str().is_some_and(|name| name.parse::<u64>().is_err() && !name.starts_with('#')) {
                camera_fields.push("name");
            }
        }
        answer.premises.push(fields(json!(camera.id.0), camera_fields));
        if command == "camera.linkFocusToLayer" {
            let target = match params.get("layer") {
                Some(value) => comp.layer(resolve_layer(comp, value)?)?,
                None => selected.iter().copied().find(|layer| layer.id != camera.id)?,
            };
            answer.premises.push(fields(json!(target.id.0), ["name"]));
            if let Some(value) = params.get("layer") {
                layer_address(&mut answer, cid, comp, value, target);
            }
        } else if let Some(value) = params.get("layer") {
            // refs resolves an explicit target even for the point-of-interest expression.
            let target = comp.layer(resolve_layer(comp, value)?)?;
            let names = if value.as_str().is_some_and(|name| name.parse::<u64>().is_err() && !name.starts_with('#')) { vec!["name"] } else { Vec::new() };
            answer.premises.push(fields(json!(target.id.0), names));
            layer_address(&mut answer, cid, comp, value, target);
        }
        return Some(answer);
    }
    None
}

/// Facts outside the target layer that scene frame sampling reads before it starts a job.
pub(crate) fn supplemental(session: &Session, command: &str, params: &Value) -> Option<Under> {
    if command != "layer.sceneEditDetection" {
        return None;
    }
    let (cid, comp) = composition(session, params)?;
    let lid = match params.get("layer") {
        Some(value) => resolve_layer(comp, value)?,
        None => *session.state.selected_layers.first()?,
    };
    let layer = comp.layer(lid)?;
    let mut answer = Under { premises: vec![fields(json!(cid.0), ["comp.attrs.frame_rate", "comp.attrs.duration"])], ..Default::default() };
    if let Some(id) = layer.source.item() {
        let names = match &session.project.item(id)?.kind {
            kition_motion_project::ItemKind::Comp(_) => vec!["kind", "comp"],
            kition_motion_project::ItemKind::Footage(_) | kition_motion_project::ItemKind::Solid(_) => vec!["kind", "data"],
            kition_motion_project::ItemKind::Folder => vec!["kind"],
        };
        answer.premises.push(fields(json!(id.0), names));
    }
    Some(answer)
}
