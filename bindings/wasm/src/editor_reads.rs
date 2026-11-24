//! What the editor's Render Queue panel, its status bar and its timeline ask the document for
//! beyond the commands: a copy of the project for the render queue to render from (so edits made
//! while it renders never reach a job), the colour under the pointer, and a layer's waveform.

use std::sync::Arc;

use kition_motion_project::render_queue::{OutputFormat, PostRenderAction, RenderStatus};
use kition_motion_time::Tick;
use serde_json::{Value, json};

use crate::{MotionDocument, movie};

/// The most buckets a waveform is drawn with.
pub const MAX_WAVEFORM_BUCKETS: u32 = 4096;
/// How much of a layer a waveform reads, in seconds (the rest of a longer layer is drawn flat).
pub const MAX_WAVEFORM_SECONDS: f64 = 600.0;
/// The rate a waveform is read at: enough for its peaks, cheap to mix.
const WAVEFORM_RATE: u32 = 2000;

impl MotionDocument {
    /// What the canvas tools name the layers they make, in the editor's language: JSON
    /// `{"shape": "形状图层"}` (numbered `形状图层 2`); a missing or empty name keeps the engine's.
    /// `labels` (`{"text": "Peach", …}`, by kind: text, shape, solid, adjustment, null, camera,
    /// light) are the labels the `layer.new…` commands give the layers they make from now on; an
    /// unknown label name is left out, and without `labels` new layers get the engine's.
    pub fn set_layer_names(&mut self, names: &str) {
        let names = serde_json::from_str::<Value>(names).unwrap_or(Value::Null);
        self.shape_layer_name = names.get("shape").and_then(Value::as_str).map(str::trim).filter(|name| !name.is_empty()).map(str::to_owned);
        self.session.new_layer_labels = names.get("labels").and_then(Value::as_object).map(|labels| {
            labels.iter().filter_map(|(kind, name)| Some((kind.clone(), kition_motion_engine::color::Label::from_name(name.as_str()?)?))).collect()
        });
    }

    /// A new shape layer's name from [`MotionDocument::set_layer_names`]: the base and the
    /// smallest number no layer of the composition has.
    pub(crate) fn new_shape_layer_name(&self) -> Option<String> {
        let base = self.shape_layer_name.as_deref()?;
        let taken: std::collections::HashSet<&str> =
            self.session.active_comp().map(|comp| comp.layers.iter().map(|layer| layer.name.as_str()).collect()).unwrap_or_default();
        (1..=10_000).map(|n| format!("{base} {n}")).find(|name| !taken.contains(name.as_str()))
    }

    /// The Render Queue panel's Render: keeps a copy of the project as it is now, which every job
    /// of this run renders from (an edit made while the queue renders reaches no job; the next
    /// Render takes a new copy). Answers the ids of the items whose Render box is on, in queue
    /// order.
    pub fn render_snapshot(&mut self) -> Value {
        let project = self.session.project.clone();
        let ids: Vec<u64> = project.render_queue.iter().filter(|item| item.render).map(|item| item.id).collect();
        self.render_snapshot = Some(project);
        json!(ids)
    }

    /// Drops the copy [`MotionDocument::render_snapshot`] kept; whether there was one.
    pub fn drop_render_snapshot(&mut self) -> bool {
        self.render_snapshot.take().is_some()
    }

    /// A render of render queue item `item` as it was when the queue's Render was pressed, written
    /// to `path` in memory (the editor names the file and the host saves it). Only the formats the
    /// web build writes as one file (H.264, WebM, GIF) render here.
    pub(crate) fn new_queue_export(&self, item: u64, path: &str) -> Result<movie::Export, String> {
        let project = self.render_snapshot.clone().ok_or("the render queue is not rendering: press Render first")?;
        let mut job =
            project.render_queue.iter().find(|queued| queued.id == item).cloned().ok_or_else(|| format!("render queue item {item} is not in the queue"))?;
        if project.comp(job.comp).is_none() {
            return Err("the item's composition is no longer in the project".to_owned());
        }
        if !matches!(
            job.output.format,
            OutputFormat::H264 | OutputFormat::WebM | OutputFormat::Gif | OutputFormat::PngSequence | OutputFormat::JpegSequence | OutputFormat::Wav
        ) {
            return Err(format!("{} is not written here: choose H.264, WebM, GIF, a PNG or JPEG sequence, or WAV", job.output.format.label()));
        }
        job.render = true;
        job.status = RenderStatus::Queued;
        job.output.output = path.to_owned();
        job.extra_outputs.clear();
        job.post_render = PostRenderAction::None;
        let mut scratch = Self::new_session(&self.services, &self.pool);
        scratch.state = self.session.state.clone();
        scratch.project = Arc::new({
            let mut copy = (*project).clone();
            copy.render_queue = vec![job];
            copy
        });
        Ok(movie::Export::new(scratch.render_stepped()?, path))
    }

