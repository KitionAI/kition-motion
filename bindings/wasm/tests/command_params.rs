//! Regression: effect.apply takes the grouped parameters effect.list
//! lists (`positionsColors/color1`, Fractal Noise `transform/scale`); layer.newCamera and
//! layer.newLight take their documented `position` and `poi`; shape.newPath makes a new shape
//! layer named `name` unless `layer` names one.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn run(doc: &mut MotionDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn value(doc: &mut MotionDocument, layer: &Value, path: &str) -> Value {
    run(doc, "prop.get", json!({ "layer": layer, "path": path }))["value"].clone()
}

#[test]
fn grouped_effect_params_camera_light_and_paths() {
    let mut doc = MotionDocument::create("params.kmotion", 1280, 720).unwrap();
    let solid = run(&mut doc, "layer.newSolid", json!({ "name": "Liquid", "color": "#2a1a5e" }))["layer"].clone();
    let applied = run(
        &mut doc,
        "effect.apply",
        json!({ "layers": [solid], "effect": "4-Color Gradient", "params": { "positionsColors/color1": "#ff0000", "positionsColors/point1": [100, 100] } }),
    );
    let prefix = applied["paths"][0].as_str().unwrap().to_owned();
    assert_eq!(value(&mut doc, &solid, &format!("{prefix}/positionsColors/point1")), json!([100.0, 100.0]));
    let noise = run(&mut doc, "effect.apply", json!({ "layers": [solid], "effect": "Fractal Noise", "params": { "transform/scale": 250 } }));
    let prefix = noise["paths"][0].as_str().unwrap().to_owned();
    assert_eq!(value(&mut doc, &solid, &format!("{prefix}/transform/scale")), json!(250.0));

    let camera = run(&mut doc, "layer.newCamera", json!({ "name": "Cam", "position": [640, 360, -1200], "poi": [640, 360, 0] }))["layer"].clone();
    assert_eq!(value(&mut doc, &camera, "transform/position"), json!([640.0, 360.0, -1200.0]));
    let light = run(&mut doc, "layer.newLight", json!({ "kind": "Point", "position": [100, 200, -300] }))["layer"].clone();
    assert_eq!(value(&mut doc, &light, "transform/position"), json!([100.0, 200.0, -300.0]));

    // With a shape layer selected, a new path without `layer` is a new layer, named.
    let first = run(&mut doc, "shape.newPath", json!({ "name": "Wave", "vertices": [[0, 0], [100, 50], [200, 0]] }));
    let second = run(&mut doc, "shape.newPath", json!({ "name": "Ripple", "vertices": [[0, 100], [100, 150], [200, 100]] }));
    assert_ne!(first["layer"], second["layer"], "{first} {second}");
    let layers = doc.inspect().unwrap()["info"]["layers"].clone();
    let named = layers.as_array().unwrap().iter().find(|l| l["id"] == second["layer"]).cloned().unwrap_or_else(|| panic!("{layers}"));
    assert_eq!(named["name"], json!("Ripple"), "{named}");
}
