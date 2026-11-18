//! Layer writes that size their result from the layer's source item rest on that item's size
//! (object-class comparison, class `items:unheld`): another program resizing the solid refuses
//! them, a change of the solid's colour does not.
mod common;

use common::*;
use serde_json::{Value, json};

async fn check(command: &str, params: Value, change: impl FnOnce(&mut Value, u64), refuse: bool) {
    let fixture = Fixture::new(2);
    let (info, _, basis) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    let mut params = params;
    params["layers"] = json!([layer]);
    rewrite(&fixture.file(), |project| {
        let comp = project["items"].as_object().unwrap().values().find(|item| item["kind"]["layers"].is_array()).unwrap().clone();
        let source = comp["kind"]["layers"].as_array().unwrap().iter().find(|candidate| candidate["id"] == layer).unwrap()["source"]["item"].as_u64().unwrap();
        change(project, source);
    });
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
    if refuse {
        let error = answer.expect_err("the source size the write derives from changed");
        assert_eq!(error["error"]["code"], "conflict", "{command}: {error}");
    } else {
        assert!(answer.is_ok(), "{command} refused an unrelated change of its source: {answer:?}");
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn effect_defaults_hold_the_source_size() {
    let resize = |project: &mut Value, item: u64| project["items"][item.to_string()]["kind"]["width"] = json!(100);
    let recolour = |project: &mut Value, item: u64| project["items"][item.to_string()]["kind"]["color"] = json!([0.0, 0.0, 1.0]);
    for (command, params) in [("effect.apply", json!({"effect":"Gradient Ramp"})), ("layer.transform", json!({"op":"fit"}))] {
        check(command, params.clone(), resize, true).await;
        check(command, params, recolour, false).await;
    }
}
