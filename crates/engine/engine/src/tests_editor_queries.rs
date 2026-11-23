//! What the web editor reads from and asks of the engine: drop-down labels and a text layer's
//! style in `layer.tree`, ranges respected by keyframe edits.

use serde_json::{Value, json};

use crate::Session;

fn comp() -> Session {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Test", "width": 640, "height": 360, "frameRate": 30, "duration": 4})).unwrap();
    s
}

/// The node at `path` in a `layer.tree` answer.
fn node<'a>(tree: &'a Value, path: &str) -> Option<&'a Value> {
    if tree["path"] == path {
        return Some(tree);
    }
    tree["children"].as_array()?.iter().find_map(|child| node(child, path))
}

#[test]
fn the_tree_gives_drop_down_labels_and_the_text_style() {
    let mut s = comp();
    let text = s.execute("layer.newText", json!({"text": "Hello"})).unwrap()["layer"].as_u64().unwrap();
    s.execute("layer.setText", json!({"layer": text, "size": 72, "fill": "#ff0000", "justify": "center", "tracking": 25})).unwrap();
    let tree = s.execute("layer.tree", json!({"layer": text})).unwrap();
    let source = node(&tree["properties"], "text/sourceText").expect("source text");
    let style = &source["style"];
    assert_eq!(style["size"], 72.0, "{style}");
    assert_eq!(style["justify"], "center");
    assert_eq!(style["tracking"], 25.0);
    assert_eq!(style["fill"], json!([1.0, 0.0, 0.0]));
    assert_eq!(style["autoLeading"], true);
    assert!((style["leading"].as_f64().unwrap() - 86.4).abs() < 1e-6);
    assert!(style["family"].as_str().is_some_and(|family| !family.is_empty()));
    let path = node(&tree["properties"], "text/pathOptions/path").expect("path option");
    assert_eq!(path["options"], json!(["None"]));
    s.execute("mask.new", json!({"layer": text, "vertices": [[0, 0], [100, 0], [100, 100]]})).unwrap();
    let tree = s.execute("layer.tree", json!({"layer": text})).unwrap();
    let options = node(&tree["properties"], "text/pathOptions/path").unwrap()["options"].as_array().unwrap().clone();
    assert_eq!(options.len(), 2, "None and the new mask: {options:?}");

    let shape = s.execute("layer.newShape", json!({"kind": "rect"})).unwrap()["layer"].as_u64().unwrap();
    let tree = s.execute("layer.tree", json!({"layer": shape})).unwrap().to_string();
    assert!(tree.contains(r#""options":["Non-Zero Winding","Even-Odd"]"#), "a fill's Fill Rule lists its labels: {tree}");
    let opacity = s.execute("layer.tree", json!({"layer": shape})).unwrap();
    let opacity = node(&opacity["properties"], "transform/opacity").unwrap();
    assert!(opacity.get("options").is_none(), "a number has no labels");
    assert_eq!(opacity["range"]["min"], 0.0);
    assert_eq!(opacity["range"]["max"], 100.0);
}

#[test]
fn a_keyframe_value_stays_in_its_propertys_range() {
    let mut s = comp();
    let solid = s.execute("layer.newSolid", json!({"color": "#336699"})).unwrap()["layer"].as_u64().unwrap();
    s.execute("prop.toggleAnimation", json!({"layer": solid, "path": "transform/opacity"})).unwrap();
    s.execute("keys.set", json!({"layer": solid, "path": "transform/opacity", "time": 0, "value": 180})).unwrap();
    let opacity = |s: &Session| s.active_comp().unwrap().layers[0].props.prop("transform/opacity").unwrap().keys[0].value.clone();
    assert_eq!(opacity(&s), kition_motion_keyframe::Value::Scalar(100.0), "Opacity cannot pass 100");
    s.execute("keys.set", json!({"layer": solid, "path": "transform/opacity", "time": 0, "value": -20})).unwrap();
    assert_eq!(opacity(&s), kition_motion_keyframe::Value::Scalar(0.0), "nor go below 0");
    // A property without a declared range takes any number.
    s.execute("prop.toggleAnimation", json!({"layer": solid, "path": "transform/rotation"})).unwrap();
    s.execute("keys.set", json!({"layer": solid, "path": "transform/rotation", "time": 0, "value": 720})).unwrap();
    let rotation = s.active_comp().unwrap().layers[0].props.prop("transform/rotation").unwrap().keys[0].value.clone();
    assert_eq!(rotation, kition_motion_keyframe::Value::Scalar(720.0));
}
