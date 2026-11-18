//! Additional command inputs established by independent-file regressions.
use kition_kit::{Premise, Under};
use kition_motion_engine::{Session, commands::resolve_layer};
use kition_motion_project::ItemId;
use serde_json::{Value, json};

fn fields(id: impl Into<Value>, names: impl IntoIterator<Item = impl Into<String>>) -> Premise {
    Premise::on(id).fields(names).positions(Vec::<String>::new())
}

pub(crate) fn extend(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    if check(session, command, params).is_err() {
        return;
    }
    project_inputs(command, params, answer);
    queue_inputs(session, command, params, answer);
    selection_comp_inputs(session, command, answer);
    stack_inputs(session, command, params, answer);
    twin_inputs(session, command, params, answer);
    clear_inputs(session, command, params, answer);
    retarget_inputs(session, command, params, answer);
    let comp_id = params
        .get("comp")
        .filter(|value| !value.is_null())
        .map(|value| {
            value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()).or_else(|| {
                session.project.items.values().find(|item| value.as_str() == Some(item.name.as_str()) && item.as_comp().is_some()).map(|item| item.id)
            })
        })
        .unwrap_or_else(|| session.active_comp_id());
    let Some((cid, comp)) = comp_id.and_then(|cid| Some((cid, session.project.comp(cid)?))) else { return };
    let targets: Vec<_> = params
        .get("layers")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(|value| resolve_layer(comp, value)).collect())
        .unwrap_or_else(|| session.state.selected_layers.clone());
    if command == "edit.duplicate" && !(params.get("layers").is_none() && !session.state.selected_props.is_empty()) && !targets.is_empty() {
        let answer = answer.get_or_insert_with(Under::default);
        // duplicate reads every existing name while finding a unique copy name. Membership is
        // a set: reordering unrelated siblings does not affect that search.
        answer.premises.push(fields(json!(cid.0), ["~layerMembers"]));
        for layer in &comp.layers {
            if !targets.contains(&layer.id) {
                answer.premises.push(fields(json!(layer.id.0), ["name"]));
            }
        }
    }
    if command == "layer.precompose" {
        let answer = answer.get_or_insert_with(Under::default);
        let mut settings = vec![
            "comp.attrs.frame_rate",
            "comp.attrs.pixel_aspect",
            "comp.attrs.background",
            "comp.attrs.shutter_angle",
            "comp.attrs.shutter_phase",
            "comp.attrs.motion_blur_samples",
            "comp.attrs.motion_blur_adaptive_limit",
            "comp.attrs.renderer",
            "comp.attrs.enable_motion_blur",
            "comp.attrs.enable_frame_blending",
            "comp.attrs.preserve_frame_rate",
            "comp.attrs.preserve_resolution",
        ];
        if params.get("mode").and_then(Value::as_str) == Some("leave") {
            if let Some(layer) = targets.first().and_then(|id| comp.layer(*id))
                && let Some(item) = layer.source.item().and_then(|id| session.project.item(id))
            {
                let mut names = vec!["kind", "attrs.label"];
                let (width, height, duration) = match &item.kind {
                    kition_motion_project::ItemKind::Comp(source) => {
                        names.extend(["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration"]);
                        (source.width, source.height, Some(source.duration))
                    }
                    kition_motion_project::ItemKind::Footage(source) => {
                        names.extend(["data.width", "data.height", "data.has_video", "data.duration"]);
                        let size = if source.has_video { (source.width, source.height) } else { (0, 0) };
                        (size.0, size.1, (source.duration.0 > 0).then_some(source.duration))
                    }
                    kition_motion_project::ItemKind::Solid(source) => {
                        names.extend(["data.width", "data.height"]);
                        (source.width, source.height, None)
                    }
                    kition_motion_project::ItemKind::Folder => (0, 0, None),
                };
                answer.premises.push(fields(json!(item.id.0), names));
                if width == 0 || height == 0 {
                    settings.extend(["comp.attrs.width", "comp.attrs.height"]);
                }
                if duration.is_none() {
                    settings.push("comp.attrs.duration");
                }
            }
        } else {
            settings.extend(["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration"]);
        }
        answer.premises.push(fields(json!(cid.0), settings));
    }
    if command == "layer.updateMarkersFromSource" {
        let answer = answer.get_or_insert_with(Under::default);
        for layer in targets.iter().filter_map(|id| comp.layer(*id)) {
            if let kition_motion_project::LayerSource::Comp { item } = layer.source {
                answer.premises.push(fields(json!(item.0), ["kind", "comp.attrs.markers"]));
            }
        }
    }
}

