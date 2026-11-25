//! Inputs the r2 object comparison found read and not held: source items, composition
//! attributes, other layers, project settings and the render queue a command's result depends
//! on. Each hold names the engine lines it follows; a fallback input is held only when the
//! fallback is taken.
use kition_kit::{Premise, Under};
use kition_motion_engine::{Session, commands::resolve_layer};
use kition_motion_project::{Comp, ItemId, ItemKind, Layer, LayerId, LayerSource};
use serde_json::{Value, json};

fn field(id: impl Into<Value>, names: impl IntoIterator<Item = impl Into<String>>) -> Premise {
    Premise::on(id).fields(names).positions(Vec::<String>::new())
}

/// The composition a step acts in: `comp` by id or name, else the active one.
fn comp_of<'a>(session: &'a Session, params: &Value) -> Option<(ItemId, &'a Comp)> {
    let cid =
        match params.get("comp").filter(|value| !value.is_null()) {
            Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()).or_else(|| {
                session.project.items.values().find(|item| value.as_str() == Some(item.name.as_str()) && item.as_comp().is_some()).map(|item| item.id)
            }),
            None => session.active_comp_id(),
        }?;
    Some((cid, session.project.comp(cid)?))
}

/// `layers` (array), else `layer`, else the selection (commands/mod.rs layers_p).
fn layers_of(session: &Session, comp: &Comp, params: &Value) -> Vec<LayerId> {
    match params.get("layers") {
        Some(Value::Array(values)) => values.iter().filter_map(|value| resolve_layer(comp, value)).collect(),
        _ => match params.get("layer").filter(|value| !value.is_null()) {
            Some(value) => resolve_layer(comp, value).into_iter().collect(),
            None => session.state.selected_layers.iter().copied().filter(|id| comp.layer(*id).is_some()).collect(),
        },
    }
}

/// `layer`, else the first selected layer (commands/mod.rs layer_p).
fn layer_of(session: &Session, comp: &Comp, params: &Value) -> Option<LayerId> {
    match params.get("layer") {
        Some(value) => resolve_layer(comp, value),
        None => session.state.selected_layers.first().copied().filter(|id| comp.layer(*id).is_some()),
    }
}

/// The size `render::source_size` reads for a layer: its source item's kind and dimensions
/// (eval.rs:327-336, `Item::dimensions` lib.rs:1163-1169). Answers whether the size is zero wide,
/// the case `effect_bounds` (eval.rs:317-325) falls back to the composition's frame for.
fn source_size(session: &Session, layer: &Layer, answer: &mut Under) -> bool {
    match &layer.source {
        LayerSource::Footage { item } | LayerSource::Comp { item } | LayerSource::Solid { item } => {
            let Some(source) = session.project.item(*item) else { return true };
            let (names, width): (&[&str], u32) = match &source.kind {
                ItemKind::Comp(comp) => (&["kind", "comp.attrs.width", "comp.attrs.height"], comp.width),
                ItemKind::Footage(footage) => (&["kind", "data.has_video", "data.width", "data.height"], if footage.has_video { footage.width } else { 0 }),
                ItemKind::Solid(solid) => (&["kind", "data.width", "data.height"], solid.width),
                ItemKind::Folder => (&["kind"], 0),
            };
            answer.premises.push(field(json!(item.0), names.iter().copied()));
            width == 0
        }
        LayerSource::Null => false,
        _ => true,
    }
}

pub(crate) fn extend(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    let Some((cid, comp)) = comp_of(session, params) else { return };
    let mut found = Under::default();
    effect_inputs(session, command, params, cid, comp, &mut found);
    camera_tool_inputs(session, command, params, cid, comp, &mut found);
    arrange_inputs(session, command, params, comp, &mut found);
    frame_inputs(session, command, params, cid, comp, &mut found);
    setting_inputs(session, command, params, cid, comp, &mut found);
    item_inputs(session, command, params, comp, &mut found);
    select_inputs(command, params, comp, &mut found);
    parent_inputs(session, command, params, cid, comp, &mut found);
    template_props_inputs(session, command, params, cid, comp, &mut found);
    naming_inputs(session, command, params, cid, comp, &mut found);
    stack_view_inputs(session, command, params, cid, comp, &mut found);
    expression_inputs(session, command, params, comp, &mut found);
    history_inputs(session, command, params, &mut found);
    render_inputs(session, command, params, cid, comp, &mut found);
    footage_inputs(session, command, params, comp, &mut found);
    source_inputs(session, command, params, cid, comp, &mut found);
    instance_inputs(session, command, params, comp, &mut found);
    reveal_inputs(session, command, cid, comp, &mut found);
    geometry_inputs(session, command, params, cid, comp, &mut found);
    job_inputs(session, command, &mut found);
    if found.premises.is_empty() && found.within.is_empty() {
        return;
    }
    let answer = answer.get_or_insert_with(Under::default);
    answer.premises.extend(found.premises);
    answer.within.extend(found.within);
}

/// Whether an effect's defaults are sized by the layer: `effects::default_value` scales only a
/// Point parameter's fractional default (effects lib.rs:565-570); a zero point and a value the
/// step gives explicitly do not depend on the size.
fn sized_defaults(spec: &kition_motion_engine::effects::EffectSpec, given: Option<&Value>) -> bool {
    spec.params.iter().any(|parameter| {
        matches!((&parameter.ui, &parameter.default), (kition_motion_project::ParamUi::Point, kition_motion_engine::keyframe::Value::Vec2(point)) if point.iter().any(|value| *value != 0.0))
            && given.and_then(|given| given.get(parameter.id)).is_none()
    })
}

/// effect.apply / effect.applyLast size a new effect's point defaults by `effect_bounds`, and
/// effect.reset sizes the defaults it restores the same way (effect.rs:160-163, 487-488): the
/// source item's dimensions, or the composition's frame for a zero-wide source. Only effects
/// with sized defaults read them.
fn effect_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    let targets: Vec<LayerId> = match command {
        "effect.apply" | "effect.applyLast" => {
            let name =
                if command == "effect.apply" { params.get("effect").and_then(Value::as_str).map(str::to_owned) } else { session.state.last_effect.clone() };
            let Some(spec) = name.as_deref().and_then(kition_motion_engine::effects::lookup) else { return };
            if !sized_defaults(spec, params.get("params")) {
                return;
            }
            layers_of(session, comp, params)
        }
        "effect.reset" => {
            let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
            let Some(effects) = layer.effects() else { return };
            let group = match params.get("effect") {
                Some(Value::Number(number)) => number.as_u64().and_then(|n| {
                    effects
                        .groups()
                        .find(|group| group.uid == n)
                        .or_else(|| usize::try_from(n).ok()?.checked_sub(1).and_then(|index| effects.groups().nth(index)))
                }),
                Some(Value::String(name)) => effects.groups().find(|group| group.name == *name || group.match_id == *name),
                _ => None,
            };
            let spec = group.and_then(|group| match &group.kind {
                kition_motion_project::GroupKind::Effect { effect } => kition_motion_engine::effects::find(effect),
                _ => None,
            });
            if !spec.is_some_and(|spec| sized_defaults(spec, None)) {
                return;
            }
            vec![layer.id]
        }
        _ => return,
    };
    let mut frame = false;
    for layer in targets.iter().filter_map(|id| comp.layer(*id)) {
        frame |= source_size(session, layer, answer);
    }
    if frame {
        answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height"]));
    }
}

/// The earlier layers `Comp::active_camera` (lib.rs:448-450) passes over to find the camera: each
/// one's source kind, video switch and in/out points, and the stack above it. A camera added or
/// switched on above it, or one whose span now covers the time, would be found instead.
fn active_camera_scan(comp: &Comp, cid: ItemId, camera: LayerId, answer: &mut Under) {
    let Some(index) = comp.layers.iter().position(|layer| layer.id == camera) else { return };
    answer.within.push(kition_kit::Within { parent: json!(cid.0), field: "comp.layers".to_owned(), from: 0, to: index, ..Default::default() });
    for layer in comp.layers.iter().take(index.saturating_add(1)) {
        answer.premises.push(Premise::on(json!(layer.id.0)).fields(["attrs.source", "switches.video"]).positions(["inPoint", "outPoint"]));
    }
}

