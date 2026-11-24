//! What the editor's Export dialog asks `renderMovie` for: the format, the stretch of the
//! composition (all of it or its work area), the size (full, half, or a width and height) and the
//! frame rate (the composition's, or another). The dialog sends them as a JSON object in place of
//! the format name; a bare format name (`h264`) renders the whole composition as it is.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use kition_motion_engine::render_queue::RenderStepper;
use kition_motion_project::render_queue::RenderStatus;
use serde_json::{Map, Value, json};

/// How far an export is: frames done of the frames it renders, and whether it has ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
    pub finished: bool,
}

impl Progress {
    pub fn to_json(self) -> String {
        json!({ "done": self.done, "total": self.total, "finished": self.finished }).to_string()
    }
}

/// A movie being rendered for the Export dialog, a batch of frames at a time.
pub(crate) struct Export {
    stepper: RenderStepper,
    path: String,
    finished: bool,
}

impl Export {
    pub(crate) fn new(stepper: RenderStepper, path: &str) -> Export {
        Export { stepper, path: path.to_owned(), finished: false }
    }

    pub(crate) fn progress(&self, finished: bool) -> Progress {
        let state = self.stepper.progress();
        Progress { done: state.done, total: state.total, finished }
    }

    /// Renders for about `milliseconds` (a negative, NaN or zero time still renders one batch).
    pub(crate) fn step(&mut self, milliseconds: f64) -> Progress {
        let budget = if milliseconds == f64::INFINITY {
            Duration::MAX
        } else if milliseconds.is_finite() && milliseconds > 0.0 {
            Duration::try_from_secs_f64(milliseconds / 1000.0).unwrap_or(Duration::MAX)
        } else {
            Duration::ZERO
        };
        self.finished = self.finished || self.stepper.step(budget);
        self.progress(self.finished)
    }

    /// The written file's path once the render has ended well, or why it did not.
    pub(crate) fn finished(&self) -> Result<String, String> {
        if !self.finished {
            return Err("the export is still rendering".to_owned());
        }
        match self.stepper.outcomes().into_iter().next() {
            Some((_, RenderStatus::Done, Some(path))) => Ok(path),
            Some((_, RenderStatus::Done, None)) => Ok(self.path.clone()),
            Some((_, RenderStatus::Failed(why), _)) => Err(format!("the render failed: {why}")),
            Some((_, RenderStatus::UserStopped, _)) => Err("the export was cancelled".to_owned()),
            Some((_, status, _)) => Err(format!("the render failed: {}", status.label())),
            None => Err("the render produced no file".to_owned()),
        }
    }

    /// Stops the render: the export removes the file it opened.
    pub(crate) fn cancel(&mut self) {
        self.stepper.cancel();
        while !self.finished {
            self.finished = self.stepper.step(Duration::MAX);
        }
    }

