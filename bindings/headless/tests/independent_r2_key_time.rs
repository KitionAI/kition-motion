mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

#[tokio::test]
async fn explicit_prop_set_holds_its_snapped_key_and_accepts_an_unrelated_key_edit() {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    for (time, value) in [(0.0, 10.0), (1.0 / 30.0, 20.0), (1.0, 30.0)] {
        doc.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":time,"value":value})).unwrap();
    }
    let comp = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let mut project: Value = serde_json::from_slice(&doc.save(None).unwrap()).unwrap();
    let props = project["items"][comp.to_string()]["kind"]["layers"][0]["props"]["children"].as_array_mut().unwrap();
    let transform = props.iter_mut().find(|group| group["match"] == "transform").unwrap();
    let opacity = transform["children"].as_array_mut().unwrap().iter_mut().find(|property| property["match"] == "opacity").unwrap();
    let keys = opacity["keys"].as_array_mut().unwrap();
    let second = keys.last().unwrap()["time"].as_i64().unwrap();
    let mut off_grid = keys[0].clone();
    off_grid["time"] = json!(second / 1000 * 17);
    keys.insert(1, off_grid);
    let bytes = serde_json::to_vec(&project).unwrap();
    for (must_refuse, time) in [(true, 1.0 / 30.0), (false, 1.0)] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[("prop.set", json!({"layer":layer,"path":OPACITY,"time":time,"value":45}))]);
        let answer =
            fixture.call("run", json!({"basis":basis,"steps":[{"command":"prop.set","params":{"layer":layer,"path":OPACITY,"time":0.017,"value":77}}]})).await;
        if must_refuse {
            let refusal = answer.unwrap_err();
            assert!(refusal["conflicts"].as_array().unwrap().iter().any(|conflict| conflict["reason"] == "changed"), "{refusal}");
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn explicit_key_time_holds_frame_rate_and_accepts_background_change() {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    doc.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":0,"value":10})).unwrap();
    let bytes = doc.save(None).unwrap();
    for (must_refuse, change) in [(true, json!({"frameRate":60})), (false, json!({"background":"#123456"}))] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[("comp.settings", change)]);
        let answer =
            fixture.call("run", json!({"basis":basis,"steps":[{"command":"prop.set","params":{"layer":layer,"path":OPACITY,"time":0.017,"value":77}}]})).await;
        if must_refuse {
            let refusal = answer.unwrap_err();
            assert!(refusal["conflicts"].as_array().unwrap().iter().any(|conflict| conflict["reason"] == "changed"), "{refusal}");
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        fixture.shutdown().await;
    }
}

async fn ease_after_colleague_change(change: &str, named: bool, must_refuse: bool) {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    for (time, value) in [(0, 10), (1, 20)] {
        doc.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":time,"value":value})).unwrap();
    }
    doc.execute("keyframe.setEase", json!({"layer":layer,"path":OPACITY,"time":1,"side":"out","speed":1})).unwrap();
    let property = doc.execute("prop.get", json!({"layer":layer,"path":OPACITY})).unwrap()["uid"].as_u64().unwrap();
    let bytes = doc.save(None).unwrap();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = fixture.inspect().await;
    match change {
        "delete" => {
            fixture.colleague(&[("keyframe.select", json!({"keys":[{"layer":layer,"path":OPACITY,"time":1}]})), ("keyframe.delete", json!({}))]);
        }
        "ease" => {
            fixture.colleague(&[("keyframe.setEase", json!({"layer":layer,"path":OPACITY,"time":1,"side":"out","speed":9}))]);
        }
        _ => {
            fixture.colleague(&[("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":3,"value":30}))]);
        }
    }
    let mut params = json!({"layer":layer,"time":1.2,"side":"out","speed":2});
    if named {
        params["path"] = json!(OPACITY);
    } else {
        params["prop"] = json!(property);
    }
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"keyframe.setEase","params":params}]})).await;
    if must_refuse {
        let refusal = answer.unwrap_err();
        assert_eq!(refusal["error"]["code"], "conflict", "{refusal}");
    } else {
        assert!(answer.is_ok(), "{answer:?}");
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn stored_ease_uses_only_the_stored_side_when_complete() {
    ease_after_colleague_change("ease", false, true).await;
    ease_after_colleague_change("distant", false, false).await;
}

#[tokio::test]
async fn nearest_ease_key_membership() {
    ease_after_colleague_change("delete", false, true).await;
}

#[tokio::test]
async fn named_property_metadata_ignores_distant_key_insertions() {
    ease_after_colleague_change("distant", true, false).await;
}