/// camera.orbit / camera.pan / camera.dolly on the active-camera branch (three_d.rs:450-466):
/// the composition's width and height enter only as fallbacks for a camera with no Position or
/// Point of Interest, so they are held only then. With `layer`
/// omitted the camera is the active one, found by scanning the stack.
fn camera_tool_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    if !matches!(command, "camera.orbit" | "camera.pan" | "camera.dolly") {
        return;
    }
    let camera = match params.get("layer") {
        Some(value) => resolve_layer(comp, value).and_then(|id| comp.layer(id)).filter(|layer| layer.is_camera()),
        None => {
            let found = comp.active_camera(session.time_of(cid));
            if let Some(camera) = found {
                active_camera_scan(comp, cid, camera.id, answer);
            }
            found
        }
    };
    let Some(camera) = camera else { return };
    // A two-node camera has a Point of Interest by definition (camera.rs:168-170), so only a
    // missing Position takes the comp-size fallback.
    if camera.props.prop("transform/position").is_none() {
        answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height"]));
    }
}

/// layer.arrange to front or back (layer.rs:990-1006) removes the picked layers and inserts them
/// at index 0 or at the end: one layer's own index does not enter the result, so it is held
/// without its stack position. Several layers keep their relative order, so their positions stay
/// held.
fn arrange_inputs(session: &Session, command: &str, params: &Value, comp: &Comp, answer: &mut Under) {
    let how = params.get("to").and_then(Value::as_str).unwrap_or("front");
    if command != "layer.arrange" || params.get("index").is_some() || params.get("above").is_some() || !matches!(how, "front" | "back") {
        return;
    }
    if let [target] = layers_of(session, comp, params).as_slice() {
        answer.premises.push(Premise::on(json!(target.0)).positions(Vec::<String>::new()));
    }
}

/// Commands that size what they make by the target's source, falling back to the composition's
/// frame for a zero-wide source, and that snap or count time in the composition's frames:
/// liquify.stroke (liquify.rs:63, 69), paint.stroke (paint.rs:56, 131), puppet.addPin
/// (puppet.rs:323, 328), light.sheen (promo_light.rs:198, 205-206; promo_ease.rs:374), and the
/// tracker commands sized by `layer_size` (track.rs:152-161), which read no frame rate.
fn frame_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    let rate = match command {
        "liquify.stroke" | "paint.stroke" | "puppet.addPin" | "light.sheen" => true,
        "track.motion" | "track.stabilize" | "track.new" | "track.reset" => false,
        _ => return,
    };
    let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
    let mut names = Vec::new();
    if source_size(session, layer, answer) {
        names.extend(["comp.attrs.width", "comp.attrs.height"]);
    }
    if rate {
        names.push("comp.attrs.frame_rate");
    }
    if !names.is_empty() {
        answer.premises.push(field(json!(cid.0), names));
    }
}

/// Composition attributes a setting reads only for some parameters.
/// - prop.reset (prop.rs:643-665, layer.rs:1319-1331): an effect parameter's default is sized by
///   `effect_bounds`; a Transform default reads the source size (Anchor Point) or the frame
///   (Position and its dimensions); the rest are constants. A given `default` is used instead.
/// - layer.cameraSettings (three_d.rs:50-72, 77-98): the frame width converts a zoom preset,
///   focal length, angle of view or f-stop; with layer.lightSettings (three_d.rs:238-241), a
///   `frame` argument without `time` is in the frame rate (commands/mod.rs:568-576).
/// - layer.enableTimeRemap (layer_time.rs:83, 113-117): the frame duration, for a reversed layer
///   whose remapping is turned on.
/// - file.replaceWithSolid (file_more.rs:126-127, 898-908): the active frame sizes the solid only
///   where the replaced item has no dimensions of its own and the size is not given.
fn setting_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    let mut names: Vec<&str> = Vec::new();
    match command {
        "prop.reset" => {
            let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
            let uid = match (params.get("prop").and_then(Value::as_u64), params.get("path").and_then(Value::as_str)) {
                (Some(uid), _) => Some(uid),
                (None, Some(path)) => layer.props.prop(path).map(|property| property.uid),
                _ => None,
            };
            let Some(property) = uid.and_then(|uid| layer.props.find(uid)) else { return };
            let in_effect = layer.props.node_chain(property.uid).is_some_and(|chain| {
                chain.into_iter().any(
                    |node| matches!(node, kition_motion_project::Node::Group(group) if matches!(group.kind, kition_motion_project::GroupKind::Effect { .. })),
                )
            });
            if in_effect {
                if source_size(session, layer, answer) {
                    names.extend(["comp.attrs.width", "comp.attrs.height"]);
                }
            } else if !matches!(layer.source, LayerSource::Camera | LayerSource::Light { .. }) {
                match property.match_id.as_str() {
                    "anchor" => {
                        source_size(session, layer, answer);
                    }
                    "position" | "positionX" | "positionY" => names.extend(["comp.attrs.width", "comp.attrs.height"]),
                    _ => {}
                }
            }
        }
        "layer.cameraSettings" | "layer.lightSettings" => {
            if command == "layer.cameraSettings" && ["preset", "focalLength", "angleOfView", "fStop"].iter().any(|name| params.get(*name).is_some()) {
                names.push("comp.attrs.width");
            }
            // time_p (commands/mod.rs:568-576): a `frame` without a `time` counts frames.
            if params.get("frame").is_some() && params.get("time").is_none() {
                names.push("comp.attrs.frame_rate");
            }
        }
        "layer.enableTimeRemap" => {
            // The frame duration (layer_time.rs:83) enters only the keys of a reversed layer
            // whose remapping is turned on (layer_time.rs:113-117).
            let want = params.get("value").and_then(Value::as_bool);
            let reversed_on = layers_of(session, comp, params).iter().filter_map(|id| comp.layer(*id)).any(|layer| {
                let has = layer.props.get("timeRemap").is_some();
                want.unwrap_or(!has) && !has && layer.layer_time(layer.in_point) > layer.layer_time(layer.out_point)
            });
            if reversed_on {
                names.push("comp.attrs.frame_rate");
            }
        }
        "file.replaceWithSolid" => {
            // items_p (file_more.rs:51-57): `items` or `item` by id, else the selected footage.
            let item = match params.get("items").or_else(|| params.get("item")) {
                Some(Value::Array(values)) => values.first().and_then(Value::as_u64).map(ItemId),
                Some(value) => value.as_u64().map(ItemId),
                None => session
                    .state
                    .project_selection
                    .iter()
                    .copied()
                    .find(|id| session.project.item(*id).is_some_and(|item| matches!(item.kind, ItemKind::Footage(_))))
                    .or_else(|| {
                        session.state.selected_layers.iter().filter_map(|id| comp.layer(*id)).find_map(|layer| match layer.source {
                            LayerSource::Footage { item } => Some(item),
                            _ => None,
                        })
                    }),
            };
            let sized = item.and_then(|id| session.project.item(id)).and_then(|item| item.dimensions()).is_some();
            if !sized
                && (params.get("width").is_none() || params.get("height").is_none())
                && let Some(active) = session.active_comp_id()
            {
                answer.premises.push(field(json!(active.0), ["comp.attrs.width", "comp.attrs.height"]));
            }
            return;
        }
        _ => return,
    }
    if !names.is_empty() {
        answer.premises.push(field(json!(cid.0), names));
    }
}

/// The project's item list as a window: an item created or deleted since refuses.
fn item_window(session: &Session, answer: &mut Under) {
    answer.within.push(kition_kit::Within {
        parent: json!("project"),
        field: "items".to_owned(),
        from: 0,
        to: session.project.items.len().saturating_add(1),
        ..Default::default()
    });
}

/// Every item's name (and kind), and the item list: a search for a free or a matching name
/// depends on all of them, including items created since.
fn item_names(session: &Session, answer: &mut Under) {
    item_window(session, answer);
    for item in session.project.items.values() {
        answer.premises.push(field(json!(item.id.0), ["kind", "name"]));
    }
}

