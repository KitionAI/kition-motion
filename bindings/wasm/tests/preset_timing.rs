//! Regressions (timing): the text presets start at `start`
//! (not the current time) and typewriter takes `frames` per character; keys.applyEase takes
//! `from`/`to` off the frame grid as prop.addKey does, snapped.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn run(doc: &mut MotionDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

/// The comp times (seconds) of a property's keys.
fn key_times(doc: &mut MotionDocument, layer: &Value, path: &str) -> Vec<f64> {
    let prop = run(doc, "prop.get", json!({ "layer": layer, "path": path }));
    prop["keys"].as_array().unwrap_or_else(|| panic!("{prop}")).iter().map(|k| k["time"].as_f64().unwrap_or_else(|| panic!("{k}"))).collect()
}

#[test]
fn issue_6_text_presets_start_at_start_and_take_frames() {
    let mut doc = MotionDocument::create("titles.kmotion", 1280, 720).unwrap();
    let head = run(&mut doc, "layer.newText", json!({ "text": "Hello there", "position": [640, 360], "size": 72 }))["layer"].clone();
    run(&mut doc, "layer.applyTextPreset", json!({ "layer": head, "preset": "trackingIn", "start": 2.0 }));
    let tracking = key_times(&mut doc, &head, "text/animators/#1/properties/tracking");
    assert!((tracking[0] - 2.0).abs() < 1e-6, "tracking keys start at 2 s: {tracking:?}");
    let line = run(&mut doc, "layer.newText", json!({ "text": "Typing", "position": [640, 500], "size": 48 }))["layer"].clone();
    // 3 frames a character, six characters: 18 frames from 1 s.
    run(&mut doc, "layer.applyTextPreset", json!({ "layer": line, "preset": "typewriter", "start": 1.0, "frames": 3 }));
    let typed = key_times(&mut doc, &line, "text/animators/#1/selectors/#1/start");
    let fps = doc.inspect().unwrap()["frameRate"].as_f64().unwrap_or(30.0);
    assert!((typed[0] - 1.0).abs() < 1e-6, "{typed:?}");
    assert!((typed[1] - (1.0 + 18.0 / fps)).abs() < 1.0 / fps, "18 frames at {fps} fps: {typed:?}");
}

#[test]
fn issue_7_apply_ease_snaps_its_range() {
    let mut doc = MotionDocument::create("ease.kmotion", 1280, 720).unwrap();
    let solid = run(&mut doc, "layer.newSolid", json!({ "name": "Badge", "color": "#ff3366" }))["layer"].clone();
    run(&mut doc, "prop.addKey", json!({ "layer": solid, "path": "transform/opacity", "time": 0.333, "value": 0 }));
    run(&mut doc, "prop.addKey", json!({ "layer": solid, "path": "transform/opacity", "time": 1.666, "value": 100 }));
    let eased = run(&mut doc, "keys.applyEase", json!({ "layer": solid, "path": "transform/opacity", "preset": "out", "from": 0.333, "to": 1.666 }));
    assert_eq!(eased["segments"], json!(1), "{eased}");
}
