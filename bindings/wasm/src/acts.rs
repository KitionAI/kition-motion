//! What each command acts on and the editor state it reads besides its parameters
//! (`KitCommand::targets`, `KitCommand::reads`), for kition-sdk's explicit call context: a host
//! sets the active composition, its current time and the layer selection a command reads from a
//! run step's `at.part`, `at.time` and `on`, and puts them back after the call (task card
//! C4-motion).
//!
//! `reads` is what the engine's code reads, not what descriptions say: `reads.json` records it,
//! and `tests/acts.rs` follows each command's definition through the engine to the selections, the
//! active comp and the current times, and fails when the record says otherwise.
use std::collections::HashMap;
use std::sync::OnceLock;

use kition_kit::EditorState;
use serde_json::Value;

/// The editor state command `id` reads besides its parameters: a selection (the selected layers,
/// and the properties, keyframes and mask vertices selected on them), the active composition
/// (`part`), and its current time (`time`).
pub fn reads(id: &str) -> &'static [EditorState] {
    static TABLE: OnceLock<HashMap<String, &'static [EditorState]>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let recorded: HashMap<String, Vec<EditorState>> = serde_json::from_str(include_str!("reads.json")).unwrap_or_default();
        recorded
            .into_iter()
            .map(|(id, states)| {
                let mut unique = Vec::new();
                for state in states {
                    if !unique.contains(&state) {
                        unique.push(state);
                    }
                }
                (id, &*Box::leak(unique.into_boxed_slice()))
            })
            .collect()
    });
    table.get(id).copied().unwrap_or(&[])
}

/// The commands that read the keyframe selection. They declare `selection`, but a call's `on`
/// carries layer ids, and keyframes have no ids: they are named by layer, property and time
/// (`keys.select {keys: [{layer, prop, time}]}`), so a run selects them with `keys.select` in an
/// earlier step. `on` keeps the keyframes selected on the layers it names and drops the others.
/// An explicit form would need keyframe ids (or `{layer, path, time}` triples) that `inspect`
/// lists and `on` may carry.
const KEY_SELECTION: &[&str] = &[
    "edit.clear",
    "edit.copy",
    "edit.cut",
    "edit.label",
    "engine.batch",
    "file.runScript",
    "keys.copy",
    "keys.delete",
    "keys.easyEase",
    "keys.easyEaseIn",
    "keys.easyEaseOut",
    "keys.exponentialScale",
    "keys.info",
    "keys.interpolation",
    "keys.move",
    "keys.nudge",
    "keys.nudgeBackward",
    "keys.nudgeBackward10",
    "keys.nudgeForward",
    "keys.nudgeForward10",
    "keys.paste",
    "keys.select",
    "keys.selectAll",
    "keys.selectEqual",
    "keys.selectFollowing",
    "keys.selectLabelGroup",
    "keys.selectPrevious",
    "keys.set",
    "keys.setLabel",
    "keys.smooth",
    "keys.timeReverse",
    "keys.toggleHold",
    "keys.transform",
    "keys.velocity",
    "keys.wiggle",
    "learn.step",
    "mask.interpolate",
    "motion.sketch",
    "puppet.recordPin",
];

/// Whether command `id` reads the keyframe selection ([`KEY_SELECTION`]).
pub fn reads_key_selection(id: &str) -> bool {
    KEY_SELECTION.contains(&id)
}

/// Parameters that do not resolve composition layers: `file.import` names a file layer;
/// `layer.style.convertToEditable` is unsupported and consumes no target (styles.rs:217).
pub const NOT_COMP_LAYERS: &[&str] = &["file.import", "layer.style.convertToEditable"];

/// The parameter that names the layers command `id` acts on, by the ids `motion_inspect` lists, in
/// place of the layer selection: `layers`, else `layer`, where the command's notation names it and
/// kition-sdk's `schema::check_targets` takes the parameter's schema (`schema` is the one the
/// notation gives, before the keys a handler reads besides it are added).
pub fn targets(id: &str, schema: Option<&Value>) -> Option<&'static str> {
    let schema = schema?;
    if NOT_COMP_LAYERS.contains(&id) {
        return None;
    }
    ["layers", "layer"].into_iter().find(|name| kition_kit::schema::check_targets(schema, name).is_ok())
}

