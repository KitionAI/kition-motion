//! The tools driven as the editor drives them: pointer events in comp pixels, the result read
//! back through the engine's own queries.

use kition_kit::KitDocument;
use kition_kit::tools::{KitTools, PointerAction, ToolMods, ToolPointer};
use serde_json::{Value, json};

use crate::MotionDocument;

fn doc() -> MotionDocument {
    let mut doc = MotionDocument::create("tools.kmotion", 1920, 1080).unwrap();
    // Snapping would pull the exact offsets below onto the comp's centre lines.
    doc.execute("view.snapping", json!({ "value": false })).unwrap();
    doc
}

/// A 200 × 100 solid centred at (960, 540): its box is (860, 490)–(1060, 590).
fn solid(doc: &mut MotionDocument, name: &str) -> u64 {
    doc.execute("layer.newSolid", json!({ "name": name, "width": 200, "height": 100, "color": "#ff0000" })).unwrap()["layer"].as_u64().unwrap()
}

fn event(action: PointerAction, x: f64, y: f64, mods: ToolMods) -> ToolPointer {
    ToolPointer { action, x, y, mods, pressure: 1.0, zoom: 1.0 }
}

fn click(doc: &mut MotionDocument, x: f64, y: f64, mods: ToolMods) {
    doc.pointer(&event(PointerAction::Down, x, y, mods)).unwrap();
    doc.pointer(&event(PointerAction::Up, x, y, mods)).unwrap();
}

/// A press at `from` (Shift only if `mods` holds it from the start: a Shift press toggles the
/// selection), dragged to `to` with `mods` held.
fn drag(doc: &mut MotionDocument, from: [f64; 2], to: [f64; 2], mods: ToolMods) {
    drag_from(doc, from, to, ToolMods { shift: false, ..mods }, mods);
}

fn drag_from(doc: &mut MotionDocument, from: [f64; 2], to: [f64; 2], press: ToolMods, mods: ToolMods) {
    doc.pointer(&event(PointerAction::Down, from[0], from[1], press)).unwrap();
    for i in 1..=8 {
        let t = f64::from(i) / 8.0;
        doc.pointer(&event(PointerAction::Drag, from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t, mods)).unwrap();
    }
    doc.pointer(&event(PointerAction::Up, to[0], to[1], mods)).unwrap();
}

fn value(doc: &mut MotionDocument, layer: u64, path: &str) -> Vec<f64> {
    let v = doc.execute("prop.get", json!({ "layer": layer, "path": path })).unwrap()["value"].clone();
    match v {
        Value::Array(a) => a.iter().filter_map(Value::as_f64).collect(),
        Value::Number(n) => vec![n.as_f64().unwrap()],
        other => panic!("not a number: {other}"),
    }
}

fn selected(doc: &MotionDocument) -> Vec<u64> {
    doc.session.state.selected_layers.iter().map(|l| l.0).collect()
}

fn near(a: &[f64], b: &[f64]) -> bool {
    a.len() >= b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
}

const SHIFT: ToolMods = ToolMods { shift: true, alt: false, command: false, control: false, space: false };

#[test]
fn tools_click_selects_the_topmost_layer_shift_click_toggles_and_empty_canvas_deselects() {
    let mut doc = doc();
    let bottom = solid(&mut doc, "Bottom");
    let top = solid(&mut doc, "Top");
    doc.execute("prop.set", json!({ "layer": bottom, "path": "transform/position", "value": [400, 300, 0] })).unwrap();
    assert_eq!(doc.tool(), "selection", "Selection is the tool a document opens with");
    click(&mut doc, 960.0, 540.0, ToolMods::default());
    assert_eq!(selected(&doc), vec![top]);
    click(&mut doc, 400.0, 300.0, SHIFT);
    assert_eq!(selected(&doc).len(), 2);
    click(&mut doc, 400.0, 300.0, SHIFT);
    assert_eq!(selected(&doc), vec![top], "Shift-click again takes it out");
    click(&mut doc, 50.0, 1000.0, ToolMods::default());
    assert!(selected(&doc).is_empty());
    // A locked layer is not picked; the one under it is.
    doc.execute("prop.set", json!({ "layer": bottom, "path": "transform/position", "value": [960, 540, 0] })).unwrap();
    doc.execute("layer.setSwitch", json!({ "layers": [top], "switch": "lock" })).unwrap();
    click(&mut doc, 960.0, 540.0, ToolMods::default());
    assert_eq!(selected(&doc), vec![bottom]);
}

