mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn deleting_a_source_holds_consumers_it_erases_and_preserves_another_layer() {
    for dependent in [true, false] {
        let fixture = Fixture::new(2);
        let (info, _, basis) = fixture.inspect().await;
        let layers = layer_ids(&info);
        let source = fixture.ok("query", json!({"objects":[layers[0]]})).await["objects"][0]["fields"]["sourceItem"].clone();
        fixture.colleague(&[opacity(layers[if dependent { 0 } else { 1 }], 23.0)]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"project.delete","params":{"items":[source]}}]})).await;
        if dependent {
            assert_eq!(answer.expect_err("deleting a source must not erase unread consumer content")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&fixture.file(), layers[1]), 23.0);
        }
        fixture.shutdown().await;
    }
}
