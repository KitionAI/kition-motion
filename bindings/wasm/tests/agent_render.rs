//! `file.render`, the kit's agent preview (`agent.render` in kit.json, KHP A24, A39): the frame of
//! an explicit composition at an explicit comp time, or a contact sheet of frames over a range, as
//! a pure function of the project. It never reads or moves the current time, never changes the
//! open composition, the selection or the dirty state, and answers the same bytes for the same
//! call.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

const RED: [u8; 3] = [255, 0, 0];
const BLUE: [u8; 3] = [0, 0, 255];

fn run(doc: &mut MotionDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

/// A 320 x 180, 30 fps, two-second composition: a red solid for its first second, a blue one for
/// its second. Answers the document and the ids of the comp, the red layer and the blue layer.
fn motion() -> (MotionDocument, u64, u64, u64) {
    let mut doc = MotionDocument::create("Motion.kmotion", 320, 180).unwrap();
    run(&mut doc, "comp.settings", json!({ "duration": 2 }));
    let red = run(&mut doc, "layer.newSolid", json!({ "name": "Red", "color": "#ff0000" }))["layer"].as_u64().unwrap();
    run(&mut doc, "layer.timing", json!({ "layers": [red], "in": 0, "out": 1 }));
    let blue = run(&mut doc, "layer.newSolid", json!({ "name": "Blue", "color": "#0000ff" }))["layer"].as_u64().unwrap();
    run(&mut doc, "layer.timing", json!({ "layers": [blue], "in": 1, "out": 2 }));
    let comp = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    (doc, comp, red, blue)
}

fn render(doc: &mut MotionDocument, params: Value) -> Value {
    run(doc, "file.render", params)
}

fn refused(doc: &mut MotionDocument, params: Value) -> String {
    match doc.execute("file.render", params.clone()) {
        Ok(answer) => panic!("{params} rendered: {}", answer["frames"]),
        Err(error) => error,
    }
}

/// Standard base64, padded.
fn base64_decode(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => panic!("not base64: {c}"),
    };
    let mut out = Vec::new();
    for chunk in text.as_bytes().chunks(4) {
        let digits: Vec<u8> = chunk.iter().copied().filter(|c| *c != b'=').map(value).collect();
        let n = digits.iter().enumerate().fold(0u32, |n, (i, d)| n | (u32::from(*d) << (18 - 6 * i)));
        out.extend((0..digits.len().saturating_sub(1)).map(|i| (n >> (16 - 8 * i)) as u8));
    }
    out
}

/// The answer's picture, decoded.
fn picture(answer: &Value) -> image::RgbaImage {
    let png = base64_decode(answer["png"].as_str().unwrap());
    image::load_from_memory(&png).unwrap().to_rgba8()
}

fn near(pixel: &image::Rgba<u8>, rgb: [u8; 3]) -> bool {
    pixel.0.iter().zip(rgb).all(|(a, b)| a.abs_diff(b) <= 2)
}

/// What the editor holds besides the document, and the document's state: the open comp, its
/// current time, the selection (per layer, and the selected layer's transform panel), history,
/// dirty state, revision, and the file's bytes.
fn editor_state(doc: &MotionDocument) -> (Value, Value, Value, Value, Value, bool, u64, Vec<u8>) {
    let inspected = doc.inspect().unwrap();
    let selected: Vec<Value> = inspected["info"]["layers"].as_array().unwrap().iter().map(|layer| layer["selected"].clone()).collect();
    (
        inspected["comp"]["id"].clone(),
        inspected["time"].clone(),
        Value::Array(selected),
        inspected["info"]["transform"].clone(),
        json!([inspected["canUndo"], inspected["canRedo"]]),
        doc.dirty(),
        doc.revision(),
        doc.save(None).unwrap(),
    )
}

#[test]
fn rendering_reads_no_current_time_and_changes_nothing() {
    let (mut doc, comp, _, blue) = motion();
    doc.mark_saved();
    let call = json!({ "target": { "comp": comp, "time": 0.5 } });
    let before = editor_state(&doc);
    let first = render(&mut doc, call.clone());
    assert_eq!(editor_state(&doc), before, "rendering moved the current time, the open comp, the selection or the dirty state");
    assert!(near(picture(&first).get_pixel(160, 90), RED), "the frame at 0.5 s is red");

    // Move the current time past the cut, select a layer, and open another composition.
    run(&mut doc, "time.set", json!({ "time": 1.5 }));
    run(&mut doc, "layer.select", json!({ "layers": [blue] }));
    run(&mut doc, "comp.new", json!({ "name": "Other", "width": 64, "height": 36, "duration": 1, "open": true }));
    assert_ne!(doc.inspect().unwrap()["comp"]["id"], json!(comp), "another comp is open");
    let before = editor_state(&doc);
    let second = render(&mut doc, call);
    assert_eq!(editor_state(&doc), before, "rendering moved the current time, the open comp, the selection or the dirty state");
    assert_eq!(second["png"], first["png"], "the same comp at the same time is the same picture whatever the editor shows");
    assert_eq!(second["frames"], first["frames"]);
    assert_eq!(second["comp"], json!(comp));
}

