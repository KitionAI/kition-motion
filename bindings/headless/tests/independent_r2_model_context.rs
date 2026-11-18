mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn model_selection_without_layer_context_accepts_an_unrelated_colleagues_layer_edit() {
    for command in ["face.model.select", "roto.model.select"] {
        let fixture = Fixture::new(1);
        let (info, comp, basis) = fixture.inspect().await;
        let layer = layer_ids(&info)[0];
        fixture.colleague(&[opacity(layer, 23.0)]);
        let answer =
            fixture.call("run", json!({"basis":basis,"target":{"comp":comp},"steps":[{"command":command,"params":{"id":"classical","wait":true}}]})).await;
        assert!(answer.is_ok(), "model preference refused unrelated layer opacity: {answer:?}");
        assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn model_on_is_invalid_without_an_intervening_change() {
    let fixture = Fixture::new(1);
    let (info, _, basis) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    let error = fixture
        .call("run", json!({"basis":basis,"steps":[{"command":"face.model.select","on":[layer],"params":{"id":"classical","wait":true}}]}))
        .await
        .unwrap_err();
    assert_eq!(error["error"]["code"], "invalid-params", "{error}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn waiting_for_sdk_invalid_model_on_is_reported_before_premise_conflict() {
    for command in ["face.model.select", "roto.model.select"] {
        let fixture = Fixture::new(1);
        let (info, comp, basis) = fixture.inspect().await;
        let layer = layer_ids(&info)[0];
        fixture.colleague(&[opacity(layer, 23.0)]);
        let answer = fixture
            .call("run", json!({"basis":basis,"target":{"comp":comp},"steps":[{"command":command,"on":[layer],"params":{"id":"classical","wait":true}}]}))
            .await;
        let error = answer.unwrap_err();
        assert_eq!(error["error"]["code"], "invalid-params", "model command has no object target or selection input: {error}");
        assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
        fixture.shutdown().await;
    }
}