/// `Project::folder_named` (lib.rs:1266-1268): the first folder by id with that name, or a new
/// one. What decides it is the kind and name of every item up to the one found, or of every
/// item and the list when none is found.
fn folder_lookup(session: &Session, name: &str, answer: &mut Under) {
    let found = session.project.folder_named(name);
    if found.is_none() {
        item_window(session, answer);
    }
    for item in session.project.items.values().filter(|item| found.is_none_or(|found| item.id <= found)) {
        answer.premises.push(field(json!(item.id.0), ["kind", "name"]));
    }
}

/// Item reads of commands that make or show items:
/// - comp.new picks the first free name among every item's (comp.rs:110-115).
/// - New solids file into the first folder named Solids (layer.rs:87-101, file_more.rs:142-152,
///   create.rs:116-118 masks from text, promo_light.rs:138 spotlight).
/// - layer.openSource / layer.revealSource show the layer's source item (layer_menu.rs:695, 721):
///   its kind.
/// - project.rename names the selected item when `item` is omitted (project_items.rs:68-77): its
///   name is the field it assigns.
/// - The Footage panel's time, In and Out clamp to the shown item's duration and frame rate, or
///   the active composition's frame rate (footage_panel.rs:42-48, 88-112).
fn item_inputs(session: &Session, command: &str, params: &Value, comp: &Comp, answer: &mut Under) {
    match command {
        "comp.new" => item_names(session, answer),
        "layer.newSolid" | "layer.newAdjustment" | "file.importSolid" | "light.spotlight" => folder_lookup(session, "Solids", answer),
        "layer.add" if params.get("op").and_then(Value::as_str) == Some("masksFromText") => folder_lookup(session, "Solids", answer),
        "layer.openSource" | "layer.revealSource" => {
            if let Some(item) = layer_of(session, comp, params).and_then(|id| comp.layer(id)).and_then(|layer| layer.source.item()) {
                answer.premises.push(field(json!(item.0), ["kind"]));
            }
        }
        "project.rename" if params.get("item").is_none() => {
            if let Some(item) = session.state.project_selection.first() {
                answer.premises.push(field(json!(item.0), ["name"]));
            }
        }
        "footage.setTime" | "footage.setIn" | "footage.setOut" => {
            let Some(view) = &session.state.footage_panel else { return };
            match session.project.item(view.item).map(|item| &item.kind) {
                Some(ItemKind::Footage(_)) => answer.premises.push(field(json!(view.item.0), ["kind", "data.kind", "data.duration", "data.frame_rate"])),
                _ => {
                    answer.premises.push(field(json!(view.item.0), ["kind"]));
                    if let Some(active) = session.active_comp_id() {
                        answer.premises.push(field(json!(active.0), ["comp.attrs.frame_rate"]));
                    }
                }
            }
        }
        _ => {}
    }
}

/// layer.select resolves the layers it names (layer.rs:301-308): a named layer deleted since
/// would drop out of the selection, so each one's existence is held, and nothing else of it.
fn select_inputs(command: &str, params: &Value, comp: &Comp, answer: &mut Under) {
    if command != "layer.select" {
        return;
    }
    let ids: Vec<LayerId> = match params.get("layers").or_else(|| params.get("layer")) {
        Some(Value::Array(values)) => values.iter().filter_map(|value| resolve_layer(comp, value)).collect(),
        Some(value) => resolve_layer(comp, value).into_iter().collect(),
        None => Vec::new(),
    };
    for id in ids {
        answer.premises.push(Premise::on(json!(id.0)).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
    }
}

/// layer.setParent (layer.rs:784-845): check_parent_chain walks the new parent's ancestors (each
/// one's parent link), and compensation re-expresses each target in the new parent's space from
/// its old parent's (parent_fixes, layer.rs:711-760): both world matrices, through each chain
/// layer's transform and source pixel aspect and the composition's pixel aspect.
fn parent_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    if command != "layer.setParent" {
        return;
    }
    let targets = layers_of(session, comp, params);
    let parent = match params.get("parent") {
        None | Some(Value::Null) => None,
        Some(value) => resolve_layer(comp, value),
    };
    let mut seen = std::collections::HashSet::new();
    let mut current = parent.and_then(|id| comp.layer(id)).and_then(|layer| layer.parent);
    while let Some(id) = current {
        if !seen.insert(id) || seen.len() > 4096 {
            break;
        }
        answer.premises.push(field(json!(id.0), ["parent"]));
        current = comp.layer(id).and_then(|layer| layer.parent);
    }
    if params.get("compensate").and_then(Value::as_bool) == Some(false) {
        return;
    }
    if let Some(parent) = parent {
        crate::rest_geometry::matrix(session, cid, comp, parent, answer);
    }
    for old in targets.iter().filter_map(|id| comp.layer(*id)).filter_map(|layer| layer.parent).filter(|old| Some(*old) != parent) {
        crate::rest_geometry::matrix(session, cid, comp, old, answer);
    }
}

/// Graphics Templates edits rebuild every instance of the edited composition through
/// `sync_project` (project template_props.rs:596-642): each layer whose source is that composition
/// gets its `templateProps` property group rebuilt from the controls and its own overrides, or
/// inserted at the top of its properties. The property a control is made from or linked to is
/// read by value, name and timing (engine template_props.rs addProperty / linkProperty).
fn template_props_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    let syncs = matches!(
        command,
        "templateProps.addProperty"
            | "templateProps.addMirror"
            | "templateProps.linkProperty"
            | "templateProps.addGroup"
            | "templateProps.rename"
            | "templateProps.remove"
            | "templateProps.move"
    );
    if !syncs {
        return;
    }
    let template_comp = if params.get("comp").is_some() { Some(cid) } else { session.state.template_props_primary.or(Some(cid)) };
    template_props_instances(session, template_comp, answer);
    if matches!(command, "templateProps.addProperty" | "templateProps.linkProperty") {
        template_props_property(comp, params, answer);
    }
}

/// templateProps.set / revert / pushToComp edit an instance layer (`layer`) of a Graphics
/// Templates composition and sync every instance of it (engine template_props.rs:641-728,
/// project template_props.rs:596-642): that composition's instances, and pushToComp writes the
/// source composition's controlled properties (held whole as the instance's source item).
fn instance_inputs(session: &Session, command: &str, params: &Value, comp: &Comp, answer: &mut Under) {
    if !matches!(command, "templateProps.set" | "templateProps.revert" | "templateProps.pushToComp") {
        return;
    }
    let Some(instance) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
    let LayerSource::Comp { item } = instance.source else { return };
    template_props_instances(session, Some(item), answer);
    // The edited instance stays held whole, as its own target.
    answer.premises.push(Premise::on(json!(instance.id.0)));
    if let Some(source) = session.project.comp(item) {
        answer.premises.push(field(json!(item.0), ["kind", "comp.attrs"]));
        for layer in &source.layers {
            answer.premises.push(Premise::on(json!(layer.id.0)).positions(Vec::<String>::new()));
        }
    }
}

fn template_props_instances(session: &Session, template_comp: Option<ItemId>, answer: &mut Under) {
    let Some(template_comp) = template_comp else { return };
    for (_, other) in session.project.comps() {
        for layer in &other.layers {
            if !matches!(layer.source, LayerSource::Comp { item } if item == template_comp) {
                continue;
            }
            let group = layer.props.children.iter().find(|node| node.match_id() == kition_motion_project::template_props::GROUP).map(|node| node.uid());
            let path = group.and_then(|uid| layer.props.match_path_of(uid)).unwrap_or_else(|| "props.children".to_owned());
            answer.premises.push(field(json!(layer.id.0), [path, "attrs.source".to_owned(), "sourceItem".to_owned()]));
        }
    }
}

fn template_props_property(comp: &Comp, params: &Value, answer: &mut Under) {
    if let Some(layer) = params.get("layer").and_then(|value| resolve_layer(comp, value)).and_then(|id| comp.layer(id)) {
        let path = match (params.get("prop").and_then(Value::as_u64), params.get("path").and_then(Value::as_str)) {
            (Some(uid), _) => layer.props.match_path_of(uid),
            (None, Some(path)) => layer.props.prop(path).and_then(|property| layer.props.match_path_of(property.uid)),
            _ => None,
        };
        if let Some(path) = path {
            answer.premises.push(Premise::on(json!(layer.id.0)).fields([path, "name".to_owned(), "attrs.stretch".to_owned()]).positions(["startTime"]));
        }
    }
}

