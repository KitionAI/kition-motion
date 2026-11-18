mod common;
use common::*;
use serde_json::{Value, json};

/// Runs one stale step after a colleague's `change`; a refused run leaves the file as saved.
async fn stale(layers: usize, change: impl FnOnce(&Fixture, &[u64]), command: &str, params: impl FnOnce(&[u64]) -> Value) -> Result<Value, Value> {
    let f = Fixture::new(layers);
    let (info, _, basis) = f.inspect().await;
    let ids = layer_ids(&info);
    change(&f, &ids);
    let saved = std::fs::read(f.file()).unwrap();
    let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params(&ids)}]})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
    }
    f.shutdown().await;
    answer
}

fn video_off(f: &Fixture, layer: u64) {
    rewrite(&f.file(), |project| {
        for item in project["items"].as_object_mut().unwrap().values_mut() {
            for entry in item["kind"]["layers"].as_array_mut().into_iter().flatten() {
                if entry["id"] == layer {
                    entry["switches"]["video"] = json!(false);
                }
            }
        }
    });
}

#[tokio::test]
async fn bring_forward_holds_the_layer_it_passes() {
    // Top first: L3, L2, L1, L0. Bringing L0 forward passes L1; swapping L2 and L1 keeps L0 where
    // it is but puts L2 next to it.
    let forward = |ids: &[u64]| json!({"layers":[ids[3]],"to":"forward"});
    let passed = stale(4, |f, ids| drop(f.colleague(&[("layer.arrange", json!({"layers":[ids[1]],"to":"backward"}))])), "layer.arrange", forward).await;
    assert_eq!(passed.expect_err("forward would pass a different layer")["error"]["code"], "conflict");
    let above = stale(4, |f, ids| drop(f.colleague(&[("layer.arrange", json!({"layers":[ids[0]],"to":"backward"}))])), "layer.arrange", forward).await;
    assert!(above.is_ok(), "a reorder above the passed layer is independent: {above:?}");
    let backward = |ids: &[u64]| json!({"layers":[ids[0]],"to":"backward"});
    let below = stale(4, |f, ids| drop(f.colleague(&[("layer.arrange", json!({"layers":[ids[2]],"to":"forward"}))])), "layer.arrange", backward).await;
    assert_eq!(below.expect_err("backward would pass a different layer")["error"]["code"], "conflict");
}

#[tokio::test]
async fn a_matte_from_the_layer_above_holds_that_layer_and_its_video_switch() {
    for op in ["above", "alpha"] {
        let params = |ids: &[u64]| json!({"layers":[ids[2]],"op":op});
        let moved = stale(3, |f, ids| drop(f.colleague(&[("layer.arrange", json!({"layers":[ids[0]],"index":2}))])), "layer.trackMatte", params).await;
        assert_eq!(moved.expect_err("another layer is above now")["error"]["code"], "conflict", "{op}");
        let hidden = stale(3, |f, ids| video_off(f, ids[1]), "layer.trackMatte", params).await;
        assert_eq!(hidden.expect_err("the matte's video switch is assigned")["error"]["code"], "conflict", "{op}");
        let unrelated = stale(3, |f, ids| drop(f.colleague(&[opacity(ids[1], 23.0)])), "layer.trackMatte", params).await;
        assert!(unrelated.is_ok(), "{op}: {unrelated:?}");
    }
}

#[tokio::test]
async fn hide_other_video_holds_every_other_video_switch_and_new_layers() {
    let params = |ids: &[u64]| json!({"layers":[ids[0]]});
    let switched = stale(3, |f, ids| video_off(f, ids[2]), "layer.hideOtherVideo", params).await;
    assert_eq!(switched.expect_err("another layer's video switch is assigned")["error"]["code"], "conflict");
    let added = stale(3, |f, _| drop(f.colleague(&[("layer.newSolid", json!({"name":"New"}))])), "layer.hideOtherVideo", params).await;
    assert_eq!(added.expect_err("a new layer would be hidden unseen")["error"]["code"], "conflict");
    let unrelated = stale(3, |f, ids| drop(f.colleague(&[opacity(ids[2], 23.0)])), "layer.hideOtherVideo", params).await;
    assert!(unrelated.is_ok(), "{unrelated:?}");
}
