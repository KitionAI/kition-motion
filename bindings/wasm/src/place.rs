//! What a command names by its place (task card C8-motion-2), for kition-sdk's premise guard
//! (`KitDocument::under`, Kit Process Protocol `context.under`): a step that names its layers by
//! stack position (`"#3"`, the third layer from the top: the target parameter's index spelling,
//! `KitCommand::index`), or that gives no layer and acts at a time (`at.time`, for a command that
//! reads the comp's current time). An agent's write with `basis` by such a place is checked
//! against what the place named then: the objects this answers now, and their windows in the
//! comp's layer list (`comp.layers` of the comp's item).
//!
//! What each place answers:
//!
//! - `"#n"` in `layer`/`layers`: the layer at that stack position now, window `[n-1, n)` (`[n-1,
//!   n-1)` past the bottom).
//! - A time that names nothing ([`names_nothing`]: moving the current time, the view, playback,
//!   undo, files, settings, renders, frames saved): no object, no window.
//! - A comp marker command (`markers.*` without `layer`) naming its marker by `at` or `index`: no
//!   object, no window, and the comp's markers as premise (`comp.attrs.markers` of the comp's item,
//!   C8-time-3): markers have no ids, and the comp's own fields hold them. A new one at the current
//!   time (`comp.addMarker`, `markers.set` with `new`) has the same marker premise. An existing
//!   marker set by `markers.set` holds that marker alone (C8-time-policy): `comp.attrs.markers.<i>`
//!   by `index`, `comp.attrs.markers[time=<ticks>]` by `at`.
//! - A command that acts on layers (declares `layer`/`layers` as its target) given none: the
//!   layers it acts on, the selection, each with its window `[i, i+1)`.
//! - Any other command at a time (a layer created, a camera, a track, a comp-wide edit at the
//!   current time): the layers alive at that time (`inPoint <= t < outPoint`), with that time as a
//!   span over each layer's `inPoint` and `outPoint` (C8-fix-7), which the guard replays exactly:
//!   the comp's stack and its time are two orders.
//! - A number in `layer`/`layers` that is no layer's id: a stack position (`resolve_layer`,
//!   `KitCommand::index_numbers`), as `"#n"`.
//!
//! And for every step, by id too, what the call needs of the layers it names ([`premise_of`]).
//! Keyframes are fields of their layer (no tool ids): a key's premise is its property's path in the
//! layer's property tree (`transform/opacity`), every key of it.
use std::collections::HashSet;
use std::sync::OnceLock;

use kition_kit::{Premise, TimeSpan, Under, Within};
use kition_motion_engine::Session;
use kition_motion_project::{Comp, ItemId, Layer, LayerId};
use kition_motion_time::Tick;
use serde_json::{Value, json};

use crate::TypedFrom;

/// The comp's layer list as the object model names it: a field of the comp's item.
const LAYERS: &str = "comp.layers";

/// The prefix of the target parameter's stack-position spelling (`"#3"`), `KitCommand::index`.
pub const INDEX: &str = "#";

/// The stack index (from 0) a value in the index spelling names: `"#3"` → 2.
pub fn index_of(value: &Value) -> Option<usize> {
    value.as_str()?.strip_prefix(INDEX)?.parse::<usize>().ok()?.checked_sub(1)
}

/// Commands whose time names no object: they move the current time or the view, play, undo,
/// read or write files and settings, render or save a frame.
pub fn names_nothing(id: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "time.",
        "view.",
        "playback.",
        "file.",
        "help.",
        "prefs.",
        "jobs.",
        "project.",
        "renderQueue.",
        "scopes.",
        "templates.",
        "scriptWindow.",
        "footage.",
        "face.model.",
        "roto.model.",
    ];
    PREFIXES.iter().any(|prefix| id.starts_with(prefix))
        || matches!(
            id,
            "edit.undo"
                | "edit.redo"
                | "edit.history"
                | "edit.history.goto"
                | "comp.saveFrameAs"
                | "comp.saveFrameAsExr"
                | "comp.saveFrameAsPsd"
                | "comp.setPosterTime"
                | "render.addOutputModule"
                | "render.preRender"
                | "render.backend"
                | "comp.workArea"
                | "expr.errors"
                | "effect.warning"
                | "templateProps.templateInfo"
                | "camera.solveStatus"
                | "roto.status"
                | "track.status"
                | "warp.status"
                | "puppet.info"
                | "mask.featherPoint.list"
                | "layer.revealExpressionErrors"
        )
}

/// Commands that name a comp marker (with no `layer`).
fn comp_marker(id: &str, params: &Value) -> bool {
    (id == "comp.addMarker" || id.starts_with("markers.")) && params.get("layer").is_none_or(Value::is_null)
}

/// The commands whose target parameter is `layer` or `layers` (`KitCommand::targets`), from the
/// commands' own notations, once.
fn takes_layers(id: &str) -> bool {
    static TAKING: OnceLock<HashSet<&'static str>> = OnceLock::new();
    TAKING
        .get_or_init(|| {
            kition_motion_engine::commands::command_specs()
                .iter()
                .filter(|spec| {
                    let command = kition_kit::KitCommand { id: spec.id.to_owned(), params: Some(spec.params.to_owned()), ..Default::default() }
                        .with_kit_schema()
                        .typed_from(crate::param_types());
                    crate::acts::targets(spec.id, command.schema.as_ref()).is_some()
                })
                .map(|spec| spec.id)
                .collect()
        })
        .contains(id)
}