/// Every layer's name in the composition and its membership: `Comp::unique_layer_name`
/// (project lib.rs:452-458) picks the first free name among them.
fn sibling_names(cid: ItemId, comp: &Comp, answer: &mut Under) {
    answer.premises.push(field(json!(cid.0), ["~layerMembers"]));
    for layer in &comp.layers {
        answer.premises.push(field(json!(layer.id.0), ["name"]));
    }
}

/// Every layer's time span in the composition and its membership.
fn spans(cid: ItemId, comp: &Comp, answer: &mut Under) {
    answer.premises.push(field(json!(cid.0), ["~layerMembers"]));
    for layer in &comp.layers {
        answer.premises.push(Premise::on(json!(layer.id.0)).fields(Vec::<String>::new()).positions(["startTime", "inPoint", "outPoint"]));
    }
}

/// Commands that name a new layer uniquely among its siblings, or that rewrite every layer:
/// - layer.add (create.rs:95-118, vector.rs), footage.overlayEdit (build.rs:834),
///   layer.applyTextPreset's digit layers (promo_text.rs:544), camera.orbitNull's null
///   (rig3d.rs:326-328, with the frame and duration it is built in), a light whose kind changes
///   and keeps its name (three_d.rs:239-298 through build::layer).
/// - transition.iris refers to its layers by name, or by stack index when names repeat, and
///   names its ring uniquely (promo_carry.rs:215-237): every name and the stack above them.
/// - footage.rippleInsertEdit shifts every later layer and splits the ones crossing the edit
///   (footage_panel.rs:169; edit.rs:111): every layer's span.
/// - file.replaceFootage retargets every layer using the item (file_more.rs:843-863).
fn naming_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    match command {
        "layer.add" => sibling_names(cid, comp, answer),
        "layer.applyTextPreset" if params.get("preset").and_then(Value::as_str) == Some("digitRoll") => sibling_names(cid, comp, answer),
        "footage.overlayEdit" => {
            // A new layer of the shown item, built in the frame and duration at the frame rate.
            sibling_names(cid, comp, answer);
            answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration", "comp.attrs.frame_rate"]));
        }
        "camera.orbitNull" => {
            sibling_names(cid, comp, answer);
            answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration"]));
        }
        "layer.lightSettings" if params.get("kind").is_some() && params.get("name").is_none() => {
            sibling_names(cid, comp, answer);
            // The light itself is rebuilt whole (three_d.rs:270-292).
            if let Some(light) = layer_of(session, comp, params) {
                answer.premises.push(Premise::on(json!(light.0)));
            }
        }
        "transition.iris" => {
            sibling_names(cid, comp, answer);
            let named = ["layer", "center"].iter().filter_map(|name| params.get(*name)).filter_map(|value| resolve_layer(comp, value));
            let deepest = named.filter_map(|id| comp.layers.iter().position(|layer| layer.id == id)).max();
            if let Some(index) = deepest {
                answer.within.push(kition_kit::Within {
                    parent: json!(cid.0),
                    field: "comp.layers".to_owned(),
                    from: 0,
                    to: index.saturating_add(1),
                    ..Default::default()
                });
            }
        }
        "footage.rippleInsertEdit" => {
            // Every later layer moves, a crossing layer is copied whole (footage_panel.rs:151-175),
            // and the new layer is named among the rest.
            spans(cid, comp, answer);
            sibling_names(cid, comp, answer);
            answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration", "comp.attrs.frame_rate"]));
            let at = params.get("at").and_then(Value::as_f64).map_or_else(|| session.time_of(cid), kition_motion_time::Tick::from_seconds_f64);
            for layer in comp.layers.iter().filter(|layer| layer.in_point < at && layer.out_point > at) {
                answer.premises.push(Premise::on(json!(layer.id.0)));
            }
        }
        "file.replaceFootage" => {
            for (id, other) in session.project.comps() {
                answer.premises.push(field(json!(id.0), ["~layerMembers"]));
                for layer in &other.layers {
                    answer.premises.push(field(json!(layer.id.0), ["attrs.source", "sourceItem"]));
                }
            }
        }
        _ => {}
    }
}

/// The 3D view commands that move the active camera to frame layers in the active-camera view
/// (three_d.rs:358-383, 533-637): the camera found by scanning the stack, the frame, and for the framed layers their
/// content bounds (source size) and world matrices.
fn stack_view_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    // A custom or orthographic view moves only the editor's view camera (three_d.rs:340-355).
    if session.state.views3d.get(&cid).is_some_and(|views| views.current.id() != "activeCamera") {
        return;
    }
    let framed: Vec<LayerId> = match command {
        "view.reset3DView" => Vec::new(),
        "view.lookAtSelected" => layers_of(session, comp, params),
        "view.lookAtAll" => {
            let t = session.time_of(cid);
            spans(cid, comp, answer);
            comp.layers
                .iter()
                .filter(|layer| !layer.is_camera() && !matches!(layer.source, LayerSource::Light { .. }) && layer.is_active_at(t))
                .map(|layer| layer.id)
                .collect()
        }
        _ => return,
    };
    if let Some(camera) = comp.active_camera(session.time_of(cid)) {
        active_camera_scan(comp, cid, camera.id, answer);
    }
    answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height"]));
    for id in framed {
        if let Some(layer) = comp.layer(id) {
            source_size(session, layer, answer);
            crate::rest_geometry::matrix(session, cid, comp, id, answer);
        }
    }
}

/// An enabled expression may read any layer of any composition and any item by name or index
/// (the expression host, render eval.rs:252): what it reads is not known before it runs, so
/// every layer (without its stack position), each composition's attributes and membership, and
/// every other item are held.
fn expression_closure(session: &Session, answer: &mut Under) {
    item_window(session, answer);
    for item in session.project.items.values() {
        if let Some(comp) = item.as_comp() {
            answer.premises.push(field(json!(item.id.0), ["kind", "name", "comp.attrs", "~layerMembers"]));
            for layer in &comp.layers {
                answer.premises.push(Premise::on(json!(layer.id.0)).positions(Vec::<String>::new()));
            }
        } else {
            answer.premises.push(Premise::on(json!(item.id.0)).positions(Vec::<String>::new()));
        }
    }
}

fn has_expression(property: Option<&kition_motion_project::Property>) -> bool {
    property.is_some_and(kition_motion_project::Property::has_expression)
}

/// Commands that evaluate a property through its expression: prop.convertExpressionToKeyframes
/// samples it at every frame (prop.rs:13, 107), freezeFrame / freezeOnLastFrame evaluate Time
/// Remap (layer_time.rs:163), light.controlWithCamera bakes the light's world frame from its and
/// the camera's transforms (rig3d.rs:375-401). The closure is held only when an expression is
/// enabled on what they evaluate.
fn expression_inputs(session: &Session, command: &str, params: &Value, comp: &Comp, answer: &mut Under) {
    let evaluated = match command {
        "prop.convertExpressionToKeyframes" => {
            let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
            match (params.get("prop").and_then(Value::as_u64), params.get("path").and_then(Value::as_str)) {
                (Some(uid), _) => has_expression(layer.props.find(uid)),
                (None, Some(path)) => has_expression(layer.props.prop(path)),
                _ => session.state.selected_props.iter().filter(|(id, _)| *id == layer.id).any(|(_, uid)| has_expression(layer.props.find(*uid))),
            }
        }
        "layer.freezeFrame" | "layer.freezeOnLastFrame" => {
            // The source time shown (render eval.rs:252): a precomposition's protected markers
            // and duration, footage's timing.
            for layer in layers_of(session, comp, params).iter().filter_map(|id| comp.layer(*id)) {
                match &layer.source {
                    LayerSource::Comp { item } => answer.premises.push(field(json!(item.0), ["kind", "comp.markers", "comp.attrs.duration"])),
                    LayerSource::Footage { item } => answer.premises.push(field(json!(item.0), ["kind", "data"])),
                    _ => {}
                }
            }
            layers_of(session, comp, params).iter().filter_map(|id| comp.layer(*id)).any(|layer| has_expression(layer.props.get("timeRemap")))
        }
        "light.controlWithCamera" => {
            let mut layers = layers_of(session, comp, params);
            layers.extend(params.get("camera").and_then(|value| resolve_layer(comp, value)));
            layers
                .iter()
                .filter_map(|id| comp.layer(*id))
                .any(|layer| layer.transform().is_some_and(|group| group.props().any(|property| property.has_expression())))
        }
        _ => return,
    };
    if evaluated {
        expression_closure(session, answer);
    }
}

