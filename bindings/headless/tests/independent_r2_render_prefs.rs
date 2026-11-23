mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn queued_project() -> (Vec<u8>, u64) {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let queued = document.execute("renderQueue.add", json!({})).unwrap();
    (document.save(None).unwrap(), queued["item"].as_u64().unwrap())
}

/// Render Queue preferences are each machine's own (schema 1, task schema-freeze): another
/// writer's preference never refuses this one's, and an unrelated queue edit applies too.
/// Overflow folders still name files, which the agent may not.
#[tokio::test]
async fn render_preferences_are_local_and_respect_host_file_policy() {
    let (bytes, item) = queued_project();
    let probes = [
        ("renderQueue.setNotify", json!({"notify": true}), "renderQueue.setNotify", json!({"notify": false})),
        ("renderQueue.setOverflowFolders", json!({"folders": ["/colleague"]}), "renderQueue.setOverflowFolders", json!({"folders": ["/mine"]})),
    ];
    for (colleague_command, colleague_params, command, params) in probes {
        for must_refuse in [true, false] {
            let fixture = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = fixture.inspect().await;
            let (other, other_params) =
                if must_refuse { (colleague_command, colleague_params.clone()) } else { ("renderQueue.setLog", json!({"item": item, "log": "plusSettings"})) };
            fixture.colleague(&[(other, other_params)]);
            let answer = fixture.call("run", json!({"basis": basis, "steps": [{"command": command, "params": params}]})).await;
            if command == "renderQueue.setOverflowFolders" {
                let error = answer.unwrap_err();
                assert_eq!(error["error"]["code"], "command-failed", "{error}");
                assert!(error["error"]["message"].as_str().unwrap().contains("commands that name files are not available to the agent"), "{error}");
                assert_eq!(error["completed"], json!([]), "{error}");
            } else {
                assert!(answer.is_ok(), "another writer's preference or queue edit must not refuse it ({must_refuse}): {answer:?}");
            }
            fixture.shutdown().await;
        }
    }
}

#[test]
fn render_preferences_declare_nothing_of_the_document() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    document.execute("renderQueue.add", json!({})).unwrap();
    for (command, params) in [("renderQueue.setNotify", json!({"notify": true})), ("renderQueue.setOverflowFolders", json!({"folders": []}))] {
        let actual = serde_json::to_value(document.under(command, &params).unwrap().unwrap()).unwrap();
        assert!(actual.get("premises").is_none_or(|v| v == &Value::Array(vec![])), "{command}: {actual}");
        assert!(actual.get("objects").is_none_or(|v| v == &Value::Array(vec![])), "{actual}");
    }
}