#[test]
fn tools_marquee_selects_the_layers_it_touches() {
    let mut doc = doc();
    let a = solid(&mut doc, "A");
    let b = solid(&mut doc, "B");
    doc.execute("prop.set", json!({ "layer": a, "path": "transform/position", "value": [300, 300, 0] })).unwrap();
    drag(&mut doc, [10.0, 10.0], [320.0, 320.0], ToolMods::default());
    assert_eq!(selected(&doc), vec![a]);
    drag(&mut doc, [10.0, 10.0], [1000.0, 520.0], ToolMods::default());
    // With Shift a marquee adds to the selection.
    click(&mut doc, 300.0, 300.0, ToolMods::default());
    drag_from(&mut doc, [1500.0, 1000.0], [1000.0, 560.0], SHIFT, SHIFT);
    assert_eq!(selected(&doc).len(), 2);
    let mut both = selected(&doc);
    both.sort_unstable();
    assert_eq!(both, {
        let mut v = vec![a, b];
        v.sort_unstable();
        v
    });
}

#[test]
fn tools_drag_moves_by_comp_pixels_in_one_undo_step() {
    let mut doc = doc();
    let layer = solid(&mut doc, "Solid");
    let steps = doc.session.history.undo.len();
    drag(&mut doc, [960.0, 540.0], [1010.0, 520.0], ToolMods::default());
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1010.0, 520.0]));
    assert_eq!(doc.session.history.undo.len(), steps + 1, "one undo step");
    // Shift holds it to the longer axis.
    drag(&mut doc, [1010.0, 520.0], [1110.0, 530.0], SHIFT);
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1110.0, 520.0]));
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1010.0, 520.0]));
}

#[test]
fn tools_moving_an_animated_position_keys_it_at_the_current_time() {
    let mut doc = doc();
    let layer = solid(&mut doc, "Solid");
    doc.execute("prop.toggleAnimation", json!({ "layer": layer, "path": "transform/position", "value": true })).unwrap();
    doc.execute("time.set", json!({ "time": 1.0 })).unwrap();
    click(&mut doc, 960.0, 540.0, ToolMods::default());
    drag(&mut doc, [960.0, 540.0], [1000.0, 540.0], ToolMods::default());
    let keys = doc.execute("prop.get", json!({ "layer": layer, "path": "transform/position" })).unwrap()["keys"].as_array().map_or(0, Vec::len);
    assert_eq!(keys, 2, "a key at 0 s and one at 1 s");
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1000.0, 540.0]));
}

