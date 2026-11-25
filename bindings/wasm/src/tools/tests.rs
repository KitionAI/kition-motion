//! The tools driven as the editor drives them: pointer events in comp pixels, the result read
//! back through the engine's own queries.

use kition_kit::KitDocument;
use kition_kit::tools::{KitTools, PointerAction, ToolMods, ToolPointer, ToolRequest};
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
    // An id no tool has (the Pen is one now).
    assert!(doc.select_tool("lasso", 1.0).is_err());
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

#[test]
fn tools_a_moved_layer_snaps_to_a_guide_and_to_the_grid_while_they_are_on() {
    let mut doc = doc();
    let layer = solid(&mut doc, "Solid");
    // A vertical guide at x = 1203: the box's right edge (1060) moved 140 px lands 3 px short of it.
    doc.execute("view.addGuide", json!({ "orientation": "vertical", "position": 1203 })).unwrap();
    let guides = doc.execute("comp.info", json!({})).unwrap()["guides"].clone();
    assert_eq!(guides, json!([{ "orientation": "vertical", "position": 1203.0 }]));
    doc.set_snap_targets(false, false);
    drag(&mut doc, [960.0, 540.0], [1100.0, 540.0], ToolMods::default());
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1100.0, 540.0]), "no snapping: where the pointer went");
    doc.execute("edit.undo", json!({})).unwrap();
    doc.set_snap_targets(true, false);
    drag(&mut doc, [960.0, 540.0], [1100.0, 540.0], ToolMods::default());
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1103.0, 540.0]), "the right edge snaps onto the guide though layer snapping is off");
    doc.execute("edit.undo", json!({})).unwrap();
    // The grid (every 100 px): the left edge (860) moved 137 px is 3 px short of the line at 1000.
    let spacing = doc.set_snap_targets(false, true);
    assert_eq!(spacing, 100.0);
    drag(&mut doc, [960.0, 540.0], [1097.0, 540.0], ToolMods::default());
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1100.0, 540.0]), "the left edge snaps onto the grid line at 1000");
    doc.execute("edit.undo", json!({})).unwrap();
    // Cmd or Ctrl held turns guide and grid snapping off for the drag.
    let command = ToolMods { command: true, ..ToolMods::default() };
    drag(&mut doc, [960.0, 540.0], [1097.0, 540.0], command);
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1097.0, 540.0]), "Cmd held: no snapping to the grid");
    doc.execute("edit.undo", json!({})).unwrap();
    doc.set_snap_targets(true, false);
    drag(&mut doc, [960.0, 540.0], [1100.0, 540.0], command);
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1100.0, 540.0]), "Cmd held: no snapping to the guide");
    doc.execute("edit.undo", json!({})).unwrap();
    // View ▸ Grid Settings: a 50 px grid, kept within bounds.
    assert_eq!(doc.set_grid_spacing(50.0), 50.0);
    assert_eq!(doc.set_grid_spacing(f64::NAN), 50.0);
    assert_eq!(doc.set_grid_spacing(0.0), 2.0);
    assert_eq!(doc.set_grid_spacing(50.0), 50.0);
    doc.set_snap_targets(false, true);
    // The left edge (860) moved 87 px is 3 px short of the line at 950.
    drag(&mut doc, [960.0, 540.0], [1047.0, 540.0], ToolMods::default());
    assert!(near(&value(&mut doc, layer, "transform/position"), &[1050.0, 540.0]), "the left edge snaps onto the 50 px grid line at 950");
}

