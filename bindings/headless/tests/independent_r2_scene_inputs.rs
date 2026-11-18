mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn scene() -> (Vec<u8>, u64, u64) {
    let mut document = MotionDocument::create(FILE, 32, 32).unwrap();
    document.execute("comp.settings", json!({"duration":0.2})).unwrap();
    let layer = document.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
    let bytes = document.save(None).unwrap();
    let project: Value = serde_json::from_slice(&bytes).unwrap();
    let source = project["items"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|item| item["kind"]["layers"].as_array())
        .flatten()
        .find(|candidate| candidate["id"] == layer)
        .unwrap()["source"]["item"]
        .as_u64()
        .unwrap();
    (bytes, layer, source)
}

#[tokio::test]
async fn scene_detection_holds_default_comp_and_source_inputs_but_not_source_comment() {
    let (bytes, layer, source) = scene();
    for change in ["rate", "source", "comment"] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, comp, basis) = fixture.inspect().await;
        match change {
            "rate" => {
                fixture.colleague(&[("comp.settings", json!({"comp":comp,"frameRate":15}))]);
            }
            "source" => {
                fixture.colleague(&[("layer.settings", json!({"layer":layer,"width":128}))]);
            }
            "comment" => rewrite(&fixture.file(), |project| {
                project["items"][source.to_string()]["comment"] = json!("independent note");
            }),
            _ => unreachable!(),
        }
        let result = fixture
            .call("run", json!({"basis":basis,"steps":[{"command":"layer.sceneEditDetection","params":{"layer":layer,"mode":"markers","wait":true}}]}))
            .await;
        if change == "comment" {
            assert!(result.is_ok(), "scene pixels do not depend on source-item comment: {result:?}");
        } else {
            let conflict = result.unwrap_err();
            assert_conflict(&conflict, &if change == "rate" { comp } else { source }.to_string(), "changed");
        }
        fixture.shutdown().await;
    }
}