fn secondary_layers(id: &str) -> &'static [&'static str] {
    static PARAMS: OnceLock<std::collections::HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    PARAMS
        .get_or_init(|| {
            kition_motion_engine::commands::command_specs()
                .iter()
                .map(|spec| {
                    let mut command = kition_kit::KitCommand { id: spec.id.to_owned(), params: Some(spec.params.to_owned()), ..Default::default() }
                        .with_kit_schema()
                        .typed_from(crate::param_types());
                    if let Some(schema) = command.schema.as_mut() {
                        crate::param_keys::ids_as_text(schema);
                    }
                    let target = crate::acts::targets(spec.id, command.schema.as_ref());
                    if let Some(schema) = command.schema.as_mut() {
                        crate::param_keys::complete(spec.id, schema);
                        crate::param_keys::ids_as_text(schema);
                        crate::acts::complete_object_arrays(spec.id, schema);
                    }
                    let layers = crate::acts::objects(spec.id, command.schema.as_ref(), target)
                        .iter()
                        .copied()
                        .filter(|name| !matches!(*name, "layer" | "layers" | "cloneSource") && crate::acts::object_type(spec.id, name) == Some("Layer"))
                        .collect();
                    (spec.id, layers)
                })
                .collect()
        })
        .get(id)
        .map_or(&[], Vec::as_slice)
}

/// The comp a call acts in: its `comp` (the item's id, as a number or its digits, or the comp's
/// name), else the active comp.
fn comp_of<'a>(session: &'a Session, params: &Value) -> Option<(ItemId, &'a Comp)> {
    let project = &session.project;
    let id = match params.get("comp").filter(|comp| !comp.is_null()) {
        Some(Value::Number(n)) => n.as_u64().map(ItemId),
        Some(Value::String(text)) => text
            .parse::<u64>()
            .ok()
            .map(ItemId)
            .filter(|id| project.comp(*id).is_some())
            .or_else(|| project.items.values().find(|item| &item.name == text && item.as_comp().is_some()).map(|item| item.id)),
        Some(_) => None,
        None => session.active_comp_id(),
    }?;
    project.comp(id).map(|comp| (id, comp))
}

/// The layer a `layer` value names in `comp`: its id (a number, or its digits), `"#n"` the n-th
/// from the top, or a number that is no layer's id read as a stack position (the engine's
/// `resolve_layer`), or its name.
fn layer_named<'a>(comp: &'a Comp, value: &Value) -> Option<&'a Layer> {
    comp.layer(kition_motion_engine::commands::resolve_layer(comp, value)?)
}

fn layer_elsewhere<'a>(session: &'a Session, value: &Value) -> Option<&'a Layer> {
    session.project.items.values().filter_map(|item| item.as_comp()).flat_map(|comp| &comp.layers).find(|layer| {
        value.as_u64().or_else(|| value.as_str()?.parse::<u64>().ok()) == Some(layer.id.0) || value.as_str().is_some_and(|name| name == layer.name)
    })
}

/// A number in `layer` that names no layer of `comp`: the stack position it is read as (from 0),
/// `KitCommand::index_numbers`.
fn index_number(session: &Session, comp: &Comp, value: &Value) -> Option<usize> {
    let id = value.as_u64()?;
    if comp.layer(LayerId(id)).is_some() || layer_elsewhere(session, value).is_some() {
        return None;
    }
    usize::try_from(id).ok()?.checked_sub(1)
}