/// The masks of a layer: per mask its vertices, its out tangents and whether it is closed.
fn masks_of(doc: &mut MotionDocument, layer: u64) -> Vec<(Vec<[f64; 2]>, Vec<[f64; 2]>, bool)> {
    let tree = doc.execute("layer.tree", json!({ "layer": layer })).unwrap();
    let masks = tree["properties"]["children"].as_array().unwrap().iter().find(|g| g["match"] == "masks").cloned().unwrap_or(json!({}));
    let pts = |v: &Value| v.as_array().map(|a| a.iter().map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()]).collect()).unwrap_or_default();
    masks["children"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|mask| {
                    let value = mask["children"].as_array().unwrap().iter().find(|p| p["match"] == "path").unwrap()["value"].clone();
                    let path = value["v"].clone();
                    assert!(path.get("vertices").is_some(), "a path: {path}");
                    (pts(&path["vertices"]), pts(&path["out_tangents"]), path["closed"].as_bool().unwrap_or(false))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn key(doc: &mut MotionDocument, name: &str) {
    doc.key(name, ToolMods::default(), 1.0).unwrap();
}

#[test]
fn tools_the_pen_draws_a_closed_mask_on_the_selected_layer_in_one_undo_step() {
    let mut doc = doc();
    let layer = solid(&mut doc, "Masked");
    click(&mut doc, 960.0, 540.0, ToolMods::default());
    assert_eq!(selected(&doc), vec![layer]);
    doc.select_tool("pen", 1.0).unwrap();
    assert_eq!(doc.tool(), "pen");
    // Three corners, a drag that curves the fourth point, and a click on the first point closes it.
    click(&mut doc, 880.0, 500.0, ToolMods::default());
    click(&mut doc, 1040.0, 500.0, ToolMods::default());
    click(&mut doc, 1040.0, 580.0, ToolMods::default());
    drag(&mut doc, [880.0, 580.0], [900.0, 600.0], ToolMods::default());
    assert!(masks_of(&mut doc, layer).is_empty(), "nothing is made until the path is finished");
    click(&mut doc, 881.0, 501.0, ToolMods::default());
    let masks = masks_of(&mut doc, layer);
    assert_eq!(masks.len(), 1, "{masks:?}");
    let (vertices, outs, closed) = &masks[0];
    assert!(closed);
    // The solid's layer space starts at its top left (860, 490).
    assert!(near(&vertices[0], &[20.0, 10.0]) && near(&vertices[2], &[180.0, 90.0]), "{vertices:?}");
    assert!(near(&outs[3], &[20.0, 20.0]) && near(&outs[0], &[0.0, 0.0]), "the dragged point has handles, the corners none: {outs:?}");
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(masks_of(&mut doc, layer).is_empty(), "one undo takes the mask away");
}

#[test]
fn tools_the_pen_keeps_an_open_path_on_enter_drops_it_on_escape_and_a_vertex_drags() {
    let mut doc = doc();
    let layer = solid(&mut doc, "Masked");
    click(&mut doc, 960.0, 540.0, ToolMods::default());
    doc.select_tool("pen", 1.0).unwrap();
    click(&mut doc, 880.0, 500.0, ToolMods::default());
    click(&mut doc, 1000.0, 520.0, ToolMods::default());
    key(&mut doc, "Escape");
    assert!(masks_of(&mut doc, layer).is_empty(), "Escape drops the drawing");
    click(&mut doc, 880.0, 500.0, ToolMods::default());
    click(&mut doc, 1000.0, 520.0, ToolMods::default());
    click(&mut doc, 1020.0, 560.0, ToolMods::default());
    key(&mut doc, "Backspace");
    key(&mut doc, "Enter");
    let masks = masks_of(&mut doc, layer);
    assert_eq!(masks.len(), 1);
    assert!(!masks[0].2, "Enter keeps it open");
    assert_eq!(masks[0].0.len(), 2, "Backspace took the last point back");
    // With the Selection tool the mask's vertex drags (and the layer stays put).
    doc.select_tool("selection", 1.0).unwrap();
    let position = value(&mut doc, layer, "transform/position");
    drag(&mut doc, [1000.0, 520.0], [1010.0, 540.0], ToolMods::default());
    let moved = masks_of(&mut doc, layer);
    assert!(near(&moved[0].0[1], &[150.0, 50.0]), "{:?}", moved[0].0);
    assert!(near(&value(&mut doc, layer, "transform/position"), &position), "the layer did not move");
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&masks_of(&mut doc, layer)[0].0[1], &[140.0, 30.0]), "one undo puts the vertex back");
}

#[test]
fn tools_the_pen_with_no_layer_selected_draws_a_new_shape_layer_in_one_undo_step() {
    let mut doc = doc();
    let below = solid(&mut doc, "Below");
    doc.execute("edit.deselectAll", json!({})).unwrap();
    assert!(selected(&doc).is_empty());
    let before = doc.execute("comp.info", json!({})).unwrap()["layers"].as_array().unwrap().len();
    doc.select_tool("pen", 1.0).unwrap();
    // Over the solid: the Pen does not take the layer under the pointer.
    click(&mut doc, 900.0, 500.0, ToolMods::default());
    click(&mut doc, 1000.0, 500.0, ToolMods::default());
    click(&mut doc, 1000.0, 580.0, ToolMods::default());
    click(&mut doc, 901.0, 501.0, ToolMods::default());
    let info = doc.execute("comp.info", json!({})).unwrap();
    let layers = info["layers"].as_array().unwrap();
    assert_eq!(layers.len(), before + 1, "a new layer: {layers:?}");
    assert_eq!(layers[0]["type"], "Shape", "{:?}", layers[0]);
    assert!(masks_of(&mut doc, below).is_empty(), "no mask on the solid under the pointer");
    let made = layers[0]["id"].as_u64().unwrap();
    assert_eq!(selected(&doc), vec![made], "the new shape layer is selected");
    doc.execute("edit.undo", json!({})).unwrap();
    assert_eq!(doc.execute("comp.info", json!({})).unwrap()["layers"].as_array().unwrap().len(), before, "one undo takes it away");
}