#[test]
fn tools_handles_scale_and_the_corner_zone_rotates() {
    let mut doc = doc();
    let layer = solid(&mut doc, "Solid");
    click(&mut doc, 960.0, 540.0, ToolMods::default());
    // The bottom right corner, dragged 100 right only: each axis on its own, the height unchanged.
    drag(&mut doc, [1060.0, 590.0], [1160.0, 590.0], ToolMods::default());
    assert!(near(&value(&mut doc, layer, "transform/scale"), &[150.0, 100.0]), "{:?}", value(&mut doc, layer, "transform/scale"));
    doc.execute("edit.undo", json!({})).unwrap();
    // With Shift, the same corner dragged 100 right and 50 down: proportional, the top left corner stays.
    drag(&mut doc, [1060.0, 590.0], [1160.0, 640.0], SHIFT);
    assert!(near(&value(&mut doc, layer, "transform/scale"), &[150.0, 150.0]), "{:?}", value(&mut doc, layer, "transform/scale"));
    // Position moves so (860, 490) stays: the anchor (100, 50) at 150 % is 150, 75 from it.
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1010.0, 565.0]), "{:?}", value(&mut doc, layer, "transform/position"));
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&value(&mut doc, layer, "transform/scale"), &[100.0, 100.0]));
    // The right side's middle: width only.
    drag(&mut doc, [1060.0, 540.0], [1160.0, 560.0], ToolMods::default());
    assert!(near(&value(&mut doc, layer, "transform/scale"), &[150.0, 100.0]));
    doc.execute("edit.undo", json!({})).unwrap();
    // Just outside the top right corner, a quarter turn about the anchor point (960, 540).
    drag(&mut doc, [1070.0, 480.0], [1020.0, 650.0], SHIFT);
    let r = value(&mut doc, layer, "transform/rotation")[0];
    assert!((r - 90.0).abs() < 1e-6 || (r - 105.0).abs() < 1e-6 || (r - 75.0).abs() < 1e-6, "snapped to 15°: {r}");
    assert_eq!(r % 15.0, 0.0);
}

#[test]
fn tools_a_parented_layer_moves_by_screen_pixels() {
    let mut doc = doc();
    let parent = solid(&mut doc, "Parent");
    let child = solid(&mut doc, "Child");
    doc.execute("prop.set", json!({ "layer": parent, "path": "transform/scale", "value": [50, 50, 100] })).unwrap();
    doc.execute("layer.setParent", json!({ "layers": [child], "parent": parent })).unwrap();
    let p = value(&mut doc, child, "transform/position");
    doc.execute("layer.select", json!({ "layers": [child] })).unwrap();
    let centre = crate::tools::selected_boxes(&doc.session).first().map(|b| b.to_comp([100.0, 50.0])).unwrap();
    drag(&mut doc, centre, [centre[0] + 10.0, centre[1]], ToolMods::default());
    let q = value(&mut doc, child, "transform/position");
    assert!((q[0] - p[0] - 20.0).abs() < 1e-6, "10 screen pixels are 20 in a parent at 50 %: {p:?} → {q:?}");
}

#[test]
fn tools_rectangle_and_ellipse_draw_shape_layers_and_return_to_selection() {
    let mut doc = doc();
    doc.select_tool("rectangle", 1.0).unwrap();
    let layers = doc.session.active_comp().map_or(0, |c| c.layers.len());
    doc.pointer(&event(PointerAction::Down, 100.0, 100.0, ToolMods::default())).unwrap();
    doc.pointer(&event(PointerAction::Drag, 300.0, 200.0, ToolMods::default())).unwrap();
    let answer = doc.pointer(&event(PointerAction::Up, 300.0, 200.0, ToolMods::default())).unwrap();
    assert!(answer.requests.iter().any(|r| matches!(r, kition_kit::tools::ToolRequest::SwitchTool { id } if id == "selection")));
    assert_eq!(doc.tool(), "selection");
    assert_eq!(doc.session.active_comp().map_or(0, |c| c.layers.len()), layers + 1);
    let id = selected(&doc)[0];
    let b = crate::tools::selected_boxes(&doc.session);
    let e = b.first().unwrap().extent();
    assert!((e[0] - 100.0).abs() < 1.0 && (e[1] - 100.0).abs() < 1.0 && (e[2] - 300.0).abs() < 1.0 && (e[3] - 200.0).abs() < 1.0, "{e:?} of layer {id}");
    doc.select_tool("ellipse", 1.0).unwrap();
    drag_from(&mut doc, [500.0, 500.0], [600.0, 560.0], SHIFT, SHIFT);
    let e = crate::tools::selected_boxes(&doc.session).first().unwrap().extent();
    assert!((e[2] - e[0] - 100.0).abs() < 1.0 && (e[3] - e[1] - 100.0).abs() < 1.0, "Shift draws a circle: {e:?}");
}