#[test]
fn two_times_are_two_pictures() {
    let (mut doc, comp, _, _) = motion();
    let early = render(&mut doc, json!({ "target": { "time": 0.5 } }));
    let late = render(&mut doc, json!({ "target": { "time": 1.5 } }));
    assert!(near(picture(&early).get_pixel(160, 90), RED));
    assert!(near(picture(&late).get_pixel(160, 90), BLUE));
    assert_ne!(early["png"], late["png"]);
    assert_eq!(early["comp"], json!(comp));
    // The frame showing at 1.5 s starts on a frame boundary at or before it.
    let time = late["frames"][0]["time"].as_f64().unwrap();
    assert!((1.46..=1.5).contains(&time), "{late}");
    assert_eq!(late["frames"][0]["frame"], json!(45));
    assert_eq!(late["frames"].as_array().map(Vec::len), Some(1));
    assert_eq!((late["columns"].as_u64(), late["rows"].as_u64()), (Some(1), Some(1)));
    assert_eq!(late["mimeType"], json!("image/png"));
}

#[test]
fn the_same_call_answers_the_same_bytes() {
    let (mut doc, _, _, _) = motion();
    for params in [json!({ "target": { "time": 1.25 } }), json!({ "target": { "range": [0, 2], "frames": 5 }, "scale": 0.5 }), json!({})] {
        let once = render(&mut doc, params.clone());
        let again = render(&mut doc, params.clone());
        assert_eq!(once, again, "{params}");
    }
}

#[test]
fn a_time_outside_the_comp_says_its_range() {
    let (mut doc, _, _, _) = motion();
    for time in [json!(2.5), json!(2), json!(-0.5), json!(60)] {
        let error = refused(&mut doc, json!({ "target": { "time": time } }));
        let (message, errors) = kition_kit::schema::parse_refusal(&error).unwrap_or_else(|| panic!("not a typed refusal: {error}"));
        assert_eq!(errors[0].path, "/target/time", "{error}");
        assert!(message.contains("from 0 to 2 s"), "says how long the comp is: {message}");
        assert!(message.contains("1.967 s"), "says where its last frame starts: {message}");
    }
    let error = refused(&mut doc, json!({ "target": { "time": "soon" } }));
    assert!(error.contains("/target/time"), "{error}");
}

#[test]
fn an_unknown_comp_lists_the_comps() {
    let (mut doc, comp, red, _) = motion();
    let error = refused(&mut doc, json!({ "target": { "comp": 999_999, "time": 0.5 } }));
    let (message, errors) = kition_kit::schema::parse_refusal(&error).unwrap_or_else(|| panic!("not a typed refusal: {error}"));
    assert_eq!(errors[0].path, "/target/comp", "{error}");
    assert!(message.contains(&comp.to_string()) && message.contains("Comp 1"), "lists the comps: {message}");
    // A project item that is not a composition (the red layer's solid) is no comp either.
    let solid = doc.save(None).unwrap();
    let project: Value = serde_json::from_slice(&solid).unwrap();
    let item = project["items"]
        .as_object()
        .and_then(|items| items.values().find(|item| item["kind"]["type"] == "Solid").and_then(|item| item["id"].as_u64()))
        .unwrap_or_else(|| panic!("a solid item in {}", project["items"]));
    let error = refused(&mut doc, json!({ "target": { "comp": item, "time": 0.5 } }));
    assert!(error.contains("/target/comp") && error.contains(&comp.to_string()), "{error}");
    assert!(!error.contains(&format!("{red} \"")), "a layer is no comp: {error}");
}