/// The render queue as a window, and each entry whole.
fn queue_window(session: &Session, answer: &mut Under) {
    answer.within.push(kition_kit::Within {
        parent: json!("project"),
        field: "renderQueue".to_owned(),
        from: 0,
        to: session.project.render_queue.len().saturating_add(1),
        ..Default::default()
    });
    for entry in &session.project.render_queue {
        answer.premises.push(Premise::on(json!(format!("render:{}", entry.id))));
    }
}

/// Everything in the document, for a step whose writes cannot be known before it runs.
fn whole_document(session: &Session, answer: &mut Under) {
    answer.premises.push(Premise::on("project"));
    item_window(session, answer);
    queue_window(session, answer);
    for item in session.project.items.values() {
        answer.premises.push(Premise::on(json!(item.id.0)).positions(Vec::<String>::new()));
        if let Some(comp) = item.as_comp() {
            answer.premises.push(field(json!(item.id.0), ["~layerMembers"]));
            for layer in &comp.layers {
                answer.premises.push(Premise::on(json!(layer.id.0)));
            }
        }
    }
}

/// What differs between the document now and a history state: the fields an undo or redo to
/// that state assigns.
fn differences(now: &kition_motion_project::Project, then: &kition_motion_project::Project, session: &Session, answer: &mut Under) {
    let mut project = Vec::new();
    if now.settings != then.settings {
        project.push("attrs.settings");
    }
    if now.render_templates != then.render_templates {
        project.push("attrs.render_templates");
    }
    if !project.is_empty() {
        answer.premises.push(field("project", project));
    }
    if now.render_queue != then.render_queue {
        queue_window(session, answer);
    }
    if now.items.len() != then.items.len() || now.items.keys().zip(then.items.keys()).any(|(a, b)| a != b) {
        item_window(session, answer);
    }
    for (id, item) in &now.items {
        let Some(old) = then.items.get(id) else { continue };
        if item == old {
            continue;
        }
        let mut names = Vec::new();
        if item.name != old.name {
            names.push("name".to_owned());
        }
        if item.parent != old.parent {
            names.push("parent".to_owned());
        }
        if item.label != old.label || item.comment != old.comment || item.proxy != old.proxy {
            names.push("attrs".to_owned());
        }
        match (&item.kind, &old.kind) {
            (ItemKind::Comp(a), ItemKind::Comp(b)) if !std::sync::Arc::ptr_eq(a, b) && a != b => {
                let ids = |comp: &Comp| comp.layers.iter().map(|layer| layer.id).collect::<Vec<_>>();
                if ids(a) != ids(b) {
                    names.push("~layerMembers".to_owned());
                    answer.within.push(kition_kit::Within {
                        parent: json!(id.0),
                        field: "comp.layers".to_owned(),
                        from: 0,
                        to: a.layers.len().saturating_add(1),
                        ..Default::default()
                    });
                }
                for layer in &a.layers {
                    if b.layer(layer.id).is_none_or(|before| before != layer) {
                        answer.premises.push(Premise::on(json!(layer.id.0)).positions(Vec::<String>::new()));
                    }
                }
                let (mut x, mut y) = ((**a).clone(), (**b).clone());
                x.layers.clear();
                y.layers.clear();
                if x != y {
                    names.push("comp.attrs".to_owned());
                }
            }
            (ItemKind::Comp(_), ItemKind::Comp(_)) => {}
            (a, b) if a != b => names.push("data".to_owned()),
            _ => {}
        }
        if !names.is_empty() {
            answer.premises.push(field(json!(id.0), names));
        }
    }
}

/// Undo, redo and history jumps restore a history state, rebased on outside rereads
/// (lib.rs:738-774; sdk_reread.rs outside_reread_keeps_agent_undo_and_the_colleagues_edit): what
/// they assign is what differs between the document now and that state, in the scope's own
/// history. A jump by history id or index holds the whole document.
fn history_inputs(session: &Session, command: &str, params: &Value, answer: &mut Under) {
    // in_scope (kdm_bridge.rs:149-170): the requested scope, else the caller's own; the session
    // keeps the other scope's history aside.
    let own = kition_motion_engine::UndoScope::of(session.origin().kind).unwrap_or(kition_motion_engine::UndoScope::Person);
    let wanted = match params.get("scope").and_then(Value::as_str) {
        Some("person") => kition_motion_engine::UndoScope::Person,
        Some("agent") => kition_motion_engine::UndoScope::Agent,
        _ => own,
    };
    let history = if wanted == session.history_scope { &session.history } else { &session.other_history };
    let pick = |stack: &[(String, std::sync::Arc<kition_motion_project::Project>)], n: u64| {
        usize::try_from(n).ok().filter(|n| *n >= 1).and_then(|n| stack.len().checked_sub(n)).and_then(|index| stack.get(index)).map(|(_, state)| state.clone())
    };
    let target = match command {
        "edit.undo" => pick(&history.undo, 1),
        "edit.redo" => pick(&history.redo, 1),
        "edit.history" => match (params.get("steps").and_then(Value::as_u64), params.get("redo").and_then(Value::as_u64)) {
            (Some(n), _) => pick(&history.undo, n),
            (None, Some(n)) => pick(&history.redo, n),
            _ => return,
        },
        "edit.history.goto" => match params.get("steps").and_then(Value::as_i64) {
            // Steps past the end stop where the history ends (edit.rs:884-889).
            Some(n) if n < 0 => pick(&history.undo, n.unsigned_abs().min(history.undo.len() as u64)),
            Some(n) => pick(&history.redo, n.unsigned_abs().min(history.redo.len() as u64)),
            None => return whole_document(session, answer),
        },
        _ => return,
    };
    // No state to go to: nothing is written.
    if let Some(state) = target {
        differences(&session.project, &state, session, answer);
    }
}

/// The project's color and renderer settings a render reads (render lib.rs:798; the color
/// pipeline's bit depth, linear blending and working space).
fn render_settings(answer: &mut Under) {
    answer.premises.push(field("project", ["attrs.settings"]));
}

/// What drawing one layer's own pixels reads: its source's pixels (precomposition closure,
/// rest_geometry::pixels), its track matte layer, and with an enabled expression anywhere on it
/// the expression closure. Its own masks and effects are part of the layer.
fn layer_pixels(session: &Session, comp: &Comp, layer: &Layer, answer: &mut Under) {
    if let Some(item) = layer.source.item() {
        let _ = crate::rest_geometry::pixels(session, item, answer);
    }
    if let Some(matte) = layer.track_matte.as_ref().and_then(|matte| comp.layer(matte.layer)) {
        answer.premises.push(Premise::on(json!(matte.id.0)).positions(Vec::<String>::new()));
        if let Some(item) = matte.source.item() {
            let _ = crate::rest_geometry::pixels(session, item, answer);
        }
    }
    let mut stack = vec![&layer.props];
    let mut expression = false;
    while let Some(group) = stack.pop() {
        for node in &group.children {
            match node {
                kition_motion_project::Node::Prop(property) => expression |= property.has_expression(),
                kition_motion_project::Node::Group(inner) => stack.push(inner),
            }
        }
    }
    if expression {
        expression_closure(session, answer);
    }
}

