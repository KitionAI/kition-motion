//! The JSON a struct's own fields become (a KDM `map`, one register per field, or one `json`
//! register), and the struct rebuilt from it.
//!
//! Each `*_header` copies a struct without its heavy parts (a composition's layers, a layer's
//! property tree, a property's value and keys), destructuring every field by name: a field added
//! to the model is a compile error here until it is placed, so nothing new is silently left out
//! of the replica.

use kition_motion_keyframe::{Keyframe, Value as PropValue};
use kition_motion_project::props::{PropGroup, Property};
use kition_motion_project::{Comp, Item, ItemKind, Layer, Project};
use kition_motion_time::Tick;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::{KdmResult, kdm_err};

pub(crate) type Fields = Map<String, Value>;

/// `value` as a JSON object (every model struct serializes to one).
pub(crate) fn object_of<T: Serialize>(value: &T) -> KdmResult<Fields> {
    match serde_json::to_value(value).map_err(kdm_err)? {
        Value::Object(fields) => Ok(fields),
        other => Err(kdm_err(format!("expected an object, got {other}"))),
    }
}

fn without(mut fields: Fields, keys: &[&str]) -> Fields {
    for key in keys {
        fields.remove(*key);
    }
    fields
}

/// `fields` with `extra` added, as a `T`.
pub(crate) fn rebuild<T: DeserializeOwned>(mut fields: Fields, extra: Vec<(&str, Value)>) -> KdmResult<T> {
    for (key, value) in extra {
        fields.insert(key.to_owned(), value);
    }
    serde_json::from_value(Value::Object(fields)).map_err(kdm_err)
}

// --- Project ------------------------------------------------------------------------------

/// The project's own fields: everything but its items, its id counter (each replica keeps its
/// own, see `crate::slot_base`) and its Render Queue (a list of objects).
pub(crate) fn project_attrs(p: &Project) -> KdmResult<Fields> {
    let Project { schema, settings, items: _, next_id: _, render_queue: _, render_templates, render_prefs } = p;
    let header = Project {
        schema: *schema,
        settings: settings.clone(),
        items: Default::default(),
        next_id: 0,
        render_queue: Vec::new(),
        render_templates: render_templates.clone(),
        render_prefs: render_prefs.clone(),
    };
    Ok(without(object_of(&header)?, &["items", "next_id", "render_queue"]))
}

// --- Items --------------------------------------------------------------------------------

/// An item without its kind (`Folder` in its place).
pub(crate) fn item_header(i: &Item) -> Item {
    let Item { id, name, label, comment, parent, kind: _, proxy } = i;
    Item { id: *id, name: name.clone(), label: *label, comment: comment.clone(), parent: *parent, kind: ItemKind::Folder, proxy: proxy.clone() }
}

/// The item's own fields: everything but its id, its name (a `string` field of its own, which an
/// agent renames with one `set`), its folder (a `ref`) and its kind.
pub(crate) fn item_attrs(i: &Item) -> KdmResult<Fields> {
    Ok(without(object_of(&item_header(i))?, &["id", "name", "parent", "kind"]))
}

/// The item's kind as the schema's enum, and the kind's own data (footage, solid; empty for a
/// folder or a composition, whose data is its `Comp` object).
pub(crate) fn item_kind(i: &Item) -> KdmResult<(&'static str, Fields)> {
    Ok(match &i.kind {
        ItemKind::Folder => ("folder", Fields::new()),
        ItemKind::Comp(_) => ("comp", Fields::new()),
        ItemKind::Footage(f) => ("footage", object_of(f)?),
        ItemKind::Solid(s) => ("solid", object_of(s)?),
    })
}

pub(crate) fn kind_from(kind: &str, data: Fields) -> KdmResult<ItemKind> {
    Ok(match kind {
        "folder" => ItemKind::Folder,
        "footage" => ItemKind::Footage(serde_json::from_value(Value::Object(data)).map_err(kdm_err)?),
        "solid" => ItemKind::Solid(serde_json::from_value(Value::Object(data)).map_err(kdm_err)?),
        other => return Err(kdm_err(format!("unknown item kind `{other}`"))),
    })
}