#[test]
fn several_root_comps_need_the_comp_named() {
    let (mut doc, first, _, _) = motion();
    let other = run(&mut doc, "comp.new", json!({ "name": "Other", "width": 64, "height": 36, "frameRate": 30, "duration": 3, "open": true }));
    let other = other["comp"].as_u64().unwrap_or_else(|| panic!("comp.new answers its id: {other}"));
    let error = refused(&mut doc, json!({ "target": { "time": 0.5 } }));
    let (message, errors) = kition_kit::schema::parse_refusal(&error).unwrap_or_else(|| panic!("not a typed refusal: {error}"));
    assert_eq!(errors[0].path, "/target/comp", "{error}");
    for (id, name, seconds) in [(first, "Comp 1", "2 s"), (other, "Other", "3 s")] {
        assert!(message.contains(&format!("{id} \"{name}\" ({seconds})")), "lists comp {id} with its name and duration: {message}");
    }
    let named = render(&mut doc, json!({ "target": { "comp": first, "time": 0.5 } }));
    assert!(near(picture(&named).get_pixel(160, 90), RED));

    // Nested in the other comp, the first is part of it: the other is the one the project shows,
    // whichever comp is open.
    run(&mut doc, "layer.addItem", json!({ "comp": other, "item": first }));
    run(&mut doc, "comp.open", json!({ "comp": first }));
    let main = render(&mut doc, json!({ "target": { "time": 0.5 } }));
    assert_eq!(main["comp"], json!(other), "{}", main["comp"]);
    assert_eq!((main["width"].as_u64(), main["height"].as_u64()), (Some(64), Some(36)));
}

#[test]
fn scale_and_max_size_size_the_picture() {
    let (mut doc, _, _, _) = motion();
    let size = |answer: &Value| (answer["width"].as_u64().unwrap(), answer["height"].as_u64().unwrap());
    let at = |doc: &mut MotionDocument, extra: Value| {
        let mut params = json!({ "target": { "time": 0.5 } });
        params.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let answer = render(doc, params);
        let shown = picture(&answer);
        assert_eq!((u64::from(shown.width()), u64::from(shown.height())), size(&answer), "the answer says the picture's size");
        answer
    };
    assert_eq!(size(&at(&mut doc, json!({}))), (320, 180), "scale 1: one pixel per comp pixel");
    assert_eq!(size(&at(&mut doc, json!({ "scale": 0.5 }))), (160, 90));
    let doubled = at(&mut doc, json!({ "scale": 2 }));
    assert_eq!(size(&doubled), (640, 360));
    assert_eq!(doubled["scale"], json!(2.0));
    let fitted = at(&mut doc, json!({ "maxSize": 100 }));
    assert_eq!(size(&fitted), (100, 56), "maxSize bounds the longer side");
    assert!((fitted["scale"].as_f64().unwrap() - 0.3125).abs() < 1e-9, "the scale drawn at, after maxSize: {}", fitted["scale"]);
    assert_eq!(size(&at(&mut doc, json!({ "scale": 2, "maxSize": 4096 }))), (640, 360));
    for bad in [json!({ "scale": 0 }), json!({ "scale": 2.5 }), json!({ "scale": "big" }), json!({ "maxSize": 15 }), json!({ "maxSize": 4097 })] {
        let mut params = json!({ "target": { "time": 0.5 } });
        params.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
        let error = refused(&mut doc, params);
        let (_, errors) = kition_kit::schema::parse_refusal(&error).unwrap_or_else(|| panic!("not a typed refusal: {error}"));
        let key = bad.as_object().unwrap().keys().next().unwrap().clone();
        assert_eq!(errors[0].path, format!("/{key}"), "{error}");
    }
}