#[test]
fn tools_the_pen_on_a_shape_layer_draws_a_mask_when_its_option_says_so() {
    let mut doc = doc();
    let shape = doc.execute("layer.newShape", json!({"kind": "rect"})).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.select", json!({"layers": [shape]})).unwrap();
    // Path items (groups whose match is `path`), not a mask's Mask Path property.
    fn items(node: &Value) -> usize {
        let own = usize::from(node["match"] == "path" && node["children"].is_array());
        own + node["children"].as_array().map_or(0, |children| children.iter().map(items).sum())
    }
    let paths = |doc: &mut MotionDocument| items(&doc.execute("layer.tree", json!({"layer": shape, "depth": 8})).unwrap()["properties"]);
    let before = paths(&mut doc);
    doc.set_pen_masks(true);
    doc.select_tool("pen", 1.0).unwrap();
    for [x, y] in [[600.0, 300.0], [700.0, 300.0], [700.0, 400.0], [601.0, 301.0]] {
        click(&mut doc, x, y, ToolMods::default());
    }
    assert_eq!(masks_of(&mut doc, shape).len(), 1, "a mask on the shape layer");
    assert_eq!(paths(&mut doc), before, "no new shape path");
    doc.set_pen_masks(false);
    // Clear of the mask's own vertices and segments (a press there edits the mask).
    for [x, y] in [[300.0, 500.0], [400.0, 500.0], [400.0, 600.0], [301.0, 501.0]] {
        click(&mut doc, x, y, ToolMods::default());
    }
    assert_eq!(masks_of(&mut doc, shape).len(), 1, "no second mask");
    assert!(paths(&mut doc) > before, "a new shape path instead");
}

/// The first shape Path item of a layer: its uid, vertices and in tangents (its own space).
fn shape_path_of(doc: &mut MotionDocument, layer: u64) -> (u64, Vec<[f64; 2]>, Vec<[f64; 2]>) {
    fn find(node: &Value) -> Option<Value> {
        if node["match"] == "path" && node["children"].is_array() {
            return Some(node.clone());
        }
        node["children"].as_array()?.iter().find_map(find)
    }
    let tree = doc.execute("layer.tree", json!({ "layer": layer })).unwrap();
    let item = find(&tree["properties"]).expect("a shape path");
    let value = item["children"].as_array().unwrap().iter().find(|p| p["match"] == "path").unwrap()["value"]["v"].clone();
    let pts = |v: &Value| v.as_array().map(|a| a.iter().map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()]).collect()).unwrap_or_default();
    (item["uid"].as_u64().unwrap(), pts(&value["vertices"]), pts(&value["in_tangents"]))
}

#[test]
fn tools_shape_path_vertices_select_move_convert_add_and_delete_in_one_undo_step_each() {
    let mut doc = doc();
    doc.select_tool("pen", 1.0).unwrap();
    // A closed square path drawn with nothing selected: a new shape layer.
    click(&mut doc, 800.0, 400.0, ToolMods::default());
    click(&mut doc, 1000.0, 400.0, ToolMods::default());
    click(&mut doc, 1000.0, 600.0, ToolMods::default());
    click(&mut doc, 800.0, 600.0, ToolMods::default());
    click(&mut doc, 801.0, 401.0, ToolMods::default());
    let layer = selected(&doc)[0];
    let (_, start, _) = shape_path_of(&mut doc, layer);
    assert_eq!(start.len(), 4, "{start:?}");
    doc.select_tool("selection", 1.0).unwrap();
    // Click the first vertex, Shift-click the second, then drag one of them: both move.
    click(&mut doc, 800.0, 400.0, ToolMods::default());
    click(&mut doc, 1000.0, 400.0, ToolMods { shift: true, ..ToolMods::default() });
    assert_eq!(doc.session.state.selected_vertices.len(), 2);
    drag_from(&mut doc, [1000.0, 400.0], [1010.0, 380.0], ToolMods::default(), ToolMods::default());
    let (_, moved, _) = shape_path_of(&mut doc, layer);
    for i in [0, 1] {
        assert!(near(&moved[i], &[start[i][0] + 10.0, start[i][1] - 20.0]), "vertex {i}: {moved:?}");
    }
    assert!(near(&moved[2], &start[2]), "an unselected vertex stays");
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&shape_path_of(&mut doc, layer).1[0], &start[0]), "one undo puts both back");
    // Alt-click: the corner becomes smooth (curve handles), and again a corner.
    click(&mut doc, 1000.0, 600.0, ToolMods { alt: true, ..ToolMods::default() });
    let (_, _, ins) = shape_path_of(&mut doc, layer);
    assert!(ins[2][0].abs() + ins[2][1].abs() > 1.0, "smooth: {ins:?}");
    doc.execute("edit.undo", json!({})).unwrap();
    // A press on a segment moves the whole path.
    drag_from(&mut doc, [900.0, 600.0], [930.0, 600.0], ToolMods::default(), ToolMods::default());
    let (_, all, _) = shape_path_of(&mut doc, layer);
    assert!((0..4).all(|i| near(&all[i], &[start[i][0] + 30.0, start[i][1]])), "{all:?}");
    doc.execute("edit.undo", json!({})).unwrap();
    // The Pen on a segment adds a vertex there.
    doc.select_tool("pen", 1.0).unwrap();
    click(&mut doc, 1000.0, 500.0, ToolMods::default());
    assert_eq!(shape_path_of(&mut doc, layer).1.len(), 5, "a vertex added on the right side");
    doc.select_tool("selection", 1.0).unwrap();
    // A box over the two bottom vertices (vertices are selected, so empty space boxes them), then Delete.
    drag_from(&mut doc, [700.0, 560.0], [1100.0, 700.0], ToolMods::default(), ToolMods::default());
    assert_eq!(doc.session.state.selected_vertices.len(), 2, "{:?}", doc.session.state.selected_vertices);
    doc.execute("edit.clear", json!({})).unwrap();
    assert_eq!(shape_path_of(&mut doc, layer).1.len(), 3, "the two boxed vertices are gone, the layer stays");
    doc.execute("edit.undo", json!({})).unwrap();
    assert_eq!(shape_path_of(&mut doc, layer).1.len(), 5);
}

