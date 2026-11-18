//! `file.render`, the kit's agent preview (`agent.render` in kit.json; KHP A24, A39): what the
//! Composition viewer shows of an explicit composition at an explicit comp time, or a contact
//! sheet of frames over a range, each labelled with its time. It is a function of the project and
//! the call alone: it never reads or moves the current time, never opens a composition, never
//! selects, never uses the viewer's 3D view or region of interest, and never makes the document
//! dirty, so the same call answers the same picture whatever the editor is doing. Frames are drawn
//! by the CPU compositor (the reference), never an accelerator, so every machine answers the same
//! bytes.
//!
//! Which composition: `target.comp` (its item id), or `comp:<id>` (or a diagnostic's `item:<id>`)
//! in `target.path`; without one, the project's main composition, the only one no other
//! composition uses as a layer (a nested precomp is part of the comp it is in). A project with
//! several such compositions has no main one, and the call must name it; the refusal lists them.

use std::collections::BTreeSet;

use kition_kit::KitCommand;
use kition_kit::schema::{ParamError, refusal};
use kition_motion_engine::Session;
use kition_motion_engine::render::{Backend, RenderOpts, Renderer};
use kition_motion_project::{Comp, ItemId, LayerId, LayerSource, Project};
use kition_motion_time::Tick;
use serde_json::{Value, json};

/// The command's id.
pub const RENDER: &str = "file.render";

/// The largest `scale` (picture pixels per comp pixel), and `maxSize`'s range and default (pixels
/// on the picture's longer side).
const MAX_SCALE: f64 = 2.0;
const MIN_SIZE: u64 = 16;
const MAX_SIZE: u64 = 4096;
const DEFAULT_SIZE: u64 = 1024;
/// How many frames a contact sheet holds, and how many a whole composition's preview shows.
const MIN_FRAMES: u64 = 2;
const MAX_FRAMES: u64 = 12;
const WHOLE_FRAMES: u64 = 6;
/// Black pixels between a contact sheet's frames.
const GAP: u32 = 2;
/// Seconds an agent may name (well beyond any composition, well within a tick count).
const MAX_SECONDS: f64 = 1.0e7;
/// The most layers a refusal lists.
const LISTED_LAYERS: usize = 20;

/// The command as the catalog lists it.
pub fn command() -> KitCommand {
    KitCommand {
        id: RENDER.to_owned(),
        title: "Render Preview".to_owned(),
        description: "Renders what the Composition viewer shows of a composition at a time, as a PNG, without moving the current time or opening the comp: `target.time` (seconds from the start of the comp, comp time) for one frame, or `target.range` [from, to] with `target.frames` (2 to 12) for a contact sheet of frames laid out in a grid, each labelled with its time. `target.comp` names the comp by its item id; without it, the project's main comp (the only one no other comp uses as a layer). Without a target it answers a contact sheet of the whole comp. Changes nothing.".to_owned(),
        params: Some(
            "{target?: {time?: seconds, comp?: item id, range?: [from, to] seconds, frames?: 2-12, path?: \"comp:<id>[/layer:<id>]\"}, scale?: >0-2 (default 1), maxSize?: 16-4096 (default 1024)} → {png (base64), mimeType, width, height, scale, comp, frames: [{time, frame, label}], columns, rows}".to_owned(),
        ),
        schema: Some(json!({
            "target": {
                "type": "object",
                "additionalProperties": false,
                "description": "What to show. `time` alone shows one frame; `range` with `frames` a contact sheet; neither, the whole comp as a contact sheet.",
                "properties": {
                    "time": { "type": "number", "description": "Seconds from the start of the comp (comp time, 0 at its first frame), from 0 to before its end. The frame showing at that time is drawn." },
                    "comp": { "type": "integer", "description": "The composition's item id (`comp.id` in motion_inspect). Needed when the project has several comps that no other comp uses." },
                    "range": { "type": "array", "items": { "type": "number" }, "description": "[from, to]: seconds from the start of the comp, from < to, within it. The frames are spread evenly from `from` to `to`, both included; `to` at the comp's end shows its last frame." },
                    "frames": { "type": "integer", "description": "How many frames the contact sheet of `range` shows, 2 to 12 (default 6)." },
                    "path": { "type": "string", "description": "A diagnostic's target: `comp:<id>` (or `item:<id>`) names the comp, `comp:<id>/layer:<id>` shows the middle of that layer's time in the comp unless `time` or `range` is given." },
                },
            },
            "scale": { "type": "number", "description": "Picture pixels per comp pixel, above 0 and at most 2 (default 1). A contact sheet's frames are each drawn at this scale. Reduced so the picture fits `maxSize`." },
            "maxSize": { "type": "integer", "description": "The most pixels on the picture's longer side, 16 to 4096 (default 1024)." },
        })),
        output_schema: Some(json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "png": { "type": "string", "required": true, "description": "The picture as base64 PNG." },
                "mimeType": { "type": "string", "required": true },
                "width": { "type": "integer", "required": true },
                "height": { "type": "integer", "required": true },
                "scale": { "type": "number", "required": true, "description": "The scale each frame was drawn at, after `maxSize`." },
                "comp": { "type": "integer", "required": true, "description": "The composition drawn." },
                "frames": { "type": "array", "required": true, "items": { "type": "json" }, "description": "Each frame drawn, in reading order: {time (comp seconds at the frame's start), frame (counted from 0), label}." },
                "columns": { "type": "integer", "required": true },
                "rows": { "type": "integer", "required": true },
            },
        })),
        mutates: false,
        enabled: true,
        ..Default::default()
    }
}

