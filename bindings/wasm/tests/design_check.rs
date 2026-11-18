//! Regression: the design check has no false positives for text an
//! expression animates (countUp), contrast is measured against what is right behind the text (a
//! card under it, not the background), and hierarchy compares only texts on screen together.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn run(doc: &mut MotionDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

/// The layers (ids) a check code is reported on.
fn on(doc: &MotionDocument, code: &str) -> Vec<u64> {
    doc.check()
        .into_iter()
        .filter(|d| d.code == code)
        .filter_map(|d| d.target.path.and_then(|p| p.rsplit("layer:").next().and_then(|id| id.parse().ok())))
        .collect()
}

#[test]
fn design_check_without_false_positives() {
    let mut doc = MotionDocument::create("stats.kmotion", 1280, 720).unwrap();
    run(&mut doc, "layer.newSolid", json!({ "name": "Background", "color": "#0b1020" }));
    // A white card with dark text on it: readable, though the background is dark too.
    run(&mut doc, "shape.newShape", json!({ "name": "Card", "kind": "rect", "size": [600, 300], "position": [640, 360], "fill": "#ffffff" }));
    let on_card = run(&mut doc, "layer.newText", json!({ "text": "Revenue", "position": [640, 360], "size": 80, "fill": "#111111", "style": "Bold" }))["layer"]
        .as_u64()
        .unwrap();
    // Dark text straight on the dark background: hard to read.
    let on_back =
        run(&mut doc, "layer.newText", json!({ "text": "Footnote", "position": [640, 650], "size": 40, "fill": "#151a2a", "style": "Bold" }))["layer"]
            .as_u64()
            .unwrap();
    let low = on(&doc, "motion.low-contrast");
    assert!(!low.contains(&on_card), "the card is behind the text: {low:?}");
    assert!(low.contains(&on_back), "{low:?}");

    // A count-up is an expression: it moves.
    let count = run(&mut doc, "layer.newText", json!({ "text": "0", "position": [640, 200], "size": 72, "fill": "#ffffff", "style": "Bold" }))["layer"]
        .as_u64()
        .unwrap();
    run(&mut doc, "layer.applyTextPreset", json!({ "layer": count, "preset": "countUp", "from": 0, "to": 42, "start": 0.2, "duration": 1 }));
    assert!(!on(&doc, "motion.static-text").contains(&count), "{:?}", on(&doc, "motion.static-text"));

    // Two titles of the same size, one after the other: never on screen together.
    let mut seq = MotionDocument::create("sequence.kmotion", 1280, 720).unwrap();
    let a = run(&mut seq, "layer.newText", json!({ "text": "Chapter one", "position": [640, 360], "size": 96, "style": "Bold" }))["layer"].clone();
    let b = run(&mut seq, "layer.newText", json!({ "text": "Chapter two", "position": [640, 360], "size": 90, "style": "Bold" }))["layer"].clone();
    run(&mut seq, "layer.timing", json!({ "layers": [a], "in": 0, "out": 3 }));
    run(&mut seq, "layer.timing", json!({ "layers": [b], "in": 3, "out": 6 }));
    assert!(on(&seq, "motion.weak-hierarchy").is_empty(), "{:?}", seq.check());
    // On screen together, they compete.
    run(&mut seq, "layer.timing", json!({ "layers": [b], "in": 1, "out": 6 }));
    assert_eq!(on(&seq, "motion.weak-hierarchy").len(), 1, "{:?}", seq.check());
}

#[test]
fn contrast_is_measured_once_the_plate_has_arrived_and_through_faint_layers() {
    let mut doc = MotionDocument::create("lower-third.kmotion", 1280, 720).unwrap();
    run(&mut doc, "layer.newSolid", json!({ "name": "Footage", "color": "#1d3b47" }));
    // A faint light shape over the backdrop does not make the backdrop light.
    let glow =
        run(&mut doc, "layer.newShape", json!({ "name": "Glow", "kind": "ellipse", "size": [1400, 1400], "position": [640, 360], "fill": "#ffd9a8" }))["layer"]
            .clone();
    run(&mut doc, "layer.setTransform", json!({ "layers": [glow], "prop": "opacity", "value": 20 }));
    // A dark plate slides in under white text: by the time the text is on, it is behind it.
    let plate =
        run(&mut doc, "layer.newShape", json!({ "name": "Plate", "kind": "rect", "size": [720, 150], "position": [460, 585], "fill": "#0e1a22" }))["layer"]
            .clone();
    run(&mut doc, "prop.addKey", json!({ "layer": plate, "path": "transform/position", "time": 0.3, "value": [-380, 585, 0] }));
    run(&mut doc, "prop.addKey", json!({ "layer": plate, "path": "transform/position", "time": 1.0, "value": [460, 585, 0] }));
    let name = run(
        &mut doc,
        "layer.newText",
        json!({ "text": "Amara Osei", "position": [146, 584], "size": 58, "fill": "#ffffff", "style": "Bold", "justify": "left" }),
    )["layer"]
        .as_u64()
        .unwrap();
    assert!(!on(&doc, "motion.low-contrast").contains(&name), "{:?}", doc.check());
    // A light text on the faint glow over the dark backdrop reads.
    let caption =
        run(&mut doc, "layer.newText", json!({ "text": "Coral lab", "position": [640, 200], "size": 40, "fill": "#ffffff", "style": "Bold" }))["layer"]
            .as_u64()
            .unwrap();
    assert!(!on(&doc, "motion.low-contrast").contains(&caption), "{:?}", doc.check());
    // Large text needs 3:1 (WCAG): white on a pink-red badge reads at 72 px, not at 16 px.
    let badge = run(&mut doc, "layer.newShape", json!({ "name": "Badge", "kind": "rounded", "size": [400, 200], "position": [1000, 360], "fill": "#ff2d55" }))
        ["layer"]
        .clone();
    assert!(badge.is_u64());
    let live = run(&mut doc, "layer.newText", json!({ "text": "LIVE", "position": [1000, 380], "size": 72, "fill": "#ffffff", "style": "Bold" }))["layer"]
        .as_u64()
        .unwrap();
    let small = run(&mut doc, "layer.newText", json!({ "text": "now", "position": [1000, 300], "size": 16, "fill": "#ffffff", "style": "Bold" }))["layer"]
        .as_u64()
        .unwrap();
    let low = on(&doc, "motion.low-contrast");
    assert!(!low.contains(&live) && low.contains(&small), "{low:?}");
}
