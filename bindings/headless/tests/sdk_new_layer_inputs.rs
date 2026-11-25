mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

/// Runs `steps` on a stale basis after another program rewrote one composition attribute. A
/// refused run leaves the file as that program saved it.
async fn stale(bytes: &[u8], field: &str, value: Value, steps: Value) -> Result<Value, Value> {
    let f = Fixture::from_bytes(bytes, Default::default());
    let (_, comp, basis) = f.inspect().await;
    rewrite(&f.file(), |project| project["items"][comp.to_string()]["kind"][field] = value);
    let saved = std::fs::read(f.file()).unwrap();
    let answer = f.call("run", json!({"basis":basis,"steps":steps})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
    }
    f.shutdown().await;
    answer
}

async fn refuses_and_applies(bytes: &[u8], read: (&str, Value), unrelated: (&str, Value), steps: Value) {
    let refused = stale(bytes, read.0, read.1, steps.clone()).await;
    assert_eq!(refused.expect_err("the write used an unread composition attribute")["error"]["code"], "conflict", "{steps}");
    let applied = stale(bytes, unrelated.0, unrelated.1, steps.clone()).await;
    assert!(applied.is_ok(), "{steps}: {applied:?}");
}

fn red() -> (&'static str, Value) {
    ("background", json!([1.0, 0.0, 0.0]))
}

fn with(steps: &[(&str, Value)]) -> (Vec<u8>, Vec<Value>) {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let answers = steps.iter().map(|(command, params)| doc.execute(command, params.clone()).unwrap()).collect();
    (doc.save(None).unwrap(), answers)
}

#[tokio::test]
async fn a_camera_from_the_view_holds_frame_size_and_duration() {
    let bytes = project(1);
    let steps = json!([{"command":"view.set3DView","params":{"view":"custom1"}},{"command":"camera.fromView"}]);
    refuses_and_applies(&bytes, ("duration", json!(254_016_000_000_i64 * 4)), red(), steps.clone()).await;
    refuses_and_applies(&bytes, ("width", json!(640)), red(), steps).await;
}

#[tokio::test]
async fn a_spotlight_holds_the_frame_it_builds_its_solid_from() {
    let bytes = project(1);
    let layer = layer_ids(&MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["info"])[0];
    let steps = json!([{"command":"light.spotlight","params":{"target":layer}}]);
    refuses_and_applies(&bytes, ("height", json!(90)), red(), steps.clone()).await;
    refuses_and_applies(&bytes, ("frame_rate", json!({"num":24,"den":1})), red(), steps).await;
}

#[tokio::test]
async fn a_light_kind_change_holds_the_frame_size_but_a_setting_does_not() {
    let (bytes, made) = with(&[("layer.newLight", json!({"kind":"Spot","name":"Key"}))]);
    let light = made[0]["layer"].as_u64().unwrap();
    refuses_and_applies(&bytes, ("width", json!(640)), red(), json!([{"command":"layer.lightSettings","params":{"layer":light,"kind":"Point"}}])).await;
    let setting = stale(&bytes, "width", json!(640), json!([{"command":"layer.lightSettings","params":{"layer":light,"intensity":80}}])).await;
    assert!(setting.is_ok(), "{setting:?}");
}

#[tokio::test]
async fn a_lift_shadow_ramp_holds_the_frame_rate_but_explicit_keys_do_not() {
    let bytes = project(1);
    let layer = layer_ids(&MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["info"])[0];
    let ramp = json!([{"command":"layer.liftShadow","params":{"layer":layer,"elevation":6,"to":12,"frames":18,"start":0}}]);
    refuses_and_applies(&bytes, ("frame_rate", json!({"num":24,"den":1})), red(), ramp).await;
    let keys = json!([{"command":"layer.liftShadow","params":{"layer":layer,"keys":[[0,6],[1,12]]}}]);
    let applied = stale(&bytes, "frame_rate", json!({"num":24,"den":1}), keys).await;
    assert!(applied.is_ok(), "{applied:?}");
}

#[tokio::test]
async fn masks_from_text_hold_the_frame_size() {
    let (bytes, made) = with(&[("layer.newText", json!({"text":"Title"}))]);
    let text = made[0]["layer"].as_u64().unwrap();
    refuses_and_applies(&bytes, ("height", json!(90)), red(), json!([{"command":"layer.add","params":{"op":"masksFromText","layers":[text]}}])).await;
}

#[tokio::test]
async fn path_nulls_hold_the_frame_and_duration_their_nulls_take() {
    let bytes = project(1);
    let layer = layer_ids(&MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["info"])[0];
    let mut doc = MotionDocument::open(FILE, &bytes).unwrap();
    doc.execute("mask.new", json!({"layer":layer,"vertices":[[0,0],[100,0],[100,100]],"closed":true})).unwrap();
    let bytes = doc.save(None).unwrap();
    for command in ["path.nullsFollowPoints", "path.pointsFollowNulls", "path.tracePath"] {
        refuses_and_applies(&bytes, ("duration", json!(254_016_000_000_i64 * 4)), red(), json!([{"command":command,"params":{"layer":layer}}])).await;
    }
}

#[tokio::test]
async fn select_next_holds_whether_shy_layers_are_hidden() {
    let (bytes, _) = with(&[("layer.newSolid", json!({"name":"Top"}))]);
    let steps = json!([{"command":"layer.selectNext"}]);
    refuses_and_applies(&bytes, ("hide_shy", json!(true)), red(), steps.clone()).await;
    refuses_and_applies(&bytes, ("hide_shy", json!(true)), red(), json!([{"command":"layer.selectPrevious"}])).await;
}

#[tokio::test]
async fn path_nulls_hold_sibling_names_and_the_parent_chain() {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let ids = layer_ids(&doc.inspect().unwrap()["info"]);
    let (layer, other) = (ids[1], ids[0]);
    doc.execute("mask.new", json!({"layer":layer,"vertices":[[0,0],[100,0],[100,100]],"closed":true})).unwrap();
    let parent = doc.execute("layer.newNull", json!({"name":"Rig"})).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.setParent", json!({"layers":[layer],"parent":parent})).unwrap();
    let bytes = doc.save(None).unwrap();
    for command in ["path.nullsFollowPoints", "path.pointsFollowNulls", "path.tracePath"] {
        let mut scratch = MotionDocument::open(FILE, &bytes).unwrap();
        let made = scratch.execute(command, json!({"layer":layer})).unwrap();
        let null = made.get("null").or_else(|| made["nulls"].get(0)).and_then(Value::as_u64).unwrap();
        let taken = scratch.inspect().unwrap()["info"]["layers"].as_array().unwrap().iter().find(|entry| entry["id"] == null).unwrap()["name"].clone();
        for (change, refuse) in [
            (("layer.rename", json!({"layer":other,"name":taken})), true),
            (("prop.set", json!({"layer":parent,"path":"transform/position","value":[50,60]})), true),
            (opacity(other, 23.0), false),
        ] {
            let f = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = f.inspect().await;
            f.colleague(&[(change.0, change.1.clone())]);
            let saved = std::fs::read(f.file()).unwrap();
            let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":{"layer":layer}}]})).await;
            if refuse {
                assert_eq!(answer.expect_err("the nulls used an unread name or parent")["error"]["code"], "conflict", "{command} after {}", change.0);
                assert_eq!(std::fs::read(f.file()).unwrap(), saved);
            } else {
                assert!(answer.is_ok(), "{command}: {answer:?}");
            }
            f.shutdown().await;
        }
    }
}
