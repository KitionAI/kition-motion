//! Why a command did not run is typed (`kition_kit::failure`, task card C8-motion), so a host
//! answers its error code without reading the wording: an unknown command is
//! `command_not_found`, one that cannot run now (nothing selected, no comp open)
//! `command_disabled`, wrong parameters and a call context the project lacks `invalid_params`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_kit::failure::{CommandFailure, parse_failure};
use kition_kit::kdm::Origin;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn project() -> (MotionDocument, u64) {
    let mut doc = MotionDocument::create("Motion.kmotion", 320, 180).unwrap();
    let layer = doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap()["layer"].as_u64().unwrap();
    (doc, layer)
}

fn reason(error: &str) -> CommandFailure {
    parse_failure(error).unwrap_or_else(|| panic!("no typed reason in {error:?}")).1
}

#[test]
fn a_commands_failure_carries_its_reason() {
    let (mut doc, layer) = project();
    assert_eq!(reason(&doc.execute("layer.noSuchCommand", json!({})).unwrap_err()), CommandFailure::NotFound);
    // A property path the layer lacks.
    assert_eq!(reason(&doc.execute("prop.set", json!({ "layer": layer, "path": "transform/nope", "value": 1 })).unwrap_err()), CommandFailure::InvalidParams);
    // A parameter the command does not take.
    assert_eq!(
        reason(&doc.execute("prop.set", json!({ "layerz": layer, "path": "transform/opacity", "value": 1 })).unwrap_err()),
        CommandFailure::InvalidParams
    );
    // A comp the project lacks.
    assert_eq!(
        reason(&doc.execute("prop.set", json!({ "comp": 9_999, "layer": layer, "path": "transform/opacity", "value": 1 })).unwrap_err()),
        CommandFailure::InvalidParams
    );
    // Nothing selected: the command cannot run now.
    doc.execute("layer.select", json!({ "layers": [] })).unwrap();
    assert_eq!(reason(&doc.execute("layer.setSwitch", json!({ "switch": "shy", "value": true })).unwrap_err()), CommandFailure::Disabled);
    let agent = Origin::agent("user:test");
    assert_eq!(reason(&doc.execute_as("layer.noSuchCommand", json!({}), &agent).unwrap_err()), CommandFailure::NotFound);
}

#[test]
fn a_call_context_the_project_lacks_is_invalid_params() {
    let (mut doc, _) = project();
    for context in [
        json!({ "time": 600 }),
        json!({ "on": [9_999] }),
        json!({ "on": ["#1"] }),
        json!({ "part": { "comp": 9_999 } }),
        json!({ "part": { "slide": 1 } }),
        json!({ "on": [true] }),
    ] {
        let context: kition_kit::CallContext = serde_json::from_value(context.clone()).unwrap();
        let error = doc.set_context(&context).unwrap_err();
        assert_eq!(reason(&error), CommandFailure::InvalidParams, "{error}");
    }
}