#[test]
fn tools_a_gradient_fills_start_point_drags_on_the_canvas_in_one_undo_step() {
    let mut doc = doc();
    let made = doc.execute("shape.newShape", json!({ "kind": "rect", "size": [400, 200], "position": [960, 540] })).unwrap();
    let layer = made["layer"].as_u64().unwrap();
    let group = made["group"].as_u64().unwrap();
    let added = doc.execute("layer.addShapeItem", json!({ "layer": layer, "kind": "gfill", "group": group })).unwrap();
    let path = format!("{}/start", added["path"].as_str().unwrap());
    doc.execute("layer.select", json!({ "layers": [layer] })).unwrap();
    let points = super::gradient::points(&doc.session);
    assert_eq!(points.len(), 1);
    let from = points[0].0.at;
    let before = value(&mut doc, layer, &path);
    drag(&mut doc, from, [from[0] + 30.0, from[1] + 10.0], ToolMods::default());
    let after = value(&mut doc, layer, &path);
    assert!(near(&after, &[before[0] + 30.0, before[1] + 10.0]), "{before:?} → {after:?}");
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&value(&mut doc, layer, &path), &before), "one undo puts it back");
}

#[test]
fn tools_path_vertices_look_unlike_the_layers_handles_which_hide_while_vertices_are_edited() {
    use kition_kit::tools::ToolOverlay;
    let mut doc = doc();
    doc.select_tool("pen", 1.0).unwrap();
    for [x, y] in [[800.0, 400.0], [1000.0, 400.0], [1000.0, 600.0], [800.0, 600.0], [801.0, 401.0]] {
        click(&mut doc, x, y, ToolMods::default());
    }
    doc.select_tool("selection", 1.0).unwrap();
    doc.execute("mask.selectVertices", json!({ "vertices": [] })).unwrap();
    let box_handles = |doc: &mut MotionDocument| {
        let answer = doc.state(0.0, 0.0, ToolMods::default(), 1.0);
        let handles = answer.overlays.iter().filter(|o| matches!(o, ToolOverlay::Anchor { size, .. } if *size >= 7.0)).count();
        let vertices: Vec<[u8; 4]> = answer
            .overlays
            .iter()
            .filter_map(|o| if let ToolOverlay::Anchor { size, color, .. } = o { (*size < 7.0).then_some(*color) } else { None })
            .collect();
        (handles, vertices)
    };
    let (handles, vertices) = box_handles(&mut doc);
    assert_eq!(handles, 8, "the layer's eight scale handles while no vertex is picked");
    assert_eq!(vertices.len(), 4);
    assert!(vertices.iter().all(|c| *c != super::SELECTION), "vertices are not drawn in the handles' colour");
    click(&mut doc, 800.0, 400.0, ToolMods::default());
    assert_eq!(doc.session.state.selected_vertices.len(), 1);
    assert_eq!(box_handles(&mut doc).0, 0, "no scale handles while a vertex is picked");
}

#[test]
fn tools_orbit_turns_gently_and_with_shift_about_one_axis_only() {
    let mut doc = doc();
    let layer = solid(&mut doc, "Card");
    doc.execute("layer.setSwitch", json!({ "layers": [layer], "switch": "threeD", "value": true })).unwrap();
    let camera = doc.execute("layer.newCamera", json!({})).unwrap()["layer"].as_u64().unwrap();
    let eye = |doc: &mut MotionDocument| value(doc, camera, "transform/position");
    let poi = value(&mut doc, camera, "transform/poi");
    let start = eye(&mut doc);
    doc.select_tool("orbit", 1.0).unwrap();
    // A 150-pixel drag across turns the camera about 24° around its point of interest (not to its side).
    drag(&mut doc, [800.0, 500.0], [950.0, 500.0], ToolMods::default());
    let turned = eye(&mut doc);
    let angle = |a: &[f64]| (a[0] - poi[0]).atan2(-(a[2] - poi[2])).to_degrees();
    let yaw = (angle(&turned) - angle(&start)).abs();
    assert!((15.0..35.0).contains(&yaw), "a 150-pixel drag turned the camera {yaw}°: {start:?} → {turned:?}");
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&eye(&mut doc), &start));
    // With Shift, a drag mostly across with a little up and down turns about the vertical axis
    // only: the camera stays at its height.
    drag(&mut doc, [800.0, 500.0], [950.0, 530.0], ToolMods { shift: true, ..ToolMods::default() });
    let level = eye(&mut doc);
    assert!((level[1] - start[1]).abs() < 1e-6, "a Shift-drag across moved the camera up or down: {start:?} → {level:?}");
    assert!(!near(&level, &start), "the Shift-drag did not turn the camera");
}