/// A refusal of the parameter at `path` (a JSON Pointer), typed for the host (`invalid_params`).
fn wrong(path: &str, message: String, expected: &str) -> String {
    refusal(&[ParamError { path: path.to_owned(), message, expected: expected.to_owned() }])
}

/// Seconds as a short label: at most three decimals, no trailing zeros.
fn seconds(t: Tick) -> String {
    let text = format!("{:.3}", t.seconds());
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0".to_owned() } else { text.to_owned() }
}

fn name_of(project: &Project, id: ItemId) -> &str {
    project.item(id).map_or("", |item| item.name.as_str())
}

/// The compositions, by id, with their names and durations, for a refusal.
fn listing(project: &Project, ids: &[ItemId]) -> String {
    ids.iter()
        .map(|id| {
            let length = project.comp(*id).map_or(Tick::ZERO, |comp| comp.duration);
            format!("{} \"{}\" ({} s)", id.0, name_of(project, *id), seconds(length))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn all_comps(project: &Project) -> Vec<ItemId> {
    let mut ids: Vec<ItemId> = project.comps().map(|(id, _)| *id).collect();
    ids.sort_unstable();
    ids
}

/// The compositions no other composition uses as a layer.
fn roots(project: &Project) -> Vec<ItemId> {
    let nested: BTreeSet<ItemId> = project
        .comps()
        .flat_map(|(id, comp)| {
            comp.layers.iter().filter_map(move |layer| match layer.source {
                LayerSource::Comp { item } if item != *id => Some(item),
                _ => None,
            })
        })
        .collect();
    all_comps(project).into_iter().filter(|id| !nested.contains(id)).collect()
}

/// The main composition: the only one no other composition uses.
fn main_comp(project: &Project) -> Result<ItemId, String> {
    let top = roots(project);
    match top[..] {
        [one] => Ok(one),
        [] if all_comps(project).is_empty() => {
            Err(wrong("/target/comp", "the project has no composition to render: make one with comp.new".to_owned(), "a composition's item id"))
        }
        _ => {
            let candidates = if top.is_empty() { all_comps(project) } else { top };
            Err(wrong(
                "/target/comp",
                format!(
                    "the project has {} compositions that no other composition uses, so none is its main one: pass `target.comp`, one of {}",
                    candidates.len(),
                    listing(project, &candidates)
                ),
                "a composition's item id",
            ))
        }
    }
}

fn number(value: &Value, path: &str, what: &str) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|n| n.is_finite() && n.abs() <= MAX_SECONDS)
        .ok_or_else(|| wrong(path, format!("`{}` is {value}: {what}", path.trim_start_matches('/').replace('/', ".")), what))
}

fn whole(value: &Value, path: &str, what: &str) -> Result<u64, String> {
    value
        .as_u64()
        .or_else(|| value.as_f64().filter(|n| n.fract() == 0.0 && (0.0..=9.0e15).contains(n)).map(|n| n as u64))
        .ok_or_else(|| wrong(path, format!("`{}` is {value}: {what}", path.trim_start_matches('/').replace('/', ".")), what))
}

/// `comp:<id>`, `item:<id>` or `comp:<id>/layer:<id>`.
fn parse_path(path: &str) -> Result<(ItemId, Option<LayerId>), String> {
    let refuse = || {
        wrong(
            "/target/path",
            format!(
                "`target.path` is {path:?}: this kit draws a composition's frames, named `comp:<id>`, `item:<id>` or `comp:<id>/layer:<id>` (a diagnostic's target); for another target pass `target.comp` and `target.time`"
            ),
            "comp:<id>[/layer:<id>]",
        )
    };
    let id = |text: &str, prefix: &str| text.strip_prefix(prefix).and_then(|id| id.parse::<u64>().ok());
    match path.split_once('/') {
        Some((comp, layer)) => {
            let comp = id(comp, "comp:").map(ItemId).ok_or_else(refuse)?;
            let layer = id(layer, "layer:").map(LayerId).ok_or_else(refuse)?;
            Ok((comp, Some(layer)))
        }
        None => id(path, "comp:").or_else(|| id(path, "item:")).map(|comp| (ItemId(comp), None)).ok_or_else(refuse),
    }
}

/// What a call asks for: the composition and the times of its frames (one, or a contact sheet's).
struct View {
    comp: ItemId,
    times: Vec<Tick>,
    sheet: bool,
}

/// The last tick of `comp`, where its last frame shows.
fn last_tick(comp: &Comp) -> Tick {
    Tick((comp.duration.0 - 1).max(0))
}

/// `given` seconds as a time of comp `id`, from 0 to before its end.
fn time_on(comp: &Comp, id: ItemId, name: &str, given: f64, path: &str) -> Result<Tick, String> {
    let t = Tick::from_seconds_f64(given);
    if t >= Tick::ZERO && t < comp.duration {
        return Ok(t);
    }
    let last = comp.frame_rate.snap(last_tick(comp));
    let message = format!(
        "{given} s is outside comp {} \"{name}\", which runs from 0 to {} s: pass a time from 0 to {} s (where its last frame starts), in comp seconds",
        id.0,
        seconds(comp.duration),
        seconds(last)
    );
    Err(wrong(path, message, "seconds from the start of the comp"))
}

/// `n` times spread evenly from `from` to `to`, both included; a time at the comp's end is its
/// last frame.
fn spread(from: Tick, to: Tick, n: u64, comp: &Comp) -> Vec<Tick> {
    let span = i128::from(to.0) - i128::from(from.0);
    let steps = i128::from(n.saturating_sub(1).max(1));
    (0..n)
        .map(|i| {
            let t = i128::from(from.0) + span * i128::from(i) / steps;
            let t = Tick(i64::try_from(t).unwrap_or(i64::MAX));
            if t >= comp.duration { last_tick(comp) } else { t }
        })
        .collect()
}

/// The middle of the time layer `layer` of comp `id` shows within it.
fn layer_middle(project: &Project, comp: &Comp, id: ItemId, layer: LayerId) -> Result<Tick, String> {
    let name = name_of(project, id);
    let Some(found) = comp.layer(layer) else {
        let layers: Vec<String> = comp.layers.iter().take(LISTED_LAYERS).map(|l| format!("{} \"{}\"", l.id.0, l.name)).collect();
        let more = if comp.layers.len() > LISTED_LAYERS { ", …" } else { "" };
        return Err(wrong(
            "/target/path",
            format!(
                "comp {} \"{name}\" has no layer {}: its layers are {}{more}",
                id.0,
                layer.0,
                if layers.is_empty() { "none".to_owned() } else { layers.join(", ") }
            ),
            "a layer of the comp",
        ));
    };
    let (from, to) = (found.in_point.min(found.out_point).max(Tick::ZERO), found.out_point.max(found.in_point).min(comp.duration));
    if from >= to {
        return Err(wrong(
            "/target/path",
            format!(
                "layer {} \"{}\" never shows in comp {} \"{name}\": it runs from {} to {} s, and the comp from 0 to {} s; pass `target.time` to show the comp at another time",
                layer.0,
                found.name,
                id.0,
                seconds(found.in_point),
                seconds(found.out_point),
                seconds(comp.duration)
            ),
            "a layer that shows in the comp",
        ));
    }
    Ok(Tick(from.0.saturating_add((to.0 - from.0) / 2)))
}

fn view(project: &Project, params: &Value) -> Result<View, String> {
    let target = match params.get("target") {
        None | Some(Value::Null) => serde_json::Map::new(),
        Some(Value::Object(target)) => target.clone(),
        Some(other) => return Err(wrong("/target", format!("`target` is {other}: an object such as {{\"time\": 1.5}}"), "an object")),
    };
    const KEYS: [&str; 5] = ["time", "comp", "range", "frames", "path"];
    if let Some(key) = target.keys().find(|key| !KEYS.contains(&key.as_str())) {
        return Err(wrong(
            &format!("/target/{key}"),
            format!("this kit draws a composition's frames and takes no `target.{key}`: it takes `time`, `comp`, `range`, `frames` and `path`"),
            "time, comp, range, frames or path",
        ));
    }
    let named = target.get("comp").filter(|v| !v.is_null()).map(|v| whole(v, "/target/comp", "a composition's item id")).transpose()?.map(ItemId);
    let (from_path, layer) = match target.get("path").filter(|v| !v.is_null()) {
        Some(Value::String(path)) => {
            let (comp, layer) = parse_path(path)?;
            (Some(comp), layer)
        }
        Some(other) => return Err(wrong("/target/path", format!("`target.path` is {other}: a string such as \"comp:1\""), "comp:<id>[/layer:<id>]")),
        None => (None, None),
    };
    if let (Some(a), Some(b)) = (named, from_path)
        && a != b
    {
        return Err(wrong("/target/comp", format!("`target.comp` {} and `target.path` name different compositions ({})", a.0, b.0), "one composition"));
    }
    let id = match named.or(from_path) {
        Some(id) => id,
        None => main_comp(project)?,
    };
    let Some(comp) = project.comp(id) else {
        let path = if named.is_some() { "/target/comp" } else { "/target/path" };
        return Err(wrong(
            path,
            format!("there is no composition {}: the project's compositions are {}", id.0, listing(project, &all_comps(project))),
            "a composition's item id",
        ));
    };
    let name = name_of(project, id);
    let time = target.get("time").filter(|v| !v.is_null());
    let range = target.get("range").filter(|v| !v.is_null());
    let frames = target.get("frames").filter(|v| !v.is_null());
    if time.is_some() && range.is_some() {
        return Err(wrong("/target/time", "give `time` for one frame, or `range` (with `frames`) for a contact sheet, not both".to_owned(), "time or range"));
    }
    if frames.is_some() && range.is_none() {
        return Err(wrong("/target/frames", "`frames` counts the frames of a `range`: give `range` [from, to] too".to_owned(), "range"));
    }
    if let Some(time) = time {
        let t = time_on(comp, id, name, number(time, "/target/time", "seconds from the start of the comp")?, "/target/time")?;
        return Ok(View { comp: id, times: vec![t], sheet: false });
    }
    if let Some(range) = range {
        let ends = range
            .as_array()
            .filter(|ends| ends.len() == 2)
            .ok_or_else(|| wrong("/target/range", format!("`target.range` is {range}: [from, to] in seconds from the start of the comp"), "[from, to]"))?;
        let from = number(&ends[0], "/target/range", "[from, to] in seconds")?;
        let to = number(&ends[1], "/target/range", "[from, to] in seconds")?;
        let (a, b) = (Tick::from_seconds_f64(from), Tick::from_seconds_f64(to));
        if !(a >= Tick::ZERO && a < b && b <= comp.duration) {
            return Err(wrong(
                "/target/range",
                format!(
                    "`target.range` [{from}, {to}] is not a span of comp {} \"{name}\", which runs from 0 to {} s: pass 0 <= from < to <= {}, in comp seconds",
                    id.0,
                    seconds(comp.duration),
                    seconds(comp.duration)
                ),
                "[from, to] within the comp",
            ));
        }
        let n = match frames {
            Some(frames) => whole(frames, "/target/frames", "a whole number from 2 to 12")?,
            None => WHOLE_FRAMES,
        };
        if !(MIN_FRAMES..=MAX_FRAMES).contains(&n) {
            return Err(wrong("/target/frames", format!("`target.frames` is {n}: a contact sheet shows {MIN_FRAMES} to {MAX_FRAMES} frames"), "2 to 12"));
        }
        return Ok(View { comp: id, times: spread(a, b, n, comp), sheet: true });
    }
    if let Some(layer) = layer {
        return Ok(View { comp: id, times: vec![layer_middle(project, comp, id, layer)?], sheet: false });
    }
    // The whole composition: a contact sheet of it, or its one frame.
    let count = if comp.duration > Tick::ZERO { u64::try_from(comp.frame_rate.frame_at(last_tick(comp))).unwrap_or(0).saturating_add(1) } else { 1 };
    let n = count.min(WHOLE_FRAMES);
    if n < MIN_FRAMES {
        return Ok(View { comp: id, times: vec![Tick::ZERO], sheet: false });
    }
    Ok(View { comp: id, times: spread(Tick::ZERO, comp.duration, n, comp), sheet: true })
}

fn scale_of(params: &Value) -> Result<f64, String> {
    match params.get("scale") {
        None | Some(Value::Null) => Ok(1.0),
        Some(value) => value.as_f64().filter(|s| s.is_finite() && *s > 0.0 && *s <= MAX_SCALE).ok_or_else(|| {
            wrong("/scale", format!("`scale` is {value}: picture pixels per comp pixel, above 0 and at most {MAX_SCALE}"), "above 0, at most 2")
        }),
    }
}

fn max_size_of(params: &Value) -> Result<u64, String> {
    match params.get("maxSize") {
        None | Some(Value::Null) => Ok(DEFAULT_SIZE),
        Some(value) => value.as_u64().filter(|size| (MIN_SIZE..=MAX_SIZE).contains(size)).ok_or_else(|| {
            wrong("/maxSize", format!("`maxSize` is {value}: the most pixels on the picture's longer side, {MIN_SIZE} to {MAX_SIZE}"), "16 to 4096")
        }),
    }
}

/// One frame of comp `id` at comp time `t` and `scale`, over the comp's background as opaque
/// 8-bit RGBA, with `label` burned in when given: the CPU compositor with the project's own
/// settings and the session's footage and expressions, none of the viewer's (no 3D view, region
/// of interest, guides or layer cache).
fn frame(session: &Session, id: ItemId, comp: &Comp, scale: f64, t: Tick, label: Option<&str>) -> (Vec<u8>, u32, u32) {
    let opts = RenderOpts { scale, backend: Backend::Cpu, ..RenderOpts::default() };
    let mut renderer = Renderer::new(&session.project, &*session.footage, opts);
    renderer.expr = session.expr.as_deref();
    let mut image = renderer.comp_frame_cpu(id, t);
    if let Some(label) = label {
        kition_motion_engine::render::label::burn_label(&mut image, label);
    }
    (image.to_rgba8_over(comp.background), image.width, image.height)
}

fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out).write_image(rgba, width, height, image::ExtendedColorType::Rgba8).map_err(|error| error.to_string())?;
    Ok(out)
}

