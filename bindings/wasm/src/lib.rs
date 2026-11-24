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
mod field_holds;
mod files;
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
    let mut out = vec![0u8; tw.saturating_mul(th).saturating_mul(4)];
    for y in 0..th {
        let sy = (y * fh / th.max(1)).min(fh - 1);
        for x in 0..tw {
            let sx = (x * fw / tw.max(1)).min(fw - 1);
            let from = (sy * fw + sx) * 4;
            let to = (y * tw + x) * 4;
            if let (Some(src), Some(dst)) = (rgba.get(from..from + 4), out.get_mut(to..to + 4)) {
                dst.copy_from_slice(src);
            }
        }
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

/// The transform properties a properties panel shows, in After Effects' order.
const TRANSFORM: [&str; 5] = ["transform/anchor", "transform/position", "transform/scale", "transform/rotation", "transform/opacity"];

/// The last full frame: its project revision, composition time, and pixels.
struct Cached {
    key: (u64, Option<u64>, Tick, u32),
    rgba: Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

/// One open Kition Motion project.
pub struct MotionDocument {
    session: Session,
    name: String,
    services: Arc<KitServices>,
    pool: Arc<MediaPool>,
    /// Footage files already lent to the pool.
    lent: HashSet<String>,
    frame: RefCell<Option<Cached>>,
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
            frame: RefCell::new(None),
            export: movie::ExportKey::new(),
            snap_guides: true,
            snap_grid: false,
            preview: 1,
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

    /// The active composition's frame at the current time, full size, over its background colour
    /// as the viewer shows it.
    fn frame(&self) -> Result<(Arc<Vec<u8>>, u32, u32), String> {
        let comp_id = self.session.active_comp_id().ok_or("no composition is open")?;
        let time = self.session.time();
        let key = (self.session.revision, Some(comp_id.0), time, self.preview);
        if let Some(cached) = self.frame.borrow().as_ref().filter(|cached| cached.key == key) {
            return Ok((cached.rgba.clone(), cached.width, cached.height));
        }
        let comp = self.session.project.comp(comp_id).ok_or("no composition is open")?;
        let scale = 1.0 / f64::from(self.preview.max(1));
        let mut renderer = Renderer::new(&self.session.project, &*self.session.footage, RenderOpts { scale, ..Default::default() });
        renderer.expr = self.session.expr.as_deref();
        renderer.cache = Some(&self.session.layer_cache);
        let image = renderer.comp_frame(comp_id, time);
        let rendered = image.to_rgba8_over(comp.background);
        let (rgba, width, height) = if self.preview > 1 {
            let (width, height) = (comp.width.max(1), comp.height.max(1));
            (Arc::new(blow_up(&rendered, image.width, image.height, width, height)), width, height)
        } else {
            (Arc::new(rendered), image.width, image.height)
        };
        *self.frame.borrow_mut() = Some(Cached { key, rgba: rgba.clone(), width, height });
        Ok((rgba, width, height))
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

    /// View ▸ Resolution: renders every `divisor`-th pixel (1 Full, 2 Half, 3 Third, 4 Quarter);
    /// anything else is refused. The picture keeps the composition's size.
    pub fn set_preview_resolution(&mut self, divisor: u32) -> Result<(), String> {
        if !(1..=4).contains(&divisor) {
            return Err(format!("the preview resolution is 1 (Full), 2 (Half), 3 (Third) or 4 (Quarter), not {divisor}"));
        }
        self.preview = divisor;
        Ok(())
    }

    /// Stops the export and drops what it wrote; whether one was running.
    pub fn export_cancel(&mut self) -> bool {
        let Some(mut export) = self.export.take() else { return false };
        export.cancel();
        let _ = self.take_file(export.path());
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
    "keys.timeReverse",
    "prop.setExpression",
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
/// "delete layers" for `edit.clear`, which deletes them (khp-bench r2: an agent asked for
/// `layer.delete`, which does not exist, and was offered `layer.select`).
fn keywords(id: &str) -> Vec<String> {
    match id {
        "effect.apply" => effect_names().to_vec(),
        "edit.clear" => ["delete layers", "remove layers"].map(str::to_owned).to_vec(),
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
    "keys.select",
    "keys.selectAll",
    "project.select",
    "mask.selectVertices",
    "camera.selectPoints",
    "comp.close",
    "comp.revealInProject",
    "comp.flowchart",
    "comp.miniFlowchart",
    "layer.revealExpressionErrors",
    "anim.reveal",
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
    if matches!(id, "engine.batch" | "jobs.list" | "jobs.wait" | "renderQueue.list" | "camera.solveStatus" | "roto.status" | "track.status" | "warp.status") {
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
                "prop.set" | "prop.addKey" | "keys.set" | "keys.setEase" => &["time"],
                id if id.starts_with("renderQueue.") => &["index"],
                "keys.select" | "keys.info" => &["keys"],
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

        /// View ▸ Resolution (see [`MotionDocument::set_preview_resolution`]).
        #[wasm_bindgen(js_name = setPreviewResolution)]
        pub fn set_preview_resolution(&mut self, divisor: u32) -> Result<(), JsError> {
            self.0.set_preview_resolution(divisor).map_err(|message| JsError::new(&message))
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
        for id in
            ["edit.selectAll", "layer.select", "view.zoomIn", "view.3d.front", "view.rulers", "time.nextFrame", "playback.toggle", "file.saveCopy", "comp.info"]
        {
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
