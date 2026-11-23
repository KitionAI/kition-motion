//! The JSON a struct's own fields become (a KDM `map`, one register per field, or one `json`
//! register), and the struct rebuilt from it.
//!
//! Each `*_header` copies a struct without its heavy parts (a composition's layers, a layer's
//! property tree, a property's value and keys), destructuring every field by name: a field added
//! to the model is a compile error here until it is placed, so nothing new is silently left out
//! of the replica.

use kition_motion_keyframe::{Keyframe, Value as PropValue};
use kition_motion_project::props::{PropGroup, Property};
use kition_motion_project::render_queue::{RenderQueueItem, RenderStatus};
use kition_motion_project::template_props::{EgControl, EgKind, GraphicsTemplate};
use kition_motion_project::{Comp, Guide, Item, ItemKind, Layer, Marker, Project, Switches};
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
/// own, see `crate::slot_base`), its Render Queue (a list of objects) and its Render Queue
/// preferences (this machine's own: notification, overflow folders; see [`crate::local`]).
pub(crate) fn project_attrs(p: &Project) -> KdmResult<Fields> {
    let Project { schema, settings, items: _, next_id: _, render_queue: _, render_templates, render_prefs: _ } = p;
    let header = Project {
        schema: *schema,
        settings: settings.clone(),
        items: Default::default(),
        next_id: 0,
        render_queue: Vec::new(),
        render_templates: render_templates.clone(),
        render_prefs: Default::default(),
    };
    Ok(without(object_of(&header)?, &["items", "next_id", "render_queue", "render_prefs"]))
}

// --- Render Queue -------------------------------------------------------------------------

/// A Render Queue item as its `item` register: what to render and how (composition, Render
/// checkbox, Render Settings, Output Modules, post-render action, log), without what one
/// machine's render left on it (status, start time, duration, last output; see
/// [`crate::local`]).
pub(crate) fn render_item(r: &RenderQueueItem) -> KdmResult<Value> {
    let RenderQueueItem { id, comp, render, status: _, settings, output, started: _, render_time: _, last_output: _, extra_outputs, post_render, log } = r;
    let definition = RenderQueueItem {
        id: *id,
        comp: *comp,
        render: *render,
        status: RenderStatus::default(),
        settings: settings.clone(),
        output: output.clone(),
        started: None,
        render_time: None,
        last_output: None,
        extra_outputs: extra_outputs.clone(),
        post_render: *post_render,
        log: *log,
    };
    Ok(Value::Object(without(object_of(&definition)?, &["status", "started", "render_time", "last_output"])))
}

/// A Render Queue item from its `item` register, with the status a machine that has not rendered
/// it gives it: queued when its Render box is checked, else unqueued.
pub(crate) fn render_item_from(item: Value) -> KdmResult<RenderQueueItem> {
    let mut item: RenderQueueItem = serde_json::from_value(item).map_err(kdm_err)?;
    item.status = unrendered(item.render);
    item.started = None;
    item.render_time = None;
    item.last_output = None;
    Ok(item)
}

/// The status of an item no render on this machine has touched.
pub(crate) fn unrendered(render: bool) -> RenderStatus {
    if render { RenderStatus::Queued } else { RenderStatus::Unqueued }
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

/// A composition without its layers, markers, guides and Graphics Templates controls (each a
/// list of objects; the template keeps its name here).
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
        markers: _,
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
        guides: _,
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
        markers: Vec::new(),
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
        guides: Vec::new(),
        graphics_template: graphics_template.as_ref().map(|t| GraphicsTemplate { name: t.name.clone(), controls: Vec::new() }),
    }
}

/// The composition's own fields: everything but its layers, markers, guides and template
/// controls (lists of objects). A Graphics Templates definition is its name alone.
pub(crate) fn comp_attrs(c: &Comp) -> KdmResult<Fields> {
    let mut attrs = without(object_of(&comp_header(c))?, &["layers", "markers", "guides"]);
    if let Some(Value::Object(template)) = attrs.get_mut("graphics_template") {
        template.remove("controls");
    }
    Ok(attrs)
}

/// A composition from its `attrs` (layers, markers, guides and template controls added by the
/// caller).
pub(crate) fn comp_from(attrs: Fields) -> KdmResult<Comp> {
    rebuild(attrs, vec![("layers", json!([])), ("markers", json!([])), ("guides", json!([]))])
}

// --- Markers, guides, template controls -----------------------------------------------------

/// A marker as its registers (everything but its comment) and its comment (a `text` field).
pub(crate) fn marker_parts(m: &Marker) -> KdmResult<(Fields, String)> {
    let Marker { time, duration, comment, label, chapter, url, protected, frame_target, cue_point } = m;
    let mut fields = Fields::new();
    fields.insert("time".into(), crate::tick::to_value(*time));
    fields.insert("duration".into(), crate::tick::to_value(*duration));
    fields.insert("label".into(), serde_json::to_value(label).map_err(kdm_err)?);
    fields.insert("chapter".into(), json!(chapter));
    fields.insert("url".into(), json!(url));
    fields.insert("protected".into(), json!(protected));
    if !frame_target.is_empty() {
        fields.insert("frameTarget".into(), json!(frame_target));
    }
    if let Some(cue) = cue_point {
        fields.insert("cuePoint".into(), serde_json::to_value(cue).map_err(kdm_err)?);
    }
    Ok((fields, comment.clone()))
}

