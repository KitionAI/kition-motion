//! # kition-motion-kit
//!
//! The Motion engine for Kition's in-page editor, on the Kition SDK. [`MotionDocument`]
//! implements [`KitDocument`] for an Kition Motion project (`.kmotion`): the picture is the active
//! composition's frame at the current time, commands are the engine's own, and the footage the
//! project uses is read through the SDK's lent files ([`kition_kit::host`]).
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod acts;
pub mod check;
mod collab;
#[cfg(test)]
mod conformance;
pub mod descriptions;
mod design;
mod editor_reads;
mod field_holds;
mod files;
mod frames;
mod movie;
mod param_keys;
mod params;
pub mod place;
mod raw_keys;
mod raw_sequences;
mod render;
mod rest_animation;
mod rest_comp;
mod rest_comp_reads;
mod rest_delete;
mod rest_document;
mod rest_effect;
mod rest_geometry;
mod rest_inputs;
mod rest_layer;
mod rest_mask_path;
mod rest_masks;
mod rest_reads;
mod rest_styles;
mod rest_visual;
mod rests;
mod state_members;
pub mod text;
mod tools;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

use kition_kit::raster::{self, Frame};
use kition_kit::{CallContext, KitCommand, KitDocument};
use kition_motion_engine::commands::prop_path;
use kition_motion_engine::context::LayerRef;
use kition_motion_engine::render::{RenderOpts, Renderer};
use kition_motion_engine::{Importer, Services, Session};
use kition_motion_media::MediaPool;
use kition_motion_project::{Footage, ItemKind};
use kition_motion_time::{TICKS_PER_SECOND, Tick};
use serde_json::{Value, json};

fn io_error(message: String) -> std::io::Error {
    let kind = if message.contains("NotFound") || message.contains("not found") { std::io::ErrorKind::NotFound } else { std::io::ErrorKind::Other };
    std::io::Error::new(kind, message)
}

/// The project file and what the engine writes live in memory; footage is a lent file.
#[derive(Default)]
struct KitServices {
    memory: Mutex<HashMap<String, Arc<[u8]>>>,
}

impl KitServices {
    fn memory(&self, path: &str) -> Option<Arc<[u8]>> {
        self.memory.lock().unwrap_or_else(PoisonError::into_inner).get(path).cloned()
    }
}

impl Services for KitServices {
    fn read_file(&self, path: &str) -> std::io::Result<Vec<u8>> {
        match self.memory(path) {
            Some(bytes) => Ok(bytes.to_vec()),
            None => kition_kit::host::read(path).map_err(io_error),
        }
    }

    fn write_file(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        self.memory.lock().unwrap_or_else(PoisonError::into_inner).insert(path.to_owned(), Arc::from(data));
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        self.memory(path).is_some() || kition_kit::host::size(path).is_ok()
    }

    fn stamp(&self, _path: &str) -> Option<(u64, u128)> {
        None
    }

    fn store_file(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        self.write_file(path, data)
    }

    fn list_dir(&self, _dir: &str) -> Vec<String> {
        Vec::new()
    }

    fn is_dir(&self, _path: &str) -> bool {
        false
    }
}

/// An RGBA picture `from_w` × `from_h` blown up to `to_w` × `to_h` by repeating its pixels (a
/// lower preview resolution shows its blocks, as a viewer does). A short source reads as
/// transparent black where it has no pixel.
fn blow_up(rgba: &[u8], from_w: u32, from_h: u32, to_w: u32, to_h: u32) -> Vec<u8> {
    let (fw, fh, tw, th) = (from_w.max(1) as usize, from_h.max(1) as usize, to_w as usize, to_h as usize);
    let row_bytes = tw.saturating_mul(4);
    let mut out = vec![0u8; row_bytes.saturating_mul(th)];
    let columns: Vec<usize> = (0..tw).map(|x| (x * fw / tw.max(1)).min(fw - 1)).collect();
    let mut previous: Option<(usize, usize)> = None;
    for y in 0..th {
        let sy = (y * fh / th.max(1)).min(fh - 1);
        let to = y * row_bytes;
        // Rows drawn from the same source row are the same: copied rather than drawn again.
        if let Some((_, from)) = previous.filter(|(source, _)| *source == sy) {
            out.copy_within(from..from + row_bytes, to);
            continue;
        }
        for (x, sx) in columns.iter().enumerate() {
            let from = (sy * fw + sx) * 4;
            let at = to + x * 4;
            if let (Some(src), Some(dst)) = (rgba.get(from..from + 4), out.get_mut(at..at + 4)) {
                dst.copy_from_slice(src);
            }
        }
        previous = Some((sy, to));
    }
    out
}

/// Lends a footage file to the media pool, which decodes from memory.
fn lend(pool: &MediaPool, path: &str) -> Result<Arc<[u8]>, String> {
    let bytes: Arc<[u8]> = kition_kit::host::read(path)?.into();
    pool.add_bytes(path, bytes.clone());
    Ok(bytes)
}

/// Import through the host: the file is lent to the pool, then probed from memory.
/// Video codecs the media crates name but cannot decode: their movies would draw black.
const UNDECODED_CODECS: [&str; 1] = ["VP8"];

struct LentImporter {
    pool: Arc<MediaPool>,
}

impl Importer for LentImporter {
    /// A movie whose pictures no decoder here reads (VP8) is refused, not imported to draw black.
    fn probe(&self, path: &str) -> Result<Footage, String> {
        let bytes = lend(&self.pool, path)?;
        let footage = kition_motion_media::probe_bytes(path, bytes).map_err(|error| error.to_string())?;
        if footage.has_video && UNDECODED_CODECS.iter().any(|codec| footage.codec.eq_ignore_ascii_case(codec)) {
            return Err(format!("{} video cannot be decoded; convert it to VP9 or H.264 and import that", footage.codec));
        }
        Ok(footage)
    }

    fn register(&self, path: &str, data: &[u8]) {
        self.pool.add_bytes(path, data.into());
    }

    fn add_frames(&self, paths: &[String]) {
        for path in paths {
            let _ = lend(&self.pool, path);
        }
    }
}

/// The transform properties a properties panel shows, in the timeline's order.
const TRANSFORM: [&str; 5] = ["transform/anchor", "transform/position", "transform/scale", "transform/rotation", "transform/opacity"];

/// One open Kition Motion project.
pub struct MotionDocument {
    session: Session,
    name: String,
    services: Arc<KitServices>,
    pool: Arc<MediaPool>,
    /// Footage files already lent to the pool.
    lent: HashSet<String>,
    /// The viewer's frames already drawn (see [`frames`]).
    frames: RefCell<frames::FrameCache>,
    /// The last frame the viewer was given, at the composition's size (unpacked and blown up
    /// from the cache once, not at each repaint or pixel read): its revision and key.
    shown: RefCell<Option<(u64, frames::FrameKey, Arc<Vec<u8>>, u32, u32)>>,
    /// The movie the Export dialog is rendering, a little at a time (kept per thread, see
    /// [`movie::ExportKey`]).
    export: movie::ExportKey,
    /// Whether a moved layer snaps to the composition's guides and to the grid (View ▸ Snap to
    /// Guides, Snap to Grid: the person's view, which the editor sets).
    snap_guides: bool,
    snap_grid: bool,
    /// View ▸ Resolution: the composition renders every `preview`-th pixel (1 Full, 2 Half,
    /// 3 Third, 4 Quarter) and the picture is that render blown up to the composition's size.
    preview: u32,
    /// The Pen's option on a shape layer: draw a mask on it (true) or a new shape path in it.
    pub(crate) pen_masks: bool,
    /// The project as it was when the Render Queue panel's Render was pressed: every job of
    /// that run renders from it (see [`editor_reads`]).
    render_snapshot: Option<Arc<kition_motion_project::Project>>,
    /// What the canvas tools name a new shape layer (`形状图层` in Chinese), numbered; `None`: the
    /// engine's English name. The editor sets it in its language (`setLayerNames`).
    pub(crate) shape_layer_name: Option<String>,
}

/// A native host (`bindings/headless`) needs the document to be `Send`: checked here, where the
/// kit is compiled, and not only by the headless binding.
const _: fn() = || {
    fn send<T: Send>() {}
    send::<MotionDocument>();
};

impl MotionDocument {
    /// Refuses a parameter `command` does not take (a typo or a guessed name) before it runs,
    /// naming the one it resembles, instead of running without it (engine rule E7a). The schemas
    /// are the ones the catalogue offers agents, read once.
    pub(crate) fn refuse_unknown(&self, command: &str, params: &Value) -> Result<(), String> {
        static SCHEMAS: std::sync::OnceLock<std::collections::HashMap<String, Value>> = std::sync::OnceLock::new();
        let schemas = SCHEMAS.get_or_init(|| self.commands().into_iter().filter_map(|command| Some((command.id, command.schema?))).collect());
        schemas.get(command).map_or(Ok(()), |schema| kition_kit::schema::refuse_unknown(command, schema, params).map_err(params_failure))
    }

    fn new_session(services: &Arc<KitServices>, pool: &Arc<MediaPool>) -> Session {
        let mut session = kition_motion_host::session();
        session.services = services.clone();
        session.footage = pool.clone();
        session.importer = Some(Arc::new(LentImporter { pool: pool.clone() }));
        // Renders land in memory; the host takes them out and saves them as new files.
        let memory = services.clone();
        let sink: Arc<kition_motion_export::Sink> = Arc::new(move |path: &str, data: Vec<u8>| {
            memory.memory.lock().unwrap_or_else(PoisonError::into_inner).insert(path.to_owned(), Arc::from(data));
        });
        session.exporter = Some(Arc::new(kition_motion_host::FileExporter { sink: Some(sink) }));
        session.models_dir = None;
        // The host owns every file (KHP A29): nothing reaches the file system by path, neither
        // the engine (auto-saves, the log file, disk caches, scripts, folders), the footage
        // pool (lent files only) nor effect parameters naming files (a LUT, an OCIO config).
        session.host_files_only = true;
        pool.set_memory_only(true);
        kition_motion_engine::effects::files::deny_paths();
        // Relative render outputs resolve against a virtual root, never the user's home folder.
        session.prefs.export.default_output_folder = "/".to_owned();
        session
    }

    fn from_session(session: Session, name: &str, services: Arc<KitServices>, pool: Arc<MediaPool>) -> Self {
        let mut doc = Self {
            session,
            name: name.to_owned(),
            services,
            pool,
            lent: HashSet::new(),
            frames: RefCell::new(frames::FrameCache::default()),
            shown: RefCell::new(None),
            export: movie::ExportKey::new(),
            snap_guides: true,
            snap_grid: false,
            preview: 1,
            pen_masks: false,
            render_snapshot: None,
            shape_layer_name: None,
        };
        doc.lend_footage();
        doc
    }

    /// Lends every footage file the project names that is not lent yet; a missing one renders
    /// as offline footage, as in the desktop app.
    fn lend_footage(&mut self) {
        let mut paths = Vec::new();
        for item in self.session.project.items.values() {
            if let ItemKind::Footage(footage) = &item.kind {
                paths.push(footage.path.clone());
                paths.extend(footage.sequence.iter().cloned());
            }
        }
        for path in paths {
            if path.is_empty() || self.lent.contains(&path) || self.services.memory(&path).is_some() {
                continue;
            }
            if lend(&self.pool, &path).is_ok() {
                self.lent.insert(path);
            }
        }
    }

    fn comp_size(&self) -> Option<(u32, u32)> {
        self.session.active_comp().map(|comp| (comp.width, comp.height))
    }

    /// What the viewer's frame now depends on besides the project (see [`frames::FrameKey`]).
    fn frame_key(&self) -> Option<frames::FrameKey> {
        self.frame_key_at(self.session.time())
    }

    /// [`Self::frame_key`] at another time.
    fn frame_key_at(&self, at: Tick) -> Option<frames::FrameKey> {
        let comp_id = self.session.active_comp_id()?;
        let comp = self.session.project.comp(comp_id)?;
        let time = comp.frame_rate.snap(at);
        let view = self
            .session
            .view_camera(comp_id)
            .map(|camera| {
                let mut bits: Vec<u64> = camera.view.0.iter().flatten().map(|v| v.to_bits()).collect();
                bits.extend([camera.eye.x, camera.eye.y, camera.eye.z, camera.zoom].iter().map(|v| v.to_bits()));
                bits.push(u64::from(camera.ortho));
                bits
            })
            .unwrap_or_default();
        let roi = self.region(comp).map(|r| r.iter().map(|v| v.to_bits()).collect()).unwrap_or_default();
        Some(frames::FrameKey { comp: comp_id, time, preview: self.preview, view, roi })
    }