// --- Compositions -------------------------------------------------------------------------

/// A composition without its layers.
pub(crate) fn comp_header(c: &Comp) -> Comp {
    let Comp {
        width,
        height,
        pixel_aspect,
        frame_rate,
        duration,
        display_start,
        background,
        work_area,
        layers: _,
        markers,
        shutter_angle,
        shutter_phase,
        motion_blur_samples,
        motion_blur_adaptive_limit,
        renderer,
        hide_shy,
        enable_motion_blur,
        enable_frame_blending,
        draft_3d,
        preserve_frame_rate,
        preserve_resolution,
        poster_time,
        global_light,
        guides,
        graphics_template,
    } = c;
    Comp {
        width: *width,
        height: *height,
        pixel_aspect: *pixel_aspect,
        frame_rate: *frame_rate,
        duration: *duration,
        display_start: *display_start,
        background: *background,
        work_area: *work_area,
        layers: Vec::new(),
        markers: markers.clone(),
        shutter_angle: *shutter_angle,
        shutter_phase: *shutter_phase,
        motion_blur_samples: *motion_blur_samples,
        motion_blur_adaptive_limit: *motion_blur_adaptive_limit,
        renderer: *renderer,
        hide_shy: *hide_shy,
        enable_motion_blur: *enable_motion_blur,
        enable_frame_blending: *enable_frame_blending,
        draft_3d: *draft_3d,
        preserve_frame_rate: *preserve_frame_rate,
        preserve_resolution: *preserve_resolution,
        poster_time: *poster_time,
        global_light: global_light.clone(),
        guides: guides.clone(),
        graphics_template: graphics_template.clone(),
    }
}

/// The composition's own fields: everything but its layers (a list of objects).
pub(crate) fn comp_attrs(c: &Comp) -> KdmResult<Fields> {
    Ok(without(object_of(&comp_header(c))?, &["layers"]))
}

pub(crate) fn comp_from(attrs: Fields) -> KdmResult<Comp> {
    rebuild(attrs, vec![("layers", json!([]))])
}

// --- Layers -------------------------------------------------------------------------------

/// The layer fields that are registers or references of their own rather than `attrs` keys.
const LAYER_OWN: [&str; 8] = ["id", "name", "start_time", "in_point", "out_point", "parent", "track_matte", "props"];

/// A layer without its property tree.
pub(crate) fn layer_header(l: &Layer) -> Layer {
    let Layer {
        id,
        name,
        source,
        label,
        comment,
        start_time,
        in_point,
        out_point,
        stretch,
        switches,
        blend_mode,
        preserve_transparency,
        track_matte,
        parent,
        markers,
        markers_locked,
        auto_orient,
        environment,
        environment_background,
        props: _,
    } = l;
    Layer {
        id: *id,
        name: name.clone(),
        source: source.clone(),
        label: *label,
        comment: comment.clone(),
        start_time: *start_time,
        in_point: *in_point,
        out_point: *out_point,
        stretch: *stretch,
        switches: switches.clone(),
        blend_mode: *blend_mode,
        preserve_transparency: *preserve_transparency,
        track_matte: *track_matte,
        parent: *parent,
        markers: markers.clone(),
        markers_locked: *markers_locked,
        auto_orient: *auto_orient,
        environment: *environment,
        environment_background: *environment_background,
        props: empty_group(),
    }
}

/// The layer's own fields that live in its `attrs` map (source, switches, markers…).
pub(crate) fn layer_attrs(l: &Layer) -> KdmResult<Fields> {
    Ok(without(object_of(&layer_header(l))?, &LAYER_OWN))
}

fn empty_group() -> PropGroup {
    PropGroup::new(0, "", "")
}

pub(crate) fn empty_group_json() -> Value {
    json!({ "uid": 0, "match": "", "name": "" })
}

// --- Property tree ------------------------------------------------------------------------