fn project_inputs(command: &str, params: &Value, answer: &mut Option<Under>) {
    let mut names = Vec::new();
    match command {
        "file.projectSettings" => {
            // Validation always checks the current engine/working-space pair.
            names.extend(["attrs.settings.color_engine", "attrs.settings.working_space"]);
            for (parameter, field) in [
                ("bitDepth", "attrs.settings.bit_depth"),
                ("linearize", "attrs.settings.linearize"),
                ("blendLinear", "attrs.settings.blend_linear"),
                ("blendColorsUsing1Gamma", "attrs.settings.blend_linear"),
                ("colorEngine", "attrs.settings.color_engine"),
                ("workingSpace", "attrs.settings.working_space"),
                ("workingGamma", "attrs.settings.working_gamma"),
                ("hdr", "attrs.settings.hdr"),
                ("outputSpace", "attrs.settings.output_space"),
                ("renderer", "attrs.settings.gpu_acceleration"),
                ("gpuAcceleration", "attrs.settings.gpu_acceleration"),
                ("timeDisplay", "attrs.settings.time_display"),
            ] {
                if params.get(parameter).is_some() {
                    names.push(field);
                }
            }
            if params.get("colorEngine").is_some() {
                names.push("attrs.settings.hdr");
            }
        }
        "file.cycleBitDepth" => names.push("attrs.settings.bit_depth"),
        "render.backend" if params.get("backend").or_else(|| params.get("renderer")).is_some() => names.push("attrs.settings.gpu_acceleration"),
        "renderQueue.saveTemplate" | "renderQueue.deleteTemplate" | "renderQueue.setTemplateDefault" => {
            // These handlers clone and replace the template collection, including defaults.
            names.push("attrs.render_templates");
        }
        "renderQueue.add" | "render.preRender" | "file.createProxy" | "renderQueue.applyTemplate" => names.push("attrs.render_templates"),
        "comp.saveFrameAs" if params.get("queue").and_then(Value::as_bool) == Some(true) => names.push("attrs.render_templates"),
        "file.newCompFromSelection" if params.get("addToRenderQueue").and_then(Value::as_bool) == Some(true) => names.push("attrs.render_templates"),
        _ => {}
    }
    if !names.is_empty() {
        answer.get_or_insert_with(Under::default).premises.push(fields("project", names));
    }
}

fn queue_inputs(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    let append = matches!(command, "renderQueue.add" | "render.preRender" | "file.createProxy")
        || command == "comp.saveFrameAs" && params.get("queue").and_then(Value::as_bool) == Some(true)
        || command == "file.newCompFromSelection" && params.get("addToRenderQueue").and_then(Value::as_bool) == Some(true);
    if append {
        let answer = answer.get_or_insert_with(Under::default);
        // Allocation reads the maximum queue id, and appends to the queue's end gap.
        answer.within.push(kition_kit::Within {
            parent: json!("project"),
            field: "renderQueue".to_owned(),
            from: 0,
            to: session.project.render_queue.len().saturating_add(1),
            ..Default::default()
        });
        for item in &session.project.render_queue {
            answer.premises.push(fields(format!("render:{}", item.id), ["id"]));
        }
    }
    if command == "render.preRender" {
        let comp = params.get("comp").and_then(Value::as_u64).map(ItemId).or_else(|| session.active_comp_id());
        if let Some(comp) = comp {
            // item_json resolves the new entry's composition name for its output path.
            answer.get_or_insert_with(Under::default).premises.push(fields(json!(comp.0), ["name"]));
        }
    }
    if !matches!(command, "render.addOutputModule" | "renderQueue.setOutputModule") {
        return;
    }
    let queue = &session.project.render_queue;
    let index = params
        .get("item")
        .and_then(Value::as_u64)
        .and_then(|id| queue.iter().position(|item| item.id == id))
        .or_else(|| params.get("index").and_then(Value::as_u64).and_then(|index| usize::try_from(index).ok()?.checked_sub(1)))
        .or_else(|| queue.len().checked_sub(1));
    let Some(index) = index else { return };
    let Some(item) = queue.get(index) else { return };
    let answer = answer.get_or_insert_with(Under::default);
    if command == "render.addOutputModule" {
        answer.premises.push(fields(format!("render:{}", item.id), ["item.output", "item.extra_outputs", "item.status"]));
    } else {
        let resize = params.get("resize");
        let height = params.get("resizeHeight").is_some() || resize.and_then(|r| r.get("height")).is_some();
        let width = params.get("resizeWidth").is_some() || resize.and_then(|r| r.get("width").or_else(|| r.get("preset"))).is_some();
        let module = params.get("module").and_then(Value::as_u64).unwrap_or(1);
        let output =
            if module == 1 { Some(&item.output) } else { module.checked_sub(2).and_then(|n| usize::try_from(n).ok()).and_then(|n| item.extra_outputs.get(n)) };
        if let Some(output) = output {
            let enabled = resize
                .and_then(Value::as_bool)
                .or_else(|| resize.and_then(|r| r.get("enabled")).and_then(Value::as_bool))
                .unwrap_or_else(|| resize.is_some_and(Value::is_object) || params.get("resizeHeight").is_some() || output.resize.enabled);
            let locked = resize.and_then(|r| r.get("lockAspect")).and_then(Value::as_bool).unwrap_or(output.resize.lock_aspect);
            if height && !width && enabled && locked {
                answer.premises.push(fields(json!(item.comp.0), ["comp.attrs.width", "comp.attrs.height", "comp.attrs.pixel_aspect"]));
            }
        }
    }
    if params.get("item").is_none() {
        answer.within.push(kition_kit::Within {
            parent: json!("project"),
            field: "renderQueue".to_owned(),
            from: index,
            to: index.saturating_add(1),
            ..Default::default()
        });
    }
}

