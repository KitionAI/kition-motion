//! Frozen r2 composition dependencies, exercised across an independent file rewrite.
mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

async fn check(command: &str, params: Value, change: (&str, Value), refuse: bool) {
    let fixture = Fixture::new(2);
    let (_, comp, basis) = fixture.inspect().await;
    let mut params = params;
    params["comp"] = json!(comp);
    if change.0 == "comp.setPosterTime" {
        fixture.colleague(&[("time.set", json!({"time":1})), change]);
    } else {
        fixture.colleague(&[change]);
    }
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
    if refuse {
        let error = answer.expect_err("a changed input or assigned field must be held");
        assert_eq!(error["error"]["code"], "conflict", "{command}: {error}");
    } else {
        assert!(answer.is_ok(), "{command} refused an unrelated field: {answer:?}");
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn scalar_settings_hold_assignments_and_accept_unrelated_fields() {
    for (mine, theirs) in [
        (json!({"name":"Mine"}), json!({"name":"Theirs"})),
        (json!({"frameRate":24}), json!({"frameRate":15})),
        (json!({"startTime":1}), json!({"startTime":2})),
        (json!({"shutterAngle":90}), json!({"shutterAngle":270})),
        (json!({"preserveResolution":true}), json!({"preserveResolution":true})),
    ] {
        check("comp.settings", mine.clone(), ("comp.settings", theirs), true).await;
        check("comp.settings", mine, ("comp.settings", json!({"background":"#123456"})), false).await;
    }
}

#[tokio::test]
async fn resize_holds_unparented_layer_position_values() {
    let bytes = project(2);
    let document = MotionDocument::open(FILE, &bytes).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    check("comp.settings", json!({"width":640}), ("prop.set", json!({"layer":layer,"path":"transform/position","value":[42,55]})), true).await;
}

#[tokio::test]
async fn resize_accepts_unrelated_child_fields() {
    let document = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    check("comp.settings", json!({"width":640}), ("layer.setComment", json!({"layer":layer,"comment":"Unrelated"})), false).await;
}

#[tokio::test]
async fn work_area_trim_holds_each_layers_timing() {
    let document = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    check("comp.trimToWorkArea", json!({}), ("layer.timing", json!({"layer":layer,"start":1})), true).await;
}

#[tokio::test]
async fn work_area_trim_accepts_unrelated_composition_fields() {
    check("comp.trimToWorkArea", json!({}), ("comp.settings", json!({"background":"#123456"})), false).await;
}

#[tokio::test]
async fn composition_switch_and_poster_hold_only_assigned_fields() {
    for (command, mine, theirs) in [
        ("comp.setSwitch", json!({"switch":"motionBlur","value":false}), json!({"switch":"motionBlur","value":false})),
        ("comp.setPosterTime", json!({}), json!({})),
    ] {
        check(command, mine.clone(), (command, theirs), true).await;
        check(command, mine, ("comp.settings", json!({"background":"#123456"})), false).await;
    }
}
