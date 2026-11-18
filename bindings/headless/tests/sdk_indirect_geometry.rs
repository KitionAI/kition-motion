mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn prop(value: &Value, name: &str) -> Option<u64> {
    if value.get("match").and_then(Value::as_str) == Some(name) {
        return value.get("uid")?.as_u64();
    }
    match value {
        Value::Object(map) => map.values().find_map(|value| prop(value, name)),
        Value::Array(items) => items.iter().find_map(|value| prop(value, name)),
        _ => None,
    }
}

#[tokio::test]
async fn applied_track_holds_the_stored_targets_parent_matrix() {
    let mut d = MotionDocument::open(FILE, &project(1)).unwrap();
    let source = layer_ids(&d.inspect().unwrap()["info"])[0];
    let target = d.execute("layer.newNull", json!({"name":"Target"})).unwrap()["layer"].as_u64().unwrap();
    let parent = d.execute("layer.newNull", json!({"name":"Parent"})).unwrap()["layer"].as_u64().unwrap();
    d.execute("layer.setParent", json!({"layers":[target],"parent":parent,"compensate":false})).unwrap();
    let tracker =
        d.execute("track.new", json!({"layer":source,"target":target,"kind":"transform","position":true,"rotation":false,"scale":false})).unwrap()["tracker"]
            .clone();
    let project: Value = serde_json::from_slice(&d.save(None).unwrap()).unwrap();
    let attach = prop(&project, "attachPoint").unwrap();
    for (time, value) in [(0, [8, 8]), (1, [10, 10])] {
        d.execute("prop.addKey", json!({"layer":source,"prop":attach,"time":time,"value":value})).unwrap();
    }
    let bytes = d.save(None).unwrap();
    for matrix in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if matrix { ("prop.set", json!({"layer":parent,"path":"transform/scale","value":[200,200]})) } else { opacity(parent, 23.0) }]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer =
            f.call("run", json!({"basis":basis,"steps":[{"command":"track.apply","params":{"layer":source,"tracker":tracker,"dimensions":"xy"}}]})).await;
        if matrix {
            assert_eq!(answer.expect_err("track output must not be recomputed through a changed parent")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), parent), 23.0);
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn auto_trace_holds_nested_pixels_but_ignores_a_nested_comment() {
    let mut d = MotionDocument::open(FILE, &project(0)).unwrap();
    let outer = d.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let inner = d.execute("comp.new", json!({"name":"Inner","width":32,"height":32,"duration":2})).unwrap()["comp"].as_u64().unwrap();
    let child = d.execute("layer.newSolid", json!({"name":"Pixels","width":8,"height":8,"color":"#ff0000"})).unwrap()["layer"].as_u64().unwrap();
    d.execute("comp.open", json!({"comp":outer})).unwrap();
    let layer = d.execute("layer.addItem", json!({"item":inner})).unwrap()["layer"].as_u64().unwrap();
    let bytes = d.save(None).unwrap();
    for pixels in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if pixels {
            ("layer.settings", json!({"comp":inner,"layer":child,"color":"#000000"}))
        } else {
            ("layer.setComment", json!({"comp":inner,"layers":[child],"comment":"Colleague"}))
        }]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f
            .call("run", json!({"basis":basis,"steps":[{"command":"layer.autoTrace","params":{"layer":layer,"channel":"red","blur":0,"minimumArea":0}}]}))
            .await;
        if pixels {
            assert_eq!(answer.expect_err("auto trace must not use unread changed nested pixels")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}