/// Commands whose work is moving the editor (the active comp, a current time, a selection, the
/// viewer): given what they act on, they keep what they set, where every other agent command
/// leaves the editor as it was ([`crate::MotionDocument::execute_as`]). A host still puts an
/// agent's run back afterwards.
pub fn moves_the_editor(id: &str) -> bool {
    id.starts_with("time.")
        || id.starts_with("view.")
        || id.starts_with("layer.select")
        || id.starts_with("keys.select")
        || matches!(
            id,
            "comp.open"
                | "comp.close"
                | "edit.selectAll"
                | "edit.deselectAll"
                | "edit.selectLabelGroup"
                | "mask.selectVertices"
                | "camera.selectPoints"
                | "project.select"
                | "comp.revealInProject"
                | "layer.openLayer"
                | "layer.openSource"
                | "layer.revealSource"
                | "anim.reveal"
                | "text.edit"
                | "text.endEdit"
                | "text.setSelection"
                | "text.moveCaret"
                | "text.insert"
                | "text.delete"
                | "layer.setText"
        )
}

/// Whether a call names what it acts on (`layer`, `layers`), where (`comp`) or when (`time`,
/// `frame`), the keys a command reads in place of the selection, the active comp or its current
/// time.
pub fn names_what_or_when(params: &Value) -> bool {
    ["layer", "layers", "comp", "time", "frame"].iter().any(|key| params.get(*key).is_some_and(|value| !value.is_null()))
}

/// Commands whose effect on the layers they are given depends on neither of a layer's two
/// positions, its place in the comp's stack nor its place in time (`startTime`, `inPoint`,
/// `outPoint`): its name, its switches, its quality (`KitCommand::position_independent`). A write
/// by id with one of these is not refused when others only moved the layer. Every other command
/// is dependent, also those that depend on neither in most calls: `prop.set` on a property
/// without keyframes sets a value, but with keyframes it keys the layer time showing at the
/// current comp time, which a move in time changes; the flag is per command, not per call.
pub const POSITION_INDEPENDENT: &[&str] =
    &["layer.rename", "layer.setSwitch", "layer.setComment", "layer.setBlendMode", "layer.quality", "layer.sampling", "layer.frameBlending"];

/// Whether command `id` is position-independent ([`POSITION_INDEPENDENT`]).
pub fn position_independent(id: &str) -> bool {
    POSITION_INDEPENDENT.contains(&id)
}

/// The type of the objects parameter `name` of command `id` names by id (kition-sdk
/// `KitCommand::objects` and `object_types`, C8-fix-8/9), when it takes integers: project items (a
/// comp, a folder, footage, by its item's id) and layers. `None` for a parameter that names no
/// object of the object model: a render queue entry (`renderQueue.*`, `render.*`: no tool id), a
/// layer of a file being imported (`file.import`'s `layer`, a position in that file), and an
/// `id` (a history step, a model, a template).
pub(crate) fn object_type(id: &str, name: &str) -> Option<&'static str> {
    Some(match name {
        "layer"
            if matches!(
                id,
                "mask.moveVertices" | "mask.deleteVertices" | "mask.interpolationOptions" | "view.lookAtAll" | "camera.move" | "layer.style.globalLight"
            ) =>
        {
            return None;
        }
        "layer" if matches!(id, "time.start" | "time.end" | "time.layerIn" | "time.layerOut" | "time.nextKey" | "time.previousKey" | "time.go") => return None,
        "layers" if id == "layer.style.convertToEditable" => return None,
        "comp" if id == "mask.interpolationOptions" => return None,
        "layer" if matches!(id, "renderQueue.add" | "renderQueue.setRenderSettings" | "file.createProxy") => return None,
        "comp" if matches!(id, "renderQueue.setRenderSettings" | "file.importMultiple" | "file.runScript") => return None,
        "item" if id == "footage.reloadChanged" => return None,
        "data" if id == "layer.alignVideoToData" => "Item",
        "keys[].layer" if matches!(id, "keys.select" | "keys.info") => "Layer",
        "visible[].layer" if matches!(id, "keys.selectAll" | "time.nextKey" | "time.previousKey" | "time.go") => "Layer",
        "vertices[].layer" if matches!(id, "mask.selectVertices" | "mask.moveVertices" | "mask.deleteVertices" | "mask.convertVertex") => "Layer",
        "target.layer" if id == "prop.pickWhip" => "Layer",
        "target.comp" if id == "prop.pickWhip" => "Item",
        "center" if id == "transition.iris" => "Layer",
        "target" if !matches!(id, "track.new" | "track.setTarget" | "camera.move" | "light.spotlight") => return None,
        "item" | "items" if id.starts_with("renderQueue.") || id.starts_with("render.") => return None,
        "comp" | "item" | "items" | "folder" | "sourceItem" | "footage" | "precomp" => "Item",
        "parent" if id == "project.newFolder" => "Item",
        "layer" if id == "file.import" => return None,
        "layer" | "layers" | "parent" | "matte" | "matteLayer" | "source" | "target" | "cloneSource" | "above" | "toLayer" | "referenceLayer" | "camera" => {
            "Layer"
        }
        "from" | "to" if id == "transition.morphRect" => "Layer",
        _ => return None,
    })
}

