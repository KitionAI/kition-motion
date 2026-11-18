//! Source replacement reads the replaced item's own data (its size, and for a placeholder its
//! frame rate and duration): a colleague's change to that item since the observation refuses
//! the replacement; a colleague's change to another item does not.
mod common;
use common::*;
use serde_json::{Value, json};

fn item_of(project: &Value, item: &Value) -> Value {
    project["items"][item.to_string()].clone()
}

fn saved_item(fixture: &Fixture, item: &Value) -> Value {
    item_of(&serde_json::from_slice(&std::fs::read(fixture.file()).unwrap()).unwrap(), item)
}

#[tokio::test]
async fn source_replacement_holds_the_replaced_items_data_but_not_another_items() {
    for command in ["file.replaceWithSolid", "file.replaceWithPlaceholder"] {
        for replaced_changed in [true, false] {
            let fixture = Fixture::new(2);
            let (info, _, basis) = fixture.inspect().await;
            let layers = layer_ids(&info);
            let source = fixture.ok("query", json!({"objects":[layers[0]]})).await["objects"][0]["fields"]["sourceItem"].clone();
            let other = fixture.ok("query", json!({"objects":[layers[1]]})).await["objects"][0]["fields"]["sourceItem"].clone();
            let resized = if replaced_changed { layers[0] } else { layers[1] };
            fixture.colleague(&[("layer.settings", json!({"layer":resized,"width":96,"height":54}))]);
            let saved = std::fs::read(fixture.file()).unwrap();
            let other_after_colleague = saved_item(&fixture, &other);
            let mut params = json!({"items":[source]});
            if command == "file.replaceWithSolid" {
                params["color"] = json!("#445566");
            }
            let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
            if replaced_changed {
                let refused = answer.expect_err("the replaced item was resized since observation");
                assert_eq!(refused["error"]["code"], "conflict", "{command}: {refused}");
                assert_eq!(std::fs::read(fixture.file()).unwrap(), saved, "{command}");
            } else {
                assert!(answer.is_ok(), "{command}: {answer:?}");
                assert_ne!(std::fs::read(fixture.file()).unwrap(), saved, "{command} wrote nothing");
                assert_eq!(saved_item(&fixture, &other), other_after_colleague, "{command} kept the colleague's resize of another item");
            }
            fixture.shutdown().await;
        }
    }
}
