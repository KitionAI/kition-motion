//! What a step names by its place (task card C8-motion-2), `KitDocument::under` as kition-sdk's
//! premise guard asks it with the step's comp and time set: a layer by stack position (`"#2"`),
//! and a time with no layer named, per kind of command (`bindings/wasm/src/place.rs`).
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::{CallContext, KitDocument, Premise, TimeSpan, Under, Within};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

/// `Main` (4 s, open) with `Red` (alive from 1 s) and `Blue` on top (alive from 0): `Blue` is
/// `#1`, `Red` `#2`.
fn fixture() -> (MotionDocument, u64, u64, u64) {
    let mut doc = MotionDocument::create("Motion.kmotion", 320, 180).unwrap();
    doc.execute("comp.settings", json!({ "duration": 4, "name": "Main" })).unwrap();
    let red = doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.timing", json!({ "layers": [red], "start": 1 })).unwrap();
    let blue = doc.execute("layer.newSolid", json!({ "name": "Blue", "color": "#0000ff" })).unwrap()["layer"].as_u64().unwrap();
    let main = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    doc.execute("layer.select", json!({ "layers": [] })).unwrap();
    (doc, main, red, blue)
}

/// `under` with the step's comp and time set, as the host asks it.
fn under(doc: &mut MotionDocument, main: u64, time: f64, on: &[Value], command: &str, params: Value) -> Option<Under> {
    doc.set_context(&CallContext { part: Some(json!({ "comp": main })), time: Some(time), on: on.to_vec() }).unwrap();
    doc.under(command, &params).unwrap()
}

fn window(main: u64, from: usize, to: usize) -> Within {
    Within { parent: json!(main), field: "comp.layers".to_owned(), from, to, time: None }
}

#[test]
fn a_stack_position_names_the_layer_there_now_and_its_window() {
    let (mut doc, main, red, blue) = fixture();
    let got = under(&mut doc, main, 0.0, &[], "prop.set", json!({ "layer": "#2", "path": "transform/opacity", "value": 5 })).unwrap();
    let opacity = Premise::on(json!(red)).fields(["transform/opacity"]).positions(Vec::<String>::new());
    assert_eq!(got, Under { objects: vec![json!(red)], within: vec![window(main, 1, 2)], premises: vec![opacity.clone()], ..Default::default() });
    let got = under(&mut doc, main, 0.0, &[], "layer.setSwitch", json!({ "layers": ["#1", "#2"], "switch": "shy" })).unwrap();
    assert_eq!(got, Under { objects: vec![json!(blue), json!(red)], within: vec![window(main, 0, 1), window(main, 1, 2)], ..Default::default() });
    // Past the bottom: no layer, where one would go.
    let got = under(&mut doc, main, 0.0, &[], "prop.set", json!({ "layer": "#9", "path": "transform/opacity", "value": 5 })).unwrap();
    assert_eq!(got, Under { objects: vec![], within: vec![window(main, 8, 8)], ..Default::default() });
    // By id, no place: only what the call needs of the layer.
    let got = under(&mut doc, main, 0.0, &[], "prop.set", json!({ "layer": red, "path": "transform/opacity", "value": 5 }));
    assert_eq!(got, Some(Under { premises: vec![opacity], ..Default::default() }));
}

#[test]
fn a_time_names_what_the_command_acts_on_there() {
    let (mut doc, main, red, blue) = fixture();
    // A time that names nothing: moving the current time.
    assert_eq!(under(&mut doc, main, 2.0, &[], "time.set", json!({ "time": 3 })), Some(Under::default()));
    // A comp marker holds only the comp markers, through the owning ref.
    let markers = Some(Under { premises: vec![Premise::on(json!(main)).fields(["comp.attrs.markers"]).positions(Vec::<String>::new())], ..Default::default() });
    assert_eq!(under(&mut doc, main, 2.0, &[], "comp.addMarker", json!({ "comment": "m" })), markers);
    assert_eq!(under(&mut doc, main, 2.0, &[], "markers.set", json!({ "new": true })), markers);
    // A layer command given no layer: the selected layers it acts on.
    let got = under(&mut doc, main, 2.0, &[json!(red)], "edit.splitLayer", json!({})).unwrap();
    assert_eq!(got, Under { objects: vec![json!(red)], within: vec![window(main, 1, 2)], ..Default::default() });
    // Any other command at a time: the layers alive then, the time as a span over their In and
    // Out points.
    let at = |seconds: i64| {
        let ticks = json!((seconds * 127_008_000_000).to_string());
        let span = TimeSpan { from: ticks.clone(), to: ticks, start: "inPoint".to_owned(), end: Some("outPoint".to_owned()), duration: None, identity: None };
        vec![Within { parent: json!(main), field: "comp.layers".to_owned(), time: Some(span), ..Default::default() }]
    };
    // A new layer's defaults are sized and spanned from the composition (rest_comp_reads.rs).
    let size_and_duration =
        || vec![Premise::on(json!(main)).fields(["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration"]).positions(Vec::<String>::new())];
    let got = under(&mut doc, main, 0.5, &[], "layer.newCamera", json!({})).unwrap();
    assert_eq!(got, Under { objects: vec![json!(blue)], within: at(1), premises: size_and_duration(), ..Default::default() });
    let got = under(&mut doc, main, 2.0, &[], "layer.newCamera", json!({})).unwrap();
    assert_eq!(got, Under { objects: vec![json!(blue), json!(red)], within: at(4), premises: size_and_duration(), ..Default::default() });
}

#[test]
fn a_named_layer_premise_does_not_put_the_comps_list_on_the_layer() {
    let (mut doc, main, red, _) = fixture();
    let got = under(&mut doc, main, 0.0, &[], "prop.set", json!({"layer":"Red","path":"transform/opacity","value":50})).unwrap();
    let held = got.premises.iter().find(|premise| premise.id == json!(red)).unwrap();
    assert_eq!(held.fields.as_ref().unwrap(), &vec!["transform/opacity".to_owned(), "name".to_owned()]);
    assert_eq!(held.positions, Some(Vec::new()), "comp.layers belongs to the composition item, not its layer");
}