/// What a call needs of the layer it acts on (C8-fix-7, task card C8-time-3), when this kit can
/// say it exactly from the command's code (`commands/layer.rs` `rename`, `commands/prop.rs` `set`
/// and `add_key`, `commands/anim.rs` `set_key`):
///
/// | command | fields | positions |
/// |---|---|---|
/// | `layer.rename` | `name` | none |
/// | `layer.timing` | none | `startTime`, `inPoint`, `outPoint` (not its stack place) |
/// | layer `markers.set` by `index` | that marker (`attrs.markers.<i>`), `attrs.markers_locked` | none |
/// | layer `markers.set` by `at` (or setting `time`) | that marker (`attrs.markers[time=<layer ticks>]`), `attrs.markers_locked`, `attrs.stretch` | `startTime` |
/// | `prop.set` of a property with no keys | the property (its named path, `transform/rotation`) | none |
/// | `prop.set` of a property with keys, `prop.addKey`, `keys.set` | the key it writes, by its layer time (`transform/opacity.keys[time=<ticks>]`; the property where the time cannot be computed), `attrs.stretch` | `startTime` |
///
/// Nothing is stated (every field and position) for any other command, and for a property named by
/// its uid (`prop`), by a path with a stack position in it (`effects/#1/…`), with `selected: true`
/// (the other selected layers are written too), or Position with Separate Dimensions (the X, Y, Z
/// properties are written). A layer named by its name (`"L5"`) needs its `name` too, which the SDK
/// follows to the layer it resolves to; its place in the stack is not read (C8-time-policy).
fn premise_of(session: &Session, layer: &Layer, command: &str, params: &Value) -> Option<Premise> {
    let id = json!(layer.id.0);
    let assigned_field = match command {
        "layer.setComment" => Some("attrs.comment"),
        "layer.setBlendMode" => Some("attrs.blend_mode"),
        "layer.quality" => Some("attrs.switches.quality"),
        "layer.sampling" => Some("attrs.switches.sampling"),
        "layer.frameBlending" => Some("attrs.switches.frame_blend"),
        _ => None,
    };
    if let Some(field) = assigned_field {
        return Some(Premise::on(id).fields([field]).positions(Vec::<String>::new()));
    }
    if command == "layer.setSwitch" {
        // `commands/layer.rs` `set_switch` assigns the one field the switch names on every target;
        // a toggle reads it on the first target in stack order, so a toggle of several layers keeps
        // its general premise. 3D also rewrites transform values (`drop_3d_values`).
        let field = match params.get("switch")?.as_str()? {
            "video" | "eye" => "attrs.switches.video",
            "audio" => "attrs.switches.audio",
            "solo" => "attrs.switches.solo",
            "lock" | "locked" => "attrs.switches.locked",
            "shy" => "attrs.switches.shy",
            "collapse" => "attrs.switches.collapse",
            "quality" => "attrs.switches.quality",
            "fx" | "effects" => "attrs.switches.effects",
            "frameBlend" => "attrs.switches.frame_blend",
            "motionBlur" => "attrs.switches.motion_blur",
            "adjustment" => "attrs.switches.adjustment",
            "guide" => "attrs.switches.guide",
            "preserveTransparency" => "attrs.preserve_transparency",
            _ => return None,
        };
        let targets = ["layer", "layers"].iter().filter_map(|key| params.get(*key)).map(|value| value.as_array().map_or(1, Vec::len)).sum::<usize>();
        if params.get("value").is_some_and(Value::is_boolean) || targets == 1 {
            return Some(Premise::on(id).fields([field]).positions(Vec::<String>::new()));
        }
        return None;
    }
    if command == "layer.setParent" && params.get("compensate").and_then(Value::as_bool) == Some(false) {
        return Some(Premise::on(id).fields(["parent"]).positions(Vec::<String>::new()));
    }
    if command == "layer.timing" {
        // `commands/layer.rs` `timing` shifts or sets the layer's start, In and Out (reading them
        // for a `delta`, an In or Out clamp, or `moveInToTime`); it never reads the layer's place in
        // the stack (C8-time-policy: a hold the handler does not read is a false refusal).
        return Some(Premise::on(id).fields(Vec::<String>::new()).positions(["startTime", "inPoint", "outPoint"]));
    }
    if command == "effect.remove" {
        let effects = layer.effects()?;
        if let Some(Value::String(name)) = params.get("effect") {
            let group = layer.props.children.iter().position(|node| node.uid() == effects.uid)?;
            let member = if effects.groups().any(|effect| effect.match_id == *name) { "meta.match" } else { "meta.name" };
            let field = format!("props.children.{group}.children[{member}={}]", serde_json::to_string(name).ok()?);
            return Some(Premise::on(id).fields([field.clone()]).positions(Vec::<String>::new()).identity(field, "uid"));
        }
        let effect = match params.get("effect")? {
            Value::Number(n) => {
                let uid = n.as_u64()?;
                effects.groups().find(|g| g.uid == uid).or_else(|| usize::try_from(uid).ok()?.checked_sub(1).and_then(|i| effects.groups().nth(i)))
            }
            Value::String(name) => effects.groups().find(|g| g.name == *name || g.match_id == *name),
            _ => None,
        }?;
        let group = layer.props.children.iter().position(|node| node.uid() == effects.uid)?;
        return Some(Premise::on(id).fields([format!("props.children.{group}.children[uid={}]", effect.uid)]).positions(Vec::<String>::new()));
    }
    if command == "effect.reorder" {
        let effects = layer.effects()?;
        let effect = match params.get("effect")? {
            Value::Number(n) => {
                let uid = n.as_u64()?;
                effects.groups().find(|g| g.uid == uid).or_else(|| usize::try_from(uid).ok()?.checked_sub(1).and_then(|index| effects.groups().nth(index)))
            }
            Value::String(name) => effects.groups().find(|g| g.name == *name || g.match_id == *name),
            _ => None,
        }?;
        let index = effects.children.iter().position(|node| node.uid() == effect.uid)?;
        let path = format!("effects/#{}", index.saturating_add(1));
        return Some(Premise::on(id).fields([format!("{path}.uid"), format!("{path}.meta")]).positions(Vec::<String>::new()));
    }
    if command == "markers.set"
        && params.get("new").and_then(Value::as_bool) != Some(true)
        && let Some((Some(owner), index, time)) = kition_motion_engine::commands::markers::addressed_marker(session, params)
        && owner == layer.id
    {
        // `commands/markers.rs` `set` finds the marker (by `index`: the index alone; by `at`: each
        // marker's comp time, through the layer's start and stretch), refuses locked markers, and
        // writes that marker's fields; a new `time` is converted through the start and stretch.
        // So the marker it lands on, by its index or by its stored time, and only what it reads.
        let by_time = params.get("index").is_none_or(Value::is_null);
        let timed = by_time || params.get("time").is_some_and(|time| !time.is_null());
        let marker = if by_time { format!("attrs.markers[time={}]", time.0) } else { format!("attrs.markers.{index}") };
        let mut fields = vec![marker, "attrs.markers_locked".to_owned()];
        if timed {
            fields.push("attrs.stretch".to_owned());
        }
        return Some(Premise::on(id).fields(fields).positions(if timed { vec!["startTime"] } else { Vec::new() }));
    }
    if command == "layer.addMarker" || command.starts_with("markers.") && params.get("layer").is_some_and(|layer| !layer.is_null()) {
        // Markers are stored in layer time; stack position and In/Out points do not affect it.
        return Some(Premise::on(id).fields(["attrs.markers", "attrs.markers_locked", "attrs.stretch"]).positions(["startTime"]));
    }
    if command == "layer.rename" {
        return Some(Premise::on(id).fields(["name"]).positions(Vec::<String>::new()));
    }
    if !matches!(command, "prop.set" | "prop.addKey" | "keys.set") {
        return crate::field_holds::premise(session, layer, command, params);
    }
    if params.get("prop").is_some_and(|prop| !prop.is_null()) || params.get("selected").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let path = params.get("path").and_then(Value::as_str).filter(|path| !path.contains('#'))?;
    let property = layer.props.prop(path)?;
    let separate = layer
        .transform()
        .is_some_and(|transform| transform.get("position").is_some_and(|position| position.uid == property.uid) && transform.get("positionX").is_some());
    if separate {
        return None;
    }
    let named = layer.props.match_path_of(property.uid).unwrap_or_else(|| path.to_owned());
    let timed = command != "prop.set" || !property.keys.is_empty();
    if !timed {
        return Some(Premise::on(id).fields([named]).positions(Vec::<String>::new()));
    }
    // The key at its layer time, named by that time (kition-sdk C8-fix-9 selectors): followed
    // across others' saves, not touched by keys added or changed elsewhere in the property. Where
    // the time cannot be computed, the whole property.
    let mut by_layer = params.clone();
    if let Some(by_layer) = by_layer.as_object_mut() {
        by_layer.insert("layer".to_owned(), json!(layer.id.0));
    }
    let mut fields = vec!["attrs.stretch".to_owned()];
    if let Some(time) = kition_motion_engine::commands::written_key_time(session, &by_layer, command) {
        fields.push(format!("{named}.keys[time={}]", time.0));
        if command == "keys.set" {
            if let Some(key) = property.keys.iter().min_by_key(|key| key.time.0.abs_diff(time.0)) {
                fields.push(format!("{named}.keys[time={}]", key.time.0));
            }
        } else if command == "prop.set" && !property.keys.iter().any(|key| key.time == time)
            || command == "prop.addKey" && params.get("value").is_none_or(Value::is_null)
        {
            // The sampled interval depends on both endpoints, including easing. Equality
            // selectors cannot reconstruct an endpoint that disappeared before this call.
            fields.extend(
                property
                    .keys
                    .iter()
                    .rev()
                    .find(|key| key.time <= time)
                    .into_iter()
                    .chain(property.keys.iter().find(|key| key.time >= time))
                    .map(|key| format!("{named}.keys[time={}]", key.time.0)),
            );
        }
    } else {
        fields.push(named);
    }
    Some(Premise::on(id).fields(fields).positions(["startTime"]))
}