/// Commands that render: a composition frame or range written to the host (comp.saveFrameAs*,
/// render.saveCurrentPreview, renderQueue.render: the composition's whole closure and the
/// project's render settings), and analyses that render one layer's pixels over time (picking
/// a colour, content-aware fill, tracking, camera solving, roto propagation, scene detection,
/// auto-trace: that layer's closure and the composition's frame timing). Camera solves file
/// their solids into the Solids folder (camera_cmds.rs:355-370).
fn render_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    match command {
        // With `queue`, saveFrameAs only adds a render queue entry (comp_more.rs:86-102).
        "comp.saveFrameAs" if params.get("queue").and_then(Value::as_bool) == Some(true) => {}
        "comp.saveFrameAs" | "comp.saveFrameAsPsd" | "comp.saveFrameAsExr" | "render.saveCurrentPreview" => {
            let _ = crate::rest_geometry::pixels(session, cid, answer);
            answer.premises.push(Premise::on(json!(cid.0)).positions(Vec::<String>::new()));
            for layer in &comp.layers {
                answer.premises.push(Premise::on(json!(layer.id.0)));
            }
            render_settings(answer);
        }
        // Without parameters it only opens the panel (content_fill.rs:386-393).
        "layer.newContentAwareFill" if params.as_object().is_some_and(serde_json::Map::is_empty) => {}
        // Traced into a new solid layer: named among the siblings, filed into Solids, built in the
        // frame (autotrace.rs:120-140); masks on the layer itself read no composition attribute.
        "layer.autoTrace" => {
            let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
            layer_pixels(session, comp, layer, answer);
            if params.get("applyToNewLayer").and_then(Value::as_bool) == Some(true) {
                sibling_names(cid, comp, answer);
                folder_lookup(session, "Solids", answer);
                answer.premises.push(field(json!(cid.0), ["comp.attrs.pixel_aspect", "comp.attrs.duration", "comp.attrs.width", "comp.attrs.height"]));
            }
        }
        "renderQueue.render" => {
            for entry in &session.project.render_queue {
                let _ = crate::rest_geometry::pixels(session, entry.comp, answer);
            }
            render_settings(answer);
        }
        "effect.pickColor"
        | "contentFill.generate"
        | "layer.newContentAwareFill"
        | "track.analyze"
        | "track.camera"
        | "camera.analyze"
        | "camera.solveStatus"
        | "camera.deletePoints"
        | "roto.propagate"
        | "roto.freeze"
        | "layer.sceneEditDetection" => {
            let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
            layer_pixels(session, comp, layer, answer);
            answer.premises.push(field(json!(cid.0), ["comp.attrs.frame_rate", "comp.attrs.duration", "comp.attrs.width", "comp.attrs.height"]));
            if matches!(command, "track.camera" | "camera.analyze" | "camera.solveStatus" | "camera.deletePoints") {
                render_settings(answer);
                folder_lookup(session, "Solids", answer);
            }
        }
        _ => {}
    }
}

/// Every layer's source in every composition, and each composition's membership.
fn source_users(session: &Session, answer: &mut Under) {
    for (id, other) in session.project.comps() {
        answer.premises.push(field(json!(id.0), ["~layerMembers"]));
        for layer in &other.layers {
            answer.premises.push(field(json!(layer.id.0), ["attrs.source", "sourceItem"]));
        }
    }
}

/// The footage `items` / `item` names, else the selected footage (file_more.rs:34-57).
fn footage_targets(session: &Session, comp: &Comp, params: &Value) -> Vec<ItemId> {
    match params.get("items").or_else(|| params.get("item")) {
        Some(Value::Array(values)) => values.iter().filter_map(Value::as_u64).map(ItemId).collect(),
        Some(value) => value.as_u64().map(ItemId).into_iter().collect(),
        None => {
            let selected: Vec<ItemId> = session
                .state
                .project_selection
                .iter()
                .copied()
                .filter(|id| session.project.item(*id).is_some_and(|item| matches!(item.kind, ItemKind::Footage(_))))
                .collect();
            if selected.is_empty() {
                session
                    .state
                    .selected_layers
                    .iter()
                    .filter_map(|id| comp.layer(*id))
                    .filter_map(|layer| match layer.source {
                        LayerSource::Footage { item } => Some(item),
                        _ => None,
                    })
                    .collect()
            } else {
                selected
            }
        }
    }
}

/// Footage and project-wide item commands:
/// - footage.check / footage.reloadChanged read every footage item's metadata and file
///   (footage_check.rs:38, footage_reload.rs:34); with no `items`, every footage item, so the
///   item list too.
/// - file.rememberInterpretation / file.reloadFootage read the target footage whole
///   (file_more.rs:807, 916).
/// - file.interpretFootage assigns the target's interpretation and retimes every layer using it
///   (file_more.rs:596-758).
/// - file.removeUnusedFootage / consolidateFootage / reduceProject decide from every item's
///   kind, metadata and folder and every layer's source (file_more.rs:241-316).
/// - file.findMissing reads footage, effects and text fonts on every layer (file_more.rs:329).
/// - keyframe.audioToKeyframes reads every audible layer's source audio (keys_more.rs).
fn footage_inputs(session: &Session, command: &str, params: &Value, comp: &Comp, answer: &mut Under) {
    let footage = || session.project.items.values().filter(|item| matches!(item.kind, ItemKind::Footage(_))).map(|item| item.id).collect::<Vec<_>>();
    match command {
        "footage.check" | "footage.reloadChanged" => {
            let targets = if params.get("items").is_some() { footage_targets(session, comp, params) } else { footage() };
            if params.get("items").is_none() {
                item_window(session, answer);
            }
            for id in targets {
                answer.premises.push(field(json!(id.0), ["kind", "data"]));
            }
        }
        "file.rememberInterpretation" | "file.reloadFootage" => {
            for id in footage_targets(session, comp, params).into_iter().take(if command == "file.rememberInterpretation" { 1 } else { usize::MAX }) {
                answer.premises.push(Premise::on(json!(id.0)).positions(Vec::<String>::new()));
            }
        }
        "file.interpretFootage" => {
            let targets = footage_targets(session, comp, params);
            for id in &targets {
                answer.premises.push(field(json!(id.0), ["kind", "data"]));
            }
            for (cid, other) in session.project.comps() {
                answer.premises.push(field(json!(cid.0), ["~layerMembers"]));
                for layer in &other.layers {
                    if layer.source.item().is_some_and(|item| targets.contains(&item)) {
                        answer.premises.push(Premise::on(json!(layer.id.0)).fields(["attrs.source", "sourceItem", "attrs.stretch"]).positions([
                            "startTime",
                            "inPoint",
                            "outPoint",
                        ]));
                    } else {
                        answer.premises.push(field(json!(layer.id.0), ["attrs.source", "sourceItem"]));
                    }
                }
            }
        }
        "file.removeUnusedFootage" | "file.consolidateFootage" | "file.reduceProject" => {
            item_window(session, answer);
            for item in session.project.items.values() {
                answer.premises.push(field(json!(item.id.0), ["kind", "data", "parent"]));
            }
            source_users(session, answer);
        }
        "file.findMissing" => {
            for id in footage() {
                answer.premises.push(field(json!(id.0), ["kind", "data"]));
            }
            for (_, other) in session.project.comps() {
                for layer in &other.layers {
                    answer.premises.push(field(json!(layer.id.0), ["attrs.source", "props"]));
                }
            }
        }
        // Environment discovery scans every composition's layers, their sources and effects
        // (app_more.rs vr_* / vr_editor).
        "comp.vr.setView" => {
            for (id, other) in session.project.comps() {
                // A cube map's width sizes its face cells (vr_editor.rs:28-52): a composition a
                // layer with the VR converter effect shows.
                let cube = session.project.comps().any(|(_, outer)| {
                    outer.layers.iter().any(|layer| {
                        layer.source.item() == Some(*id)
                            && layer.effects().is_some_and(|effects| {
                                effects
                                    .groups()
                                    .any(|group| matches!(&group.kind, kition_motion_project::GroupKind::Effect { effect } if effect == "ec.vr.converter"))
                            })
                    })
                });
                let names: &[&str] = if cube { &["~layerMembers", "comp.attrs.width"] } else { &["~layerMembers"] };
                answer.premises.push(field(json!(id.0), names.iter().copied()));
                for layer in &other.layers {
                    answer.premises.push(field(json!(layer.id.0), ["attrs.source", "sourceItem", "props"]));
                }
            }
        }
        "keyframe.audioToKeyframes" => {
            for layer in &comp.layers {
                if let Some(item) = layer.source.item() {
                    let _ = crate::rest_geometry::pixels(session, item, answer);
                }
            }
        }
        _ => {}
    }
}