/// Stack neighbours a write takes from the composition's order (layer.rs:948-990,
/// layer_menu.rs:107-118 and 603-650): the layer Bring Forward / Send Backward passes, the layer
/// above or below that becomes a track matte (its video switch is assigned), and every other
/// layer Hide Other Video switches off, including layers added since.
fn stack_inputs(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    let how = params.get("to").and_then(Value::as_str).unwrap_or("front");
    let op = params.get("op").and_then(Value::as_str).unwrap_or_default();
    let relevant = match command {
        "layer.arrange" => params.get("index").is_none() && params.get("above").is_none() && matches!(how, "forward" | "backward"),
        "layer.trackMatte" => matches!(op, "above" | "below" | "alpha" | "alphaInverted" | "luma" | "lumaInverted"),
        "layer.hideOtherVideo" => true,
        _ => false,
    };
    if !relevant {
        return;
    }
    let cid = match params.get("comp").filter(|value| !value.is_null()) {
        Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()),
        None => session.active_comp_id(),
    };
    let Some((cid, comp)) = cid.and_then(|cid| Some((cid, session.project.comp(cid)?))) else { return };
    let targets: Vec<_> = match params.get("layers") {
        Some(Value::Array(values)) => values.iter().filter_map(|value| resolve_layer(comp, value)).collect(),
        Some(value) => resolve_layer(comp, value).into_iter().collect(),
        None => params.get("layer").and_then(|value| resolve_layer(comp, value)).map_or_else(|| session.state.selected_layers.clone(), |id| vec![id]),
    };
    let order: Vec<_> = comp.layers.iter().map(|layer| layer.id).collect();
    let picked = |index: usize| order.get(index).is_some_and(|id| targets.contains(id));
    let answer = answer.get_or_insert_with(Under::default);
    let hold = |index: usize, answer: &mut Under, video: bool| {
        let Some(id) = order.get(index) else { return };
        answer.within.push(kition_kit::Within { parent: json!(cid.0), field: "comp.layers".to_owned(), from: index, to: index + 1, ..Default::default() });
        answer.premises.push(fields(json!(id.0), if video { vec!["attrs.switches.video"] } else { Vec::new() }));
    };
    match command {
        "layer.arrange" if how == "forward" => {
            for index in 1..order.len() {
                if picked(index) && !picked(index - 1) {
                    hold(index - 1, answer, false);
                }
            }
        }
        "layer.arrange" => {
            for index in 0..order.len().saturating_sub(1) {
                if picked(index) && !picked(index + 1) {
                    hold(index + 1, answer, false);
                }
            }
        }
        "layer.trackMatte" => {
            for target in &targets {
                let Some(index) = order.iter().position(|id| id == target) else { continue };
                let existing = comp.layer(*target).and_then(|layer| layer.track_matte).filter(|_| !matches!(op, "above" | "below"));
                if let Some(matte) = existing {
                    answer.premises.push(fields(json!(matte.layer.0), ["attrs.switches.video"]));
                } else if op == "below" {
                    hold(index + 1, answer, true);
                } else if let Some(above) = index.checked_sub(1) {
                    hold(above, answer, true);
                }
            }
        }
        _ => {
            answer.premises.push(fields(json!(cid.0), ["~layerMembers"]));
            for layer in comp.layers.iter().filter(|layer| !targets.contains(&layer.id)) {
                answer.premises.push(fields(json!(layer.id.0), ["attrs.source", "attrs.switches.video"]));
            }
        }
    }
}

