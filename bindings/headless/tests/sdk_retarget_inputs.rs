mod common;
use common::*;
use serde_json::{Value, json};

fn layer_source(project: &Value, layer: u64) -> Value {
    project["items"].as_object().unwrap().values().filter_map(|item| item["kind"]["layers"].as_array()).flatten().find(|item| item["id"] == layer).unwrap()["source"].clone()
}
fn retarget(project: &mut Value, layer: u64, source: Value) {
    for item in project["items"].as_object_mut().unwrap().values_mut() {
        if let Some(layers) = item["kind"]["layers"].as_array_mut() {
            for item in layers {
                if item["id"] == layer {
                    item["source"] = source.clone();
                }
            }
        }
    }
}
#[tokio::test]
async fn source_replacement_holds_its_users_but_preserves_user_opacity() {
    for command in ["file.replaceWithSolid", "file.replaceWithPlaceholder"] {
        for source_changed in [true, false] {
            let fixture = Fixture::new(2);
            let (info, _, basis) = fixture.inspect().await;
            let layers = layer_ids(&info);
            let source = fixture.ok("query", json!({"objects":[layers[0]]})).await["objects"][0]["fields"]["sourceItem"].clone();
            if source_changed {
                rewrite(&fixture.file(), |project| {
                    let other = layer_source(project, layers[1]);
                    retarget(project, layers[0], other);
                });
            } else {
                fixture.colleague(&[opacity(layers[0], 23.0)]);
            }
            let saved = std::fs::read(fixture.file()).unwrap();
            let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":{"items":[source],"width":320,"height":180}}]})).await;
            if source_changed {
                assert_eq!(answer.expect_err("the source user changed since observation")["error"]["code"], "conflict");
                assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
            } else {
                assert!(answer.is_ok(), "{answer:?}");
                assert_eq!(opacity_on_disk(&fixture.file(), layers[0]), 23.0);
            }
            fixture.shutdown().await;
        }
    }
}