/// Runs `file.render` on `session`'s project: see the module.
pub fn render(session: &Session, params: &Value) -> Result<Value, String> {
    let scale = scale_of(params)?;
    let max_size = max_size_of(params)? as f64;
    let project = &session.project;
    let view = view(project, params)?;
    let Some(comp) = project.comp(view.comp) else { return Err(format!("there is no composition {}", view.comp.0)) };
    let rate = comp.frame_rate;
    let (cw, ch) = (f64::from(comp.width.max(1)), f64::from(comp.height.max(1)));
    let count = view.times.len().max(1);
    let columns = if view.sheet { (count as f64).sqrt().ceil().max(1.0) as usize } else { 1 };
    let rows = count.div_ceil(columns);
    // Room for the gaps, and a pixel per frame for rounding, within maxSize.
    let room = |cells: usize| max_size - f64::from(GAP) * (cells.saturating_sub(1)) as f64 - if view.sheet { cells as f64 } else { 0.0 };
    let fitted = scale.min(room(columns) / (columns as f64 * cw)).min(room(rows) / (rows as f64 * ch));
    if !(fitted.is_finite() && fitted > 0.0) {
        return Err(wrong("/maxSize", format!("`maxSize` {max_size} is too small for {count} frames: give a larger one"), "a larger maxSize"));
    }
    let frames: Vec<Value> = view
        .times
        .iter()
        .map(|t| {
            let at = rate.snap(*t);
            json!({ "time": at.seconds(), "frame": rate.frame_at(at), "label": format!("{} s", seconds(at)) })
        })
        .collect();
    let (rgba, width, height) = if view.sheet {
        let cells: Vec<(Vec<u8>, u32, u32)> =
            view.times.iter().zip(&frames).map(|(t, info)| frame(session, view.comp, comp, fitted, rate.snap(*t), info["label"].as_str())).collect();
        let (cw, ch) = cells.first().map_or((1, 1), |(_, w, h)| (*w, *h));
        let width = cw.saturating_mul(columns as u32).saturating_add(GAP.saturating_mul(columns as u32 - 1));
        let height = ch.saturating_mul(rows as u32).saturating_add(GAP.saturating_mul(rows as u32 - 1));
        let mut sheet = [0u8, 0, 0, 255].repeat(width as usize * height as usize);
        for (index, (pixels, w, h)) in cells.iter().enumerate() {
            let (x0, y0) = ((index % columns) as u32 * (cw + GAP), (index / columns) as u32 * (ch + GAP));
            for y in 0..(*h).min(height.saturating_sub(y0)) {
                let row = (y * w * 4) as usize;
                let take = ((*w).min(width.saturating_sub(x0)) * 4) as usize;
                let at = (((y0 + y) * width + x0) * 4) as usize;
                if let (Some(to), Some(from)) = (sheet.get_mut(at..at + take), pixels.get(row..row + take)) {
                    to.copy_from_slice(from);
                }
            }
        }
        (sheet, width, height)
    } else {
        let t = view.times.first().copied().unwrap_or(Tick::ZERO);
        frame(session, view.comp, comp, fitted, rate.snap(t), None)
    };
    let png = encode_png(&rgba, width, height)?;
    Ok(json!({
        "png": crate::base64_encode(&png),
        "mimeType": "image/png",
        "width": width,
        "height": height,
        "scale": fitted,
        "comp": view.comp.0,
        "frames": frames,
        "columns": columns,
        "rows": rows,
    }))
}