#[test]
fn tools_a_box_corner_on_a_vertex_scales_when_dragged_and_a_double_click_edits_the_path_until_escape() {
    use kition_kit::tools::ToolOverlay;
    let mut doc = doc();
    doc.select_tool("pen", 1.0).unwrap();
    for [x, y] in [[800.0, 400.0], [1000.0, 400.0], [1000.0, 600.0], [800.0, 600.0], [801.0, 401.0]] {
        click(&mut doc, x, y, ToolMods::default());
    }
    let layer = selected(&doc)[0];
    doc.select_tool("selection", 1.0).unwrap();
    doc.execute("mask.selectVertices", json!({ "vertices": [] })).unwrap();
    let handles = |doc: &mut MotionDocument| {
        doc.state(0.0, 0.0, ToolMods::default(), 1.0).overlays.iter().filter(|o| matches!(o, ToolOverlay::Anchor { size, .. } if *size >= 7.0)).count()
    };
    let (_, start, _) = shape_path_of(&mut doc, layer);
    let scale = value(&mut doc, layer, "transform/scale");
    // A drag from the top-left corner (a vertex too) scales the layer; the path keeps its vertices.
    drag(&mut doc, [800.0, 400.0], [760.0, 360.0], ToolMods::default());
    assert!(!near(&value(&mut doc, layer, "transform/scale"), &scale), "the corner drag did not scale the layer");
    assert!(near(&shape_path_of(&mut doc, layer).1[0], &start[0]), "the corner drag moved the vertex");
    assert!(doc.session.state.selected_vertices.is_empty());
    doc.execute("edit.undo", json!({})).unwrap();
    // A double-click on the path's edge edits it: the transform box hides, Escape brings it back.
    doc.pointer(&event(PointerAction::DoubleClick, 900.0, 600.0, ToolMods::default())).unwrap();
    assert_eq!(handles(&mut doc), 0, "no scale handles while the path is edited");
    KitTools::key(&mut doc, "Escape", ToolMods::default(), 1.0).unwrap();
    assert!(doc.session.state.selected_vertices.is_empty());
    assert_eq!(handles(&mut doc), 8, "Escape ends path editing: the box is back");
}

#[test]
fn tools_selection_picked_after_the_pen_shows_the_new_layers_box_not_its_vertices() {
    use kition_kit::tools::ToolOverlay;
    let mut doc = doc();
    doc.select_tool("pen", 1.0).unwrap();
    for [x, y] in [[800.0, 400.0], [1000.0, 400.0], [1000.0, 600.0], [801.0, 401.0]] {
        click(&mut doc, x, y, ToolMods::default());
    }
    doc.select_tool("selection", 1.0).unwrap();
    let handles =
        doc.state(0.0, 0.0, ToolMods::default(), 1.0).overlays.iter().filter(|o| matches!(o, ToolOverlay::Anchor { size, .. } if *size >= 7.0)).count();
    assert!(doc.session.state.selected_vertices.is_empty(), "V after the Pen picked vertices");
    assert_eq!(handles, 8, "V after the Pen shows the transform box");
}

#[test]
fn tools_in_a_3d_view_the_boxes_follow_the_view_and_2d_layers_are_not_picked() {
    let mut doc = doc();
    let flat = solid(&mut doc, "Flat");
    let card = solid(&mut doc, "Card");
    doc.execute("layer.setSwitch", json!({ "layers": [card], "switch": "threeD", "value": true })).unwrap();
    doc.execute("prop.set", json!({ "layer": card, "path": "transform/rotationY", "value": 60 })).unwrap();
    doc.execute("layer.select", json!({ "layers": [card] })).unwrap();
    let corners = |doc: &MotionDocument| super::selected_boxes(&doc.session).first().map(|b| b.corners());
    let camera = corners(&doc).unwrap();
    doc.execute("view.set3DView", json!({ "view": "top" })).unwrap();
    let top = corners(&doc).expect("the 3D layer still has a box in the Top view");
    assert!(camera.iter().zip(&top).any(|(a, b)| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() > 1.0), "the box did not follow the view: {camera:?} {top:?}");
    // The 2D layer is not in the Top view: a click where it covers the comp's middle picks nothing of it.
    doc.execute("edit.deselectAll", json!({})).unwrap();
    click(&mut doc, 960.0, 540.0, ToolMods::default());
    assert!(!selected(&doc).contains(&flat), "a 2D layer was picked in the Top view: {:?}", selected(&doc));
}