#[test]
fn a_contact_sheet_shows_frames_over_a_range_with_their_times() {
    let (mut doc, _, _, _) = motion();
    let sheet = render(&mut doc, json!({ "target": { "range": [0, 2], "frames": 4 } }));
    let frames = sheet["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 4, "{sheet}");
    let times: Vec<f64> = frames.iter().map(|frame| frame["time"].as_f64().unwrap()).collect();
    assert_eq!(times[0], 0.0);
    assert!(times.windows(2).all(|pair| pair[0] < pair[1]), "{times:?}");
    assert!((1.9..2.0).contains(&times[3]), "the range's end shows the last frame: {times:?}");
    assert_eq!(frames[3]["frame"], json!(59));
    assert_eq!(frames[0]["label"], json!("0 s"));
    assert_eq!(frames[3]["label"], json!("1.967 s"));
    for frame in frames {
        assert!(frame["label"].as_str().is_some_and(|label| label.ends_with(" s")), "{frame}");
    }
    assert_eq!((sheet["columns"].as_u64(), sheet["rows"].as_u64()), (Some(2), Some(2)));
    let shown = picture(&sheet);
    let (w, h) = (shown.width(), shown.height());
    assert_eq!((w, h), (642, 362), "two 320 x 180 frames across and down, 2 px apart");
    // The first frame (top left) is red, the last (bottom right) blue.
    assert!(near(shown.get_pixel(w / 4, h / 4 - h / 16), RED));
    assert!(near(shown.get_pixel(w * 3 / 4, h * 3 / 4 - h / 16), BLUE));
    // Each frame carries its time, burned in: light text in its bottom left corner.
    let label_pixels = |x0: u32, y0: u32| {
        (x0..x0 + w / 4)
            .flat_map(|x| (y0 + h / 4..y0 + h / 2).map(move |y| (x, y)))
            .filter(|&(x, y)| shown.get_pixel(x, y).0[..3].iter().all(|c| *c > 200))
            .count()
    };
    for (x0, y0) in [(0, 0), (w / 2, 0), (0, h / 2), (w / 2, h / 2)] {
        assert!(label_pixels(x0, y0) > 10, "no label in the cell at {x0}, {y0}");
    }
    assert_eq!(render(&mut doc, json!({ "target": { "range": [0, 2], "frames": 4 } })), sheet, "deterministic");
    let twelve = render(&mut doc, json!({ "target": { "range": [0.5, 1.5], "frames": 12 } }));
    assert_eq!(twelve["frames"].as_array().map(Vec::len), Some(12));
    assert_eq!((twelve["columns"].as_u64(), twelve["rows"].as_u64()), (Some(4), Some(3)));
}

#[test]
fn a_contact_sheet_refuses_what_it_cannot_lay_out() {
    let (mut doc, _, _, _) = motion();
    for (target, path) in [
        (json!({ "range": [0, 2], "frames": 1 }), "/target/frames"),
        (json!({ "range": [0, 2], "frames": 13 }), "/target/frames"),
        (json!({ "frames": 3 }), "/target/frames"),
        (json!({ "range": [1.5, 0.5], "frames": 3 }), "/target/range"),
        (json!({ "range": [0, 9], "frames": 3 }), "/target/range"),
        (json!({ "range": [0], "frames": 3 }), "/target/range"),
        (json!({ "range": [0, 2], "time": 1 }), "/target/time"),
        (json!({ "sequence": 1 }), "/target/sequence"),
    ] {
        let error = refused(&mut doc, json!({ "target": target }));
        let (_, errors) = kition_kit::schema::parse_refusal(&error).unwrap_or_else(|| panic!("not a typed refusal: {error}"));
        assert_eq!(errors[0].path, path, "{target}: {error}");
    }
}

#[test]
fn without_a_target_it_shows_the_whole_comp() {
    let (mut doc, _, _, _) = motion();
    let whole = render(&mut doc, json!({ "scale": 1 }));
    let frames = whole["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 6, "a contact sheet of the comp: {whole}");
    assert_eq!(frames[0]["time"], json!(0.0));
    assert_eq!(frames[5]["frame"], json!(59), "the last frame of the comp");
    // A comp three frames long shows its three frames.
    run(&mut doc, "comp.settings", json!({ "duration": 0.1 }));
    let short = render(&mut doc, json!({}));
    assert_eq!(short["frames"].as_array().map(Vec::len), Some(3), "{short}");
}

#[test]
fn a_diagnostic_target_names_the_comp_and_the_layer() {
    let (mut doc, comp, red, blue) = motion();
    let shown = render(&mut doc, json!({ "target": { "path": format!("comp:{comp}/layer:{blue}") } }));
    assert!(near(picture(&shown).get_pixel(160, 90), BLUE), "a layer's target shows the middle of the layer");
    assert_eq!(shown["frames"][0]["frame"], json!(45), "1.5 s, the middle of 1 s to 2 s: {shown}");
    let shown = render(&mut doc, json!({ "target": { "path": format!("comp:{comp}/layer:{red}") } }));
    assert!(near(picture(&shown).get_pixel(160, 90), RED));
    let at = render(&mut doc, json!({ "target": { "path": format!("comp:{comp}/layer:{blue}"), "time": 0.25 } }));
    assert!(near(picture(&at).get_pixel(160, 90), RED), "a time given wins over the layer's middle");
    let item = render(&mut doc, json!({ "target": { "path": format!("item:{comp}"), "time": 1.75 } }));
    assert!(near(picture(&item).get_pixel(160, 90), BLUE), "a diagnostic's item target names the comp");
    let error = refused(&mut doc, json!({ "target": { "path": "layer:3" } }));
    assert!(error.contains("comp:<id>"), "{error}");
    let error = refused(&mut doc, json!({ "target": { "path": format!("comp:{comp}/layer:999999") } }));
    assert!(error.contains("/target/path") && error.contains(&red.to_string()), "lists the comp's layers: {error}");
    // A layer that never shows inside its comp has no time to show.
    run(&mut doc, "layer.timing", json!({ "layers": [red], "delta": 5 }));
    let error = refused(&mut doc, json!({ "target": { "path": format!("comp:{comp}/layer:{red}") } }));
    assert!(error.contains("/target/path"), "{error}");
}