/// A marker from its registers and comment.
pub(crate) fn marker_from(fields: &Fields, comment: String) -> KdmResult<Marker> {
    let get = |key: &str| fields.get(key).cloned().unwrap_or(Value::Null);
    let text = |key: &str| get(key).as_str().map(str::to_owned).unwrap_or_default();
    Ok(Marker {
        time: crate::tick::from_value(&get("time"))?,
        duration: crate::tick::from_value(&get("duration"))?,
        comment,
        label: serde_json::from_value(get("label")).map_err(kdm_err)?,
        chapter: text("chapter"),
        url: text("url"),
        protected: get("protected").as_bool().unwrap_or(false),
        frame_target: text("frameTarget"),
        cue_point: match get("cuePoint") {
            Value::Null => None,
            cue => Some(serde_json::from_value(cue).map_err(kdm_err)?),
        },
    })
}

/// A guide as its registers.
pub(crate) fn guide_parts(g: &Guide) -> Fields {
    let Guide { vertical, position } = g;
    let mut fields = Fields::new();
    fields.insert("vertical".into(), json!(vertical));
    fields.insert("position".into(), json!(position));
    fields
}

pub(crate) fn guide_from(fields: &Fields) -> KdmResult<Guide> {
    Ok(Guide {
        vertical: fields.get("vertical").and_then(Value::as_bool).ok_or_else(|| kdm_err("a guide's `vertical` is not a boolean"))?,
        position: fields.get("position").and_then(Value::as_f64).ok_or_else(|| kdm_err("a guide's `position` is not a number"))?,
    })
}

/// A Graphics Templates control as its registers: its id, name, kind (a group's without its
/// children, which are a list of objects) and the control it shows as.
pub(crate) fn control_parts(c: &EgControl) -> KdmResult<Fields> {
    let EgControl { id, name, kind, as_type } = c;
    let kind = match kind {
        EgKind::Group { .. } => json!({ "type": "Group" }),
        other => serde_json::to_value(other).map_err(kdm_err)?,
    };
    let mut fields = Fields::new();
    fields.insert("id".into(), json!(id));
    fields.insert("name".into(), json!(name));
    fields.insert("kind".into(), kind);
    fields.insert(
        "asType".into(),
        match as_type {
            Some(t) => serde_json::to_value(t).map_err(kdm_err)?,
            None => Value::Null,
        },
    );
    Ok(fields)
}

/// A control from its registers (a group's children added by the caller).
pub(crate) fn control_from(fields: &Fields) -> KdmResult<EgControl> {
    let id = fields.get("id").and_then(Value::as_u64).ok_or_else(|| kdm_err("a template control's id is not a whole number"))?;
    let name = fields.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
    let mut kind = fields.get("kind").cloned().unwrap_or(Value::Null);
    if kind.get("type").and_then(Value::as_str) == Some("Group")
        && let Value::Object(group) = &mut kind
    {
        group.insert("children".into(), json!([]));
    }
    let kind: EgKind = serde_json::from_value(kind).map_err(kdm_err)?;
    let as_type = match fields.get("asType") {
        None | Some(Value::Null) => None,
        Some(t) => Some(serde_json::from_value(t.clone()).map_err(kdm_err)?),
    };
    Ok(EgControl { id, name, kind, as_type })
}

/// A layer's switches as one register each.
pub(crate) fn switch_fields(s: &Switches) -> KdmResult<Fields> {
    object_of(s)
}

/// A layer's switches from its `switches` map: a switch the map lacks (a layer a batch created
/// without it) is at its default.
pub(crate) fn switches_from(fields: Fields) -> KdmResult<Value> {
    let mut all = switch_fields(&Switches::default())?;
    all.extend(fields);
    let switches: Switches = serde_json::from_value(Value::Object(all)).map_err(kdm_err)?;
    serde_json::to_value(switches).map_err(kdm_err)
}

// --- Layers -------------------------------------------------------------------------------

/// The layer fields that are registers, references, maps or lists of their own rather than
/// `attrs` keys.
const LAYER_OWN: [&str; 10] = ["id", "name", "start_time", "in_point", "out_point", "parent", "track_matte", "props", "switches", "markers"];

/// A layer without its property tree, switches (a map of their own) and markers (a list of
/// objects).
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
        switches: _,
        blend_mode,
        preserve_transparency,
        track_matte,
        parent,
        markers: _,
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
        switches: Switches::default(),
        blend_mode: *blend_mode,
        preserve_transparency: *preserve_transparency,
        track_matte: *track_matte,
        parent: *parent,
        markers: Vec::new(),
        markers_locked: *markers_locked,
        auto_orient: *auto_orient,
        environment: *environment,
        environment_background: *environment_background,
        props: empty_group(),
    }
}

/// The layer's own fields that live in its `attrs` map (source, label, blend mode…).
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