#[test]
fn tools_a_3d_view_shows_the_ground_the_3d_layers_and_the_camera_and_light() {
    use kition_kit::tools::ToolOverlay;
    let mut doc = doc();
    let card = solid(&mut doc, "Card");
    doc.execute("layer.setSwitch", json!({ "layers": [card], "switch": "threeD", "value": true })).unwrap();
    doc.execute("layer.newCamera", json!({})).unwrap();
    doc.execute("layer.newLight", json!({ "kind": "Spot" })).unwrap();
    let lines = |doc: &mut MotionDocument, color: [u8; 4]| {
        doc.state(0.0, 0.0, ToolMods::default(), 1.0).overlays.iter().filter(|o| matches!(o, ToolOverlay::Line { color: c, .. } if *c == color)).count()
    };
    assert_eq!(lines(&mut doc, super::view3d::CAMERA), 0, "the active camera's view draws no camera icon");
    doc.execute("view.set3DView", json!({ "view": "top" })).unwrap();
    assert!(lines(&mut doc, super::view3d::GRID) >= 10, "no ground grid in the Top view");
    assert!(lines(&mut doc, super::view3d::CAMERA) >= 8, "no camera icon and frustum in the Top view");
    assert!(lines(&mut doc, super::view3d::LIGHT) >= 5, "no light icon in the Top view");
    assert!(lines(&mut doc, super::view3d::OUTLINE) >= 4, "the 3D layer's outline is missing in the Top view");
}

#[test]
fn tools_the_text_selection_and_caret_follow_characters_moved_by_an_animator() {
    use kition_kit::tools::ToolOverlay;
    let mut doc = doc();
    doc.select_tool("text", 1.0).unwrap();
    click(&mut doc, 200.0, 300.0, ToolMods::default());
    KitTools::text(&mut doc, "ABCD", 1.0).unwrap();
    let id = selected(&doc)[0];
    // Select every character (Shift+Home from the end).
    doc.key("Home", SHIFT, 1.0).unwrap();
    let highlights = |doc: &mut MotionDocument| -> Vec<[[f64; 2]; 4]> {
        doc.state(0.0, 0.0, ToolMods::default(), 1.0)
            .overlays
            .iter()
            .filter_map(|o| if let ToolOverlay::Highlight { quad, .. } = o { Some(*quad) } else { None })
            .collect()
    };
    let still = highlights(&mut doc);
    assert!(!still.is_empty(), "no selection drawn");
    // An animator moves every character 80 px down: the highlight goes with them.
    doc.execute("layer.addTextAnimator", json!({ "layer": id, "properties": ["position"] })).unwrap();
    doc.execute("prop.set", json!({ "layer": id, "path": "text/animators/#1/properties/position", "value": [0, 80, 0] })).unwrap();
    let moved = highlights(&mut doc);
    let top = |quads: &[[[f64; 2]; 4]]| quads.iter().flat_map(|q| q.iter().map(|p| p[1])).fold(f64::INFINITY, f64::min);
    assert!((top(&moved) - top(&still) - 80.0).abs() < 1.0, "the highlight stayed where the text was before animation: {} → {}", top(&still), top(&moved));
}

/// A 3D solid at the comp's centre, selected.
fn card(doc: &mut MotionDocument) -> u64 {
    let card = solid(doc, "Card");
    doc.execute("layer.setSwitch", json!({ "layers": [card], "switch": "threeD", "value": true })).unwrap();
    doc.execute("layer.select", json!({ "layers": [card] })).unwrap();
    card
}

#[test]
fn tools_the_3d_arrows_move_a_3d_layer_along_one_axis_in_one_undo_step() {
    use super::gizmo::{Part, spot};
    let mut doc = doc();
    let card = card(&mut doc);
    let start = value(&mut doc, card, "transform/position");
    // The X arrow, dragged 60 px right: the layer moves along X only (the default camera shows z = 0 at 1:1).
    let tip = spot(&doc.session, Part::Axis(0), 1.0).expect("the X arrow shows");
    drag(&mut doc, tip, [tip[0] + 60.0, tip[1] + 25.0], ToolMods::default());
    let moved = value(&mut doc, card, "transform/position");
    assert!((moved[0] - start[0] - 60.0).abs() < 1.0 && (moved[1] - start[1]).abs() < 1e-6 && (moved[2] - start[2]).abs() < 1e-6, "{start:?} → {moved:?}");
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&value(&mut doc, card, "transform/position"), &start), "one undo step puts it back");
    // In the Top view the Z arrow is on screen: dragging it changes the depth only.
    doc.execute("view.set3DView", json!({ "view": "top" })).unwrap();
    let tip = spot(&doc.session, Part::Axis(2), 1.0).expect("the Z arrow shows in the Top view");
    let base = spot(&doc.session, Part::Plane(0, 2), 1.0).unwrap();
    let away = [tip[0] - base[0], tip[1] - base[1]];
    drag(&mut doc, tip, [tip[0] + away[0] * 0.5, tip[1] + away[1] * 0.5], ToolMods::default());
    let deeper = value(&mut doc, card, "transform/position");
    assert!((deeper[2] - start[2]).abs() > 1.0 && (deeper[0] - start[0]).abs() < 1e-6 && (deeper[1] - start[1]).abs() < 1e-6, "{start:?} → {deeper:?}");
}