    /// The region of interest the viewer renders (View ▸ Region of Interest), inside the
    /// composition's frame; none when there is none or it misses the frame.
    fn region(&self, comp: &kition_motion_project::Comp) -> Option<[f64; 4]> {
        let [x, y, w, h] = self.session.state.region_of_interest?;
        let (cw, ch) = (f64::from(comp.width.max(1)), f64::from(comp.height.max(1)));
        let (x0, y0) = (x.max(0.0).floor(), y.max(0.0).floor());
        let (x1, y1) = ((x + w).min(cw).ceil(), (y + h).min(ch).ceil());
        [x0, y0, x1, y1].iter().all(|v| v.is_finite()).then_some(())?;
        (x1 - x0 >= 1.0 && y1 - y0 >= 1.0).then_some([x0, y0, x1 - x0, y1 - y0])
    }

    /// The active composition's frame at the current time, at the composition's size, over its
    /// background colour as the viewer shows it: through the 3D view chosen (View ▸ 3D View), only
    /// the region of interest rendered (the rest grey), at View ▸ Resolution (rendered at that
    /// resolution and blown up to the composition's size). A frame drawn before (same revision and
    /// settings) comes from the frame cache, which keeps it as rendered.
    fn frame(&self) -> Result<(Arc<Vec<u8>>, u32, u32), String> {
        let key = self.frame_key().ok_or("no composition is open")?;
        let revision = self.session.revision;
        let cached = self.frames.borrow_mut().get(revision, &key);
        if let Some(drawn) = cached {
            if let Some((_, _, rgba, w, h)) = self.shown.borrow().as_ref().filter(|(at, shown, ..)| *at == revision && *shown == key) {
                return Ok((rgba.clone(), *w, *h));
            }
            if let Some(rgba) = drawn.rgba() {
                return Ok(self.show(revision, key, &rgba, drawn.width, drawn.height));
            }
        }
        let (rgba, width, height, ms) = self.render_frame(&key)?;
        self.frames.borrow_mut().put(revision, key.clone(), &rgba, width, height, ms);
        Ok(self.show(revision, key, &rgba, width, height))
    }

    /// A frame as rendered for `key` (at the preview resolution), at the composition's size as the
    /// viewer shows it, remembered as the last frame shown.
    fn show(&self, revision: u64, key: frames::FrameKey, rgba: &[u8], width: u32, height: u32) -> (Arc<Vec<u8>>, u32, u32) {
        let size = self.session.project.comp(key.comp).map(|comp| (comp.width.max(1), comp.height.max(1)));
        let blown = match size {
            Some((to_w, to_h)) if (key.preview > 1 || !key.roi.is_empty()) && (width, height) != (to_w, to_h) => {
                (Arc::new(blow_up(rgba, width, height, to_w, to_h)), to_w, to_h)
            }
            _ => (Arc::new(rgba.to_vec()), width, height),
        };
        *self.shown.borrow_mut() = Some((revision, key, blown.0.clone(), blown.1, blown.2));
        blown
    }

    /// Renders the viewer's frame for `key` (see [`Self::frame`]) at the preview resolution (a
    /// region of interest drawn where it is in that frame, the rest grey); answers it and the
    /// milliseconds it took.
    fn render_frame(&self, key: &frames::FrameKey) -> Result<(Vec<u8>, u32, u32, f64), String> {
        let comp_id = key.comp;
        let started = web_time::Instant::now();
        let comp = self.session.project.comp(comp_id).ok_or("no composition is open")?;
        let scale = 1.0 / f64::from(key.preview.max(1));
        let roi = self.region(comp);
        let opts = RenderOpts { scale, view: self.session.view_camera(comp_id), roi, ..Default::default() };
        let mut renderer = Renderer::new(&self.session.project, &*self.session.footage, opts);
        renderer.expr = self.session.expr.as_deref();
        renderer.cache = Some(&self.session.layer_cache);
        let image = renderer.comp_frame(comp_id, key.time);
        let mut rendered = image.to_rgba8_over(comp.background);
        let (mut width, mut height) = (image.width, image.height);
        if let Some([x, y, _, _]) = roi {
            // The region drawn where it is in the frame; the rest of the frame grey.
            let (fw, fh) = (((f64::from(comp.width) * scale).round() as u32).max(1), ((f64::from(comp.height) * scale).round() as u32).max(1));
            let mut full = [48u8, 48, 48, 255].repeat(fw as usize * fh as usize);
            let (ox, oy) = ((x * scale).round() as usize, (y * scale).round() as usize);
            for row in 0..height as usize {
                let take = (width as usize).min((fw as usize).saturating_sub(ox)) * 4;
                let from = row * width as usize * 4;
                let to = ((oy + row) * fw as usize + ox) * 4;
                if let (Some(dst), Some(src)) = (full.get_mut(to..to + take), rendered.get(from..from + take)) {
                    dst.copy_from_slice(src);
                }
            }
            rendered = full;
            (width, height) = (fw, fh);
        }
        Ok((rendered, width, height, started.elapsed().as_secs_f64() * 1000.0))
    }

    /// Cache Work Area in Background: renders, for about `milliseconds`, the work area's frames
    /// the frame cache does not hold yet for the viewer's settings now, from the current time on
    /// (round to the work area's start), without showing them. It stops at the cache's memory
    /// budget rather than drop frames it holds. Answers `{done, total, full}`: frames held of the
    /// work area's, and whether the budget stopped it. An edit (a new revision) empties the cache,
    /// so the next call starts again.
    pub fn cache_ahead(&self, milliseconds: f64) -> Value {
        let Some(comp) = self.session.active_comp() else { return json!({ "done": 0, "total": 0, "full": false }) };
        let rate = comp.frame_rate;
        let (from, to) = if comp.work_area.1 > comp.work_area.0 { comp.work_area } else { (Tick(0), comp.duration) };
        let (first, end) = (rate.frame_at(rate.snap_nearest(from)), rate.frame_at(rate.snap_nearest(to)));
        let total = end.saturating_sub(first).max(0);
        // At most a few thousand frames are looked at.
        let total = total.min(100_000);
        let revision = self.session.revision;
        let started = web_time::Instant::now();
        let budget = if milliseconds.is_finite() { milliseconds.clamp(0.0, 2000.0) } else { 0.0 };
        let now = rate.frame_at(self.session.time()).clamp(first, end.saturating_sub(1).max(first));
        let mut full = false;
        for step in 0..total {
            let frame = first + (now - first + step).rem_euclid(total.max(1));
            let Some(key) = self.frame_key_at(rate.tick_of(frame)) else { break };
            if self.frames.borrow().holds(revision, &key) {
                continue;
            }
            if started.elapsed().as_secs_f64() * 1000.0 >= budget {
                break;
            }
            let Ok((rgba, width, height, ms)) = self.render_frame(&key) else { break };
            if !self.frames.borrow_mut().put_ahead(revision, key, &rgba, width, height, ms) {
                full = true;
                break;
            }
        }
        let held =
            (0..total).filter(|step| self.frame_key_at(rate.tick_of(first + step)).is_some_and(|key| self.frames.borrow().holds(revision, &key))).count();
        let cache = self.frames.borrow();
        let report = cache.report(revision, None, |_| 0);
        json!({ "done": held, "total": total, "full": full, "bytes": report["bytes"], "budget": cache.budget(), "preview": self.preview })
    }

    /// What the frame cache holds for the viewer's settings now (the frames of the active
    /// composition, by number from its start), how many bytes, and the frames shown, rendered and
    /// found in the cache since the document opened (playback's frame rate is read from them).
    pub fn preview_cache(&self) -> Value {
        let key = self.frame_key();
        let rate = self.session.active_comp().map(|comp| comp.frame_rate);
        self.frames.borrow().report(self.session.revision, key.as_ref(), |t| rate.map_or(0, |rate| rate.frame_at(t)))
    }

    /// View ▸ Purge ▸ Image Cache: drops every frame kept.
    pub fn purge_preview_cache(&mut self) {
        self.frames.borrow_mut().clear();
        *self.shown.borrow_mut() = None;
    }

    /// The most memory the frame cache keeps, in megabytes (the editor sets it from the device's
    /// memory), kept between 400 and 1536; a number that is not finite leaves it as it was.
    /// Answers the budget now, in megabytes.
    pub fn set_preview_cache_budget(&self, megabytes: f64) -> f64 {
        let mut cache = self.frames.borrow_mut();
        if megabytes.is_finite() && megabytes > 0.0 {
            let bytes = (megabytes.min(1_000_000.0) * 1024.0 * 1024.0) as usize;
            cache.set_budget(bytes);
        }
        cache.budget() as f64 / (1024.0 * 1024.0)
    }

    /// What `comp.info` reports for the active composition (layers, timing), with each layer's
    /// animated properties (by the path `prop.*` commands take) and their key times in composition
    /// seconds, and whether it is selected.
    fn comp_info(&self) -> Value {
        let mut scratch = Session { project: self.session.project.clone(), state: self.session.state.clone(), ..Session::default() };
        let mut info = scratch.execute("comp.info", json!({})).unwrap_or(Value::Null);
        let Some(comp) = self.session.active_comp() else { return info };
        let seconds = |tick: Tick| tick.0 as f64 / TICKS_PER_SECOND as f64;
        if let Some(layers) = info.get_mut("layers").and_then(Value::as_array_mut) {
            for row in layers {
                let Some(layer) = row.get("id").and_then(Value::as_u64).and_then(|id| comp.layers.iter().find(|layer| layer.id.0 == id)) else {
                    continue;
                };
                let stretch = layer.stretch / 100.0;
                let mut animated = Vec::new();
                layer.props.walk("", &mut |_, prop| {
                    if !prop.keys.is_empty() {
                        let times: Vec<f64> = prop.keys.iter().map(|key| seconds(layer.start_time) + seconds(key.time) * stretch).collect();
                        let path = prop_path(&layer.props, prop.uid);
                        animated.push(json!({ "path": path, "name": prop.name, "times": times }));
                    }
                });
                row["animated"] = Value::Array(animated);
                row["selected"] = Value::Bool(self.session.state.selected_layers.contains(&layer.id));
                // The properties selected on it (a click on a property's name in the timeline),
                // by the paths `prop.*` take: what Add Expression and the like act on.
                let chosen: Vec<Value> = self
                    .session
                    .state
                    .selected_props
                    .iter()
                    .filter(|(id, _)| *id == layer.id)
                    .filter_map(|(_, uid)| prop_path(&layer.props, *uid))
                    .map(Value::String)
                    .collect();
                row["selectedProps"] = Value::Array(chosen);
            }
        }
        // The first selected layer's transform at the current time, for a properties panel.
        if let Some(layer) = self.session.state.selected_layers.first() {
            let time = seconds(self.session.time());
            let transform: Vec<Value> = TRANSFORM
                .iter()
                .filter_map(|path| scratch.execute("prop.get", json!({ "layer": layer.0, "path": path, "time": time, "timeBase": "comp" })).ok())
                .map(|prop| json!({ "path": format!("transform/{}", prop["match"].as_str().unwrap_or_default()), "name": prop["name"], "type": prop["type"], "value": prop["value"], "animated": prop["animated"] }))
                .collect();
            info["transform"] = json!({ "layer": layer.0, "properties": transform });
        }
        info
    }

    /// The active composition rendered as a movie (`format`: `h264` by default, or the Export
    /// dialog's settings, [`movie::queue_params`]) named `path`, from
    /// a copy of the project, so the document and its Render Queue are left as they are. Returns
    /// the file's bytes; the host saves it as a new file.
    pub fn render_movie(&self, path: &str, format: &str) -> Result<Vec<u8>, String> {
        let mut export = self.new_export(path, format)?;
        while !export.step(f64::INFINITY).finished {}
        let path = export.finished()?;
        self.take_file(&path).ok_or_else(|| "the render produced no file".to_owned())
    }

    /// A render of the active composition as a movie named `path` (`settings` as
    /// [`MotionDocument::render_movie`] takes them), from a copy of the project, so the document
    /// and its Render Queue are left as they are. Nothing has rendered yet.
    fn new_export(&self, path: &str, settings: &str) -> Result<movie::Export, String> {
        // A Render Queue panel job (`{"queueItem": id}`) renders from the queue's copy.
        if let Some(item) = movie::queue_item(settings) {
            return self.new_queue_export(item, path);
        }
        let comp = self.session.active_comp_id().ok_or("no composition is open")?;
        let mut scratch = Self::new_session(&self.services, &self.pool);
        scratch.state = self.session.state.clone();
        scratch.project = Arc::new({
            let mut project = (*self.session.project).clone();
            project.render_queue.clear();
            project
        });
        scratch.execute("renderQueue.add", movie::queue_params(comp.0, path, settings)?).map_err(|error| error.to_string())?;
        Ok(movie::Export::new(scratch.render_stepped()?, path))
    }