/// A layer named by its name needs that name; the SDK holds the name's resolved identity.
fn by_name(premise: Premise) -> Premise {
    let mut fields = premise.fields.clone().unwrap_or_default();
    fields.push("name".to_owned());
    Premise { fields: premise.fields.map(|_| fields), ..premise }
}

/// The comp markers a call reads or writes when it names a comp marker by `at` or `index`
/// (`markers.set`, `markers.delete`, `markers.convert` without `layer`): the comp item's
/// `comp.attrs.markers`.
fn comp_markers(comp_id: ItemId) -> Premise {
    Premise::on(json!(comp_id.0)).fields(["comp.attrs.markers"]).positions(Vec::<String>::new())
}

/// What command `command` given `params` depends on now (see the module): the layers a place
/// names and their windows, and the premises this kit can state ([`premise_of`]); `Ok(None)` when
/// it names none by place and states none, or the kit cannot say. No edit: the host has set the
/// step's comp and time.
pub fn under(session: &Session, command: &str, params: &Value) -> Result<Option<Under>, String> {
    if let Some(answer) = crate::rests::under(session, command, params) {
        let mut answer = Some(answer);
        crate::rest_comp_reads::extend(session, command, params, &mut answer);
        crate::rest_comp_reads::extend_sources(session, command, params, &mut answer);
        let mut answer = answer.unwrap_or_default();
        merge_premises(&mut answer);
        return Ok(Some(answer));
    }
    let mut under = under_targets(session, command, params)?;
    crate::rest_comp_reads::extend(session, command, params, &mut under);
    crate::rest_comp_reads::extend_sources(session, command, params, &mut under);
    if command.starts_with("renderQueue.") && command != "renderQueue.list" {
        let queue = &session.project.render_queue;
        let index = params
            .get("item")
            .and_then(Value::as_u64)
            .and_then(|id| queue.iter().position(|item| item.id == id))
            .or_else(|| params.get("index").and_then(Value::as_u64).and_then(|index| usize::try_from(index).ok()?.checked_sub(1)))
            .or_else(|| (queue.len() == 1).then_some(0));
        let answer = under.get_or_insert_with(Under::default);
        if let Some(index) = index {
            if let Some(item) = queue.get(index) {
                answer.premises.push(Premise::on(format!("render:{}", item.id)).fields(["item", "comp"]).positions(Vec::<String>::new()));
            }
            if params.get("index").is_some() {
                answer.within.push(Within {
                    parent: json!("project"),
                    field: "renderQueue".to_owned(),
                    from: index,
                    to: index.saturating_add(1),
                    ..Default::default()
                });
            }
        }
        if params.get("template").is_some() {
            answer.premises.push(Premise::on("project").fields(["attrs.render_templates"]).positions(Vec::<String>::new()));
        }
    }
    let Some((comp_id, comp)) = comp_of(session, params) else { return Ok(under) };
    if command == "layer.timing" {
        // `commands/layer.rs` `timing` snaps `delta`, `start`, `in` and `out` to the comp's frame
        // grid and keeps one frame (`frame_duration`) in a trim; the moves to the current time
        // read neither (r2 rows layer/21-24).
        let op = params.get("op").and_then(Value::as_str).unwrap_or("set");
        let snaps = match op {
            "trimInToTime" | "trimOutToTime" => true,
            "moveInToTime" | "moveOutToTime" => false,
            _ => ["delta", "start", "in", "out"].iter().any(|key| params.get(*key).and_then(Value::as_f64).is_some()),
        };
        if snaps {
            under
                .get_or_insert_with(Under::default)
                .premises
                .push(Premise::on(json!(comp_id.0)).fields(["comp.attrs.frame_rate"]).positions(Vec::<String>::new()));
        }
    }
    if command == "comp.workArea" {
        under.get_or_insert_with(Under::default).premises.push(
            Premise::on(json!(comp_id.0)).fields(["comp.attrs.work_area", "comp.attrs.frame_rate", "comp.attrs.duration"]).positions(Vec::<String>::new()),
        );
    }
    crate::rest_effect::extend(session, command, params, &mut under);
    if command == "keys.set"
        && let Some(destination) = params.get("newTime")
        && let Some(layer) = params.get("layer").and_then(|value| layer_named(comp, value))
        && let Some(property) = params.get("path").and_then(Value::as_str).and_then(|path| layer.props.prop(path))
        && let Some(source) = kition_motion_engine::commands::written_key_time(session, params, command)
        && property.keys.iter().any(|key| key.time == source)
    {
        let mut destination_params = params.clone();
        destination_params["time"] = destination.clone();
        // newTime uses the same comp-frame snapping as prop.addKey.
        if let Some(destination) = kition_motion_engine::commands::written_key_time(session, &destination_params, "prop.addKey")
            && let Some(path) = layer.props.match_path_of(property.uid)
        {
            let fields: Vec<_> = property
                .keys
                .iter()
                .filter(|key| key.time != source && source.min(destination) <= key.time && key.time <= source.max(destination))
                .map(|key| format!("{path}.keys[time={}].time", key.time.0))
                .collect();
            if !fields.is_empty() {
                under.get_or_insert_with(Under::default).premises.push(Premise::on(json!(layer.id.0)).fields(fields).positions(Vec::<String>::new()));
            }
        }
    }
    if command == "layer.arrange"
        && let Some(index) = params.get("index").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok())
    {
        let targets: Vec<_> = params.get("layers").and_then(Value::as_array).into_iter().flatten().filter_map(|v| layer_named(comp, v).map(|l| l.id)).collect();
        let remaining: Vec<_> = comp.layers.iter().enumerate().filter(|(_, layer)| !targets.contains(&layer.id)).collect();
        let gap = remaining.get(index.saturating_sub(1)).map_or(comp.layers.len(), |(index, _)| *index);
        under.get_or_insert_with(Under::default).within.push(Within {
            parent: json!(comp_id.0),
            field: LAYERS.to_owned(),
            from: gap,
            to: gap,
            ..Default::default()
        });
    }
    if command == "effect.reorder"
        && let Some(layer) = params.get("layer").and_then(|v| layer_named(comp, v))
        && let Some(effects) = layer.effects()
        && let Some(index) = params.get("index").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok())
        && let Some(group_index) = layer.props.children.iter().position(|node| node.uid() == effects.uid)
    {
        let source = match params.get("effect") {
            Some(Value::Number(n)) => n.as_u64().and_then(|uid| {
                effects.groups().find(|g| g.uid == uid).or_else(|| usize::try_from(uid).ok()?.checked_sub(1).and_then(|i| effects.groups().nth(i)))
            }),
            Some(Value::String(name)) => effects.groups().find(|g| g.name == *name || g.match_id == *name),
            _ => None,
        };
        if let Some(source) = source {
            let remaining: Vec<_> = effects.children.iter().enumerate().filter(|(_, node)| node.uid() != source.uid).collect();
            let gap = remaining.get(index.saturating_sub(1)).map_or(effects.children.len(), |(index, _)| *index);
            under.get_or_insert_with(Under::default).within.push(Within {
                parent: json!(layer.id.0),
                field: format!("props.children.{group_index}.children"),
                from: gap,
                to: gap,
                ..Default::default()
            });
        }
    }
    let mut extra = Vec::new();
    let mut references: Vec<&Value> = match command {
        "keys.info" => params.get("keys").and_then(Value::as_array).into_iter().flatten().filter_map(|key| key.get("layer")).collect(),
        "prop.pickWhip" => params.get("target").and_then(|target| target.get("layer")).into_iter().collect(),
        "mask.selectVertices" | "mask.moveVertices" | "mask.deleteVertices" | "mask.convertVertex" => {
            params.get("vertices").and_then(Value::as_array).into_iter().flatten().filter_map(|vertex| vertex.get("layer")).collect()
        }
        "transition.iris" => params.get("center").filter(|value| !value.is_array()).into_iter().collect(),
        _ => Vec::new(),
    };
    references.extend(secondary_layers(command).iter().filter_map(|name| params.get(*name)).filter(|value| !value.is_array()));
    for value in references {
        if let Some(layer) = layer_named(comp, value).or_else(|| layer_elsewhere(session, value)) {
            extra.push(Premise::on(json!(layer.id.0)));
            if let Some(index) = index_of(value).or_else(|| index_number(session, comp, value)) {
                let answer = under.get_or_insert_with(Under::default);
                answer.objects.push(json!(layer.id.0));
                answer.within.push(Within {
                    parent: json!(comp_id.0),
                    field: LAYERS.to_owned(),
                    from: index,
                    to: index.saturating_add(1),
                    ..Default::default()
                });
            }
        }
    }
    if command == "keys.select" {
        for key in params.get("keys").and_then(Value::as_array).into_iter().flatten() {
            if let Some(value) = key.get("layer")
                && let Some(layer) = layer_named(comp, value).or_else(|| layer_elsewhere(session, value))
            {
                if let Some(premise) = premise_of(session, layer, "keys.set", key) {
                    if comp.layer(layer.id).is_none() {
                        under.get_or_insert_with(Under::default).within.push(Within {
                            parent: json!(comp_id.0),
                            field: LAYERS.to_owned(),
                            from: 0,
                            to: comp.layers.len(),
                            ..Default::default()
                        });
                    }
                    extra.push(premise);
                } else {
                    extra.push(Premise::on(json!(layer.id.0)));
                }
            }
        }
    }
    if command == "layer.setParent" && params.get("compensate").and_then(Value::as_bool) == Some(false) {
        let mut current = params.get("parent").and_then(|value| layer_named(comp, value));
        let mut seen = HashSet::new();
        while let Some(layer) = current {
            if !seen.insert(layer.id) || seen.len() > 4096 {
                break;
            }
            extra.push(Premise::on(json!(layer.id.0)).fields(["parent"]).positions(Vec::<String>::new()));
            current = layer.parent.and_then(|id| comp.layer(id));
        }
    }
    if command == "layer.setParent" && params.get("compensate").and_then(Value::as_bool) != Some(false) {
        let mut pending = Vec::new();
        let targets = params.get("layers").and_then(Value::as_array).cloned().unwrap_or_else(|| {
            params.get("layer").cloned().map_or_else(|| session.state.selected_layers.iter().map(|id| json!(id.0)).collect(), |layer| vec![layer])
        });
        for value in &targets {
            if let Some(layer) = layer_named(comp, value) {
                pending.extend(layer.parent);
            }
        }
        if let Some(parent) = params.get("parent").and_then(|value| layer_named(comp, value)) {
            pending.push(parent.id);
        }
        if !pending.is_empty() {
            extra.push(Premise::on(json!(comp_id.0)).fields(["comp.attrs.pixel_aspect"]).positions(Vec::<String>::new()));
        }
        let mut seen = HashSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(layer) = comp.layer(id) else { continue };
            let mut fields =
                vec!["parent".to_owned(), "attrs.stretch".to_owned(), "attrs.switches".to_owned(), "attrs.auto_orient".to_owned(), "sourceItem".to_owned()];
            if let Some(transform) = layer.transform() {
                for property in [
                    "anchor",
                    "position",
                    "positionX",
                    "positionY",
                    "positionZ",
                    "scale",
                    "rotation",
                    "rotationX",
                    "rotationY",
                    "rotationZ",
                    "orientation",
                    "pointOfInterest",
                    "skew",
                    "skewAxis",
                ] {
                    if let Some(path) = transform.get(property).and_then(|property| layer.props.match_path_of(property.uid)) {
                        fields.push(path);
                    }
                }
            }
            extra.push(Premise::on(json!(id.0)).fields(fields).positions(["startTime"]));
            if let Some(source) = layer.source.item() {
                let aspect = if session.project.comp(source).is_some() { "comp.attrs.pixel_aspect" } else { "data.pixel_aspect" };
                extra.push(Premise::on(json!(source.0)).fields(["kind", aspect]).positions(Vec::<String>::new()));
            }
            pending.extend(layer.parent);
        }
    }
    // align.rs::to_parent maps a comp-space displacement through each ancestor's local
    // matrix. Opacity, names, source dimensions and stack position do not enter that matrix.
    if matches!(command, "layer.nudge" | "layer.align" | "layer.distribute") {
        let values = params.get("layers").and_then(Value::as_array);
        let count =
            values.map(Vec::len).or_else(|| params.get("layer").filter(|value| !value.is_null()).map(|_| 1)).unwrap_or(session.state.selected_layers.len());
        if count > 4096 {
            return Err("too many layers to compare their parent transforms; split the write into smaller runs".to_owned());
        }
        let targets: Vec<_> = if let Some(values) = values {
            values.iter().filter_map(|value| layer_named(comp, value).map(|layer| layer.id)).collect()
        } else if let Some(value) = params.get("layer").filter(|value| !value.is_null()) {
            layer_named(comp, value).map_or_else(Vec::new, |layer| vec![layer.id])
        } else {
            session.state.selected_layers.clone()
        };
        let depth = if command == "layer.nudge" { 63 } else { 64 };
        let mut ancestors = HashSet::new();
        for target in targets {
            let mut current = comp.layer(target).and_then(|layer| layer.parent);
            let mut chain = HashSet::new();
            while let Some(id) = current {
                if !chain.insert(id) || chain.len() > depth {
                    break;
                }
                if ancestors.len() >= 4096 && !ancestors.contains(&id) {
                    return Err("too many parent transforms to compare; split the write into smaller runs".to_owned());
                }
                let Some(layer) = comp.layer(id) else { break };
                current = layer.parent;
                if !ancestors.insert(id) {
                    continue;
                }
                let mut fields = vec![
                    "parent".to_owned(),
                    "attrs.stretch".to_owned(),
                    "attrs.switches.three_d".to_owned(),
                    "attrs.source".to_owned(),
                    "attrs.auto_orient".to_owned(),
                ];
                if let Some(transform) = layer.transform() {
                    for name in [
                        "anchor",
                        "position",
                        "positionX",
                        "positionY",
                        "positionZ",
                        "scale",
                        "rotation",
                        "rotationX",
                        "rotationY",
                        "rotationZ",
                        "orientation",
                        "pointOfInterest",
                    ] {
                        if let Some(path) = transform.get(name).and_then(|property| layer.props.match_path_of(property.uid)) {
                            fields.push(path);
                        }
                    }
                } else {
                    fields.push("props.children".to_owned());
                }
                extra.push(Premise::on(json!(id.0)).fields(fields).positions(["startTime"]));
            }
        }
    }
    // A Layer Control value is an object reference only for that property's value type.
    if matches!(command, "prop.set" | "prop.addKey" | "keys.set")
        && let Some(layer) = params.get("layer").and_then(|value| layer_named(comp, value))
        && let Some(property) = params
            .get("prop")
            .and_then(Value::as_u64)
            .and_then(|uid| layer.props.find(uid))
            .or_else(|| params.get("path").and_then(Value::as_str).and_then(|path| layer.props.prop(path)))
        && property.value.kind_name() == "layer"
        && let Some(id) = params.get("value").and_then(Value::as_u64).filter(|id| *id > 0)
    {
        extra.push(Premise::on(json!(id)));
    }
    if !extra.is_empty() {
        under.get_or_insert_with(Under::default).premises.extend(extra);
    }
    if let Some(additional) = crate::rest_animation::supplemental(session, command, params) {
        let answer = under.get_or_insert_with(Under::default);
        answer.premises.extend(additional.premises);
        answer.objects.extend(additional.objects);
        answer.within.extend(additional.within);
        if additional.covers.is_some() {
            answer.covers = additional.covers;
        }
    }
    if let Some(additional) = crate::rests::supplemental(session, command, params) {
        under.get_or_insert_with(Under::default).premises.extend(additional.premises);
    }
    if let Some(answer) = under.as_mut() {
        merge_premises(answer);
    }
    Ok(under)
}