    /// Where the movie was (or would have been) written.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

thread_local! {
    /// The exports being rendered, by [`ExportKey`]. A stepped render is a future that stays on
    /// the thread it started on (a browser worker's WebGPU device is awaited inside it), so it is
    /// not `Send` and is kept here, per thread, rather than in the document, which a native host
    /// may move between threads (`MotionDocument` stays `Send`). Asked from another thread, an
    /// export is simply not found.
    static EXPORTS: RefCell<HashMap<u64, Export>> = RefCell::new(HashMap::new());
}

/// Names one document's export in this thread's [`EXPORTS`]; dropping the key drops the export.
pub(crate) struct ExportKey(u64);

impl ExportKey {
    pub(crate) fn new() -> ExportKey {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        ExportKey(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// Keeps `export` as this document's export, dropping one it had.
    pub(crate) fn put(&self, export: Export) {
        let _ = EXPORTS.try_with(|exports| exports.borrow_mut().insert(self.0, export));
    }

    /// Takes this document's export out.
    pub(crate) fn take(&self) -> Option<Export> {
        EXPORTS.try_with(|exports| exports.borrow_mut().remove(&self.0)).ok().flatten()
    }

    /// Runs `f` on this document's export, if it has one on this thread.
    pub(crate) fn with<T>(&self, f: impl FnOnce(&mut Export) -> T) -> Option<T> {
        // The export is taken out while `f` runs, so a render calling back here cannot borrow
        // the map twice.
        let mut export = self.take()?;
        let out = f(&mut export);
        self.put(export);
        Some(out)
    }
}

impl Drop for ExportKey {
    fn drop(&mut self) {
        drop(self.take());
    }
}

/// The formats the Export dialog offers: the ones the web build encodes into one file.
const FORMATS: [&str; 3] = ["h264", "webm", "gif"];
/// The size of a custom output, in pixels on each side.
const SIZE: std::ops::RangeInclusive<u64> = 16..=8192;

/// The render queue item an export renders when its settings are `{"queueItem": id}` (the Render
/// Queue panel's jobs), else `None`.
pub(crate) fn queue_item(settings: &str) -> Option<u64> {
    let settings = settings.trim();
    if !settings.starts_with('{') {
        return None;
    }
    serde_json::from_str::<Value>(settings).ok()?.get("queueItem")?.as_u64()
}

/// The `renderQueue.add` parameters that render comp `comp` to `path` with `settings`: a format
/// name, or a JSON object `{format, span: "comp"|"workArea", size: "full"|"half"|{width, height},
/// frameRate: fps|null}`. Anything else is refused with what it takes.
pub(crate) fn queue_params(comp: u64, path: &str, settings: &str) -> Result<Value, String> {
    let settings = settings.trim();
    if !settings.starts_with('{') {
        return Ok(json!({ "comp": comp, "format": if settings.is_empty() { "h264" } else { settings }, "output": path }));
    }
    let given: Map<String, Value> = serde_json::from_str(settings).map_err(|error| format!("the export settings are not a JSON object: {error}"))?;
    let mut params = json!({ "comp": comp, "output": path });
    for (key, value) in &given {
        match (key.as_str(), value) {
            ("format", Value::String(format)) if FORMATS.contains(&format.as_str()) => params["format"] = json!(format),
            ("span", Value::String(span)) if span == "comp" || span == "workArea" => params["timeSpan"] = json!(span),
            ("size", Value::String(size)) if size == "full" || size == "half" => params["resolution"] = json!(size),
            ("size", Value::Object(size)) => {
                let side = |name: &str| size.get(name).and_then(Value::as_u64).filter(|n| SIZE.contains(n));
                let (Some(width), Some(height)) = (side("width"), side("height")) else {
                    return Err(format!("the export size needs a width and a height from {} to {} pixels", SIZE.start(), SIZE.end()));
                };
                params["resize"] = json!({ "enabled": true, "width": width, "height": height, "lockAspect": false });
            }
            ("frameRate", Value::Null) => params["frameRate"] = Value::Null,
            ("frameRate", Value::Number(rate)) if rate.as_f64().is_some_and(|fps| fps.is_finite() && fps > 0.0 && fps <= 240.0) => {
                params["frameRate"] = value.clone()
            }
            _ => {
                return Err(format!(
                    "the export setting `{key}` is {value}: it takes format ({}), span (comp or workArea), size (full, half or {{width, height}}) and frameRate (fps or null)",
                    FORMATS.join(", ")
                ));
            }
        }
    }
    if params.get("format").is_none() {
        params["format"] = json!("h264");
    }
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_name_renders_the_whole_composition() {
        assert_eq!(queue_params(1, "a.mp4", "h264").unwrap(), json!({ "comp": 1, "format": "h264", "output": "a.mp4" }));
        assert_eq!(queue_params(1, "a.mp4", "").unwrap()["format"], "h264");
    }

    #[test]
    fn the_dialogs_settings_become_render_queue_parameters() {
        let params = queue_params(3, "a.webm", r#"{"format":"webm","span":"workArea","size":{"width":320,"height":180},"frameRate":15}"#).unwrap();
        assert_eq!(params["format"], "webm");
        assert_eq!(params["timeSpan"], "workArea");
        assert_eq!(params["resize"], json!({ "enabled": true, "width": 320, "height": 180, "lockAspect": false }));
        assert_eq!(params["frameRate"], 15);
        assert_eq!(queue_params(3, "a.mp4", r#"{"size":"half","frameRate":null}"#).unwrap()["resolution"], "half");
    }

    #[test]
    fn hostile_settings_are_refused_with_what_they_take() {
        for bad in [
            r#"{"format":"exe"}"#,
            r#"{"size":{"width":0,"height":10}}"#,
            r#"{"frameRate":-1}"#,
            r#"{"span":"forever"}"#,
            r#"{"output":"/etc"}"#,
            "{",
            r#"{"size":{"width":1e30,"height":5}}"#,
        ] {
            assert!(queue_params(1, "a.mp4", bad).is_err(), "{bad} should be refused");
        }
    }
}
