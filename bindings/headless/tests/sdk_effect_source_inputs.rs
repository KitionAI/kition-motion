mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

/// The comp `Main` is item 1 in `project(..)`.
const COMP: &str = "1";

fn source(bytes: &[u8], layer: u64) -> u64 {
    let project: Value = serde_json::from_slice(bytes).unwrap();
    project["items"].as_object().unwrap().values().filter_map(|item| item["kind"]["layers"].as_array()).flatten().find(|item| item["id"] == layer).unwrap()
        ["source"]["item"]
        .as_u64()
        .unwrap()
}

/// The agent runs `setup` on a two-solid project and reads it; another program changes the
/// file (`change`: a JSON rewrite or a colleague's engine), and the agent runs `step` against its
/// basis. The file is unchanged when the step is refused.
async fn stale(setup: &[(&str, Value)], change: impl FnOnce(&std::path::Path), step: Value) -> Result<Value, Value> {
    let f = Fixture::new(2);
    if !setup.is_empty() {
        let steps: Vec<Value> = setup.iter().map(|(command, params)| json!({"command":command,"params":params})).collect();
        f.ok("run", json!({"steps":steps})).await;
    }
    let (_, _, basis) = f.inspect().await;
    change(&f.file());
    let saved = std::fs::read(f.file()).unwrap();
    let answer = f.call("run", json!({"basis":basis,"steps":[step]})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
    }
    f.shutdown().await;
    answer
}

fn ids() -> (Vec<u8>, u64, u64) {
    let bytes = project(2);
    let solid = layer_ids(&MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["info"])[0];
    let item = source(&bytes, solid);
    (bytes, solid, item)
}

#[tokio::test]
async fn effect_point_defaults_hold_the_source_size_and_the_frame_only_for_a_zero_wide_source() {
    // Gradient Ramp's start and end points default to fractions of the layer's size.
    let (_, solid, item) = ids();
    let ramp = json!({"effect":"Gradient Ramp","layers":[solid]});
    let steps = [
        (Vec::new(), json!({"command":"effect.apply","params":{"effect":"Gradient Ramp","layers":[solid]}})),
        (vec![("effect.apply", ramp.clone())], json!({"command":"effect.applyLast","params":{"layers":[solid]}})),
        (vec![("effect.apply", ramp.clone())], json!({"command":"effect.reset","params":{"layer":solid,"effect":1}})),
    ];
    for (setup, step) in steps {
        let command = step["command"].clone();
        // The solid's size sizes the effect's point defaults: a resized solid refuses.
        let resized = stale(&setup, |file| rewrite(file, |project| project["items"][item.to_string()]["kind"]["width"] = json!(100)), step.clone()).await;
        assert_eq!(resized.expect_err("source size")["error"]["code"], "conflict", "{command}");
        // Its colour does not, and neither does the frame of a composition it is not sized by.
        let recoloured =
            stale(&setup, |file| rewrite(file, |project| project["items"][item.to_string()]["kind"]["color"] = json!([0.0, 0.0, 1.0])), step.clone()).await;
        assert!(recoloured.is_ok(), "{command}: {recoloured:?}");
        let reframed = stale(&setup, |file| rewrite(file, |project| project["items"][COMP]["kind"]["width"] = json!(640)), step.clone()).await;
        assert!(reframed.is_ok(), "{command}: {reframed:?}");
    }
}

#[tokio::test]
async fn an_effect_on_a_text_layer_holds_the_frame_it_is_sized_by() {
    // Text has no source item: effect_bounds falls back to the composition's frame.
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let text = doc.execute("layer.newText", json!({"text":"Title"})).unwrap()["layer"].as_u64().unwrap();
    let setup = [("layer.newText", json!({"text":"Title"}))];
    let step = json!({"command":"effect.apply","params":{"effect":"Gradient Ramp","layers":[text]}});
    let reframed = stale(&setup, |file| rewrite(file, |project| project["items"][COMP]["kind"]["width"] = json!(640)), step.clone()).await;
    assert_eq!(reframed.expect_err("text effects are sized by the frame")["error"]["code"], "conflict");
    let renamed = stale(&setup, |file| rewrite(file, |project| project["items"][COMP]["name"] = json!("Other")), step).await;
    assert!(renamed.is_ok(), "{renamed:?}");
}

#[tokio::test]
async fn a_camera_tool_on_a_named_camera_does_not_hold_the_frame_but_the_active_camera_holds_the_stack_above() {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let camera = doc.execute("layer.newCamera", json!({"name":"Camera"})).unwrap()["layer"].as_u64().unwrap();
    let setup = [("layer.newCamera", json!({"name":"Camera"}))];
    for (command, params) in [("camera.orbit", json!({"dx":10,"dy":5})), ("camera.pan", json!({"dx":10,"dy":5})), ("camera.dolly", json!({"amount":20}))] {
        let mut named = params.clone();
        named["layer"] = json!(camera);
        // The frame enters only as a fallback for a camera without a Position.
        let reframed =
            stale(&setup, |file| rewrite(file, |project| project["items"][COMP]["kind"]["width"] = json!(640)), json!({"command":command,"params":named}))
                .await;
        assert!(reframed.is_ok(), "{command}: {reframed:?}");
        // Without `layer`, the active camera is the topmost video-enabled one: another camera
        // switched on above it would be moved instead.
        let above =
            stale(&setup, |file| drop(colleague(file, &[("layer.newCamera", json!({"name":"Other Camera"}))])), json!({"command":command,"params":params}))
                .await;
        assert_eq!(above.expect_err("another camera became active")["error"]["code"], "conflict", "{command}");
        let below = stale(
            &setup,
            |file| rewrite(file, |project| project["items"][COMP]["kind"]["layers"][2]["name"] = json!("Renamed")),
            json!({"command":command,"params":params}),
        )
        .await;
        assert!(below.is_ok(), "{command}: {below:?}");
    }
}

#[tokio::test]
async fn bringing_a_layer_to_the_front_ignores_layers_added_or_moved_since() {
    let (_, solid, _) = ids();
    let bottom = {
        let doc = MotionDocument::open(FILE, &project(2)).unwrap();
        layer_ids(&doc.inspect().unwrap()["info"])[1]
    };
    for to in ["front", "back"] {
        let step = json!({"command":"layer.arrange","params":{"layers":[bottom],"to":to}});
        let added = stale(&[], |file| drop(colleague(file, &[("layer.newSolid", json!({"name":"Late","color":"#00ff00"}))])), step.clone()).await;
        assert!(added.is_ok(), "{to}: {added:?}");
        let renamed =
            stale(&[], |file| rewrite(file, |project| project["items"][COMP]["kind"]["layers"][0]["name"] = json!(format!("{solid} renamed"))), step).await;
        assert!(renamed.is_ok(), "{to}: {renamed:?}");
    }
}
