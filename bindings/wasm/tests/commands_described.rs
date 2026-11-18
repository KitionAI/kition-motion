//! Every command says what it does and every reading command an agent is offered declares what it
//! answers (kition-sdk `check_commands`); run on a project with one solid layer, the reading
//! commands' answers match their output schemas (`bindings/wasm/src/outputs.json`).
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn project() -> MotionDocument {
    let mut doc = MotionDocument::create("a.kmotion", 64, 36).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
    doc
}

fn matches(schema: &Value, value: &Value) -> Result<(), String> {
    match schema["type"].as_str() {
        Some("json") | None => Ok(()),
        Some("object") => match value.as_object() {
            Some(object) => kition_kit::schema::check_params(&schema["properties"], object),
            None => Err(format!("expected an object, got {value}")),
        },
        Some("array") => value.is_array().then_some(()).ok_or_else(|| format!("expected an array, got {value}")),
        Some("boolean") => value.is_boolean().then_some(()).ok_or_else(|| format!("expected a boolean, got {value}")),
        Some("integer") => value.is_i64().then_some(()).or(value.is_u64().then_some(())).ok_or_else(|| format!("expected an integer, got {value}")),
        Some("number") => value.is_number().then_some(()).ok_or_else(|| format!("expected a number, got {value}")),
        Some("string") => value.is_string().then_some(()).ok_or_else(|| format!("expected a string, got {value}")),
        Some(other) => Err(format!("unknown output type {other}")),
    }
}

#[test]
fn every_command_is_described_and_reading_commands_answer_their_output_schema() {
    let commands = project().commands();
    kition_kit::testing::check_commands(&commands, kition_motion_kit::descriptions::NO_OUTPUT).unwrap();
    let mut checked = 0;
    for command in commands.iter().filter(|command| !command.mutates && command.schema.is_some()) {
        let Some(schema) = &command.output_schema else { continue };
        kition_kit::schema::validate_output(schema).unwrap_or_else(|error| panic!("{}: {error}", command.id));
        if let Ok(answer) = project().execute(&command.id, json!({}))
            && !answer.is_null()
        {
            matches(schema, &answer).unwrap_or_else(|error| panic!("{}: {error}", command.id));
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} reading commands answered with no parameters");
}

#[test]
fn file_open_says_it_opens_the_project_at_path() {
    let description = kition_motion_kit::descriptions::description("file.open");
    assert!(description.contains("`path`") && description.contains("project"), "{description}");
}

/// Commands that are one action under two names (one handler), so they share a text.
const SAME_ACTION: &[(&str, &str)] = &[("layer.addTextSelector", "text.addSelector"), ("layer.new3dPrimitive", "layer.newPrimitive")];

#[test]
fn no_two_commands_share_a_description_and_none_only_repeats_its_name() {
    let commands = project().commands();
    let mut seen: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    let mut shared = vec![];
    for command in &commands {
        let text = command.description.as_str();
        let starts = text.trim_start_matches('`').starts_with(command.id.as_str());
        assert!(!starts, "{} only repeats its name: {text}", command.id);
        if let Some(other) = seen.insert(text, &command.id) {
            let pair = if other < command.id.as_str() { (other, command.id.as_str()) } else { (command.id.as_str(), other) };
            if !SAME_ACTION.contains(&pair) {
                shared.push(format!("{} and {}: {text}", pair.0, pair.1));
            }
        }
    }
    assert!(shared.is_empty(), "commands sharing one description:\n{}", shared.join("\n"));
}

#[test]
fn effect_apply_says_its_settings_are_set_in_the_same_step() {
    let description = kition_motion_kit::descriptions::description("effect.apply");
    assert!(description.contains("`params`") && description.contains("paths"), "{description}");
}