    /// Starts an export as [`MotionDocument::render_movie`] makes it, without rendering yet:
    /// [`MotionDocument::export_step`] renders a little at a time, [`MotionDocument::export_finish`]
    /// hands over the file and [`MotionDocument::export_cancel`] stops. Starting again drops an
    /// unfinished export.
    pub fn export_start(&mut self, path: &str, settings: &str) -> Result<movie::Progress, String> {
        self.export_cancel();
        let export = self.new_export(path, settings)?;
        let progress = export.progress(false);
        self.export.put(export);
        Ok(progress)
    }

    /// Renders the export for about `milliseconds` (at least one batch of frames) and says how
    /// far it is.
    pub fn export_step(&mut self, milliseconds: f64) -> Result<movie::Progress, String> {
        self.export.with(|export| export.step(milliseconds)).ok_or_else(|| "no export is running".to_owned())
    }

    /// The finished export's files, by name, left in memory for [`Self::take_file`]: the one
    /// file of a movie or a sound, or every frame of an image sequence (its output named the
    /// frame number `[#####]`), in order. An export that failed answers why.
    pub fn export_finish_files(&mut self) -> Result<Vec<String>, String> {
        let export = self.export.take().ok_or("no export is running")?;
        let first = export.finished()?;
        let pattern = export.path().to_owned();
        let names = if pattern.contains('#') { self.sequence_files(&pattern) } else { vec![first.trim_start_matches('/').to_owned()] };
        if names.is_empty() {
            return Err("the render produced no file".to_owned());
        }
        Ok(names)
    }

    /// The files in memory an image sequence named `pattern` wrote (`Comp_[#####].png`: the
    /// part before the run of `#`, digits, the part after), in order.
    fn sequence_files(&self, pattern: &str) -> Vec<String> {
        let pattern = pattern.trim_start_matches('/');
        let Some(at) = pattern.find('#') else { return Vec::new() };
        let prefix = pattern.get(..at).unwrap_or("").trim_end_matches('[');
        let suffix = pattern.get(at..).unwrap_or("").trim_start_matches('#').trim_start_matches(']');
        let memory = self.services.memory.lock().unwrap_or_else(PoisonError::into_inner);
        let mut names: Vec<String> = memory
            .keys()
            .map(|key| key.trim_start_matches('/'))
            .filter(|key| {
                key.len() > prefix.len() + suffix.len()
                    && key.starts_with(prefix)
                    && key.ends_with(suffix)
                    && key.get(prefix.len()..key.len() - suffix.len()).is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            })
            .map(str::to_owned)
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// File ▸ Dependencies ▸ Collect Files: a copy of the project and every footage file it uses,
    /// in memory under `folder` (inside the project's folder) for the host to write: the project
    /// as `<folder>/<project name>` with its footage pointing at `(Footage)/<file>`, each file
    /// beside it there (same names made unique). Answers `{files, missing}`: the names written (for
    /// [`Self::take_file`]) and the footage that could not be read (left pointing where it was).
    pub fn collect_files(&self, folder: &str) -> Result<Value, String> {
        let folder = folder.trim().trim_matches('/');
        if folder.is_empty() || folder.split('/').any(|part| part.is_empty() || part == "." || part == "..") || folder.contains('\\') {
            return Err("the folder must be a name inside the project's folder".to_owned());
        }
        let mut project = (*self.session.project).clone();
        let mut taken: HashSet<String> = HashSet::new();
        let mut copied: HashMap<String, String> = HashMap::new();
        let mut files = Vec::new();
        let mut missing = Vec::new();
        let mut memory: Vec<(String, Arc<[u8]>)> = Vec::new();
        for item in project.items.values_mut() {
            let ItemKind::Footage(footage) = &mut item.kind else { continue };
            for path in std::iter::once(&mut footage.path).chain(footage.sequence.iter_mut()) {
                if path.is_empty() {
                    continue;
                }
                if let Some(done) = copied.get(path.as_str()) {
                    *path = done.clone();
                    continue;
                }
                let Ok(bytes) = self.services.read_file(path) else {
                    missing.push(path.clone());
                    continue;
                };
                let base = path.rsplit(['/', '\\']).next().unwrap_or("footage").to_owned();
                let (stem, extension) = match base.rsplit_once('.') {
                    Some((stem, extension)) if !stem.is_empty() => (stem.to_owned(), format!(".{extension}")),
                    _ => (base.clone(), String::new()),
                };
                let mut name = base.clone();
                let mut n = 2;
                while !taken.insert(name.to_lowercase()) {
                    name = format!("{stem} {n}{extension}");
                    n += 1;
                }
                let relative = format!("(Footage)/{name}");
                memory.push((format!("{folder}/{relative}"), Arc::from(bytes)));
                copied.insert(path.clone(), relative.clone());
                *path = relative;
            }
        }
        let project_name = self.name.rsplit('/').next().filter(|name| !name.is_empty()).unwrap_or("Project.kmotion").to_owned();
        memory.push((format!("{folder}/{project_name}"), Arc::from(project.to_content_json().into_bytes())));
        let mut written = self.services.memory.lock().unwrap_or_else(PoisonError::into_inner);
        for (name, bytes) in memory {
            files.push(name.clone());
            written.insert(name, bytes);
        }
        Ok(json!({ "files": files, "missing": missing }))
    }

    /// The finished export's file; an export that failed answers why, one still running says so.
    pub fn export_finish(&mut self) -> Result<Vec<u8>, String> {
        let export = self.export.take().ok_or("no export is running")?;
        let path = export.finished()?;
        self.take_file(&path).ok_or_else(|| "the render produced no file".to_owned())
    }

    /// Whether the Selection tool snaps moved layers to the guides and to the grid; answers the
    /// grid's spacing in composition pixels (what the editor draws the grid at).
    pub fn set_snap_targets(&mut self, guides: bool, grid: bool) -> f64 {
        self.snap_guides = guides;
        self.snap_grid = grid;
        self.session.prefs.grids.grid_spacing
    }

    /// View ▸ Grid Settings: the grid's spacing in composition pixels (what the Selection tool snaps
    /// to), kept between 2 and 10 000; a spacing that is not a finite number leaves it as it was.
    /// Answers the spacing now.
    pub fn set_grid_spacing(&mut self, spacing: f64) -> f64 {
        if spacing.is_finite() {
            self.session.prefs.grids.grid_spacing = spacing.clamp(2.0, 10_000.0);
        }
        self.session.prefs.grids.grid_spacing
    }

    /// View ▸ Resolution: renders every `divisor`-th pixel (1 Full, 2 Half, 3 Third, 4 Quarter);
    /// anything else is refused. The picture keeps the composition's size.
    pub fn set_preview_resolution(&mut self, divisor: u32) -> Result<(), String> {
        if !(1..=4).contains(&divisor) {
            return Err(format!("the preview resolution is 1 (Full), 2 (Half), 3 (Third) or 4 (Quarter), not {divisor}"));
        }
        self.preview = divisor;
        Ok(())
    }

    /// The Pen's option on a shape layer: `true` draws a mask on it, `false` (the default) a new
    /// path in its contents.
    pub fn set_pen_masks(&mut self, masks: bool) {
        self.pen_masks = masks;
    }

    /// Stops the export and drops what it wrote; whether one was running.
    pub fn export_cancel(&mut self) -> bool {
        let Some(mut export) = self.export.take() else { return false };
        export.cancel();
        let _ = self.take_file(export.path());
        // An image sequence's frames written so far go too.
        if export.path().contains('#') {
            for name in self.sequence_files(export.path()) {
                let _ = self.take_file(&name);
            }
        }
        true
    }

    /// The keyframed values of one property of a layer in the active composition (`path` as
    /// `prop.*` take it) at `count` evenly spaced composition times from `from` to `to` seconds,
    /// as JSON (an array of values): the curves a graph editor draws, without an expression.
    /// `count` is capped at [`MAX_SAMPLES`]; an unknown layer or path answers an error.
    pub fn curve_samples(&self, layer: u64, path: &str, from: f64, to: f64, count: u32) -> Result<String, String> {
        let comp = self.session.active_comp().ok_or("no composition is open")?;
        let layer = comp.layers.iter().find(|candidate| candidate.id.0 == layer).ok_or("no such layer")?;
        let mut found = None;
        layer.props.walk("", &mut |_, prop| {
            if found.is_none() && prop_path(&layer.props, prop.uid).as_deref() == Some(path) {
                found = Some(prop);
            }
        });
        let prop = found.ok_or("no such property")?;
        if !from.is_finite() || !to.is_finite() {
            return Err("the times must be numbers".to_owned());
        }
        let count = count.clamp(2, MAX_SAMPLES);
        let step = (to - from) / f64::from(count - 1);
        let values: Vec<Value> =
            (0..count).map(|index| prop.value_at(layer.layer_time(Tick::from_seconds_f64(from + step * f64::from(index)))).to_json()).collect();
        serde_json::to_string(&values).map_err(|error| error.to_string())
    }

    /// The active composition's sound from `from` seconds for `seconds` (at most
    /// [`MAX_AUDIO_SECONDS`]) at `rate` Hz, stereo and interleaved (L R L R …): what preview
    /// playback plays. Empty when the composition has nothing audible.
    pub fn mix_audio(&self, from: f64, seconds: f64, rate: u32) -> Result<Vec<f32>, String> {
        let comp = self.session.active_comp_id().ok_or("no composition is open")?;
        if !from.is_finite() || !seconds.is_finite() || seconds <= 0.0 || !(8_000..=192_000).contains(&rate) {
            return Err("from and seconds must be numbers, seconds above 0, and the rate 8000 to 192000 Hz".to_owned());
        }
        if !kition_motion_engine::render::audio::comp_has_audio(&self.session.project, comp) {
            return Ok(Vec::new());
        }
        let frames = (seconds.min(MAX_AUDIO_SECONDS) * f64::from(rate)).round() as usize;
        let start = Tick::from_seconds_f64(from.max(0.0));
        Ok(kition_motion_engine::render::audio::mix_comp(
            &self.session.project,
            &*self.session.footage,
            self.session.expr.as_deref(),
            comp,
            start,
            frames,
            rate,
        ))
    }

    /// Whether running `command` with `params` (JSON) now would change the project, tried on a copy
    /// so nothing changes: the editor hides a menu row that would do nothing (Center Anchor Point
    /// on a layer whose anchor is already there, Trim In Point at the in point). Only the commands
    /// in [`TRIED_COMMANDS`] are tried; any other, a refused one, or bad JSON answers false.
    pub fn would_change(&self, command: &str, params: &str) -> bool {
        if !TRIED_COMMANDS.contains(&command) {
            return false;
        }
        let Ok(params) = serde_json::from_str::<Value>(params) else { return false };
        let mut scratch = Session {
            project: self.session.project.clone(),
            state: self.session.state.clone(),
            services: self.session.services.clone(),
            footage: self.session.footage.clone(),
            ..Session::default()
        };
        let before = scratch.project.clone();
        scratch.execute(command, params).is_ok() && !scratch.project.same_content(&before)
    }

    /// Takes a file the engine wrote (an export) out of memory; the host saves it.
    pub fn take_file(&self, path: &str) -> Option<Vec<u8>> {
        if path == self.name {
            return None;
        }
        let mut memory = self.services.memory.lock().unwrap_or_else(PoisonError::into_inner);
        let rooted = format!("/{}", path.trim_start_matches('/'));
        memory.remove(path).or_else(|| memory.remove(&rooted)).map(|bytes| bytes.to_vec())
    }
}

/// The places in [`KitDocument::inspect`] that are the editor's state, not the document's: the
/// current time (`/time`), the selected layer's transform at that time (the properties panel,
/// `/info/transform`), and whether each layer is selected (`selected`, a key of every layer row and
/// of nothing else). They are no part of what `view.describe` answers an agent (`motion_inspect`'s
/// details), so moving the current time or selecting layers changes nothing the agent reads of the
/// document, while keyframe times and the animated properties' paths, which are the document's,
/// stay. `kition-motion` passes them to kition-headless as its volatile keys.
pub const EDITOR_STATE: &[&str] = &["/time", "/info/transform", "selected"];

/// The most samples [`MotionDocument::curve_samples`] takes per call.
pub const MAX_SAMPLES: u32 = 2000;

/// The commands [`MotionDocument::would_change`] tries: layer and keyframe edits that change
/// only the project, never a file or the history beyond the copy.
pub const TRIED_COMMANDS: &[&str] = &[
    "layer.centerAnchor",
    "layer.transform",
    "layer.timing",
    "layer.freezeFrame",
    "layer.align",
    "layer.distribute",
    "layer.sequence",
    "keyframe.timeReverse",
    "prop.setExpression",
    "path.convertToBezier",
    "layer.style.removeAll",
    "layer.enableTimeRemap",
    "file.removeUnusedFootage",
    "keyframe.exponentialScale",
    "prop.convertExpressionToKeyframes",
    "view.reset3DView",
    // Rove Across Time on the selected keyframes (a first or last one cannot rove).
    "keyframe.interpolation",
];

/// The longest stretch of sound [`MotionDocument::mix_audio`] mixes per call, in seconds.
pub const MAX_AUDIO_SECONDS: f64 = 10.0;

/// The command the agent's `<prefix>_export` runs (KHP A41, kition-kits `KitExport`): the active
/// composition as an MP4, its bytes in the answer (base64 `data`), so no path reaches the engine
/// and it may run headless. Rendered from a copy, as [`MotionDocument::render_movie`].
const EXPORT_MOVIE: &str = "file.exportMovie";

/// Every effect's name, the keywords of `effect.apply` (KHP command keywords): an agent's search
/// for `gradient ramp` or `blur` finds the command the effect is a parameter of (khp-bench agents
/// found none: the effects are no commands of their own).
fn effect_names() -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names: Vec<String> = kition_motion_engine::effects::all().into_iter().map(|effect| effect.name.to_string()).collect();
        names.sort();
        names.dedup();
        names
    })
}