#[test]
fn tools_text_tool_types_a_new_layer_and_edits_an_existing_one() {
    let mut doc = doc();
    doc.select_tool("text", 1.0).unwrap();
    click(&mut doc, 200.0, 300.0, ToolMods::default());
    let answer = KitTools::text(&mut doc, "Hi", 1.0).unwrap();
    assert!(answer.wants_text);
    KitTools::text(&mut doc, "你好", 1.0).unwrap();
    doc.key("Backspace", ToolMods::default(), 1.0).unwrap();
    doc.key("Enter", ToolMods::default(), 1.0).unwrap();
    KitTools::text(&mut doc, "x", 1.0).unwrap();
    let id = selected(&doc)[0];
    let text = doc.execute("prop.get", json!({ "layer": id, "path": "text/sourceText" })).unwrap()["value"].clone();
    let text = text.get("text").and_then(Value::as_str).map(str::to_owned).or_else(|| text.as_str().map(str::to_owned)).unwrap();
    assert_eq!(text, "Hi你\nx");
    let answer = doc.key("Escape", ToolMods::default(), 1.0).unwrap();
    assert!(!answer.wants_text);
    // The Selection tool's double-click edits it with all of it selected; typing replaces it.
    doc.select_tool("selection", 1.0).unwrap();
    let b = crate::tools::selected_boxes(&doc.session);
    let c = b.first().map(|b| b.to_comp([(b.bounds[0] + b.bounds[2]) / 2.0, (b.bounds[1] + b.bounds[3]) / 2.0])).unwrap();
    doc.pointer(&event(PointerAction::DoubleClick, c[0], c[1], ToolMods::default())).unwrap();
    KitTools::text(&mut doc, "New", 1.0).unwrap();
    let text = doc.execute("prop.get", json!({ "layer": id, "path": "text/sourceText" })).unwrap()["value"].clone();
    assert_eq!(text.get("text").and_then(Value::as_str).or_else(|| text.as_str()), Some("New"));
}

#[test]
fn tools_refuse_hostile_events_without_a_panic() {
    let mut doc = doc();
    assert!(doc.pointer(&event(PointerAction::Down, f64::NAN, 1.0, ToolMods::default())).is_err());
    assert!(doc.select_tool("pen", 1.0).is_err());
    let _ = doc.pointer(&ToolPointer { zoom: 0.0, ..event(PointerAction::Move, 1e300, -1e300, ToolMods::default()) }).unwrap();
    let _ = doc.state(f64::INFINITY, 0.0, ToolMods::default(), f64::NAN);
    let _ = doc.key("Escape", ToolMods::default(), -1.0).unwrap();
    let _ = KitTools::text(&mut doc, "x", 1.0).unwrap();
}

#[test]
fn a_double_click_on_a_precomp_layer_opens_its_composition() {
    let mut doc = doc();
    let outer = doc.session.active_comp_id().unwrap();
    let layer = solid(&mut doc, "Inside");
    let made = doc.execute("layer.precompose", json!({ "layers": [layer], "name": "Nested" })).unwrap();
    assert_eq!(doc.session.active_comp_id(), Some(outer), "pre-composing stays in the outer composition");
    // Outside the precomp layer's box (the nested composition's frame) nothing opens; on it, its
    // composition does.
    doc.pointer(&event(PointerAction::DoubleClick, -40.0, -40.0, ToolMods::default())).unwrap();
    assert_eq!(doc.session.active_comp_id(), Some(outer));
    doc.pointer(&event(PointerAction::DoubleClick, 960.0, 540.0, ToolMods::default())).unwrap();
    assert_eq!(doc.session.active_comp_id().map(|c| c.0), made["comp"].as_u64());
}