/// A property without its uid, value, keys and expression (its `meta` register).
fn prop_header(p: &Property) -> Property {
    let Property { uid: _, match_id, name, value: _, keys: _, expr: _, ui, spatial, hold_only, three_d_only, two_d_only, shown_dims, static_only } = p;
    Property {
        uid: 0,
        match_id: match_id.clone(),
        name: name.clone(),
        value: PropValue::Bool(false),
        keys: Vec::new(),
        expr: None,
        ui: ui.clone(),
        spatial: *spatial,
        hold_only: *hold_only,
        three_d_only: *three_d_only,
        two_d_only: *two_d_only,
        shown_dims: *shown_dims,
        static_only: *static_only,
    }
}

pub(crate) fn prop_meta(p: &Property) -> KdmResult<Value> {
    Ok(Value::Object(without(object_of(&prop_header(p))?, &["uid", "value", "keys", "expr"])))
}

/// Whether two properties differ in their `meta` register only (anything but value, keys and
/// expression).
pub(crate) fn prop_meta_eq(a: &Property, b: &Property) -> bool {
    prop_header(a) == prop_header(b)
}

/// A group without its uid, children and enabled switch (its `meta` register).
fn group_header(g: &PropGroup) -> PropGroup {
    let PropGroup { uid: _, match_id, name, kind, enabled: _, children: _ } = g;
    PropGroup { uid: 0, match_id: match_id.clone(), name: name.clone(), kind: kind.clone(), enabled: true, children: Vec::new() }
}

pub(crate) fn group_meta(g: &PropGroup) -> KdmResult<Value> {
    Ok(Value::Object(without(object_of(&group_header(g))?, &["uid", "children", "enabled"])))
}

pub(crate) fn group_meta_eq(a: &PropGroup, b: &PropGroup) -> bool {
    group_header(a) == group_header(b)
}

/// A property from its registers (keys added by the caller).
pub(crate) fn prop_from(uid: u64, meta: Value, value: Value, expr: Value) -> KdmResult<Property> {
    let Value::Object(meta) = meta else { return Err(kdm_err("a property's meta is not an object")) };
    let mut extra = vec![("uid", json!(uid)), ("value", value), ("keys", json!([]))];
    if !expr.is_null() {
        extra.push(("expr", expr));
    }
    rebuild(meta, extra)
}

/// A group from its registers (children added by the caller).
pub(crate) fn group_from(uid: u64, meta: Value, enabled: bool) -> KdmResult<PropGroup> {
    let Value::Object(meta) = meta else { return Err(kdm_err("a group's meta is not an object")) };
    rebuild(meta, vec![("uid", json!(uid)), ("enabled", json!(enabled)), ("children", json!([]))])
}

// --- Keyframes ----------------------------------------------------------------------------

/// A keyframe as its three registers: its time, its value, and everything else (interpolation,
/// eases, tangents, flags, label).
pub(crate) fn key_parts(k: &Keyframe) -> KdmResult<(Value, Value, Value)> {
    let Keyframe {
        time,
        value,
        in_interp,
        out_interp,
        in_ease,
        out_ease,
        continuous,
        auto_bezier,
        spatial_in,
        spatial_out,
        spatial_auto,
        spatial_continuous,
        roving,
        label,
    } = k;
    let rest = Keyframe {
        time: Tick::ZERO,
        value: PropValue::Bool(false),
        in_interp: *in_interp,
        out_interp: *out_interp,
        in_ease: in_ease.clone(),
        out_ease: out_ease.clone(),
        continuous: *continuous,
        auto_bezier: *auto_bezier,
        spatial_in: *spatial_in,
        spatial_out: *spatial_out,
        spatial_auto: *spatial_auto,
        spatial_continuous: *spatial_continuous,
        roving: *roving,
        label: *label,
    };
    let data = Value::Object(without(object_of(&rest)?, &["time", "value"]));
    Ok((crate::tick::to_value(*time), serde_json::to_value(value).map_err(kdm_err)?, data))
}

pub(crate) fn key_from(time: Tick, value: Value, data: Value) -> KdmResult<Keyframe> {
    let Value::Object(data) = data else { return Err(kdm_err("a keyframe's data is not an object")) };
    rebuild(data, vec![("time", json!(time.0)), ("value", value)])
}

/// A property value as its `value` register.
pub(crate) fn value_json(v: &PropValue) -> KdmResult<Value> {
    serde_json::to_value(v).map_err(kdm_err)
}
