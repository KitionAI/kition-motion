mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn raw_item_deletion_holds_owned_content_and_preserves_another_composition() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let main = document.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let first = layer_ids(&document.inspect().unwrap()["info"])[0];
    let other = document.execute("comp.new", json!({"name":"Other","width":320,"height":180})).unwrap()["comp"].as_u64().unwrap();
    let second = document.execute("layer.newSolid", json!({"name":"Other source"})).unwrap()["layer"].as_u64().unwrap();
    let bytes = document.save(None).unwrap();
    for dependent in [true, false] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[("comp.open", json!({"comp":if dependent { main } else { other }})), opacity(if dependent { first } else { second }, 23.0)]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("batch", json!({"basis":basis,"ops":[{"op":"delete","target":main}],"changes":"all"})).await;
        if dependent {
            assert_eq!(answer.expect_err("deleting a composition must not erase an unread owned-layer change")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            let receipt = answer.unwrap();
            assert!(receipt["changes"]["deleted"].as_array().unwrap().iter().any(|object| object["id"] == main));
            assert_eq!(opacity_on_disk(&fixture.file(), second), 23.0);
        }
        fixture.shutdown().await;
    }
}
