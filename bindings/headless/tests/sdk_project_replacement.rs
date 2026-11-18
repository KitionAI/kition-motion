mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn replacing_a_project_refuses_unread_settings_and_accepts_a_fresh_basis() {
    for changed in [true, false] {
        let fixture = Fixture::new(1);
        let (_, _, basis) = fixture.inspect().await;
        if changed {
            fixture.colleague(&[("file.projectSettings", json!({"bitDepth":16}))]);
        }
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"file.newProject","params":{}}]})).await;
        if changed {
            assert_eq!(answer.expect_err("new project must not discard an unread settings change")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert!(serde_json::from_slice::<serde_json::Value>(&std::fs::read(fixture.file()).unwrap()).unwrap()["items"].as_object().unwrap().is_empty());
        }
        fixture.shutdown().await;
    }
}