/// Whether a parameter's schema takes integers: alone, in a `oneOf`, or as a list's items.
fn takes_integers(node: &Value) -> bool {
    match node.get("oneOf").and_then(Value::as_array) {
        Some(branches) => branches.iter().any(takes_integers),
        None => match node.get("type").and_then(Value::as_str) {
            Some("integer") => true,
            Some("array") => node.get("items").is_some_and(takes_integers),
            _ => false,
        },
    }
}

fn object_params(id: &str, schema: &Value, prefix: &str, out: &mut Vec<String>) {
    for (name, node) in schema.as_object().into_iter().flatten() {
        let path = format!("{prefix}{name}");
        if object_type(id, &path).is_some()
            && (takes_integers(node)
                || (path == "data" && id == "layer.alignVideoToData")
                || path == "referenceLayer"
                || (path == "items"
                    && matches!(
                        id,
                        "file.rememberInterpretation"
                            | "file.replaceWithPlaceholder"
                            | "file.replaceWithSolid"
                            | "file.revealInFinder"
                            | "project.move"
                            | "project.delete"
                            | "project.duplicate"
                            | "project.select"
                            | "project.usage"
                    ))
                || (path == "layers" && matches!(id, "edit.paste" | "layer.mask.remove" | "layer.mask.lockOthers" | "path.rotoBezier" | "path.freeTransform"))
                || (path == "comp" && id == "file.createProxy")
                || (path == "cloneSource" && id == "paint.options")
                || (path == "toLayer" && id == "markers.convert"))
        {
            out.push(path.clone());
        }
        if let Some(properties) = node.get("properties") {
            object_params(id, properties, &format!("{path}."), out);
        }
        if let Some(properties) = node.get("items").and_then(|items| items.get("properties")) {
            object_params(id, properties, &format!("{path}[]."), out);
        }
    }
}

/// Source-declared reference lists need an item schema so hosts can mark each reference.
pub fn complete_object_arrays(id: &str, schema: &mut Value) {
    let mut paths = Vec::new();
    object_params(id, schema, "", &mut paths);
    for path in paths {
        if let Some(node) = schema.get_mut(&path).and_then(Value::as_object_mut)
            && node.get("type").and_then(Value::as_str) == Some("array")
            && node.get("items").is_none_or(|items| items.get("type").and_then(Value::as_str) == Some("json"))
        {
            node.insert("items".into(), serde_json::json!({"oneOf":[{"type":"integer"},{"type":"string"}]}));
        }
    }
}

