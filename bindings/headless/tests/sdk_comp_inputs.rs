mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn center_holds_composition_size_but_ignores_background() {
    for size in [true, false] {
        let f = Fixture::new(2);
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        f.colleague(&[("comp.settings", if size { json!({"width":640,"anchor":0}) } else { json!({"background":"#ff0000"}) })]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.transform","params":{"layers":[layer],"op":"center"}}]})).await;
        if size {
            assert_eq!(answer.expect_err("center recomputed from changed size")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}