/// `selected: true` edits (prop.rs:230-370) apply the same value, stopwatch or key to the same
/// property of every other selected, unlocked layer whose value is of the same kind.
fn twin_inputs(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    if !matches!(command, "prop.set" | "prop.toggleAnimation" | "prop.toggleKey") || params.get("selected").and_then(Value::as_bool) != Some(true) {
        return;
    }
    let cid = match params.get("comp").filter(|value| !value.is_null()) {
        Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()),
        None => session.active_comp_id(),
    };
    let Some(comp) = cid.and_then(|cid| session.project.comp(cid)) else { return };
    let Some(layer) = params.get("layer").and_then(|value| resolve_layer(comp, value)).and_then(|id| comp.layer(id)) else { return };
    let uid = match (params.get("prop").and_then(Value::as_u64), params.get("path").and_then(Value::as_str)) {
        (Some(uid), _) => Some(uid),
        (None, Some(path)) => layer.props.prop(path).map(|property| property.uid),
        _ => None,
    };
    let Some(path) = uid.and_then(|uid| layer.props.match_path_of(uid)) else { return };
    if !session.state.selected_layers.contains(&layer.id) {
        return;
    }
    let answer = answer.get_or_insert_with(Under::default);
    for twin in session.state.selected_layers.iter().filter(|id| **id != layer.id).filter_map(|id| comp.layer(*id)) {
        if twin.props.prop(&path).is_some() {
            answer.premises.push(fields(json!(twin.id.0), ["attrs.switches.locked".to_owned(), path.clone()]));
        }
    }
}

