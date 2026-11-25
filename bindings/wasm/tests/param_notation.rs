//! Every command's parameter notation is read whole: each entry names its key, so the schema the
//! catalogue offers agents has a parameter for every key the notation lists
//! (regressions/issue-unreadable-param-notes). `check_commands` compares only schemas that are the
//! bare notation's, and the kit types and completes every schema after converting it, so the
//! notation is checked here directly.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn project() -> MotionDocument {
    MotionDocument::create("a.kmotion", 64, 36).unwrap()
}

#[test]
fn every_notation_entry_names_a_key() {
    let mut problems = Vec::new();
    for command in project().commands() {
        let Some(params) = command.params.as_deref() else { continue };
        let converted = kition_kit::schema::from_kit_params(params).unwrap_or_else(|error| panic!("{}: {error}", command.id));
        if !converted.unreadable.is_empty() {
            problems.push(format!("{}: {:?}", command.id, converted.unreadable));
        }
    }
    assert!(problems.is_empty(), "notation entries with no key the parser reads:\n{}", problems.join("\n"));
}

#[test]
fn the_keys_the_notation_lost_are_parameters() {
    let commands = project().commands();
    let cases: &[(&str, &[&str])] = &[
        ("comp.settings", &["anchor"]),
        ("layer.newSolid", &["color"]),
        ("layer.setText", &["hScale", "vScale", "tsume"]),
        ("camera.orbit", &["yaw", "dx", "pitch", "dy"]),
        ("layer.newModel", &["path"]),
        ("keyframe.interpolation", &["in", "out"]),
        ("keyframe.velocity", &["inInfluence", "outInfluence"]),
        ("keyframe.setEase", &["influence"]),
        ("view.customRgb", &["red", "green", "blue", "white"]),
        ("shape.toolOptions", &["fillOpacity"]),
    ];
    for (id, keys) in cases {
        let command = commands.iter().find(|command| command.id == *id).unwrap_or_else(|| panic!("no command {id}"));
        let schema = command.schema.as_ref().unwrap_or_else(|| panic!("{id}: no schema"));
        for key in *keys {
            assert!(schema.get(*key).is_some(), "{id}: no parameter `{key}` in {schema}");
        }
    }
}

#[test]
fn a_call_with_the_keys_is_not_refused() {
    let mut doc = project();
    let solid = doc.execute("layer.newSolid", json!({ "name": "Red", "color": [1.0, 0.0, 0.0] })).unwrap();
    assert!(solid.get("layer").is_some(), "{solid}");
    let text = doc.execute("layer.newText", json!({ "text": "Hi" })).unwrap();
    let layer = text["layer"].clone();
    doc.execute("layer.setText", json!({ "layer": layer, "hScale": 120, "vScale": 90, "tsume": 50 })).unwrap();
    doc.execute("shape.toolOptions", json!({ "fillOpacity": 50 })).unwrap();
    let rgb = doc.execute("view.customRgb", json!({ "red": [0.64, 0.33], "green": [0.30, 0.60], "blue": [0.15, 0.06], "white": [0.3127, 0.3290] })).unwrap();
    assert_eq!(rgb["red"], json!([0.64, 0.33]), "{rgb}");
}