/// A command's search keywords (KHP command keywords): the effect names for `effect.apply`, and
/// "delete layers" for `edit.delete`, which deletes them (khp-bench r2: an agent asked for
/// `layer.delete`, which does not exist, and was offered `layer.select`).
fn keywords(id: &str) -> Vec<String> {
    match id {
        "effect.apply" => effect_names().to_vec(),
        "edit.delete" => ["delete layers", "remove layers"].map(str::to_owned).to_vec(),
        _ => Vec::new(),
    }
}

/// The commands that make or change a text layer: their answers carry design advice on it.
const TEXT_COMMANDS: &[&str] = &["layer.newText", "layer.setText"];

impl MotionDocument {
    /// `result` with `design`: advice on the text layer `command` made or changed (the answer's
    /// `layer`, else the one `layer` named), when there is any.
    fn with_design(&self, command: &str, layer: Option<Value>, mut result: Value) -> Value {
        if !TEXT_COMMANDS.contains(&command) {
            return result;
        }
        let Some(comp) = self.session.active_comp() else { return result };
        let id = result.get("layer").cloned().or(layer).and_then(|v| match v {
            Value::Number(n) => n.as_u64().map(kition_motion_project::LayerId),
            Value::String(name) => comp.layers.iter().find(|l| l.name == name).map(|l| l.id),
            _ => None,
        });
        let Some(id) = id else { return result };
        let advice = design::on_layer(&self.session.project, comp, id);
        if advice.is_empty() {
            return result;
        }
        if result.is_null() {
            // layer.setText answers nothing of its own.
            return json!({ "design": advice });
        }
        if let Some(object) = result.as_object_mut() {
            object.insert("design".into(), Value::Array(advice));
        }
        result
    }
}

impl MotionDocument {
    fn export_movie(&self, params: &Value) -> Result<Value, String> {
        let format = params.get("format").and_then(Value::as_str).unwrap_or("mp4");
        if format != "mp4" {
            return Err(format!("Motion exports mp4, not {format}"));
        }
        let bytes = self.render_movie("export.mp4", "h264")?;
        Ok(json!({ "data": base64_encode(&bytes), "mime": "video/mp4", "bytes": bytes.len() }))
    }
}

/// Standard base64 with padding.
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        for k in 0..4 {
            out.push(if k <= chunk.len() { char::from(TABLE[((n >> (18 - 6 * k)) & 63) as usize]) } else { '=' });
        }
    }
    out
}

/// Families whose commands change only what this person sees or the application's own settings
/// (the current time, previews, panels, preferences), never the project.
const VIEWING_FAMILIES: [&str; 7] = ["time.", "playback.", "help.", "app.", "window.", "prefs.", "shortcuts."];

/// The `view.` commands that change the project: guides are the composition's, and the 3D view
/// commands that move a camera move its layer. The rest of `view.` is this person's viewer.
const EDITING_VIEWS: [&str; 8] = [
    "view.addGuide",
    "view.clearGuides",
    "view.importGuides",
    "view.moveGuide",
    "view.removeGuide",
    "view.lookAtSelected",
    "view.lookAtAll",
    "view.reset3DView",
];

/// Single journaled commands that never change the project: selections, the clipboard, closing
/// or revealing things, and exports that only write a file.
const VIEWING_COMMANDS: [&str; 27] = [
    "edit.selectAll",
    "edit.deselectAll",
    "edit.selectLabelGroup",
    "edit.copy",
    "edit.copyWithPropertyLinks",
    "edit.copyWithRelativePropertyLinks",
    "edit.copyExpressionOnly",
    "layer.select",
    "layer.selectNext",
    "layer.selectPrevious",
    "keyframe.select",
    "keyframe.selectAll",
    "project.select",
    "mask.selectVertices",
    "camera.selectPoints",
    "comp.close",
    "comp.revealInProject",
    "comp.flowchart",
    "comp.miniFlowchart",
    "layer.revealExpressionErrors",
    "animation.reveal",
    "text.fonts",
    "file.saveCopy",
    "file.saveCopyAsXml",
    "file.exportLottie",
    "file.exportTimeline",
    "mediaBrowser.go",
];

/// Whether running command `id` changes the document (the catalog's `mutates`): the engine
/// journals every command but pure queries, including viewing ones, so the journal flag alone
/// over-reports. Undo, redo, and the file commands that open, revert, or import report true.
fn changes_document(id: &str, journal: bool) -> bool {
    if matches!(id, "engine.batch" | "job.list" | "job.wait" | "renderQueue.list" | "camera.solveStatus" | "roto.status" | "track.status" | "warp.status") {
        return true;
    }
    if !journal || VIEWING_COMMANDS.contains(&id) || VIEWING_FAMILIES.iter().any(|family| id.starts_with(family)) {
        return false;
    }
    !id.starts_with("view.") || EDITING_VIEWS.contains(&id)
}

impl KitDocument for MotionDocument {
    fn batch_settler(&self) -> Option<kition_kit::BatchSettler> {
        Some(Box::new(|tx| kdm_model::settle_batch(tx).map_err(|error| error.to_string())))
    }
    /// What is wrong with the project (KHP A21): see [`check`]. Footage counts as
    /// present when it was lent to the engine or the host can read it.
    fn check(&self) -> Vec<kition_kit::diagnostics::Diagnostic> {
        check::check(&self.session.project, self.session.active_comp_id(), &mut |path| self.lent.contains(path) || self.services.exists(path))
    }

    fn open(name: &str, bytes: &[u8]) -> Result<Self, String> {
        let services = Arc::new(KitServices::default());
        services.write_file(name, bytes).map_err(|error| error.to_string())?;
        let pool = Arc::new(MediaPool::new());
        let mut session = Self::new_session(&services, &pool);
        session.execute("file.open", json!({ "path": name })).map_err(|error| error.to_string())?;
        if session.active_comp_id().is_none() {
            let first = session.project.items.iter().find_map(|(id, item)| matches!(item.kind, ItemKind::Comp(_)).then_some(*id));
            if let Some(id) = first {
                session.execute("comp.open", json!({ "comp": id.0 })).map_err(|error| error.to_string())?;
                session.mark_saved();
            }
        }
        Ok(Self::from_session(session, name, services, pool))
    }

    /// A new project with one composition of the given size: 30 fps, ten seconds, black.
    fn create(name: &str, width: u32, height: u32) -> Result<Self, String> {
        let services = Arc::new(KitServices::default());
        let pool = Arc::new(MediaPool::new());
        let mut session = Self::new_session(&services, &pool);
        session.execute("file.newProject", json!({})).map_err(|error| error.to_string())?;
        session
            .execute("comp.new", json!({ "name": "Comp 1", "width": width, "height": height, "frameRate": 30, "duration": 10, "open": true }))
            .map_err(|error| error.to_string())?;
        session.mark_saved();
        Ok(Self::from_session(session, name, services, pool))
    }

    fn width(&self) -> u32 {
        self.comp_size().map_or(0, |(w, _)| w)
    }

    fn height(&self) -> u32 {
        self.comp_size().map_or(0, |(_, h)| h)
    }

    fn revision(&self) -> u64 {
        self.session.revision
    }

    fn dirty(&self) -> bool {
        self.session.is_dirty()
    }

    fn render(&self, x: f64, y: f64, scale: f64, width: u32, height: u32) -> Result<Vec<u8>, String> {
        let (rgba, w, h) = self.frame()?;
        Ok(raster::render(Frame { rgba: &rgba, width: w, height: h }, x, y, scale, width, height))
    }

    fn render_area(&self, x0: i32, y0: i32, x1: i32, y1: i32, width: u32, height: u32) -> Result<Vec<u8>, String> {
        let (rgba, w, h) = self.frame()?;
        Ok(raster::render_area(Frame { rgba: &rgba, width: w, height: h }, x0, y0, x1, y1, width, height))
    }

    fn execute(&mut self, command: &str, params: Value) -> Result<Value, String> {
        self.refuse_unknown(command, &params)?;
        if command == EXPORT_MOVIE {
            return self.export_movie(&params);
        }
        if command == render::RENDER {
            return render::render(&self.session, &params);
        }
        rest_document::check(&self.session, command, &params)?;
        rest_delete::check(&self.session, command, &params)?;
        rest_inputs::check(&self.session, command, &params)?;
        rest_geometry::check(&self.session, command, &params)?;
        let layer = params.get("layer").cloned();
        let result = self.session.execute(command, params).map_err(command_failure)?;
        rest_document::after(&mut self.session, command);
        self.lend_footage();
        Ok(self.with_design(command, layer, result))
    }

    /// The project, the active composition (layers, timing), the current time, and history.
    fn inspect(&self) -> Result<Value, String> {
        let comp = self.session.active_comp().map(|comp| {
            json!({
                "id": self.session.active_comp_id().map(|id| id.0),
                "name": self.session.project.items.get(&self.session.active_comp_id().unwrap_or_default()).map(|item| item.name.clone()),
                "width": comp.width,
                "height": comp.height,
                "frameRate": comp.frame_rate.as_f64(),
                "duration": comp.duration.0 as f64 / TICKS_PER_SECOND as f64,
            })
        });
        let design = self.session.active_comp().map(|comp| design::review(&self.session.project, comp)).unwrap_or_default();
        Ok(json!({
            "comp": comp,
            "design": design,
            "info": self.comp_info(),
            "time": self.session.time().0 as f64 / TICKS_PER_SECOND as f64,
            "items": self.session.project.items.len(),
            "canUndo": !self.session.history.undo.is_empty(),
            "canRedo": !self.session.history.redo.is_empty(),
        }))
    }