    /// The colour the viewer shows at composition pixel (`x`, `y`) at the current time, as
    /// 8-bit RGBA (composited over the composition's background, as the viewer draws it); `None`
    /// off the picture or with no composition open.
    pub fn pixel_at(&self, x: f64, y: f64) -> Option<[u8; 4]> {
        if !(x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0) {
            return None;
        }
        let (rgba, width, height) = self.frame().ok()?;
        let (px, py) = (x.floor() as u64, y.floor() as u64);
        if px >= u64::from(width) || py >= u64::from(height) {
            return None;
        }
        let at = usize::try_from(py.checked_mul(u64::from(width))?.checked_add(px)?.checked_mul(4)?).ok()?;
        let pixel: [u8; 4] = rgba.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(pixel)
    }

    /// Layer `layer`'s sound in the active composition as `count` peaks (0 to 1, the loudest
    /// sample of either channel in each stretch) from its in point to its out point, heard on its
    /// own with its Audio Levels and audio effects, whether or not it is muted: what the timeline
    /// draws on an audio layer's bar. `{from, to, peaks}`, seconds of composition time; `peaks`
    /// is empty for a layer with no sound.
    pub fn layer_waveform(&self, layer: u64, count: u32) -> Result<Value, String> {
        let comp_id = self.session.active_comp_id().ok_or("no composition is open")?;
        let comp = self.session.project.comp(comp_id).ok_or("no composition is open")?;
        let found = comp.layers.iter().find(|each| each.id.0 == layer).ok_or_else(|| format!("no layer {layer} in the composition"))?;
        let from = found.in_point.max(Tick::ZERO);
        let to = found.out_point.min(comp.duration);
        let span = (to.seconds() - from.seconds()).min(MAX_WAVEFORM_SECONDS);
        let count = count.clamp(1, MAX_WAVEFORM_BUCKETS) as usize;
        let mut alone = (*self.session.project).clone();
        if let Some(copy) = alone.comp_mut(comp_id) {
            copy.layers.retain(|each| each.id.0 == layer);
            for each in &mut copy.layers {
                each.switches.audio = true;
                each.switches.solo = false;
            }
        }
        if span.is_nan() || span <= 0.0 || !kition_motion_engine::render::audio::comp_has_audio(&alone, comp_id) {
            return Ok(json!({ "from": from.seconds(), "to": to.seconds(), "peaks": [] }));
        }
        let frames = (span * f64::from(WAVEFORM_RATE)).ceil() as usize;
        let samples =
            kition_motion_engine::render::audio::mix_comp(&alone, &*self.session.footage, self.session.expr.as_deref(), comp_id, from, frames, WAVEFORM_RATE);
        let per = frames.div_ceil(count).max(1);
        let peaks: Vec<f64> = samples
            .chunks(per * 2)
            .map(|chunk| f64::from(chunk.iter().fold(0.0f32, |loudest, sample| if sample.is_finite() { loudest.max(sample.abs()) } else { loudest }).min(1.0)))
            .map(|peak| (peak * 1000.0).round() / 1000.0)
            .collect();
        Ok(json!({ "from": from.seconds(), "to": from.seconds() + span, "peaks": peaks }))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use kition_kit::KitDocument;
    use serde_json::json;

    use crate::MotionDocument;

    fn doc() -> MotionDocument {
        let mut doc = MotionDocument::create("a.kmotion", 64, 36).unwrap();
        doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
        doc
    }

    #[test]
    fn new_shape_layers_take_the_editors_name_numbered() {
        let mut doc = doc();
        assert_eq!(doc.new_shape_layer_name(), None);
        doc.set_layer_names(r#"{"shape": "形状图层"}"#);
        assert_eq!(doc.new_shape_layer_name().as_deref(), Some("形状图层 1"));
        doc.execute("layer.newShape", json!({ "name": "形状图层 1" })).unwrap();
        assert_eq!(doc.new_shape_layer_name().as_deref(), Some("形状图层 2"));
        doc.set_layer_names("not json");
        assert_eq!(doc.new_shape_layer_name(), None);
    }

    /// The labels the editor gives new layers by kind: a Layer ▸ New text layer, null and camera
    /// get theirs (one undo step each, as before); a duplicate keeps its original's.
    #[test]
    fn new_layers_get_the_editors_label_for_their_kind() {
        let mut doc = doc();
        let label = |doc: &MotionDocument, id: u64| {
            doc.session.active_comp().and_then(|c| c.layer(kition_motion_project::LayerId(id)).map(|l| l.label.name().to_string()))
        };
        let made = |doc: &mut MotionDocument, command: &str| doc.execute(command, json!({})).unwrap()["layer"].as_u64().unwrap();
        // Without labels from the editor: the engine's (a null is red).
        let null = made(&mut doc, "layer.newNull");
        assert_eq!(label(&doc, null).as_deref(), Some("Red"));
        doc.set_layer_names(r#"{"labels": {"text": "Peach", "null": "Brown", "camera": "Cyan", "solid": "Not a label"}}"#);
        let text = made(&mut doc, "layer.newText");
        let null = made(&mut doc, "layer.newNull");
        let camera = made(&mut doc, "layer.newCamera");
        let solid = made(&mut doc, "layer.newSolid");
        assert_eq!(label(&doc, text).as_deref(), Some("Peach"));
        assert_eq!(label(&doc, null).as_deref(), Some("Brown"));
        assert_eq!(label(&doc, camera).as_deref(), Some("Cyan"));
        assert_eq!(label(&doc, solid).as_deref(), Some("Red"), "an unknown label name keeps the engine's");
        // One undo step takes the new layer away whole.
        let before = doc.session.active_comp().map(|c| c.layers.len()).unwrap();
        doc.execute("edit.undo", json!({})).unwrap();
        assert_eq!(doc.session.active_comp().map(|c| c.layers.len()).unwrap(), before - 1);
        // A label set by hand stays on a duplicate.
        doc.execute("edit.label", json!({"label": "Purple", "layers": [text], "target": "layers"})).unwrap();
        let copy = doc.execute("edit.duplicate", json!({"layers": [text]})).unwrap();
        let copy = copy["layers"].as_array().and_then(|a| a.first()).and_then(serde_json::Value::as_u64).or_else(|| copy["layer"].as_u64());
        if let Some(copy) = copy {
            assert_eq!(label(&doc, copy).as_deref(), Some("Purple"));
        }
    }

    #[test]
    fn the_pixel_under_the_pointer_is_the_one_the_viewer_shows() {
        let doc = doc();
        let [r, g, b, a] = doc.pixel_at(10.5, 10.5).unwrap();
        assert!(r > 250 && g < 5 && b < 5 && a == 255, "{r} {g} {b} {a}");
        assert!(doc.pixel_at(64.0, 0.0).is_none());
        assert!(doc.pixel_at(-1.0, 3.0).is_none());
        assert!(doc.pixel_at(f64::NAN, 3.0).is_none());
    }

    #[test]
    fn a_silent_layer_has_no_waveform_and_an_unknown_one_is_refused() {
        let doc = doc();
        let layer = doc.session.active_comp().unwrap().layers[0].id.0;
        let wave = doc.layer_waveform(layer, 100).unwrap();
        assert_eq!(wave["peaks"], json!([]));
        assert!(doc.layer_waveform(999_999, 10).is_err());
    }

    #[test]
    fn a_tone_layer_draws_its_peaks_even_muted() {
        let mut doc = doc();
        let layer = doc.session.active_comp().unwrap().layers[0].id.0;
        doc.execute("effect.apply", json!({ "layer": layer, "effect": "ec.audio.tone" })).unwrap();
        doc.execute("layer.setSwitch", json!({ "layers": [layer], "switch": "audio", "value": false })).unwrap();
        let wave = doc.layer_waveform(layer, 50).unwrap();
        let peaks = wave["peaks"].as_array().unwrap();
        assert!(!peaks.is_empty() && peaks.iter().all(|peak| peak.as_f64().unwrap() > 0.01), "{wave}");
    }

    #[test]
    fn a_queue_job_renders_the_project_as_it_was_when_render_was_pressed() {
        let mut doc = doc();
        doc.execute("renderQueue.add", json!({ "format": "gif", "timeSpan": "custom", "start": 0, "end": 0.2 })).unwrap();
        let ids = doc.render_snapshot();
        let id = ids[0].as_u64().unwrap();
        // Deleted after Render was pressed: the job still renders from the copy.
        doc.execute("renderQueue.remove", json!({ "item": id })).unwrap();
        let mut export = doc.new_queue_export(id, "job.gif").unwrap();
        while !export.step(f64::INFINITY).finished {}
        let path = export.finished().unwrap();
        assert!(doc.take_file(&path).is_some_and(|bytes| bytes.starts_with(b"GIF")));
        assert!(doc.drop_render_snapshot());
        assert!(doc.new_queue_export(id, "job.gif").is_err());
    }

    #[test]
    fn a_queue_job_writes_a_png_sequence_with_alpha_and_a_wav() {
        let mut doc = doc();
        doc.execute(
            "renderQueue.add",
            json!({ "format": "png", "channels": "rgba", "timeSpan": "custom", "start": 0, "end": 0.1, "output": "seq/Shot_[#####].png" }),
        )
        .unwrap();
        doc.execute("renderQueue.add", json!({ "format": "wav", "timeSpan": "custom", "start": 0, "end": 0.5, "output": "Sound.wav" })).unwrap();
        let ids = doc.render_snapshot();
        let (png, wav) = (ids[0].as_u64().unwrap(), ids[1].as_u64().unwrap());
        doc.export.put(doc.new_queue_export(png, "Shot_[#####].png").unwrap());
        while !doc.export_step(f64::INFINITY).unwrap().finished {}
        let names = doc.export_finish_files().unwrap();
        assert!(!names.is_empty() && names.iter().all(|name| name.starts_with("Shot_") && name.ends_with(".png")), "{names:?}");
        assert!(names.iter().all(|name| doc.take_file(name).is_some_and(|bytes| bytes.get(25) == Some(&6))), "RGBA PNGs");
        doc.export.put(doc.new_queue_export(wav, "Sound.wav").unwrap());
        while !doc.export_step(f64::INFINITY).unwrap().finished {}
        assert_eq!(doc.export_finish_files().unwrap(), vec!["Sound.wav".to_owned()]);
        assert!(doc.take_file("Sound.wav").is_some_and(|bytes| bytes.starts_with(b"RIFF")));
    }

    #[test]
    fn frames_cached_ahead_fill_the_work_area_and_an_edit_starts_again() {
        let mut doc = doc();
        doc.execute("comp.settings", json!({ "duration": 1.0, "frameRate": 10 })).unwrap();
        let first = doc.cache_ahead(0.0);
        assert_eq!(first["total"], json!(10));
        // Enough time for every frame: the work area is all held, none was shown.
        let all = doc.cache_ahead(2000.0);
        assert_eq!((all["done"].as_i64(), all["full"].as_bool()), (Some(10), Some(false)), "{all}");
        let report = doc.preview_cache();
        assert_eq!(report["cached"].as_array().map(Vec::len), Some(10));
        assert_eq!(report["shown"], json!(0), "frames cached ahead are not shown");
        // The viewer's frame now comes from the cache.
        doc.render(0.0, 0.0, 1.0, 64, 36).unwrap();
        assert_eq!(doc.preview_cache()["hits"], json!(1));
        // An edit: the cache holds nothing of the new revision until it fills again.
        doc.execute("layer.newSolid", json!({ "name": "Blue", "color": "#0000ff" })).unwrap();
        assert_eq!(doc.cache_ahead(0.0)["done"], json!(0));
        assert_eq!(doc.cache_ahead(2000.0)["done"], json!(10));
    }
}
