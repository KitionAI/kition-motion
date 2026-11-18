//! A switch or marker-lock assignment holds the field it assigns, not its whole layer (object-class
//! comparison, class `target:whole-for-fields`): a colleague's change of another field of the same
//! layer applies, a change of the assigned field refuses (same-field decision).
mod common;

use common::*;
use serde_json::{Value, json};

async fn check(command: &str, params: Value, change: (&str, Value), refuse: bool) {
    let fixture = Fixture::new(2);
    let (info, _, basis) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    let mut params = params;
    params["layers"] = json!([layer]);
    let mut change = change;
    if change.0 == "layer.setComment" {
        change.1["layer"] = json!(layer);
    } else {
        change.1["layers"] = json!([layer]);
    }
    fixture.colleague(&[change.clone()]);
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
    if refuse {
        let error = answer.expect_err("the assigned field changed");
        assert_eq!(error["error"]["code"], "conflict", "{command} {params} after {change:?}: {error}");
    } else {
        assert!(answer.is_ok(), "{command} {params} refused an unrelated change {change:?}: {answer:?}");
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn an_explicit_switch_holds_only_its_field() {
    let mine = json!({"switch":"shy","value":true});
    check("layer.setSwitch", mine.clone(), ("layer.setSwitch", json!({"switch":"shy","value":true})), true).await;
    check("layer.setSwitch", mine.clone(), ("layer.setSwitch", json!({"switch":"solo","value":true})), false).await;
    check("layer.setSwitch", mine, ("layer.setComment", json!({"comment":"Unrelated"})), false).await;
}

#[tokio::test]
async fn a_single_layer_toggle_holds_only_its_field() {
    let mine = json!({"switch":"motionBlur"});
    check("layer.setSwitch", mine.clone(), ("layer.setSwitch", json!({"switch":"motionBlur","value":true})), true).await;
    check("layer.setSwitch", mine, ("layer.setSwitch", json!({"switch":"solo","value":true})), false).await;
}
