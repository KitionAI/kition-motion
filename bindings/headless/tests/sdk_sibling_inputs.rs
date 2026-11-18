mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn duplicate_holds_sibling_names_but_ignores_sibling_opacity() {
    for names in [true, false] {
        let f = Fixture::new(2);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        f.colleague(&[if names { ("layer.rename", json!({"layer":layers[1],"name":"L 2"})) } else { opacity(layers[1], 23.0) }]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"edit.duplicate","params":{"layers":[layers[0]]}}]})).await;
        if names {
            assert_eq!(answer.expect_err("copy naming changed")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), layers[1]), 23.0);
        }
        f.shutdown().await;
    }
}
