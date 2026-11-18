mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn fixture() -> (Vec<u8>, u64, u64) {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    for (time, value) in [(0, 10), (1, 20), (4, 40)] {
        document.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":time,"value":value})).unwrap();
    }
    document.execute("keys.setEase", json!({"layer":layer,"path":OPACITY,"time":1,"side":"out","speed":1})).unwrap();
    let property = document.execute("prop.get", json!({"layer":layer,"path":OPACITY})).unwrap()["uid"].as_u64().unwrap();
    (document.save(None).unwrap(), layer, property)
}

async fn apply_after(command: &str, change: &str, named: bool, refuse: bool) {
    let (bytes, layer, property) = fixture();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = fixture.inspect().await;
    match change {
        "delete" => {
            fixture.colleague(&[("keys.select", json!({"keys":[{"layer":layer,"path":OPACITY,"time":1}]})), ("keys.delete", json!({}))]);
        }
        "nearer" => {
            fixture.colleague(&[("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":1.1,"value":30}))]);
        }
        "distant" => {
            fixture.colleague(&[("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":3,"value":30}))]);
        }
        "distant-value" => {
            fixture.colleague(&[("prop.set", json!({"layer":layer,"path":OPACITY,"time":4,"value":90}))]);
        }
        "distant-delete" => {
            fixture.colleague(&[("keys.select", json!({"keys":[{"layer":layer,"path":OPACITY,"time":4}]})), ("keys.delete", json!({}))]);
        }
        _ => {
            fixture.colleague(&[("prop.set", json!({"layer":layer,"path":"transform/rotation","value":45}))]);
        }
    }
    let before = std::fs::read(fixture.file()).unwrap();
    let mut params =
        if command == "keys.setEase" { json!({"layer":layer,"time":1.2,"side":"out","speed":2}) } else { json!({"layer":layer,"time":1.2,"value":77}) };
    if named {
        params["path"] = json!(OPACITY);
    } else {
        params["prop"] = json!(property);
    }
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
    if refuse {
        let refusal = answer.unwrap_err();
        assert_eq!(refusal["error"]["code"], "conflict", "{refusal}");
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before, "refused write changed the saved file");
    } else {
        assert!(answer.is_ok(), "{answer:?}");
        let saved: Value = serde_json::from_slice(&std::fs::read(fixture.file()).unwrap()).unwrap();
        fn property_with_uid(value: &Value, uid: u64) -> Option<&Value> {
            if value["uid"] == json!(uid) {
                return Some(value);
            }
            match value {
                Value::Object(fields) => fields.values().find_map(|value| property_with_uid(value, uid)),
                Value::Array(values) => values.iter().find_map(|value| property_with_uid(value, uid)),
                _ => None,
            }
        }
        let keys = property_with_uid(&saved, property).unwrap()["keys"].as_array().unwrap();
        let nearest = keys.iter().find(|key| key["time"] == json!(254_016_000_000_i64)).unwrap();
        if command == "keys.setEase" {
            assert_eq!(nearest["out_ease"][0]["speed"], json!(2.0), "{nearest}");
        } else {
            assert_eq!(nearest["value"]["v"], json!(77.0), "{nearest}");
        }
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn nearest_key_address_refuses_deleted_or_replaced_destination() {
    for command in ["keys.setEase", "keys.set"] {
        for named in [false, true] {
            apply_after(command, "delete", named, true).await;
            apply_after(command, "nearer", named, true).await;
        }
    }
}

#[tokio::test]
async fn named_stored_ease_accepts_changes_that_keep_its_destination_and_inputs() {
    for change in ["distant", "distant-value", "distant-delete", "other-property"] {
        apply_after("keys.setEase", change, true, false).await;
    }
}

#[tokio::test]
async fn uid_stored_ease_accepts_changes_that_keep_its_destination_and_inputs() {
    for change in ["distant", "distant-value", "distant-delete", "other-property"] {
        apply_after("keys.setEase", change, false, false).await;
    }
}

#[tokio::test]
async fn nearest_key_value_accepts_changes_that_keep_its_destination_and_inputs() {
    for named in [false, true] {
        for change in ["distant", "distant-value", "distant-delete", "other-property"] {
            apply_after("keys.set", change, named, false).await;
        }
    }
}

#[tokio::test]
async fn nearest_named_property_refuses_when_equal_named_siblings_trade_places() {
    let (bytes, layer, property) = fixture();
    fn transform_children(project: &mut Value) -> &mut Vec<Value> {
        let comp = project["items"].as_object_mut().unwrap().values_mut().find(|item| item["kind"]["layers"].is_array()).unwrap();
        let groups = comp["kind"]["layers"][0]["props"]["children"].as_array_mut().unwrap();
        let transform = groups.iter_mut().find(|group| group["match"] == "transform").unwrap();
        transform["children"].as_array_mut().unwrap()
    }
    let mut project: Value = serde_json::from_slice(&bytes).unwrap();
    let children = transform_children(&mut project);
    let index = children.iter().position(|node| node["uid"] == json!(property)).unwrap();
    let mut duplicate = children[index].clone();
    duplicate["uid"] = json!(200_000);
    children.insert(index + 1, duplicate);
    let fixture = Fixture::from_bytes(&serde_json::to_vec(&project).unwrap(), Default::default());
    let (_, _, basis) = fixture.inspect().await;
    rewrite(&fixture.file(), |project| transform_children(project).swap(index, index + 1));
    let before = std::fs::read(fixture.file()).unwrap();
    let answer = fixture
        .call("run", json!({"basis":basis,"steps":[{"command":"keys.setEase","params":{"layer":layer,"path":OPACITY,"time":1.2,"side":"out","speed":2}}]}))
        .await;
    assert_eq!(answer.unwrap_err()["error"]["code"], "conflict");
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    fixture.shutdown().await;
}
