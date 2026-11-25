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
    s.execute("keyframe.set", json!({"layer": solid, "path": "transform/opacity", "time": 0, "value": 180})).unwrap();
    let opacity = |s: &Session| s.active_comp().unwrap().layers[0].props.prop("transform/opacity").unwrap().keys[0].value.clone();
    assert_eq!(opacity(&s), kition_motion_keyframe::Value::Scalar(100.0), "Opacity cannot pass 100");
    s.execute("keyframe.set", json!({"layer": solid, "path": "transform/opacity", "time": 0, "value": -20})).unwrap();
    assert_eq!(opacity(&s), kition_motion_keyframe::Value::Scalar(0.0), "nor go below 0");
    // A property without a declared range takes any number.
    s.execute("prop.toggleAnimation", json!({"layer": solid, "path": "transform/rotation"})).unwrap();
    s.execute("keyframe.set", json!({"layer": solid, "path": "transform/rotation", "time": 0, "value": 720})).unwrap();
    let rotation = s.active_comp().unwrap().layers[0].props.prop("transform/rotation").unwrap().keys[0].value.clone();
    assert_eq!(rotation, kition_motion_keyframe::Value::Scalar(720.0));
}

/// A malformed project's slider range (its minimum above its maximum, or NaN) and keys sharing a
/// time made `prop.set` and `keyframe.set` panic in `clamp`; they now keep the value or the key.
#[test]
fn a_broken_range_or_keys_at_one_time_do_not_panic() {
    let mut s = comp();
    let solid = s.execute("layer.newSolid", json!({"color": "#336699"})).unwrap()["layer"].as_u64().unwrap();
    for (min, max) in [(100.0, 0.0), (f64::NAN, 100.0), (0.0, f64::NAN)] {
        set_opacity_ui(&mut s, kition_motion_project::ParamUi::Slider { min, max, slider_min: 0.0, slider_max: 100.0, decimals: 1 });
        let set = s.execute("prop.set", json!({"layer": solid, "path": "transform/opacity", "value": 50}));
        assert!(set.is_ok(), "prop.set with the range {min}..{max}: {set:?}");
    }
    // Two keys at one time with the next a tick later: a move of the second has no room between its
    // neighbours and keeps it where it is.
    set_opacity_ui(&mut s, kition_motion_project::ParamUi::Number);
    for t in 0..5 {
        s.execute("prop.addKey", json!({"layer": solid, "path": "transform/opacity", "time": t, "value": 10 * t})).unwrap();
    }
    let one = kition_motion_time::Tick::from_seconds_f64(1.0);
    edit_opacity(&mut s, |pr| {
        pr.keys[2].time = one;
        pr.keys[3].time = kition_motion_time::Tick(one.0 + 1);
    });
    let moved = s.execute("keyframe.set", json!({"layer": solid, "path": "transform/opacity", "time": 1, "newTime": 3}));
    assert!(moved.is_ok(), "{moved:?}");
}

fn edit_opacity(s: &mut Session, f: impl FnOnce(&mut kition_motion_project::Property)) {
    let id = s.active_comp_id().unwrap();
    let project = std::sync::Arc::make_mut(&mut s.project);
    let comp = project.comp_mut(id).unwrap();
    f(comp.layers[0].props.prop_mut("transform/opacity").unwrap());
}

fn set_opacity_ui(s: &mut Session, ui: kition_motion_project::ParamUi) {
    edit_opacity(s, |pr| pr.ui = ui);
}