/// Source sizes and stack facts of promotion, roto and tracking commands, and the layer
/// insertion of imports into the composition:
/// - camera.move sizes its framing by each target's source (promo_camera.rs:88-172);
///   transition.morphRect / zoomThrough by both layers' sources and world rectangles, and
///   zoomThrough by their relative stack order (promo_carry.rs:59-142, 333-351, 490-590).
/// - roto.stroke and track.extractFaceMeasurements size by the source, the frame for a
///   zero-wide one (roto_cmds.rs:66-110; mask_interp.rs:224-244); track.mask samples the source
///   pixels (mask_interp.rs).
/// - file.import / mediaBrowser.import with `addToComp` add each item through layer.addItem
///   (file.rs:186-215): named among the siblings, built in the frame at its rate.
/// - layer.precompose clears other layers' parents and mattes on the moved layers
///   (layer.rs:1062): those dependents.
fn source_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    let named = |names: &[&str]| -> Vec<LayerId> { names.iter().filter_map(|name| params.get(*name)).filter_map(|value| resolve_layer(comp, value)).collect() };
    match command {
        "camera.move" => {
            let mut targets = named(&["target"]);
            if targets.is_empty() {
                targets = layers_of(session, comp, params);
            }
            for layer in targets.iter().filter_map(|id| comp.layer(*id)) {
                source_size(session, layer, answer);
            }
        }
        "transition.morphRect" | "transition.zoomThrough" => {
            let layers = named(&["from", "to", "layer", "toLayer"]);
            for id in &layers {
                if let Some(layer) = comp.layer(*id) {
                    source_size(session, layer, answer);
                    crate::rest_geometry::matrix(session, cid, comp, *id, answer);
                }
            }
            let indices: Vec<usize> = layers.iter().filter_map(|id| comp.layers.iter().position(|layer| layer.id == *id)).collect();
            if let (Some(from), Some(to)) = (indices.iter().min(), indices.iter().max()) {
                answer.within.push(kition_kit::Within {
                    parent: json!(cid.0),
                    field: "comp.layers".to_owned(),
                    from: *from,
                    to: to.saturating_add(1),
                    ..Default::default()
                });
            }
        }
        "roto.stroke" | "track.extractFaceMeasurements" => {
            if let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id))
                && source_size(session, layer, answer)
            {
                answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height"]));
            }
        }
        "track.mask" => {
            if let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) {
                layer_pixels(session, comp, layer, answer);
            }
        }
        "file.import" | "mediaBrowser.import" if params.get("addToComp").and_then(Value::as_bool) == Some(true) => {
            sibling_names(cid, comp, answer);
            answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration", "comp.attrs.frame_rate"]));
        }
        "layer.precompose" => {
            let targets = layers_of(session, comp, params);
            for layer in comp.layers.iter().filter(|layer| !targets.contains(&layer.id)) {
                let tied = layer.parent.is_some_and(|parent| targets.contains(&parent))
                    || layer.track_matte.as_ref().is_some_and(|matte| targets.contains(&matte.layer));
                if tied {
                    answer.premises.push(field(json!(layer.id.0), ["parent", "matte", "matteLayer"]));
                }
            }
            // Each moved layer is held whole by default, its `~dependents` included: a layer tied
            // to it since refuses.
        }
        _ => {}
    }
}

/// animation.reveal with nothing selected reveals every layer's animated properties, sizing each by
/// its source and the frame (animation.rs:311-343).
fn reveal_inputs(session: &Session, command: &str, cid: ItemId, comp: &Comp, answer: &mut Under) {
    if command != "animation.reveal" {
        return;
    }
    let targets: Vec<LayerId> =
        if session.state.selected_layers.is_empty() { comp.layers.iter().map(|layer| layer.id).collect() } else { session.state.selected_layers.clone() };
    if session.state.selected_layers.is_empty() {
        answer.premises.push(field(json!(cid.0), ["~layerMembers"]));
    }
    answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height"]));
    for layer in targets.iter().filter_map(|id| comp.layer(*id)) {
        source_size(session, layer, answer);
        answer.premises.push(field(json!(layer.id.0), ["props"]));
    }
}

/// Geometry, tracking and template commands:
/// - comp.cropToLayerBounds (comp_more.rs:18-60): the layers' content bounds through their world
///   matrices, then the frame, every unparented layer's Position and the guides it shifts.
/// - camera.setFocusToLayer (focus.rs): the camera's and the layer's world positions.
/// - shape.newPath (shape_tool.rs:267-317): the target's comp-space projection.
/// - track.property tracks `source` (else the next trackable layer in the stack), sized like
///   track.new (track.rs:228-247); track.setType's Stabilize-to-other default target is the layer
///   above, and added points are sized by the layer (track.rs:152-167, 263-305).
/// - templateProps.exportTemplate archives the composition's whole closure and project settings
///   (template_props.rs:806); templateProps.importTemplate adds its composition to the active one through
///   layer.addItem.
/// - layer.sceneEditDetection's split modes insert new layers named among the siblings.
fn geometry_inputs(session: &Session, command: &str, params: &Value, cid: ItemId, comp: &Comp, answer: &mut Under) {
    match command {
        "comp.cropToLayerBounds" => {
            for id in layers_of(session, comp, params) {
                if let Some(layer) = comp.layer(id) {
                    source_size(session, layer, answer);
                    crate::rest_geometry::matrix(session, cid, comp, id, answer);
                }
            }
            answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height", "comp.attrs.pixel_aspect", "comp.guides", "~layerMembers"]));
            for layer in &comp.layers {
                answer.premises.push(field(json!(layer.id.0), ["~resizeInputs"]));
            }
        }
        "camera.setFocusToLayer" => {
            for name in ["camera", "layer"] {
                if let Some(id) = params.get(name).and_then(|value| resolve_layer(comp, value)) {
                    crate::rest_geometry::matrix(session, cid, comp, id, answer);
                }
            }
        }
        "shape.newPath" => {
            // draw_target (shape_tool.rs:267-277): `layer`, else a selected shape layer, else a
            // new layer.
            let target = match params.get("layer") {
                Some(value) => resolve_layer(comp, value),
                None => {
                    session.state.selected_layers.iter().copied().find(|id| comp.layer(*id).is_some_and(|layer| matches!(layer.source, LayerSource::Shape)))
                }
            };
            if let Some(id) = target {
                crate::rest_geometry::matrix(session, cid, comp, id, answer);
                if comp.layer(id).is_some_and(Layer::is_3d)
                    && let Some(camera) = comp.active_camera(session.time_of(cid))
                {
                    active_camera_scan(comp, cid, camera.id, answer);
                }
            }
        }
        "track.property" => {
            let source = match params.get("source") {
                Some(value) => resolve_layer(comp, value),
                None => {
                    answer.premises.push(field(json!(cid.0), ["~layerMembers"]));
                    for layer in &comp.layers {
                        answer.premises.push(field(json!(layer.id.0), ["attrs.source", "sourceItem", "switches.video"]));
                    }
                    None
                }
            };
            if let Some(layer) = source.and_then(|id| comp.layer(id))
                && source_size(session, layer, answer)
            {
                answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height"]));
            }
        }
        "track.setType" => {
            let Some(layer) = layer_of(session, comp, params).and_then(|id| comp.layer(id)) else { return };
            if source_size(session, layer, answer) {
                answer.premises.push(field(json!(cid.0), ["comp.attrs.width", "comp.attrs.height"]));
            }
            if let Some(index) = comp.layers.iter().position(|other| other.id == layer.id).and_then(|index| index.checked_sub(1)) {
                answer.within.push(kition_kit::Within {
                    parent: json!(cid.0),
                    field: "comp.layers".to_owned(),
                    from: index,
                    to: index + 1,
                    ..Default::default()
                });
                if let Some(above) = comp.layers.get(index) {
                    answer.premises.push(Premise::on(json!(above.id.0)).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
                }
            }
        }
        "templateProps.exportTemplate" => {
            let _ = crate::rest_geometry::pixels(session, cid, answer);
            answer.premises.push(Premise::on(json!(cid.0)).positions(Vec::<String>::new()));
            for layer in &comp.layers {
                answer.premises.push(Premise::on(json!(layer.id.0)));
            }
            render_settings(answer);
        }
        "templateProps.importTemplate" => {
            if let Some(active) = session.active_comp_id().and_then(|id| Some((id, session.project.comp(id)?))) {
                sibling_names(active.0, active.1, answer);
                answer.premises.push(field(json!(active.0.0), ["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration", "comp.attrs.frame_rate"]));
            }
        }
        // The source composition's settings are cloned whole (rig3d.rs stereoRig).
        "camera.stereoRig" => answer.premises.push(Premise::on(json!(cid.0)).positions(Vec::<String>::new())),
        "layer.sceneEditDetection" if matches!(params.get("mode").and_then(Value::as_str), Some("split" | "splitPrecompose")) => {
            sibling_names(cid, comp, answer);
            // The detected layer stays held whole.
            if let Some(id) = layer_of(session, comp, params) {
                answer.premises.push(Premise::on(json!(id.0)));
            }
        }
        _ => {}
    }
}

/// The status commands apply a finished job's result to the layer the job was started on, not to
/// a parameter (roto.rs, tracking.rs, mask_track.rs, camera_track.rs): that layer whole (and,
/// for tracker and solver results, its source pixels), and the layers waiting for re-analysis.
fn job_inputs(session: &Session, command: &str, answer: &mut Under) {
    let jobs: Vec<(ItemId, LayerId)> = match command {
        "roto.status" => {
            session.roto_job.iter().map(|job| (job.comp, job.layer)).chain(session.roto_pending.iter().map(|(comp, layer, _)| (*comp, *layer))).collect()
        }
        "track.status" => session.track_job.iter().map(|job| (job.comp, job.layer)).chain(session.mask_job.iter().map(|job| (job.comp, job.layer))).collect(),
        "camera.solveStatus" => {
            session.camera_job.iter().map(|job| (job.comp, job.layer)).chain(session.camera_pending.iter().map(|(comp, layer, _)| (*comp, *layer))).collect()
        }
        _ => return,
    };
    for (cid, id) in jobs {
        let Some(comp) = session.project.comp(cid) else { continue };
        let Some(layer) = comp.layer(id) else { continue };
        answer.premises.push(Premise::on(json!(id.0)).positions(Vec::<String>::new()));
        // A roto result is the job's own matte; tracker and solver results are sized by the
        // source (tracking.rs:320-331, camera_track.rs:379).
        if command != "roto.status" {
            layer_pixels(session, comp, layer, answer);
        }
    }
}

/// One effect as a content premise compares it: its uid, match name, display name and a digest of
/// its whole subtree.
fn effect_nodes(layer: &Layer) -> Value {
    use std::hash::{Hash, Hasher};
    let Some(effects) = layer.effects() else { return json!([]) };
    json!(
        effects
            .groups()
            .take(4096)
            .map(|effect| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(effect).unwrap_or_default().hash(&mut hasher);
                json!([effect.uid, effect.match_id, effect.name, hasher.finish().to_string()])
            })
            .collect::<Vec<_>>()
    )
}