pub(crate) fn merge_premises(answer: &mut Under) {
    let mut merged: std::collections::BTreeMap<String, Premise> = Default::default();
    for premise in std::mem::take(&mut answer.premises) {
        if let Some(previous) = merged.get_mut(&premise.id.to_string()) {
            for (to, from) in [(&mut previous.fields, premise.fields), (&mut previous.positions, premise.positions)] {
                match (to.as_mut(), from) {
                    (Some(to), Some(from)) => {
                        to.extend(from);
                        to.sort();
                        to.dedup();
                    }
                    (_, None) => *to = None,
                    _ => {}
                }
            }
            previous.identities.extend(premise.identities);
        } else {
            merged.insert(premise.id.to_string(), premise);
        }
    }
    answer.premises = merged.into_values().collect();
}

fn under_targets(session: &Session, command: &str, params: &Value) -> Result<Option<Under>, String> {
    if (matches!(command, "keys.select" | "keys.info") && params.get("keys").is_some_and(Value::is_array))
        || (matches!(command, "mask.selectVertices" | "mask.moveVertices" | "mask.deleteVertices" | "mask.convertVertex")
            && params.get("vertices").is_some_and(Value::is_array))
    {
        return Ok(Some(Under::default()));
    }
    if matches!(command, "keys.delete" | "keys.move" | "keys.transform") {
        let premises = session
            .state
            .selected_keys
            .iter()
            .filter_map(|key| {
                let comp = session.active_comp()?;
                let layer = comp.layer(key.layer)?;
                let path = layer.props.match_path_of(key.prop)?;
                Some(Premise::on(json!(layer.id.0)).fields([format!("{path}.keys[time={}]", key.time.0), "attrs.stretch".to_owned()]).positions(["startTime"]))
            })
            .collect();
        return Ok(Some(Under { premises, ..Default::default() }));
    }
    let comp = comp_of(session, params);
    let given: Vec<&Value> = ["layer", "layers"]
        .iter()
        .filter_map(|key| params.get(*key))
        .flat_map(|value| match value {
            Value::Array(items) => items.iter().collect::<Vec<_>>(),
            other => vec![other],
        })
        .filter(|value| !value.is_null())
        .collect();
    let spelled: Vec<usize> =
        given.iter().filter_map(|value| index_of(value).or_else(|| comp.and_then(|(_, comp)| index_number(session, comp, value)))).collect();
    // Layers named by id (or by name) are no place: only what the call needs of them.
    if spelled.is_empty() && !given.is_empty() {
        let Some((_, comp)) = comp else { return Ok(None) };
        let mut within = Vec::new();
        let premises: Vec<Premise> = given
            .iter()
            .filter_map(|value| {
                let layer = layer_named(comp, value).or_else(|| layer_elsewhere(session, value))?;
                let mut premise = premise_of(session, layer, command, params)?;
                // A name (no id, no stack position) names the first layer from the top with it.
                let named = value.as_str().is_some_and(|text| text.parse::<u64>().is_err());
                if let Some((comp_id, _)) = comp_of(session, params) {
                    // A name names the first layer from the top with it: its place in the stack is
                    // not read (C8-time-policy); the name and the SDK's resolved identity are held.
                    if comp.layers.iter().all(|candidate| candidate.id != layer.id) {
                        within.push(Within { parent: json!(comp_id.0), field: LAYERS.to_owned(), from: 0, to: comp.layers.len(), ..Default::default() });
                        if let Some(positions) = premise.positions.as_mut() {
                            positions.push(LAYERS.to_owned());
                        }
                    }
                }
                Some(if named { by_name(premise) } else { premise })
            })
            .collect();
        // Named by id, its time names no layer (kition-sdk C8-fix-9 asks a step's time by id too).
        return Ok(Some(Under { premises, within, ..Default::default() }));
    }
    let Some((id, comp)) = comp else {
        return if spelled.is_empty() { Ok(None) } else { Err("no comp is open: a stack position (\"#n\") names a layer of the active comp".to_owned()) };
    };
    let parent = json!(id.0);
    let window = |from: usize, to: usize| Within { parent: parent.clone(), field: LAYERS.to_owned(), from, to, ..Default::default() };
    if !spelled.is_empty() {
        let mut under = Under::default();
        for index in spelled {
            let found = comp.layers.get(index);
            under.within.push(window(index, if found.is_some() { index + 1 } else { index }));
            under.objects.extend(found.map(|layer| json!(layer.id.0)));
            under.premises.extend(found.and_then(|layer| premise_of(session, layer, command, params)));
        }
        return Ok(Some(under));
    }
    if names_nothing(command) {
        return Ok(Some(Under::default()));
    }
    if comp_marker(command, params) {
        // An existing comp marker set by its index or its time: that marker only (C8-time-policy;
        // `commands/markers.rs` `set` reads and writes it alone, then re-sorts by time).
        if command == "markers.set"
            && params.get("new").and_then(Value::as_bool) != Some(true)
            && let Some((None, index, time)) = kition_motion_engine::commands::markers::addressed_marker(session, params)
        {
            let marker = if params.get("index").is_none_or(Value::is_null) {
                format!("comp.attrs.markers[time={}]", time.0)
            } else {
                format!("comp.attrs.markers.{index}")
            };
            return Ok(Some(Under { premises: vec![Premise::on(json!(id.0)).fields([marker]).positions(Vec::<String>::new())], ..Default::default() }));
        }
        return Ok(Some(Under { premises: vec![comp_markers(id)], ..Default::default() }));
    }
    if takes_layers(command) {
        // Given no layer, it acts on the selected layers of the comp.
        let mut under = Under::default();
        for layer in &session.state.selected_layers {
            if let Some(index) = comp.layers.iter().position(|candidate| candidate.id == *layer) {
                under.objects.push(json!(layer.0));
                under.within.push(window(index, index + 1));
            }
        }
        return Ok(Some(under));
    }
    let t = if session.active_comp_id() == Some(id) { session.time() } else { session.time_of(id) };
    Ok(Some(alive_at(comp, t, parent)))
}

/// The layers of `comp` alive at comp time `t` (`inPoint <= t < outPoint`), with that time as a
/// place in the comp's layer list over each layer's `inPoint` and `outPoint` (C8-fix-7): the guard
/// replays exactly which layers were alive there since `basis`; the stack is another order.
fn alive_at(comp: &Comp, t: Tick, parent: Value) -> Under {
    let objects = comp.layers.iter().filter(|layer| layer.in_point <= t && t < layer.out_point).map(|layer| json!(layer.id.0)).collect();
    let at = json!(t.0.to_string());
    let span = TimeSpan { from: at.clone(), to: at, start: "inPoint".to_owned(), end: Some("outPoint".to_owned()), duration: None, identity: None };
    Under { objects, within: vec![Within { parent, field: LAYERS.to_owned(), time: Some(span), ..Default::default() }], ..Default::default() }
}
