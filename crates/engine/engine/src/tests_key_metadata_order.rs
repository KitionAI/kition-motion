//! Key attributes preserve the order used by the next stagger gesture.

use std::sync::Arc;

use kition_motion_keyframe::Interp;
use kition_motion_project::LayerId;
use serde_json::json;

use crate::{KeyRef, Session, time::FrameRate};

fn setup() -> (Session, u64) {
    let mut s = Session::default();
    s.execute_checked("comp.new", json!({"width": 64, "height": 64, "frameRate": 30, "duration": 4})).unwrap();
    let layer = s.execute_checked("layer.newSolid", json!({"name": "Box", "color": "#fff"})).unwrap()["layer"].as_u64().unwrap();
    (s, layer)
}

fn add(s: &mut Session, layer: u64, path: &str, time: i64, value: serde_json::Value) {
    s.execute_checked("prop.addKey", json!({"layer": layer, "path": path, "time": time, "value": value})).unwrap();
}

fn uid(s: &Session, layer: u64, path: &str) -> u64 {
    s.active_comp().unwrap().layer(LayerId(layer)).unwrap().props.prop(path).unwrap().uid
}

fn key(layer: u64, prop: u64, frame: i64) -> KeyRef {
    KeyRef { layer: LayerId(layer), prop, time: FrameRate::FPS_30.tick_of(frame) }
}

#[test]
fn easing_preserves_interleaved_layer_and_property_order_with_one_undo() {
    let (mut s, first) = setup();
    let second = s.execute_checked("layer.newSolid", json!({"name": "Other", "color": "#fff"})).unwrap()["layer"].as_u64().unwrap();
    for layer in [first, second] {
        for time in [0, 1] {
            add(&mut s, layer, "transform/opacity", time, json!(time * 25));
        }
    }
    add(&mut s, first, "transform/position", 1, json!([10, 20]));
    let a = uid(&s, first, "transform/opacity");
    let b = uid(&s, second, "transform/opacity");
    let position = uid(&s, first, "transform/position");
    s.execute_checked(
        "keyframe.select",
        json!({"keys": [
            {"layer": second, "prop": b, "time": 1},
            {"layer": first, "prop": position, "time": 1},
            {"layer": first, "prop": a, "time": 0},
            {"layer": second, "prop": b, "time": 0}
        ]}),
    )
    .unwrap();
    let order = s.state.selected_keys.clone();
    let before = s.project.clone();
    s.history.clear();
    s.execute_checked("keyframe.easyEase", json!({})).unwrap();
    assert_eq!(s.state.selected_keys, order);
    assert_eq!(s.history.undo.len(), 1);
    for selected in &order {
        let keys = &s.active_comp().unwrap().layer(selected.layer).unwrap().props.find(selected.prop).unwrap().keys;
        assert_eq!(keys.iter().find(|k| k.time == selected.time).unwrap().out_interp, Interp::Bezier);
    }
    s.execute_checked("edit.undo", json!({})).unwrap();
    assert!(Arc::ptr_eq(&s.project, &before));
    assert_eq!(s.state.selected_keys, order);
}

#[test]
fn roving_keeps_the_selected_key_identity_and_arbitrary_order() {
    let (mut s, layer) = setup();
    for (time, x) in [(0, 0), (1, 25), (2, 100)] {
        add(&mut s, layer, "transform/position", time, json!([x, 0]));
    }
    let prop = uid(&s, layer, "transform/position");
    s.execute_checked(
        "keyframe.select",
        json!({"keys": [
            {"layer": layer, "prop": prop, "time": 1},
            {"layer": layer, "prop": prop, "time": 2},
            {"layer": layer, "prop": prop, "time": 0}
        ]}),
    )
    .unwrap();
    s.execute_checked("keyframe.interpolation", json!({"roving": true})).unwrap();
    assert_eq!(s.state.selected_keys, vec![key(layer, prop, 15), key(layer, prop, 60), key(layer, prop, 0)]);
    let position = s.active_comp().unwrap().layer(LayerId(layer)).unwrap().props.find(prop).unwrap();
    assert_eq!(position.keys[1].value.as_vec2(), [25.0, 0.0]);
    assert_eq!(position.keys[1].time, FrameRate::FPS_30.tick_of(15));
    assert!(position.keys[1].roving);
    assert!(!position.keys[0].roving && !position.keys[2].roving);
}

