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

#[tokio::test]
async fn render_preferences_hold_the_assigned_field_and_respect_host_file_policy() {
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
            if must_refuse {
                let error = answer.unwrap_err();
                assert_conflict(&error, "project", "changed");
            } else if command == "renderQueue.setOverflowFolders" {
                let error = answer.unwrap_err();
                assert_eq!(error["error"]["code"], "command-failed", "{error}");
                assert!(error["error"]["message"].as_str().unwrap().contains("commands that name files are not available to the agent"), "{error}");
                assert_eq!(error["completed"], json!([]), "{error}");
            } else {
                assert!(answer.is_ok(), "unrelated queue edit must apply: {answer:?}");
            }
            fixture.shutdown().await;
        }
    }
}

#[test]
fn render_preferences_declare_only_the_assigned_project_field() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    document.execute("renderQueue.add", json!({})).unwrap();
    for (command, params, field) in [
        ("renderQueue.setNotify", json!({"notify": true}), "attrs.render_prefs.notify"),
        ("renderQueue.setOverflowFolders", json!({"folders": []}), "attrs.render_prefs.overflow_folders"),
    ] {
        let actual = serde_json::to_value(document.under(command, &params).unwrap().unwrap()).unwrap();
        let premises = actual["premises"].as_array().unwrap();
        assert_eq!(premises.len(), 1, "{command}: {actual}");
        assert_eq!(premises[0]["id"], "project", "{command}: {actual}");
        assert_eq!(premises[0]["fields"], json!([field]), "{command}: {actual}");
        assert_eq!(premises[0]["positions"], json!([]), "{command}: {actual}");
        assert!(actual.get("objects").is_none_or(|v| v == &Value::Array(vec![])), "{actual}");
    }
}
