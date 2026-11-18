mod common;
use common::*;
use serde_json::{Value, json};

async fn stale(change: impl FnOnce(&Fixture, &[u64]), step: impl FnOnce(&[u64]) -> Value) -> (Result<Value, Value>, f64) {
    let f = Fixture::new(2);
    let (info, _, basis) = f.inspect().await;
    let ids = layer_ids(&info);
    change(&f, &ids);
    let saved = std::fs::read(f.file()).unwrap();
    let steps = json!([{"command":"layer.select","params":{"layers":ids}}, step(&ids)]);
    let answer = f.call("run", json!({"basis":basis,"steps":steps})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
    }
    let twin = opacity_on_disk(&f.file(), ids[1]);
    f.shutdown().await;
    (answer, twin)
}

#[tokio::test]
async fn selected_twins_hold_the_property_the_write_assigns() {
    let steps: [fn(&[u64]) -> Value; 3] = [
        |ids| json!({"command":"prop.set","params":{"layer":ids[0],"path":OPACITY,"value":30,"selected":true}}),
        |ids| json!({"command":"prop.toggleAnimation","params":{"layer":ids[0],"path":OPACITY,"selected":true}}),
        |ids| json!({"command":"prop.toggleKey","params":{"layer":ids[0],"path":OPACITY,"selected":true}}),
    ];
    for step in steps {
        let (refused, twin) = stale(|f, ids| drop(f.colleague(&[opacity(ids[1], 23.0)])), step).await;
        assert_eq!(refused.expect_err("a twin's changed opacity would be overwritten")["error"]["code"], "conflict");
        assert_eq!(twin, 23.0);
        let (applied, _) = stale(|f, ids| drop(f.colleague(&[("prop.set", json!({"layer":ids[1],"path":"transform/rotation","value":15}))])), step).await;
        assert!(applied.is_ok(), "{applied:?}");
    }
}