/// Deleting named layers (edit.rs:196-220, edit.cut through delete) unparents and compensates
/// their children and drops mattes on them. Each deleted layer is held whole (`~dependents`: the
/// layers whose parent or matte it is, so a layer tied to it after the read refuses) but not its
/// stack index; each child whole (its transform is recomputed and assigned), each matted layer's
/// matte, and the deleted layer's ancestors, whose transforms the compensation composes
/// (layer.rs parent_fixes). No other layer is named: a layer added since, or an unrelated
/// reparenting, does not refuse the deletion.
fn clear_inputs(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    if !matches!(command, "edit.clear" | "edit.cut") {
        return;
    }
    let Some(values) = params.get("layers") else { return };
    let cid = match params.get("comp").filter(|value| !value.is_null()) {
        Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()),
        None => session.active_comp_id(),
    };
    let Some(comp) = cid.and_then(|cid| session.project.comp(cid)) else { return };
    let targets: Vec<_> = match values {
        Value::Array(values) => values.iter().filter_map(|value| resolve_layer(comp, value)).collect(),
        value => resolve_layer(comp, value).into_iter().collect(),
    };
    let answer = answer.get_or_insert_with(Under::default);
    let mut orphaned = false;
    for layer in comp.layers.iter().filter(|layer| !targets.contains(&layer.id)) {
        if layer.parent.is_some_and(|parent| targets.contains(&parent)) {
            orphaned = true;
            answer.premises.push(Premise::on(json!(layer.id.0)).positions(Vec::<String>::new()));
        }
        if layer.track_matte.as_ref().is_some_and(|matte| targets.contains(&matte.layer)) {
            answer.premises.push(fields(json!(layer.id.0), ["matte", "matteLayer"]));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for target in &targets {
        answer.premises.push(Premise::on(json!(target.0)).positions(Vec::<String>::new()));
        if !orphaned {
            continue;
        }
        let mut current = comp.layer(*target).and_then(|layer| layer.parent);
        while let Some(id) = current {
            if targets.contains(&id) || !seen.insert(id) || seen.len() > 4096 {
                break;
            }
            answer.premises.push(Premise::on(json!(id.0)).positions(Vec::<String>::new()));
            current = comp.layer(id).and_then(|layer| layer.parent);
        }
    }
}

fn selection_comp_inputs(session: &Session, command: &str, answer: &mut Option<Under>) {
    if command != "file.newCompFromSelection" {
        return;
    }
    let answer = answer.get_or_insert_with(Under::default);
    for item in session.state.project_selection.iter().filter_map(|id| session.project.item(*id)) {
        let mut names = vec!["kind", "name", "attrs.label"];
        match &item.kind {
            kition_motion_project::ItemKind::Comp(_) => {
                names.extend(["comp.attrs.width", "comp.attrs.height", "comp.attrs.pixel_aspect", "comp.attrs.frame_rate", "comp.attrs.duration"])
            }
            kition_motion_project::ItemKind::Footage(_) => names.extend([
                "data.kind",
                "data.width",
                "data.height",
                "data.pixel_aspect",
                "data.frame_rate",
                "data.duration",
                "data.has_video",
                "data.loop_count",
            ]),
            kition_motion_project::ItemKind::Solid(_) => names.extend(["data.width", "data.height", "data.pixel_aspect"]),
            kition_motion_project::ItemKind::Folder => {}
        }
        answer.premises.push(fields(json!(item.id.0), names));
    }
}

fn retarget_inputs(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    if !matches!(command, "file.replaceWithSolid" | "file.replaceWithPlaceholder") {
        return;
    }
    let answer = answer.get_or_insert_with(Under::default);
    // retarget_layers scans every composition and source reference. The membership guards
    // retain removed recipients and compositions created since the observation as well.
    answer.within.push(kition_kit::Within {
        parent: json!("project"),
        field: "items".to_owned(),
        from: 0,
        to: session.project.items.len().saturating_add(1),
        ..Default::default()
    });
    let replaced = replaced_item(session, params);
    for item in session.project.items.values() {
        // replace_with_solid and replace_with_placeholder (file_more.rs:876-913) read the
        // replaced item's own data: its size, and a placeholder's frame rate and duration. The
        // SDK takes a named premise instead of its whole-object hold, so the premise names it.
        let names: &[&str] = if Some(item.id) == replaced { &["kind", "data"] } else { &["kind"] };
        answer.premises.push(fields(json!(item.id.0), names.iter().copied()));
        if let Some(comp) = item.as_comp() {
            answer.premises.push(fields(json!(item.id.0), ["~layerMembers"]));
            for layer in &comp.layers {
                answer.premises.push(fields(json!(layer.id.0), ["attrs.source", "sourceItem"]));
            }
        }
    }
    if command == "file.replaceWithSolid"
        && replaced.and_then(|id| session.project.item(id)).is_some_and(|item| item.dimensions().is_none())
        && let Some(cid) = session.active_comp_id()
    {
        let mut names = Vec::new();
        if params.get("width").is_none() {
            names.push("comp.attrs.width");
        }
        if params.get("height").is_none() {
            names.push("comp.attrs.height");
        }
        if !names.is_empty() {
            answer.premises.push(fields(json!(cid.0), names));
        }
    }
}

/// The item a source replacement replaces, as items_p (file_more.rs:34-57) picks it: the first
/// `items` or `item` id, else the selected footage, else the footage of the selected layers.
fn replaced_item(session: &Session, params: &Value) -> Option<ItemId> {
    use kition_motion_project::{ItemKind, LayerSource};
    match params.get("items").or_else(|| params.get("item")) {
        Some(Value::Array(ids)) => return ids.first().and_then(Value::as_u64).map(ItemId),
        Some(Value::Number(id)) => return id.as_u64().map(ItemId),
        _ => {}
    }
    let footage = |id: &ItemId| matches!(session.project.item(*id).map(|item| &item.kind), Some(ItemKind::Footage(_)));
    session.state.project_selection.iter().copied().find(footage).or_else(|| {
        let comp = session.active_comp()?;
        session.state.selected_layers.iter().filter_map(|id| comp.layer(*id)).find_map(|layer| match layer.source {
            LayerSource::Footage { item } => Some(item),
            _ => None,
        })
    })
}

pub(crate) fn check(session: &Session, command: &str, params: &Value) -> Result<(), String> {
    if matches!(command, "file.replaceWithSolid" | "file.replaceWithPlaceholder") {
        let mut objects = session.project.items.len();
        for item in session.project.items.values() {
            if let Some(comp) = item.as_comp() {
                objects = objects.saturating_add(comp.layers.len());
            }
        }
        if objects > 65_536 {
            return Err("too many source users to guard; split or simplify the project".to_owned());
        }
    }
    if matches!(command, "edit.duplicate" | "layer.updateMarkersFromSource") {
        let count = params.get("layers").and_then(Value::as_array).map_or(session.state.selected_layers.len(), Vec::len);
        if count > 4096 {
            return Err("too many layers to guard this operation; split the selection".to_owned());
        }
    }
    Ok(())
}
