//! Editing a motion project end to end: commands build a composition, the project saves to
//! `.kmotion`, reopens from those bytes, and describes itself exactly as it did before saving
//! (every layer's transform, text, effects and masks, the comp's settings and markers). Saving
//! the reopened project gives the same project again.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn run(doc: &mut MotionDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params.clone()).unwrap_or_else(|error| panic!("{command} {params}: {error}"))
}

fn layer(answer: &Value) -> u64 {
    answer["layer"].as_u64().unwrap_or_else(|| panic!("no layer in {answer}"))
}

/// What the project says about itself: the comp, and for every layer its transform, text,
/// every effect and every mask, read in full, and the markers.
fn described(doc: &mut MotionDocument) -> Value {
    let comp = run(doc, "comp.info", json!({}));
    let mut layers = Vec::new();
    for entry in comp["layers"].as_array().unwrap() {
        let id = entry["id"].as_u64().unwrap();
        let read = |doc: &mut MotionDocument, path: &str| doc.execute("prop.get", json!({ "layer": id, "path": path })).ok();
        let mut groups = json!({ "transform": read(doc, "transform"), "text": read(doc, "text") });
        for (group, kind) in [("effects", "effect"), ("masks", "mask")] {
            let count = read(doc, group).and_then(|value| value["children"].as_array().map(Vec::len)).unwrap_or(0);
            let items: Vec<Value> = (1..=count).map(|n| read(doc, &format!("{group}/#{n}")).unwrap_or(Value::Null)).collect();
            groups[kind] = Value::Array(items);
        }
        layers.push(json!({ "layer": entry, "properties": groups }));
    }
    let markers = run(doc, "marker.list", json!({}));
    json!({ "comp": comp, "markers": markers, "layers": layers })
}