    fn commands(&self) -> Vec<KitCommand> {
        let mut commands: Vec<kition_kit::KitCommand> = {
            let mut commands: Vec<KitCommand> = kition_motion_engine::commands::command_specs()
                .iter()
                .map(|spec| {
                    KitCommand {
                        id: spec.id.to_owned(),
                        title: spec.label.trim_end_matches("...").trim_end_matches('…').to_owned(),
                        menu: (!spec.menu.is_empty()).then(|| spec.menu.iter().map(|part| (*part).to_owned()).collect()),
                        shortcut: spec.shortcut.map(str::to_owned),
                        params: Some(spec.params.to_owned()),
                        mutates: changes_document(spec.id, spec.journal),
                        enabled: self.session.is_enabled(spec.id),
                        description: descriptions::description(spec.id).to_owned(),
                        output_schema: output_schema(spec.id, changes_document(spec.id, spec.journal)),
                        keywords: keywords(spec.id),
                        reads: acts::reads(spec.id),
                        ..Default::default()
                    }
                    .with_kit_schema()
                    .typed_from(param_types())
                })
                .collect();
            commands.push(
                KitCommand {
                    id: EXPORT_MOVIE.to_owned(),
                    title: "Export Movie".to_owned(),
                    description: descriptions::description(EXPORT_MOVIE).to_owned(),
                    output_schema: output_schema(EXPORT_MOVIE, false),
                    menu: Some(vec!["File".to_owned(), "Export".to_owned()]),
                    params: Some("{format?: mp4} → {data: base64, mime, bytes}".to_owned()),
                    mutates: false,
                    enabled: self.session.active_comp_id().is_some(),
                    // The movie is of the active comp.
                    reads: &[kition_kit::EditorState::Part],
                    ..Default::default()
                }
                .with_kit_schema(),
            );
            commands
        };
        for command in &mut commands {
            if let Some(schema) = command.schema.as_mut() {
                param_keys::ids_as_text(schema);
            }
            command.targets = acts::targets(&command.id, command.schema.as_ref());
            // `"#3"` in a layer parameter is the comp's third layer from the top: a place, not an
            // id, answered by `under` ([`place`]).
            if command.targets.is_some() {
                command.index = Some(place::INDEX);
                // A number that is no layer's id is a stack position too (`resolve_layer`).
                command.index_numbers = true;
            }
            command.position_independent = acts::position_independent(&command.id);
            if let Some(schema) = command.schema.as_mut() {
                param_keys::complete(&command.id, schema);
                param_keys::ids_as_text(schema);
                acts::complete_object_arrays(&command.id, schema);
            }
            // The parameters naming layers and items by id, and their types (kition-sdk C8-fix-9):
            // a comp item's id given to `layer` is no layer, and with `index_numbers` a stack position.
            command.objects = acts::objects(&command.id, command.schema.as_ref(), command.targets);
            command.object_types = acts::object_types(&command.id, command.schema.as_ref(), command.targets, command.objects);
            command.name_field = command.targets.map(|_| "name");
            command.name_params = acts::name_params(&command.id, command.schema.as_ref(), command.targets, command.objects);
            command.places = match command.id.as_str() {
                "layer.arrange" => &["index", "above"],
                "effect.reorder" => &["index", "effect"],
                "effect.remove" => &["effect"],
                "prop.set" | "prop.addKey" | "keyframe.set" | "keyframe.setEase" => &["time"],
                id if id.starts_with("renderQueue.") => &["index"],
                "keyframe.select" | "keyframe.info" => &["keys"],
                "prop.pickWhip" => &["target"],
                "mask.selectVertices" | "mask.moveVertices" | "mask.deleteVertices" | "mask.convertVertex" => &["vertices"],
                "path.convertToBezier" | "layer.mask.motionBlur" | "layer.mask.featherFalloff" => &["mask"],
                "transition.iris" => &["center"],
                _ => &[],
            };
        }
        kition_kit::files::declare(&mut commands, files::FILES);
        commands.push(render::command());
        commands
    }

    /// The commands that cannot run now, without building the full list and its schemas: what
    /// Kition asks after every change for menu and shortcut states.
    fn disabled_commands(&self) -> Vec<String> {
        kition_motion_engine::commands::command_specs().iter().filter(|spec| !(self.session.is_enabled(spec.id))).map(|spec| spec.id.to_owned()).collect()
    }

    /// The active comp (and the open comp tabs), each comp's current time, the selected layers and
    /// the properties, keyframes and mask vertices selected on them, the Project panel's selection,
    /// the camera track points and current track, each comp's 3D view and the region of interest,
    /// as a snapshot ([`kition_motion_engine::context::CallState`]), with the members the premise
    /// guard compares a command's declared reads by ([`state_members`]).
    fn editor_state(&self) -> Value {
        serde_json::to_value(self.session.call_state()).map(state_members::with_members).unwrap_or(Value::Null)
    }

    /// Puts back what [`KitDocument::editor_state`] answered, leaving out what the project no
    /// longer has; no edit.
    fn restore_editor_state(&mut self, saved: &Value) -> Result<(), String> {
        if saved.is_null() {
            return Ok(());
        }
        let state: kition_motion_engine::context::CallState =
            serde_json::from_value(state_members::snapshot(saved)).map_err(|error| format!("this is not the editor state this kit answered: {error}"))?;
        self.session.restore_call_state(&state);
        Ok(())
    }

    fn reconcile_reread(&mut self, fresh: &Self) -> Result<bool, String> {
        // The host has restored our state on the fresh document. Any dropped identity
        // belongs to an outside edit, so retain the previous session and report it.
        let before = state_members::snapshot(&self.editor_state());
        let restored = state_members::snapshot(&fresh.editor_state());
        if before != restored {
            let field = before
                .as_object()
                .into_iter()
                .flatten()
                .find(|(key, value)| restored.get(*key) != Some(*value))
                .map(|(key, _)| key.as_str())
                .unwrap_or("target");
            return Err(format!("deleted or unresolved editor {field} after file reread"));
        }
        self.session.reconcile_reread(&fresh.session.project).map_err(|error| error.to_string())?;
        self.lend_footage();
        Ok(true)
    }

    fn fork_for_premises(&self) -> Result<Self, String> {
        let mut copy = match self.session.kdm() {
            Some(replica) => Self::open_with_replica(&self.name, &[], &replica.snapshot().map_err(|error| error.to_string())?, &[])?,
            None => Self::open(&self.name, &self.save(None)?)?,
        };
        *copy.services.memory.lock().unwrap_or_else(PoisonError::into_inner) = self.services.memory.lock().unwrap_or_else(PoisonError::into_inner).clone();
        copy.session.state = self.session.state.clone();
        copy.session.prefs = self.session.prefs.clone();
        copy.restore_editor_state(&self.editor_state())?;
        Ok(copy)
    }

