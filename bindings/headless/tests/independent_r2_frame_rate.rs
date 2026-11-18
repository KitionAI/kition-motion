//! Layer writes that snap to the composition's frame grid rest on its frame rate (object-class
//! comparison, class `comp:unheld`): a colleague's rate change refuses them, an unrelated
//! composition change does not.
mod common;

use common::*;
use serde_json::{Value, json};

async fn check(params: Value, change: (&str, Value), refuse: bool) {
    check_command("layer.timing", params, change, refuse).await;
}

async fn check_command(command: &str, params: Value, change: (&str, Value), refuse: bool) {
    let fixture = Fixture::new(2);
    let (info, comp, basis) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    let mut params = params;
    if command == "layer.hop" {
        params["layer"] = json!(layer);
    } else {
        params["layers"] = json!([layer]);
    }
    let mut change = change;
    change.1["comp"] = json!(comp);
    fixture.colleague(&[("time.set", json!({"time":0.1})), change.clone()]);
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params,"at":{"time":0.1}}]})).await;
    if refuse {
        let error = answer.expect_err("the frame grid the write snaps to changed");
        assert_eq!(error["error"]["code"], "conflict", "{command} {params} after {change:?}: {error}");
    } else {
        assert!(answer.is_ok(), "{command} {params} refused an unrelated composition change {change:?}: {answer:?}");
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn snapped_timing_holds_the_frame_rate() {
    for params in [json!({"start":0.1}), json!({"in":0.1}), json!({"op":"trimInToTime"}), json!({"op":"trimOutToTime"})] {
        check(params.clone(), ("comp.settings", json!({"frameRate":24})), true).await;
        check(params, ("comp.settings", json!({"background":"#123456"})), false).await;
    }
}

#[tokio::test]
async fn moves_to_the_current_time_do_not_read_the_frame_rate() {
    check(json!({"op":"moveInToTime"}), ("comp.settings", json!({"frameRate":24})), false).await;
}

#[tokio::test]
async fn frame_based_layer_edits_hold_the_frame_rate() {
    for (command, params) in [("layer.timeStretch", json!({"percent":200})), ("layer.timeReverse", json!({})), ("layer.hop", json!({"to":[40,40]}))] {
        check_command(command, params.clone(), ("comp.settings", json!({"frameRate":24})), true).await;
        check_command(command, params, ("comp.settings", json!({"background":"#123456"})), false).await;
    }
}