#[test]
fn attribute_edit_deduplicates_legacy_refs_and_drops_stale_refs_without_snapping() {
    let (mut s, layer) = setup();
    for time in [0, 1] {
        add(&mut s, layer, "transform/opacity", time, json!(20));
    }
    let prop = uid(&s, layer, "transform/opacity");
    let first = key(layer, prop, 0);
    let second = key(layer, prop, 30);
    s.state.selected_keys = vec![second, key(layer, prop, 10), first, second, key(layer, u64::MAX, 0), first, key(u64::MAX, prop, 0)];
    s.history.clear();
    let result = s.execute_checked("keyframe.toggleHold", json!({})).unwrap();
    assert_eq!(result, json!(2));
    assert_eq!(s.state.selected_keys, vec![second, first]);
    assert_eq!(s.history.undo.len(), 1);
    let opacity = s.active_comp().unwrap().layer(LayerId(layer)).unwrap().props.find(prop).unwrap();
    assert!(opacity.keys.iter().all(|k| k.out_interp == Interp::Hold));
    s.execute_checked("edit.undo", json!({})).unwrap();
    let opacity = s.active_comp().unwrap().layer(LayerId(layer)).unwrap().props.find(prop).unwrap();
    assert!(opacity.keys.iter().all(|k| k.out_interp == Interp::Linear));
}

/// A keyframe move undone: the selected key is selected where it is back (not at the time it was
/// moved to), a click on it then selects it once, and the selection is a set everywhere
/// (`keyframe.info` lists each key once, a key given twice is selected once). Redo takes the
/// selection along again. Before, the selection kept a reference to the moved-to time, which
/// `keyframe.info` read as the nearest key: the same key twice, and Wiggler found nothing to do.
#[test]
fn a_moved_key_undone_and_clicked_again_is_selected_once() {
    let (mut s, layer) = setup();
    for time in [0, 1, 2] {
        add(&mut s, layer, "transform/opacity", time, json!(time * 30));
    }
    let prop = uid(&s, layer, "transform/opacity");
    let at = |frame: i64| json!({"layer": layer, "prop": prop, "time": frame as f64 / 30.0});
    s.execute_checked("keyframe.select", json!({"keys": [at(30)]})).unwrap();
    s.execute_checked("keyframe.move", json!({"delta": 0.5})).unwrap();
    assert_eq!(s.state.selected_keys, vec![key(layer, prop, 45)]);
    s.execute_checked("edit.undo", json!({})).unwrap();
    assert_eq!(s.state.selected_keys, vec![key(layer, prop, 30)], "the selection follows the key back");
    s.execute_checked("keyframe.select", json!({"keys": [at(30)], "add": true})).unwrap();
    assert_eq!(s.state.selected_keys, vec![key(layer, prop, 30)]);
    let info = s.execute_checked("keyframe.info", json!({})).unwrap();
    assert_eq!(info.as_array().map(Vec::len), Some(1), "{info}");
    s.execute_checked("keyframe.select", json!({"keys": [at(30), at(30), at(60)]})).unwrap();
    assert_eq!(s.state.selected_keys, vec![key(layer, prop, 30), key(layer, prop, 60)], "a key given twice is selected once");
    // A stale reference (as an undo left one before) never stands for its neighbour.
    s.state.selected_keys = vec![key(layer, prop, 45), key(layer, prop, 30)];
    assert_eq!(s.execute_checked("keyframe.info", json!({})).unwrap().as_array().map(Vec::len), Some(1));
    let refused = s.execute_checked("keyframe.wiggle", json!({"frequency": 5, "magnitude": 10})).unwrap_err().to_string();
    assert!(refused.contains("select at least 2 keyframes"), "{refused}");
    // Redo moves the key and its selection again.
    s.execute_checked("keyframe.select", json!({"keys": [at(30)]})).unwrap();
    s.execute_checked("edit.redo", json!({})).unwrap();
    assert_eq!(s.state.selected_keys, vec![key(layer, prop, 45)]);
}

/// Wiggler and Smoother say whether they changed anything (the editor says why when not).
#[test]
fn wiggler_answers_whether_it_changed_the_keys() {
    let (mut s, layer) = setup();
    for (time, value) in [(0.0, 0), (1.0 / 30.0, 50)] {
        s.execute_checked("prop.addKey", json!({"layer": layer, "path": "transform/opacity", "time": time, "value": value})).unwrap();
    }
    let prop = uid(&s, layer, "transform/opacity");
    s.execute_checked("keyframe.select", json!({"keys": [{"layer": layer, "prop": prop, "time": 0.0}, {"layer": layer, "prop": prop, "time": 1.0 / 30.0}]}))
        .unwrap();
    let answer = s.execute_checked("keyframe.wiggle", json!({"frequency": 5, "magnitude": 10})).unwrap();
    assert_eq!(answer["changed"], json!(false), "keys one frame apart leave no room: {answer}");
}