    /// Sets what a call acts in and on: `part` `{"comp": id}` (the comp's item id, as `file.render`'s
    /// `target.comp` names it) becomes the active comp; `time`, seconds of comp time, its current
    /// time (on the start of the frame showing it, as `file.render` draws it); `on`, layer ids of
    /// that comp (or names only one of its layers has), exactly the layer selection. A stack
    /// position or `#n` is refused: `on` takes ids. A comp, layer or time the project lacks is
    /// refused, saying what is valid, and nothing changes. No edit.
    fn set_context(&mut self, context: &CallContext) -> Result<(), String> {
        let comp = context.part.as_ref().map(call_comp).transpose().map_err(params_failure)?;
        let on = context
            .on
            .iter()
            .map(|member| match member {
                Value::Number(n) => n.as_u64().map(LayerRef::Id).ok_or_else(|| {
                    params_failure(format!("`on` holds {member}: layer ids are whole numbers, as motion_inspect lists them (`id` in details.info.layers)"))
                }),
                Value::String(name) => Ok(LayerRef::Name(name.clone())),
                other => Err(params_failure(format!("`on` holds {other}: give layer ids as motion_inspect lists them (`id` in details.info.layers)"))),
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Every refusal here is of the call's context: a comp, layer or time the project lacks.
        self.session.set_call_context(comp, context.time, &on).map_err(params_failure)
    }

    /// What a step names by its place now: a layer by stack position (`"#3"`) or a time with no
    /// layer named ([`place`]), for the premise guard. No edit.
    fn under(&self, command: &str, params: &Value) -> Result<Option<kition_kit::Under>, String> {
        if let Some(answer) = raw_sequences::under(&self.session, command, params)? {
            return Ok(Some(answer));
        }
        if let Some(answer) = rest_delete::under(&self.session, command, params)? {
            return Ok(Some(answer));
        }
        if let Some(answer) = raw_keys::under(&self.session, command, params)? {
            return Ok(Some(answer));
        }
        let mut answer = place::under(&self.session, command, params)?;
        if let Some(styles) = rest_styles::under(&self.session, command, params)? {
            let answer = answer.get_or_insert_with(kition_kit::Under::default);
            answer.premises.retain(|premise| !styles.premises.iter().any(|style| style.id == premise.id));
            answer.premises.extend(styles.premises);
        }
        rest_inputs::extend(&self.session, command, params, &mut answer);
        rest_reads::extend(&self.session, command, params, &mut answer);
        rest_reads::narrow(command, &mut answer);
        // engine.batch and file.runScript run their steps in order (batch.rs:54-75,
        // file_more.rs:430): each literal step's own inputs, as it would hold them alone. A step
        // that refers to an earlier step's result (`$1.layer`) names an object made in the run.
        // learn.step showMe runs the current tutorial step's commands (learn.rs:422-426).
        let shown: Vec<Value> = if command == "learn.step" && matches!(params.get("action").and_then(Value::as_str), Some("showMe" | "show")) {
            self.session
                .learn_current()
                .and_then(|(tutorial, index)| {
                    tutorial.steps.get(index).map(|step| step.show.iter().map(|(command, params)| json!({"command":command,"params":params})).collect())
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let steps = if command == "learn.step" { Some(&shown) } else { params.get("steps").and_then(Value::as_array) };
        if matches!(command, "engine.batch" | "file.runScript" | "learn.step") {
            for step in steps.into_iter().flatten().take(256) {
                let (Some(nested), Some(nested_params)) = (step.get("command").and_then(Value::as_str), step.get("params")) else { continue };
                if matches!(nested, "engine.batch" | "file.runScript") || nested_params.to_string().contains("\"$") {
                    continue;
                }
                if let Ok(Some(inner)) = self.under(nested, nested_params) {
                    let answer = answer.get_or_insert_with(kition_kit::Under::default);
                    answer.premises.extend(inner.premises);
                    answer.within.extend(inner.within);
                    answer.objects.extend(inner.objects);
                }
            }
        }
        rest_geometry::extend(&self.session, command, params, &mut answer)?;
        rest_document::extend(&self.session, command, params, &mut answer)?;
        if let Some(answer) = answer.as_mut() {
            place::merge_premises(answer);
        }
        Ok(answer)
    }

    fn under_step(&self, command: &str, params: &Value, step: &kition_kit::PremiseStep) -> Result<Option<kition_kit::Under>, String> {
        let mut answer = self.under(command, params)?;
        raw_keys::prefix(command, params, step, &mut answer);
        Ok(answer)
    }

    fn covered(&self, command: &str, params: &Value, objects: &[kition_kit::CoveredAsk]) -> Result<Option<kition_kit::CoveredAt>, String> {
        Ok(raw_sequences::covered(&self.session, command, params, objects)
            .or_else(|| raw_keys::covered(&self.session, command, params, objects))
            .or_else(|| rest_effect::covered(&self.session, command, params, objects))
            .or_else(|| rest_animation::covered(&self.session, command, params, objects)))
    }

    fn content_at(&self, ask: &kition_kit::ContentAsk, _history: &dyn kition_kit::HistoricalBlobs) -> Result<Option<bool>, String> {
        Ok(raw_keys::content(ask).or_else(|| rest_masks::content(&self.session, ask)).or_else(|| rest_reads::content(&self.session, ask)))
    }

    fn derived(&self, ids: &[Value], names: &[String]) -> Result<Vec<kition_kit::DerivedValues>, String> {
        let mut values = rest_animation::derived(&self.session, ids, names);
        for derived in rest_comp::derived(&self.session, ids, names)
            .into_iter()
            .chain(rest_effect::derived(&self.session, ids, names))
            .chain(rest_reads::derived(&self.session, ids, names))
            .chain(rest_styles::derived(&self.session, ids, names)?)
            .chain(raw_keys::derived(&self.session, ids, names)?)
            .chain(raw_sequences::derived(&self.session, ids, names)?)
        {
            if let Some(existing) = values.iter_mut().find(|value| value.id == derived.id) {
                existing.values.extend(derived.values);
            } else {
                values.push(derived);
            }
        }
        Ok(values)
    }

    /// `document.text` (`KitText 1.0`): every composition's text layers as plain text ([`text`]).
    fn text(&self, limit: usize) -> Result<kition_kit::text::DocumentText, String> {
        Ok(text::document_text(&self.session.project, limit))
    }

    fn save(&self, _name: Option<&str>) -> Result<Vec<u8>, String> {
        Ok(self.session.project.to_content_json().into_bytes())
    }

    fn mark_saved(&mut self) {
        self.session.mark_saved();
        if self.session.path.is_none() {
            self.session.path = Some(self.name.clone());
        }
    }

    /// The active composition as a movie, for the host's `document.export` (KHP A41): the
    /// manifest's `exports` (`mp4`, H.264), rendered from a copy of the project as
    /// [`MotionDocument::render_movie`] does, so the document and its Render Queue are untouched.
    fn exports() -> Vec<&'static str> {
        vec!["mp4"]
    }

    fn export(&self, format: &str, _options: &Value) -> Result<Vec<u8>, String> {
        let codec = match format {
            "mp4" => "h264",
            other => return Err(format!("Motion exports mp4, not {other}")),
        };
        self.render_movie(&format!("export.{format}"), codec)
    }
}

/// Why a command did not run, with the reason typed (`kition_kit::failure`) so a host answers its
/// error code without reading the wording: an unknown command, one that cannot run now (or with no
/// comp open), and wrong parameters (a comp the project lacks among them). The engine's other
/// errors carry no reason of their own and are passed on as they read.
pub(crate) fn command_failure(error: kition_motion_engine::EngineError) -> String {
    use kition_kit::failure::{CommandFailure, failure};
    use kition_motion_engine::EngineError;
    match error {
        EngineError::UnknownCommand(_) => failure(CommandFailure::NotFound, error),
        EngineError::Disabled(..) | EngineError::NoComp => failure(CommandFailure::Disabled, error),
        EngineError::BadParams { .. } | EngineError::NoSuchComp(_) => failure(CommandFailure::InvalidParams, error),
        EngineError::Project(_) | EngineError::Other(_) => error.to_string(),
    }
}

/// A call refused for its parameters or its context, for a reason the engine's schema cannot
/// state.
pub(crate) fn params_failure(message: impl std::fmt::Display) -> String {
    kition_kit::failure::failure(kition_kit::failure::CommandFailure::InvalidParams, message)
}

/// A call's part as the comp it names: `{"comp": id}`, the comp's item id, as a number or as the
/// string receipts print (`"1"`).
fn call_comp(part: &Value) -> Result<u64, String> {
    let fields = part.as_object().filter(|fields| fields.len() == 1);
    match fields.and_then(|fields| fields.get("comp")) {
        Some(id) => id.as_u64().or_else(|| id.as_str().and_then(|text| text.parse::<u64>().ok())).ok_or_else(|| {
            format!("`part.comp` is {id}: give {{\"comp\": id}} with the comp's item id as motion_inspect lists it (details.comp.id), a whole number")
        }),
        None => {
            Err(format!("`part` is {part}: this kit's parts are comps, {{\"comp\": id}} with the comp's item id as motion_inspect lists it (details.comp.id)"))
        }
    }
}

/// Types of parameters the notation leaves untyped, read from how the engine takes each one
/// (`str_p` a string, `b_p` a boolean, `f_p` a number, `comp_id` an item id or name, …), so agents
/// see real types (KHP A5): `param_types.json`, per command id and key.
fn param_types() -> &'static serde_json::Map<String, serde_json::Value> {
    static TYPES: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>> = std::sync::OnceLock::new();
    TYPES.get_or_init(|| serde_json::from_str(include_str!("param_types.json")).unwrap_or_default())
}

/// Types a command's parameter keys from a kit's table of known types, per command and key. The
/// table wins over a type the SDK guessed from a key's name (`size` is a number there; a shape's
/// `size` is `[w, h]`, khp-bench r3), never over one the notation spells out (an enum or a choice
/// of types).
trait TypedFrom {
    fn typed_from(self, types: &serde_json::Map<String, serde_json::Value>) -> Self;
}

impl TypedFrom for KitCommand {
    fn typed_from(mut self, types: &serde_json::Map<String, serde_json::Value>) -> Self {
        let (Some(known), Some(schema)) =
            (types.get(&self.id).and_then(serde_json::Value::as_object), self.schema.as_mut().and_then(serde_json::Value::as_object_mut))
        else {
            return self;
        };
        for (key, node) in schema.iter_mut() {
            let spelled_out = ["oneOf", "anyOf", "enum", "const"].iter().any(|word| node.get(*word).is_some());
            if let (false, Some(typed)) = (spelled_out, known.get(key)) {
                let required = node.get("required").cloned();
                let mut typed = typed.clone();
                if let (Some(required), Some(object)) = (required, typed.as_object_mut()) {
                    object.insert("required".into(), required);
                }
                *node = typed;
            }
        }
        self
    }
}

/// What a reading command answers (KHP A1 `output.schema`), from `outputs.json` (measured by
/// running each one; `tests/commands.rs` checks the answers still match).
fn output_schema(id: &str, mutates: bool) -> Option<serde_json::Value> {
    static OUTPUTS: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>> = std::sync::OnceLock::new();
    if mutates {
        return None;
    }
    OUTPUTS.get_or_init(|| serde_json::from_str(include_str!("outputs.json")).unwrap_or_default()).get(id).cloned()
}

kition_kit::export_kit_document!(MotionDocument as "MotionDocument");
kition_kit::export_kit_tools!(MotionDocument as "MotionDocument");

/// Adds a font file's faces for text the bundled faces do not cover, such as Chinese in a web
/// build, which has no system fonts (standard E9): text layers fall back to them. Returns the
/// faces added.
pub fn add_font(bytes: &[u8]) -> usize {
    kition_motion_text::fonts::add_font_data(bytes.to_vec()).len()
}

/// Motion's own additions to the exported class: files the engine wrote, and the collaboration
/// members (engine contract 2, see [`collab`]).
#[cfg(target_arch = "wasm32")]
pub mod web {
    use kition_kit::wasm_bindgen::prelude::*;
    use serde_json::json;

    use crate::__kition_kit_export::WebDocument;

    /// Adds a font file's faces for text the bundled faces do not cover; returns how many.
    #[wasm_bindgen(js_name = addFont, wasm_bindgen = kition_kit::wasm_bindgen)]
    pub fn add_font(bytes: &[u8]) -> u32 {
        crate::add_font(bytes) as u32
    }

    #[wasm_bindgen(js_class = "MotionDocument", wasm_bindgen = kition_kit::wasm_bindgen)]
    impl WebDocument {
        /// A file the engine wrote (a render), taken out of memory; `undefined` when there is none.
        #[wasm_bindgen(js_name = takeFile)]
        pub fn take_file(&self, path: &str) -> Option<Vec<u8>> {
            self.0.take_file(path)
        }

        /// One property's values over a stretch of time, as JSON (see [`MotionDocument::curve_samples`]).
        #[wasm_bindgen(js_name = curveSamples)]
        pub fn curve_samples(&self, layer: f64, path: &str, from: f64, to: f64, count: u32) -> Result<String, JsError> {
            if !(layer >= 0.0 && layer.fract() == 0.0 && layer < 9.0e15) {
                return Err(JsError::new("the layer must be a layer id"));
            }
            self.0.curve_samples(layer as u64, path, from, to, count).map_err(|message| JsError::new(&message))
        }

        /// Whether a command would change the project now (see [`MotionDocument::would_change`]).
        #[wasm_bindgen(js_name = wouldChange)]
        pub fn would_change(&self, command: &str, params: &str) -> bool {
            self.0.would_change(command, params)
        }

        /// The active composition's sound over a stretch of time (see [`MotionDocument::mix_audio`]).
        #[wasm_bindgen(js_name = mixAudio)]
        pub fn mix_audio(&self, from: f64, seconds: f64, rate: u32) -> Result<Vec<f32>, JsError> {
            self.0.mix_audio(from, seconds, rate).map_err(|message| JsError::new(&message))
        }

        /// Starts exporting the active composition as a movie named `path` (see
        /// [`MotionDocument::export_start`]); answers the progress as JSON `{done, total, finished}`.
        #[wasm_bindgen(js_name = exportStart)]
        pub fn export_start(&mut self, path: &str, settings: &str) -> Result<String, JsError> {
            self.0.export_start(path, settings).map(|progress| progress.to_json()).map_err(|message| JsError::new(&message))
        }

        /// Renders the export for about `milliseconds` (see [`MotionDocument::export_step`]).
        #[wasm_bindgen(js_name = exportStep)]
        pub fn export_step(&mut self, milliseconds: f64) -> Result<String, JsError> {
            self.0.export_step(milliseconds).map(|progress| progress.to_json()).map_err(|message| JsError::new(&message))
        }

        /// The finished export's files' names, as JSON, for `takeFile` (see [`MotionDocument::export_finish_files`]).
        #[wasm_bindgen(js_name = exportFinishFiles)]
        pub fn export_finish_files(&mut self) -> Result<String, JsError> {
            self.0.export_finish_files().map(|names| json!(names).to_string()).map_err(|message| JsError::new(&message))
        }

        /// The finished export's bytes (see [`MotionDocument::export_finish`]).
        #[wasm_bindgen(js_name = exportFinish)]
        pub fn export_finish(&mut self) -> Result<Vec<u8>, JsError> {
            self.0.export_finish().map_err(|message| JsError::new(&message))
        }

        /// View ▸ Snap to Guides and Snap to Grid for the Selection tool; answers the grid's
        /// spacing (see [`MotionDocument::set_snap_targets`]).
        #[wasm_bindgen(js_name = setSnapTargets)]
        pub fn set_snap_targets(&mut self, guides: bool, grid: bool) -> f64 {
            self.0.set_snap_targets(guides, grid)
        }

        /// View ▸ Grid Settings (see [`MotionDocument::set_grid_spacing`]).
        #[wasm_bindgen(js_name = setGridSpacing)]
        pub fn set_grid_spacing(&mut self, spacing: f64) -> f64 {
            self.0.set_grid_spacing(spacing)
        }

        /// View ▸ Resolution (see [`MotionDocument::set_preview_resolution`]).
        #[wasm_bindgen(js_name = setPreviewResolution)]
        pub fn set_preview_resolution(&mut self, divisor: u32) -> Result<(), JsError> {
            self.0.set_preview_resolution(divisor).map_err(|message| JsError::new(&message))
        }

        /// The Pen's option on a shape layer (see [`MotionDocument::set_pen_masks`]).
        #[wasm_bindgen(js_name = setPenMasks)]
        pub fn set_pen_masks(&mut self, masks: bool) {
            self.0.set_pen_masks(masks)
        }

        /// The frame cache for the viewer's settings now, as JSON (see [`MotionDocument::preview_cache`]).
        #[wasm_bindgen(js_name = previewCache)]
        pub fn preview_cache(&self) -> String {
            self.0.preview_cache().to_string()
        }

        /// Cache Work Area in Background: renders frames ahead for about `milliseconds`; answers
        /// JSON `{done, total, full}` (see [`MotionDocument::cache_ahead`]).
        #[wasm_bindgen(js_name = cacheAhead)]
        pub fn cache_ahead(&self, milliseconds: f64) -> String {
            self.0.cache_ahead(milliseconds).to_string()
        }

        /// The frame cache's memory budget in megabytes; answers the budget now (see
        /// [`MotionDocument::set_preview_cache_budget`]).
        #[wasm_bindgen(js_name = setPreviewCacheBudget)]
        pub fn set_preview_cache_budget(&self, megabytes: f64) -> f64 {
            self.0.set_preview_cache_budget(megabytes)
        }

        /// Drops every frame the cache keeps (see [`MotionDocument::purge_preview_cache`]).
        #[wasm_bindgen(js_name = purgePreviewCache)]
        pub fn purge_preview_cache(&mut self) {
            self.0.purge_preview_cache()
        }

        /// The Render Queue panel's Render: a copy of the project for its jobs; answers the ids
        /// to render as JSON (see [`MotionDocument::render_snapshot`]).
        #[wasm_bindgen(js_name = renderSnapshot)]
        pub fn render_snapshot(&mut self) -> String {
            self.0.render_snapshot().to_string()
        }

        /// Drops the render queue's copy (see [`MotionDocument::drop_render_snapshot`]).
        #[wasm_bindgen(js_name = dropRenderSnapshot)]
        pub fn drop_render_snapshot(&mut self) -> bool {
            self.0.drop_render_snapshot()
        }

        /// What the canvas tools name new layers, as JSON `{shape}` (see [`MotionDocument::set_layer_names`]).
        #[wasm_bindgen(js_name = setLayerNames)]
        pub fn set_layer_names(&mut self, names: &str) {
            self.0.set_layer_names(names)
        }

        /// The colour the viewer shows at a composition pixel, `[r, g, b, a]` (0 to 255); empty off
        /// the picture (see [`MotionDocument::pixel_at`]).
        #[wasm_bindgen(js_name = pixelAt)]
        pub fn pixel_at(&self, x: f64, y: f64) -> Vec<u8> {
            self.0.pixel_at(x, y).map(|pixel| pixel.to_vec()).unwrap_or_default()
        }

        /// A layer's sound as peaks over its span, as JSON (see [`MotionDocument::layer_waveform`]).
        #[wasm_bindgen(js_name = layerWaveform)]
        pub fn layer_waveform(&self, layer: f64, count: u32) -> Result<String, JsError> {
            if !(layer >= 0.0 && layer.fract() == 0.0 && layer < 9.0e15) {
                return Err(JsError::new("the layer must be a layer id"));
            }
            self.0.layer_waveform(layer as u64, count).map(|wave| wave.to_string()).map_err(|message| JsError::new(&message))
        }

        /// Where the Selection tool's 3D handles are on the canvas, as JSON (see [`MotionDocument::gizmo_handles`]).
        #[wasm_bindgen(js_name = gizmoHandles)]
        pub fn gizmo_handles(&self, zoom: f64) -> String {
            self.0.gizmo_handles(zoom).to_string()
        }

        /// The two-view layout's second view: the length of a JSON header `{width, height, lines,
        /// layers, origin, scale}` (four bytes, little-endian), the header, then the RGBA picture;
        /// `aspect` is the pane's width over its height (see [`MotionDocument::view_frame`]).
        #[wasm_bindgen(js_name = viewFrame)]
        pub fn view_frame(&self, view: &str, width: u32, aspect: Option<f64>) -> Result<Vec<u8>, JsError> {
            let frame = self.0.view_frame(view, width, aspect).map_err(|message| JsError::new(&message))?;
            // The header (JSON, its length first as four bytes) and the picture in one answer.
            let head = json!({ "width": frame.width, "height": frame.height, "lines": frame.lines, "layers": frame.layers, "origin": frame.origin, "scale": frame.scale }).to_string().into_bytes();
            let mut out = Vec::with_capacity(4 + head.len() + frame.rgba.len());
            out.extend_from_slice(&u32::try_from(head.len()).unwrap_or(u32::MAX).to_le_bytes());
            out.extend_from_slice(&head);
            out.extend_from_slice(&frame.rgba);
            Ok(out)
        }

        /// Collect Files into `folder`: the names written, as JSON (see [`MotionDocument::collect_files`]).
        #[wasm_bindgen(js_name = collectFiles)]
        pub fn collect_files(&self, folder: &str) -> Result<String, JsError> {
            self.0.collect_files(folder).map(|answer| answer.to_string()).map_err(|message| JsError::new(&message))
        }

        /// Stops the export and keeps no file (see [`MotionDocument::export_cancel`]).
        #[wasm_bindgen(js_name = exportCancel)]
        pub fn export_cancel(&mut self) -> bool {
            self.0.export_cancel()
        }

        // The collaboration members (engine contract 2). Shapes mirror `kition-sdk`'s
        // `session::web` (`export_kit_session!` does not apply here; see `crate::collab`).

        pub fn version(&mut self) -> Result<Vec<u8>, JsError> {
            self.0.version().map_err(|message| JsError::new(&message))
        }

        #[wasm_bindgen(js_name = exportChanges)]
        pub fn export_changes(&mut self, since: &[u8]) -> Result<Vec<u8>, JsError> {
            self.0.export_changes(since).map_err(|message| JsError::new(&message))
        }

        #[wasm_bindgen(js_name = importChanges)]
        pub fn import_changes(&mut self, bytes: &[u8]) -> Result<String, JsError> {
            self.0
                .import_changes(bytes)
                .map(|damage| serde_json::to_string(&damage).unwrap_or_else(|_| "null".to_owned()))
                .map_err(|message| JsError::new(&message))
        }

        pub fn snapshot(&mut self) -> Result<Vec<u8>, JsError> {
            self.0.snapshot().map_err(|message| JsError::new(&message))
        }

        pub fn undo(&mut self) -> Result<String, JsError> {
            self.0.undo().map(|(done, damage)| json!({ "done": done, "damage": damage }).to_string()).map_err(|message| JsError::new(&message))
        }

        pub fn redo(&mut self) -> Result<String, JsError> {
            self.0.redo().map(|(done, damage)| json!({ "done": done, "damage": damage }).to_string()).map_err(|message| JsError::new(&message))
        }

        #[wasm_bindgen(js_name = canUndo)]
        pub fn can_undo(&self) -> bool {
            self.0.can_undo()
        }

        #[wasm_bindgen(js_name = canRedo)]
        pub fn can_redo(&self) -> bool {
            self.0.can_redo()
        }

        /// `origin` is the KDM's commit form, the JSON array `[kind, principal, label]`.
        #[wasm_bindgen(js_name = executeAs)]
        pub fn execute_as(&mut self, command: &str, params: &str, origin: &str) -> Result<String, JsError> {
            let params = kition_kit::parse_params(params).map_err(|message| JsError::new(&message))?;
            let origin =
                kition_motion_engine::Origin::decode(origin).ok_or_else(|| JsError::new(&format!("origin is not [kind, principal, label]: {origin}")))?;
            self.0.execute_as(command, params, &origin).map(|value| value.to_string()).map_err(|message| JsError::new(&message))
        }

        /// Every blob the replica references: always empty, Motion never embeds one.
        pub fn blobs(&self) -> Result<Vec<u8>, JsError> {
            self.0.blobs().map_err(|message| JsError::new(&message))
        }

        /// Adds a peer's blobs: a no-op, Motion never embeds one.
        #[wasm_bindgen(js_name = addBlobs)]
        pub fn add_blobs(&mut self, bytes: &[u8]) -> Result<(), JsError> {
            self.0.add_blobs(bytes).map_err(|message| JsError::new(&message))
        }

        #[wasm_bindgen(js_name = kdmResets)]
        pub fn kdm_resets(&self) -> u32 {
            self.0.kdm_resets()
        }

        /// Opens a file with the replica a host stored for it (the Kit UI protocol's `replica`
        /// capability); a replica that does not open is ignored and the file opens plainly.
        #[wasm_bindgen(js_name = openWithReplica)]
        pub fn open_with_replica(name: &str, bytes: &[u8], snapshot: &[u8], blobs: &[u8]) -> Result<WebDocument, JsError> {
            crate::MotionDocument::open_with_replica(name, bytes, snapshot, blobs).map(Self).map_err(|message| JsError::new(&message))
        }

        /// Whether the project has made its replica yet (it makes one when first asked for it).
        #[wasm_bindgen(getter, js_name = hasReplica)]
        pub fn has_replica(&self) -> bool {
            self.0.has_replica()
        }

        /// The replica's own document id, for the Kit UI protocol's `replica/info` message.
        /// `undefined` before the project has made a replica.
        #[wasm_bindgen(getter, js_name = docId)]
        pub fn doc_id(&self) -> Option<String> {
            self.0.doc_id()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kition_kit::testing::check_contract;

    /// A 64 × 36 composition with a red solid filling it: no footage needed.
    fn fixture() -> Vec<u8> {
        let mut doc = MotionDocument::create("Motion.kmotion", 64, 36).unwrap();
        doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
        doc.save(None).unwrap()
    }

    #[test]
    fn mutates_means_the_command_changes_the_document() {
        let doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let commands = doc.commands();
        let mutates = |id: &str| commands.iter().find(|command| command.id == id).unwrap_or_else(|| panic!("no {id}")).mutates;
        for id in
            ["edit.undo", "edit.redo", "layer.newSolid", "view.addGuide", "view.clearGuides", "view.lookAtAll", "file.open", "file.revert", "comp.settings"]
        {
            assert!(mutates(id), "{id} changes the document");
        }
        for id in [
            "edit.selectAll",
            "layer.select",
            "view.zoomIn",
            "view.threeD.front",
            "view.rulers",
            "time.nextFrame",
            "playback.toggle",
            "file.saveCopy",
            "comp.info",
        ] {
            assert!(!mutates(id), "{id} does not change the document");
        }
    }

    /// Every command reported as not changing the document leaves the replica as it was, on a
    /// project with a selected layer (commands that need other parameters fail and are skipped).
    #[test]
    fn commands_that_do_not_mutate_never_commit_to_the_replica() {
        let bytes = fixture();
        let ids: Vec<String> = MotionDocument::open("Motion.kmotion", &bytes).unwrap().commands().into_iter().filter(|c| !c.mutates).map(|c| c.id).collect();
        let mut ran = 0;
        for id in ids {
            let mut doc = MotionDocument::open("Motion.kmotion", &bytes).unwrap();
            doc.execute("edit.selectAll", json!({})).unwrap();
            let before = doc.version().unwrap();
            if doc.execute(&id, json!({})).is_ok() {
                ran += 1;
                assert_eq!(doc.version().unwrap(), before, "{id} committed to the replica");
            }
        }
        assert!(ran > 50, "only {ran} commands ran");
    }

    #[test]
    fn keeps_the_kit_contract() {
        check_contract::<MotionDocument>(&fixture(), "Motion.kmotion", |doc| {
            doc.execute("layer.newSolid", json!({ "name": "Blue", "color": "#0000ff", "width": 32, "height": 36 })).map(|_| ())
        })
        .unwrap();
    }

    #[test]
    fn shows_the_frame_at_the_current_time() {
        let doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        assert_eq!((doc.width(), doc.height()), (64, 36));
        let red = doc.render(0.0, 0.0, 1.0, 64, 36).unwrap();
        assert_eq!(&red[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn reports_layers_with_their_key_times() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let layer = doc.inspect().unwrap()["info"]["layers"][0]["id"].clone();
        doc.execute("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 0.0, "value": 0.0 })).unwrap();
        doc.execute("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 1.0, "value": 100.0 })).unwrap();
        doc.execute("layer.select", json!({ "layers": [layer] })).unwrap();
        let row = doc.inspect().unwrap()["info"]["layers"][0].clone();
        assert_eq!(row["selected"], true);
        assert_eq!(row["animated"][0]["times"], json!([0.0, 1.0]), "{row}");
        doc.execute("time.set", json!({ "time": 0.5 })).unwrap();
        let transform = doc.inspect().unwrap()["info"]["transform"].clone();
        let opacity = transform["properties"].as_array().unwrap().iter().find(|prop| prop["path"] == "transform/opacity").unwrap().clone();
        assert_eq!(opacity["animated"], true);
        assert!((opacity["value"].as_f64().unwrap() - 50.0).abs() < 1.0, "halfway: {opacity}");
        doc.execute("time.set", json!({ "time": 0.0 })).unwrap();
        let faded = doc.render(0.0, 0.0, 1.0, 64, 36).unwrap();
        assert_eq!(&faded[..4], &[0, 0, 0, 255], "fully transparent at 0 s, over the black background");
        doc.execute("time.set", json!({ "time": 1.0 })).unwrap();
        assert_eq!(&doc.render(0.0, 0.0, 1.0, 64, 36).unwrap()[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn renders_the_composition_to_mp4_in_memory() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute("comp.settings", json!({ "duration": 0.5 })).unwrap();
        doc.execute("renderQueue.add", json!({ "format": "h264", "output": "Motion.mp4" })).unwrap();
        let rendered = doc.execute("renderQueue.render", json!({ "wait": true })).unwrap();
        let bytes = doc.take_file("Motion.mp4").unwrap_or_else(|| panic!("no Motion.mp4 in memory: {rendered}"));
        assert_eq!(&bytes[4..8], b"ftyp", "an MP4");
        assert!(doc.take_file("Motion.kmotion").is_none());
    }

    #[test]
    fn renders_a_movie_without_touching_the_document() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute("comp.settings", json!({ "duration": 0.5 })).unwrap();
        doc.mark_saved();
        let revision = doc.revision();
        let bytes = doc.render_movie("clips/Motion.mp4", "h264").unwrap();
        assert_eq!(&bytes[4..8], b"ftyp");
        assert_eq!(doc.revision(), revision);
        assert!(!doc.dirty());
        assert!(doc.session.project.render_queue.is_empty());
    }

    #[test]
    fn the_blend_mode_schema_lists_every_mode_the_engine_takes() {
        let modes: Vec<String> =
            param_types()["layer.setBlendMode"]["mode"]["enum"].as_array().unwrap().iter().map(|mode| mode.as_str().unwrap().to_owned()).collect();
        assert_eq!(modes.len(), kition_motion_engine::color::BlendMode::ALL.len());
        let labels: Vec<&str> = kition_motion_engine::color::BlendMode::ALL.iter().map(|mode| mode.label()).collect();
        assert_eq!(modes, labels, "the schema's order is the engine's");
        let doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let command = doc.commands().into_iter().find(|command| command.id == "layer.setBlendMode").unwrap();
        assert_eq!(command.schema.unwrap()["mode"]["enum"].as_array().map(Vec::len), Some(labels.len()), "the editor reads the modes from the command");
    }

    #[test]
    fn a_lower_preview_resolution_draws_blocks_at_the_compositions_size() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let full = doc.frame().unwrap();
        doc.set_preview_resolution(4).unwrap();
        let quarter = doc.frame().unwrap();
        assert_eq!((quarter.1, quarter.2), (full.1, full.2), "the picture keeps the composition's size");
        let px = |frame: &(Arc<Vec<u8>>, u32, u32), x: u32, y: u32| frame.0[((y * frame.1 + x) * 4) as usize..][..4].to_vec();
        for (x, y) in [(0, 0), (5, 9), (12, 3)] {
            assert_eq!(px(&quarter, x - x % 4, y - y % 4), px(&quarter, x, y), "4 × 4 blocks at ({x}, {y})");
        }
        assert!(doc.set_preview_resolution(0).is_err() && doc.set_preview_resolution(5).is_err());
        doc.set_preview_resolution(1).unwrap();
        assert_eq!(doc.frame().unwrap().0, full.0, "Full draws as before");
    }

    #[test]
    fn a_cached_frame_shows_the_same_pixels_as_one_just_rendered_at_every_resolution() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute("time.set", json!({ "time": 0.5 })).unwrap();
        let mut held = Vec::new();
        for preview in 1..=4 {
            doc.set_preview_resolution(preview).unwrap();
            doc.purge_preview_cache();
            let rendered = doc.frame().unwrap();
            let hits = doc.frames.borrow().hits;
            // Not the frame remembered as shown: unpacked from the cache and blown up again.
            *doc.shown.borrow_mut() = None;
            doc.frames.borrow_mut().clear_last_for_tests();
            let cached = doc.frame().unwrap();
            assert_eq!(doc.frames.borrow().hits, hits + 1, "at 1/{preview} the second frame came from the cache");
            assert_eq!((cached.1, cached.2), (rendered.1, rendered.2));
            assert!(cached.0 == rendered.0, "at 1/{preview} the cached frame has the same pixels");
            held.push(doc.preview_cache()["rawBytes"].as_u64().unwrap());
        }
        // The cache keeps a frame at its preview resolution: Half a quarter of Full, and so on.
        let (w, h) = (doc.width() as u64, doc.height() as u64);
        assert_eq!(held[0], w * h * 4);
        for (preview, bytes) in held.iter().enumerate().skip(1) {
            let d = preview as u64 + 1;
            assert!(*bytes <= w.div_ceil(d) * h.div_ceil(d) * 4, "1/{d}: {bytes} bytes held");
        }
    }

    #[test]
    fn a_default_composition_is_cached_from_end_to_end_within_the_budget() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute("comp.new", json!({ "name": "Comp 1", "width": 1920, "height": 1080, "frameRate": 30, "duration": 10, "open": true })).unwrap();
        doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
        let mut answer = json!({});
        for _ in 0..200 {
            answer = doc.cache_ahead(2000.0);
            if answer["done"] == answer["total"] || answer["full"] == json!(true) {
                break;
            }
        }
        assert_eq!((answer["done"].as_u64(), answer["total"].as_u64(), answer["full"].as_bool()), (Some(300), Some(300), Some(false)), "{answer}");
        let report = doc.preview_cache();
        let (bytes, raw) = (report["bytes"].as_u64().unwrap(), report["rawBytes"].as_u64().unwrap());
        assert_eq!(raw, 300 * 1920 * 1080 * 4, "300 full frames");
        assert!(bytes <= frames::BUDGET as u64 && bytes * 10 < raw, "{bytes} bytes held for {raw} bytes of pixels");
    }

    #[test]
    fn the_mask_mode_schema_lists_every_mode_the_engine_takes() {
        let modes: Vec<String> =
            param_types()["layer.mask.mode"]["mode"]["enum"].as_array().unwrap().iter().map(|mode| mode.as_str().unwrap().to_owned()).collect();
        let labels: Vec<&str> = kition_motion_engine::project::MaskMode::ALL.iter().map(|mode| mode.label()).collect();
        assert_eq!(modes, labels, "the schema's order is the engine's");
    }

    #[test]
    fn an_export_renders_a_batch_at_a_time_and_reports_its_frames() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute("comp.settings", json!({ "duration": 1.0 })).unwrap();
        let revision = doc.revision();
        let start = doc.export_start("Motion.webm", r#"{"format":"webm","size":"half"}"#).unwrap();
        assert!(!start.finished);
        let mut seen = vec![];
        loop {
            let progress = doc.export_step(0.0).unwrap();
            seen.push(progress.done);
            if progress.finished {
                assert_eq!(progress.done, progress.total);
                break;
            }
            assert!(progress.done <= progress.total, "{progress:?}");
            assert!(seen.len() < 1000, "the export never ends");
        }
        assert!(seen.len() > 2, "rendered in several steps: {seen:?}");
        assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "progress only goes up: {seen:?}");
        let bytes = doc.export_finish().unwrap();
        assert_eq!(&bytes[..4], &[0x1a, 0x45, 0xdf, 0xa3], "a WebM (EBML) file");
        assert_eq!(doc.revision(), revision);
        assert!(doc.export_finish().is_err(), "the export is over");
    }

    #[test]
    fn roving_would_change_a_middle_keyframe_but_not_a_first_or_last_one() {
        let mut doc = MotionDocument::create("Rove.kmotion", 64, 36).unwrap();
        let layer = doc.execute("layer.newSolid", json!({ "name": "Dot" })).unwrap()["layer"].clone();
        for (time, x) in [(0.0, 0.0), (1.0, 30.0), (2.0, 60.0)] {
            doc.execute("prop.addKey", json!({ "layer": layer, "path": "transform/position", "time": time, "value": [x, 10.0, 0.0] })).unwrap();
        }
        let select = |doc: &mut MotionDocument, time: f64| {
            doc.execute("keyframe.select", json!({ "keys": [{ "layer": layer, "path": "transform/position", "time": time }] })).unwrap()
        };
        select(&mut doc, 0.0);
        assert!(!doc.would_change("keyframe.interpolation", r#"{"roving":true}"#), "a first keyframe cannot rove");
        select(&mut doc, 2.0);
        assert!(!doc.would_change("keyframe.interpolation", r#"{"roving":true}"#), "a last keyframe cannot rove");
        select(&mut doc, 1.0);
        assert!(doc.would_change("keyframe.interpolation", r#"{"roving":true}"#), "a middle keyframe can");
    }

    #[test]
    fn collect_files_copies_the_project_and_its_footage_into_a_folder() {
        let mut doc = MotionDocument::create("Shots/Motion.kmotion", 64, 36).unwrap();
        // Footage the engine can read from memory (as the host's lent files are).
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="#f00"/></svg>"##;
        for path in ["art/logo.svg", "more/logo.svg"] {
            doc.services.write_file(path, svg).unwrap();
            let footage = kition_motion_media::probe_bytes(path, Arc::from(svg.to_vec())).unwrap();
            Arc::make_mut(&mut doc.session.project).add_item("logo.svg", Default::default(), None, ItemKind::Footage(footage));
        }
        let answer = doc.collect_files("Collected").unwrap();
        let files: Vec<String> = answer["files"].as_array().unwrap().iter().map(|name| name.as_str().unwrap().to_owned()).collect();
        assert_eq!(answer["missing"], json!([]), "{answer}");
        assert!(files.contains(&"Collected/(Footage)/logo.svg".to_owned()) && files.contains(&"Collected/(Footage)/logo 2.svg".to_owned()), "{files:?}");
        assert!(files.contains(&"Collected/Motion.kmotion".to_owned()));
        let copy = String::from_utf8(doc.take_file("Collected/Motion.kmotion").unwrap()).unwrap();
        assert!(
            copy.contains("(Footage)/logo.svg") && copy.contains("(Footage)/logo 2.svg") && !copy.contains("art/logo.svg"),
            "the copy points at its own footage"
        );
        assert_eq!(doc.take_file("Collected/(Footage)/logo.svg").unwrap(), svg.to_vec());
        // The open project still points where it did.
        assert!(String::from_utf8(doc.save(None).unwrap()).unwrap().contains("art/logo.svg"));
        for bad in ["", "..", "a/../b", "/"] {
            assert!(doc.collect_files(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn png_and_jpeg_sequences_write_a_file_per_frame_and_wav_the_sound() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute("comp.settings", json!({ "duration": 0.2, "frameRate": 10 })).unwrap();
        doc.export_start("Shot_[#####].png", r#"{"format":"png","channels":"rgba","size":{"width":32,"height":18}}"#).unwrap();
        while !doc.export_step(50.0).unwrap().finished {}
        let names = doc.export_finish_files().unwrap();
        assert_eq!(names, vec!["Shot_00000.png".to_owned(), "Shot_00001.png".to_owned()], "{names:?}");
        let first = doc.take_file(&names[0]).unwrap();
        assert_eq!(&first[1..4], b"PNG");
        // An RGBA PNG: colour type 6 in its header.
        assert_eq!(first[25], 6, "the PNG keeps its alpha");
        doc.export_start("Shot_[#####].jpg", r#"{"format":"jpeg","size":{"width":32,"height":18}}"#).unwrap();
        while !doc.export_step(50.0).unwrap().finished {}
        let names = doc.export_finish_files().unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.iter().all(|name| doc.take_file(name).is_some_and(|bytes| bytes.starts_with(&[0xff, 0xd8]))));
        // Stopped half way, a sequence leaves none of its frames.
        doc.execute("comp.settings", json!({ "duration": 2.0 })).unwrap();
        doc.export_start("Cut_[#####].png", r#"{"format":"png"}"#).unwrap();
        doc.export_step(0.0).unwrap();
        assert!(doc.export_cancel());
        assert!(doc.sequence_files("Cut_[#####].png").is_empty());
        doc.export_start("Sound.wav", r#"{"format":"wav"}"#).unwrap();
        while !doc.export_step(50.0).unwrap().finished {}
        let names = doc.export_finish_files().unwrap();
        assert_eq!(names, vec!["Sound.wav".to_owned()]);
        assert!(doc.take_file("Sound.wav").is_some_and(|bytes| bytes.starts_with(b"RIFF")));
    }

    #[test]
    fn a_cancelled_export_leaves_no_file() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute("comp.settings", json!({ "duration": 1.0 })).unwrap();
        doc.export_start("Motion.mp4", "h264").unwrap();
        let progress = doc.export_step(0.0).unwrap();
        assert!(!progress.finished && progress.done > 0, "{progress:?}");
        assert!(doc.export_cancel());
        assert!(doc.take_file("Motion.mp4").is_none(), "no file after cancel");
        assert!(doc.export_step(0.0).is_err() && doc.export_finish().is_err());
        assert!(!doc.export_cancel());
        assert!(doc.session.project.render_queue.is_empty());
        // A finished export still finishes as before after a cancelled one.
        doc.export_start("Motion.gif", r#"{"format":"gif","size":{"width":64,"height":36}}"#).unwrap();
        while !doc.export_step(50.0).unwrap().finished {}
        assert_eq!(&doc.export_finish().unwrap()[..3], b"GIF");
    }

    #[test]
    fn imports_footage_through_lent_files() {
        let dir = std::env::temp_dir().join(format!("kition-motion-lent-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("art")).unwrap();
        std::fs::write(
            dir.join("art/logo.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="36"><rect width="64" height="36" fill="#00ff00"/></svg>"##,
        )
        .unwrap();
        kition_kit::host::set_root(Some(dir.clone()));
        let mut doc = MotionDocument::create("Motion.kmotion", 64, 36).unwrap();
        doc.execute("file.import", json!({ "paths": ["art/logo.svg"], "addToComp": true })).unwrap();
        let green = doc.render(0.0, 0.0, 1.0, 64, 36).unwrap();
        assert_eq!(&green[(18 * 64 + 32) * 4..(18 * 64 + 32) * 4 + 4], &[0, 255, 0, 255]);
        // Reopened from its saved bytes, the project lends the same file again.
        let saved = doc.save(None).unwrap();
        let reopened = MotionDocument::open("Motion.kmotion", &saved).unwrap();
        assert_eq!(&reopened.render(32.0, 18.0, 1.0, 1, 1).unwrap(), &[0, 255, 0, 255]);
        kition_kit::host::set_root(None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn bad_files_are_errors_not_panics() {
        assert!(MotionDocument::open("Broken.kmotion", b"{not a project").is_err());
        assert!(MotionDocument::open("Broken.kmotion", b"").is_err());
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        assert!(doc.execute("no.such.command", json!({})).is_err());
    }

    #[test]
    fn takes_a_font_for_text_its_bundled_faces_do_not_cover() {
        assert_eq!(add_font(b"not a font"), 0);
        let font = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../kition-fonts/fonts/droid-sans-fallback/DroidSansFallbackFull.ttf");
        let Ok(bytes) = std::fs::read(font) else { return };
        assert!(add_font(&bytes) > 0);
    }
}