fn project(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

/// Saves, reopens from the saved bytes, and checks the reopened project is the one saved.
fn saves_and_reopens(mut doc: MotionDocument) -> MotionDocument {
    let before = described(&mut doc);
    let saved = doc.save(Some("Promo.kmotion")).unwrap();
    doc.mark_saved();
    assert!(!doc.dirty());
    let mut reopened = MotionDocument::open("Promo.kmotion", &saved).unwrap();
    assert_eq!(described(&mut reopened), before);
    assert_eq!(project(&reopened.save(Some("Promo.kmotion")).unwrap()), project(&saved));
    reopened
}

fn new_comp() -> MotionDocument {
    let mut doc = MotionDocument::create("Promo.kmotion", 640, 360).unwrap();
    run(&mut doc, "comp.settings", json!({ "width": 640, "height": 360, "frameRate": 25, "duration": 4, "background": "#101018" }));
    doc
}

#[test]
fn layers_and_their_settings_survive_save_and_reopen() {
    let mut doc = new_comp();
    let sky = layer(&run(&mut doc, "layer.newSolid", json!({ "name": "Backdrop", "color": "#203040" })));
    let title =
        layer(&run(&mut doc, "layer.newText", json!({ "text": "Spring fair \u{6625}\u{5b63}", "position": [320, 160], "size": 48, "fill": "#ffffff" })));
    let badge = layer(&run(&mut doc, "layer.newShape", json!({ "fill": "#5645d4" })));
    let rig = layer(&run(&mut doc, "layer.newNull", json!({})));
    run(&mut doc, "layer.rename", json!({ "layer": sky, "name": "Sky" }));
    run(&mut doc, "layer.rename", json!({ "layer": rig, "name": "Rig" }));
    run(&mut doc, "layer.setBlendMode", json!({ "layers": [badge], "mode": "Screen" }));
    run(&mut doc, "layer.setParent", json!({ "layers": [title], "parent": rig }));
    run(&mut doc, "layer.timing", json!({ "layers": [badge], "in": 0.5, "out": 3.5 }));
    run(&mut doc, "prop.set", json!({ "layer": title, "path": "text/sourceText", "value": "Spring fair \u{6625}\u{5b63} \u{1f338}" }));
    run(&mut doc, "comp.addMarker", json!({ "time": 2, "comment": "Logo in" }));

    let mut reopened = saves_and_reopens(doc);
    let comp = run(&mut reopened, "comp.info", json!({}));
    let names: Vec<&str> = comp["layers"].as_array().unwrap().iter().map(|entry| entry["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"Sky") && names.contains(&"Rig"), "{names:?}");
    let text = run(&mut reopened, "prop.get", json!({ "layer": title, "path": "text/sourceText" }));
    assert_eq!(text["value"], "Spring fair \u{6625}\u{5b63} \u{1f338}");
    assert!(run(&mut reopened, "marker.list", json!({})).to_string().contains("Logo in"));
}

#[test]
fn keyframes_eases_and_expressions_survive_save_and_reopen() {
    let mut doc = new_comp();
    let title = layer(&run(&mut doc, "layer.newText", json!({ "text": "Kition", "position": [320, 180], "size": 40, "fill": "#ffffff" })));
    run(&mut doc, "prop.addKey", json!({ "layer": title, "path": "transform/opacity", "time": 0, "value": 0 }));
    run(&mut doc, "prop.addKey", json!({ "layer": title, "path": "transform/opacity", "time": 1, "value": 100 }));
    run(&mut doc, "prop.addKey", json!({ "layer": title, "path": "transform/position", "time": 0, "value": [320, 240] }));
    run(&mut doc, "prop.addKey", json!({ "layer": title, "path": "transform/position", "time": 1.5, "value": [320, 180] }));
    run(
        &mut doc,
        "keyframe.select",
        json!({ "keys": [{ "layer": title, "path": "transform/opacity", "time": 0 }, { "layer": title, "path": "transform/opacity", "time": 1 }] }),
    );
    run(&mut doc, "keyframe.easyEase", json!({}));
    run(&mut doc, "prop.set", json!({ "layer": title, "path": "transform/rotation", "value": -8 }));
    run(&mut doc, "prop.setExpression", json!({ "layer": title, "path": "transform/scale", "expression": "[100 + time * 10, 100 + time * 10]" }));

    let mut reopened = saves_and_reopens(doc);
    let opacity = run(&mut reopened, "prop.get", json!({ "layer": title, "path": "transform/opacity" }));
    let keys = opacity["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|key| key["in"] == "Bezier" && key["out"] == "Bezier"), "{opacity}");
    let scale = run(&mut reopened, "prop.get", json!({ "layer": title, "path": "transform/scale" }));
    assert_eq!(scale["expression"], "[100 + time * 10, 100 + time * 10]");
}

#[test]
fn effects_and_masks_survive_save_and_reopen() {
    let mut doc = new_comp();
    let sky = layer(&run(&mut doc, "layer.newSolid", json!({ "name": "Sky", "color": "#203040" })));
    run(&mut doc, "effect.apply", json!({ "layers": [sky], "effect": "Gaussian Blur" }));
    run(&mut doc, "effect.apply", json!({ "layer": sky, "effect": "Fill" }));
    run(&mut doc, "prop.set", json!({ "layer": sky, "path": "effects/#1/blurriness", "value": 12 }));
    run(&mut doc, "effect.reorder", json!({ "layer": sky, "effect": "Fill", "index": 1 }));
    run(&mut doc, "mask.new", json!({ "layer": sky, "vertices": [[100, 60], [540, 60], [540, 300], [100, 300]], "mode": "Subtract" }));
    run(&mut doc, "mask.setClosed", json!({ "layer": sky, "mask": 1 }));
    run(&mut doc, "prop.set", json!({ "layer": sky, "path": "masks/#1/feather", "value": [8, 8] }));

    let mut reopened = saves_and_reopens(doc);
    let effects = run(&mut reopened, "prop.get", json!({ "layer": sky, "path": "effects" }));
    let order: Vec<&str> = effects["children"].as_array().unwrap().iter().map(|effect| effect["name"].as_str().unwrap()).collect();
    assert_eq!(order, ["Fill", "Gaussian Blur"]);
    let blur = run(&mut reopened, "prop.get", json!({ "layer": sky, "path": "effects/#2/blurriness" }));
    assert_eq!(blur["value"], 12.0);
    let path = run(&mut reopened, "prop.get", json!({ "layer": sky, "path": "masks/#1/path" }));
    assert_eq!(path["value"]["v"]["closed"], true);
}