#[test]
fn tools_a_3d_plane_square_moves_in_two_axes_and_shift_keeps_one() {
    use super::gizmo::{Part, spot};
    let mut doc = doc();
    let card = card(&mut doc);
    let start = value(&mut doc, card, "transform/position");
    let square = spot(&doc.session, Part::Plane(0, 1), 1.0).expect("the XY square shows");
    drag(&mut doc, square, [square[0] + 40.0, square[1] + 30.0], ToolMods::default());
    let moved = value(&mut doc, card, "transform/position");
    assert!((moved[0] - start[0] - 40.0).abs() < 1.0 && (moved[1] - start[1] - 30.0).abs() < 1.0 && (moved[2] - start[2]).abs() < 1e-6, "{moved:?}");
    doc.execute("edit.undo", json!({})).unwrap();
    let square = spot(&doc.session, Part::Plane(0, 1), 1.0).unwrap();
    drag_from(&mut doc, square, [square[0] + 40.0, square[1] + 10.0], ToolMods::default(), SHIFT);
    let kept = value(&mut doc, card, "transform/position");
    assert!((kept[0] - start[0] - 40.0).abs() < 1.0 && (kept[1] - start[1]).abs() < 1e-6, "Shift keeps the main direction: {kept:?}");
}

#[test]
fn tools_a_3d_ring_turns_the_layer_about_its_axis_and_shift_steps_15_degrees() {
    use super::gizmo::{Part, spot};
    let mut doc = doc();
    let card = card(&mut doc);
    // The Z ring faces the default camera: a drag along it turns Z Rotation.
    // Along the ring: 30° about the layer's anchor point (the comp's centre).
    let along = |p: [f64; 2]| {
        let (dx, dy) = (p[0] - 960.0, p[1] - 540.0);
        let (s, c) = 30f64.to_radians().sin_cos();
        [960.0 + dx * c - dy * s, 540.0 + dx * s + dy * c]
    };
    let on = spot(&doc.session, Part::Ring(2), 1.0).expect("the Z ring shows");
    drag(&mut doc, on, along(on), ToolMods::default());
    let turned = value(&mut doc, card, "transform/rotation")[0];
    assert!(turned.abs() > 15.0, "the Z ring did not turn the layer: {turned}");
    assert!(near(&value(&mut doc, card, "transform/rotationX"), &[0.0]) && near(&value(&mut doc, card, "transform/rotationY"), &[0.0]));
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&value(&mut doc, card, "transform/rotation"), &[0.0]), "one undo step");
    let on = spot(&doc.session, Part::Ring(2), 1.0).unwrap();
    drag_from(&mut doc, on, along(on), ToolMods::default(), SHIFT);
    let stepped = value(&mut doc, card, "transform/rotation")[0];
    assert!(stepped.abs() > 1.0 && (stepped / 15.0 - (stepped / 15.0).round()).abs() < 1e-9, "Shift turns in 15° steps: {stepped}");
    // A camera has the handles too (seen from the Top view).
    let camera = doc.execute("layer.newCamera", json!({})).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.select", json!({ "layers": [camera] })).unwrap();
    doc.execute("view.set3DView", json!({ "view": "top" })).unwrap();
    let before = value(&mut doc, camera, "transform/position");
    let tip = spot(&doc.session, Part::Axis(0), 1.0).expect("the camera's X arrow shows in the Top view");
    drag(&mut doc, tip, [tip[0] + 30.0, tip[1]], ToolMods::default());
    let after = value(&mut doc, camera, "transform/position");
    assert!(after[0] > before[0] + 1.0 && (after[1] - before[1]).abs() < 1e-6, "{before:?} → {after:?}");
}