/// The parameters of command `id` besides its targets that name objects by id (kition-sdk
/// `KitCommand::objects`, C8-fix-8/9): those of [`object_type`] whose schema takes integers.
/// kition-sdk reads a whole number as an object's id only in the targets and these; a write over a
/// colleague's change of the layer or item one names is then refused. `tests/ids.rs` checks that
/// every integer parameter whose words say id is marked.
pub fn objects(id: &str, schema: Option<&Value>, targets: Option<&str>) -> &'static [&'static str] {
    static TABLE: OnceLock<std::sync::Mutex<HashMap<String, &'static [&'static str]>>> = OnceLock::new();
    let mut table = TABLE.get_or_init(|| std::sync::Mutex::new(HashMap::new())).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(found) = table.get(id) {
        return found;
    }
    let mut paths = Vec::new();
    if let Some(schema) = schema {
        object_params(id, schema, "", &mut paths);
        // Nested target parameters are forwarded to prop_ref, including its composition.
        if id == "prop.pickWhip" && schema.get("target").is_some() && !paths.iter().any(|path| path == "target.comp") {
            paths.push("target.comp".to_owned());
        }
        if id == "mask.deleteVertices" && schema.get("vertices").is_some() && !paths.iter().any(|path| path == "vertices[].layer") {
            paths.push("vertices[].layer".to_owned());
        }
    }
    let found: Vec<&'static str> = paths.into_iter().filter(|name| Some(name.as_str()) != targets).map(|name| &*Box::leak(name.into_boxed_str())).collect();
    let leaked: &'static [&'static str] = Box::leak(found.into_boxed_slice());
    table.insert(id.to_owned(), leaked);
    leaked
}

/// These references use the layer/item resolvers, which accept names. Numeric-only references
/// are excluded according to their readers, even where the public notation suggests names.
pub fn name_params(id: &str, schema: Option<&Value>, targets: Option<&str>, objects: &[&'static str]) -> &'static [(&'static str, &'static str)] {
    type Names = &'static [(&'static str, &'static str)];
    static TABLE: OnceLock<std::sync::Mutex<HashMap<String, Names>>> = OnceLock::new();
    let mut table = TABLE.get_or_init(|| std::sync::Mutex::new(HashMap::new())).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(found) = table.get(id) {
        return found;
    }
    let found = objects
        .iter()
        .copied()
        .filter(|name| {
            Some(*name) != targets
                && *name != "cloneSource"
                && !(id == "project.newFolder" && *name == "parent")
                && !(id == "file.replaceWithLayeredComp" && *name == "item")
        })
        .map(|name| {
            // A name mark on the list itself excludes its integer members from object guards.
            let name = if schema.and_then(|schema| schema.get(name)).and_then(|node| node.get("type")).and_then(Value::as_str) == Some("array") {
                &*Box::leak(format!("{name}[]").into_boxed_str())
            } else {
                name
            };
            (name, "name")
        })
        .collect::<Vec<_>>();
    let leaked: Names = Box::leak(found.into_boxed_slice());
    table.insert(id.to_owned(), leaked);
    leaked
}

/// The type each of the targets and object parameters of command `id` names (kition-sdk
/// `KitCommand::object_types`, C8-fix-9): a whole number naming an object of another type there is
/// no premise, and in the targets (with `index_numbers`) a stack position: `layer: 1` where 1 is
/// the comp item's id is the comp's first layer, as the engine reads it.
pub fn object_types(id: &str, schema: Option<&Value>, targets: Option<&str>, objects: &[&str]) -> &'static [(&'static str, &'static [&'static str])] {
    type Types = &'static [(&'static str, &'static [&'static str])];
    static TABLE: OnceLock<std::sync::Mutex<HashMap<String, Types>>> = OnceLock::new();
    let mut table = TABLE.get_or_init(|| std::sync::Mutex::new(HashMap::new())).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(found) = table.get(id) {
        return found;
    }
    let mut found: Vec<(&'static str, &'static [&'static str])> = targets
        .into_iter()
        .chain(objects.iter().copied())
        .filter_map(|name| {
            let ty: &'static str = object_type(id, name)?;
            Some((&*Box::leak(name.to_owned().into_boxed_str()), &*Box::leak(vec![ty].into_boxed_slice())))
        })
        .collect();
    for name in objects {
        if schema.and_then(|schema| schema.get(*name)).and_then(|node| node.get("type")).and_then(Value::as_str) == Some("array")
            && let Some(ty) = object_type(id, name)
        {
            found.push((&*Box::leak(format!("{name}[]").into_boxed_str()), &*Box::leak(vec![ty].into_boxed_slice())));
        }
    }
    let leaked: Types = Box::leak(found.into_boxed_slice());
    table.insert(id.to_owned(), leaked);
    leaked
}