/// The private derived facts content premises compare: `~effectNodes` of each layer and
/// `~markerList` of each layer and composition item.
pub(crate) fn derived(session: &Session, ids: &[Value], names: &[String]) -> Vec<kition_kit::DerivedValues> {
    let requested = |id: u64| ids.is_empty() || ids.iter().any(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()) == Some(id));
    let named = |name: &str| names.is_empty() || names.iter().any(|value| value == name);
    let mut answer = Vec::new();
    for item in session.project.items.values() {
        let Some(comp) = item.as_comp() else { continue };
        if requested(item.id.0) && named("markerList") {
            answer.push(kition_kit::DerivedValues {
                id: json!(item.id.0),
                values: serde_json::Map::from_iter([("markerList".to_owned(), serde_json::to_value(&comp.markers).unwrap_or(Value::Null))]),
            });
        }
        for layer in comp.layers.iter().filter(|layer| requested(layer.id.0)) {
            let mut values = serde_json::Map::new();
            if named("markerList") {
                values.insert("markerList".to_owned(), serde_json::to_value(&layer.markers).unwrap_or(Value::Null));
            }
            if named("effectNodes") {
                values.insert("effectNodes".to_owned(), effect_nodes(layer));
            }
            if !values.is_empty() {
                answer.push(kition_kit::DerivedValues { id: json!(layer.id.0), values });
            }
        }
    }
    answer
}

/// Whether the region a content premise names changed: the effect it removes (by uid) or the
/// marker it sets (by index).
pub(crate) fn content(session: &Session, ask: &kition_kit::ContentAsk) -> Option<bool> {
    let id = ask.premise.id.as_u64().or_else(|| ask.premise.id.as_str()?.parse().ok())?;
    let region = &ask.premise.region;
    let layer = || session.project.items.values().filter_map(|item| item.as_comp()).find_map(|comp| comp.layer(LayerId(id)));
    match ask.premise.field.as_str() {
        "~effectNodes" => {
            let now = effect_nodes(layer()?);
            let pick = |list: &Value| -> Option<Value> {
                let list = list.as_array()?;
                let uid = region.get("uid").and_then(Value::as_u64)?;
                let found = list.iter().find(|entry| entry[0].as_u64() == Some(uid));
                Some(found.cloned().unwrap_or(Value::Null))
            };
            Some(pick(&ask.before)? != pick(&now)?)
        }
        "~markerList" => {
            let now = match session.project.comp(ItemId(id)) {
                Some(comp) => serde_json::to_value(&comp.markers).ok()?,
                None => serde_json::to_value(&layer()?.markers).ok()?,
            };
            let pick = |list: &Value| -> Option<Value> {
                let list = list.as_array()?;
                let found = match (region.get("index").and_then(Value::as_u64), region.get("time")) {
                    (Some(index), _) => list.get(usize::try_from(index).ok()?),
                    (None, Some(time)) => list.iter().find(|marker| &marker["time"] == time),
                    _ => return None,
                };
                Some(found.cloned().unwrap_or(Value::Null))
            };
            Some(pick(&ask.before)? != pick(&now)?)
        }
        _ => None,
    }
}

/// Selector premises the guard cannot replay through a structural edit of the same list (an
/// effect named by uid in the effects group, a marker named by index in a markers array) become content premises on a private derived fact: the layer or composition
/// is held for its existence, and the kit compares the named effect or marker itself.
pub(crate) fn narrow(command: &str, answer: &mut Option<Under>) {
    let Some(answer) = answer.as_mut() else { return };
    let mut content = Vec::new();
    for premise in &mut answer.premises {
        let Some(fields) = premise.fields.as_mut() else { continue };
        let before = content.len();
        let mut kept = Vec::new();
        for field in std::mem::take(fields) {
            let region = match command {
                "effect.remove" if field.starts_with("props.children.") => {
                    let selector = field.rsplit_once(".children[").map(|(_, selector)| selector.trim_end_matches(']').to_owned());
                    // An effect named by name keeps its selector premise: its identity tells a
                    // renamed effect (`relocated`) from an edited one, which a content premise
                    // cannot (the extra matrix's effect-rename cell).
                    match selector.as_deref().and_then(|selector| selector.split_once('=')) {
                        Some(("uid", uid)) => uid.parse::<u64>().ok().map(|uid| ("~effectNodes", json!({"uid":uid}))),
                        _ => None,
                    }
                }
                "marker.set" => {
                    let rest = field.strip_prefix("comp.markers").or_else(|| field.strip_prefix("markers"));
                    match rest {
                        Some(rest) if rest.starts_with('.') => rest[1..].parse::<u64>().ok().map(|index| ("~markerList", json!({"index":index}))),
                        // A marker named by its stored time keeps its selector premise: a marker
                        // another agent of this server retimed is reported `changed` through it,
                        // while the content basis of that edit is not retained at this pin.
                        _ => None,
                    }
                }
                _ => None,
            };
            match region {
                Some((name, region)) => content.push(kition_kit::ContentPremise { id: premise.id.clone(), field: name.to_owned(), region }),
                None => kept.push(field),
            }
        }
        *fields = kept;
        if content.len() > before {
            premise.identities.clear();
        }
    }
    answer.content.extend(content);
}