/// In the default (front) view the X and Y rings are seen edge-on: each is a band around its line,
/// clear of the arrows, and a drag along it turns X or Y Rotation; the Z arrow, pointing at the
/// viewer, is a dot beside the origin that a vertical drag moves in depth (up: away).
#[test]
fn tools_edge_on_rings_turn_and_the_z_dot_moves_in_depth_in_the_front_view() {
    use super::gizmo::{Part, spot};
    let mut doc = doc();
    let card = card(&mut doc);
    let rx = spot(&doc.session, Part::Ring(0), 1.0).expect("the X ring shows edge-on");
    let ry = spot(&doc.session, Part::Ring(1), 1.0).expect("the Y ring shows edge-on");
    // The X ring's band is upright, the Y ring's level: a drag along each turns it.
    drag(&mut doc, rx, [rx[0], rx[1] - 40.0], ToolMods::default());
    let turned = value(&mut doc, card, "transform/rotationX")[0];
    assert!(turned.abs() > 15.0, "the edge-on X ring turned the layer by {turned}°");
    assert!(near(&value(&mut doc, card, "transform/rotationY"), &[0.0]) && near(&value(&mut doc, card, "transform/rotation"), &[0.0]));
    doc.execute("edit.undo", json!({})).unwrap();
    assert!(near(&value(&mut doc, card, "transform/rotationX"), &[0.0]), "one undo step");
    drag(&mut doc, ry, [ry[0] + 40.0, ry[1]], ToolMods::default());
    let turned = value(&mut doc, card, "transform/rotationY")[0];
    assert!(turned.abs() > 15.0, "the edge-on Y ring turned the layer by {turned}°");
    assert!(near(&value(&mut doc, card, "transform/rotationX"), &[0.0]));
    doc.execute("edit.undo", json!({})).unwrap();
    // The Z dot: up 45 px moves the layer 45 px away (the default camera shows z = 0 at 1:1).
    let start = value(&mut doc, card, "transform/position");
    let dot = spot(&doc.session, Part::Axis(2), 1.0).expect("the Z arrow is a dot in the front view");
    drag(&mut doc, dot, [dot[0], dot[1] - 45.0], ToolMods::default());
    let moved = value(&mut doc, card, "transform/position");
    assert!((moved[2] - start[2] - 45.0).abs() < 2.0 && near(&moved[..2], &start[..2]), "{start:?} → {moved:?}");
}

/// Over a 3D handle the status bar says what it does (the depth dot: moving forward or back);
/// leaving the handles clears it.
#[test]
fn tools_the_status_bar_says_what_the_3d_handle_under_the_pointer_does() {
    use super::gizmo::{Part, spot};
    let mut doc = doc();
    card(&mut doc);
    let status = |doc: &mut MotionDocument, p: [f64; 2]| -> Option<String> {
        let response = doc.pointer(&event(PointerAction::Move, p[0], p[1], ToolMods::default())).unwrap();
        response.requests.iter().find_map(|request| match request {
            ToolRequest::Status { text } => Some(text.clone()),
            _ => None,
        })
    };
    let cases = [
        (Part::Axis(0), "Drag to move along X"),
        (Part::Axis(1), "Drag to move along Y"),
        (Part::Axis(2), "Drag up or down to move forward or back (depth)"),
        (Part::Ring(0), "Drag to rotate around X"),
        (Part::Ring(1), "Drag to rotate around Y"),
    ];
    for (part, said) in cases {
        let at = spot(&doc.session, part, 1.0).unwrap_or_else(|| panic!("{part:?} shows in the front view"));
        assert_eq!(status(&mut doc, at).as_deref(), Some(said), "{part:?}");
        // Off the handles: cleared.
        assert_eq!(status(&mut doc, [5.0, 5.0]).as_deref(), Some(""), "{part:?}");
    }
    // Still on the same handle: said once.
    let dot = spot(&doc.session, Part::Axis(2), 1.0).unwrap();
    status(&mut doc, dot);
    assert_eq!(status(&mut doc, [dot[0] + 0.5, dot[1]]), None);
}

#[test]
fn tools_the_second_view_renders_another_3d_view_with_its_ground_and_outlines() {
    let mut doc = doc();
    card(&mut doc);
    let (rgba, width, height, lines, layers) = doc.view_frame("top", 320).unwrap();
    assert_eq!((width, height), (320, 180));
    assert_eq!(rgba.len(), 320 * 180 * 4);
    assert!(lines.as_array().is_some_and(|lines| lines.len() >= 10), "the Top view's ground and outlines: {lines}");
    // The 3D layer with its name, label colour, selection and outline (edge-on from the top: flat).
    let card = layers.as_array().and_then(|layers| layers.first()).cloned().unwrap_or_default();
    assert_eq!(card["name"], "Card", "{layers}");
    assert_eq!(card["selected"], true);
    assert_eq!(card["quad"].as_array().map(Vec::len), Some(4));
    assert_eq!(card["label"].as_array().map(Vec::len), Some(3));
    // The viewer's own view is untouched.
    assert!(doc.session.view_camera(doc.session.active_comp_id().unwrap()).is_none());
    let (_, _, _, none, no_layers) = doc.view_frame("activeCamera", 320).unwrap();
    assert_eq!(none, serde_json::json!([]), "the active camera's view draws no ground");
    assert_eq!(no_layers, serde_json::json!([]));
    assert!(doc.view_frame("sideways", 320).is_err());
}
