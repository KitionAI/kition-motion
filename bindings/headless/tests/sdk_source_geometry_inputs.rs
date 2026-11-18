mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn source(bytes: &[u8], layer: u64) -> u64 {
    let project: Value = serde_json::from_slice(bytes).unwrap();
    project["items"].as_object().unwrap().values().filter_map(|item| item["kind"]["layers"].as_array())
        .flatten().find(|item| item["id"] == layer).unwrap()["source"]["item"].as_u64().unwrap()
}

#[tokio::test]
async fn alignment_holds_source_dimensions_but_ignores_source_color() {
    let bytes = project(1);
    let layer = layer_ids(&MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["info"])[0];
    let item = source(&bytes, layer);
    for dimensions in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        rewrite(&f.file(), |project| {
            if dimensions {
                project["items"][item.to_string()]["kind"]["width"] = json!(100);
            } else {
                project["items"][item.to_string()]["kind"]["color"] = json!([0.0, 0.0, 1.0]);
            }
        });
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.align","params":{"layers":[layer],"edge":"right"}}]})).await;
        if dimensions {
            assert_eq!(answer.expect_err("alignment used unread source bounds")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn parent_compensation_holds_source_aspect_but_ignores_source_color() {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let ids = layer_ids(&doc.inspect().unwrap()["info"]);
    doc.execute("prop.set", json!({"layer":ids[0],"path":"transform/rotation","value":35})).unwrap();
    let bytes = doc.save(None).unwrap();
    let item = source(&bytes, ids[0]);
    for aspect in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        rewrite(&f.file(), |project| {
            if aspect {
                project["items"][item.to_string()]["kind"]["pixel_aspect"] = json!(2.0);
            } else {
                project["items"][item.to_string()]["kind"]["color"] = json!([0.0, 0.0, 1.0]);
            }
        });
        let saved = std::fs::read(f.file()).unwrap();
        let answer =
            f.call("run", json!({"basis":basis,"steps":[{"command":"layer.setParent","params":{"layers":[ids[1]],"parent":ids[0],"compensate":true}}]})).await;
        if aspect {
            assert_eq!(answer.expect_err("parent compensation used unread source aspect")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}
