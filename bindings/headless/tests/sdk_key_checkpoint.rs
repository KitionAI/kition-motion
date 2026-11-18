mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn opacity_prop(value: &mut Value) -> Option<&mut Value> {
    if value.get("match").and_then(Value::as_str) == Some("opacity") {
        return Some(value);
    }
    match value {
        Value::Object(map) => map.values_mut().find_map(opacity_prop),
        Value::Array(items) => items.iter_mut().find_map(opacity_prop),
        _ => None,
    }
}

#[tokio::test]
async fn waiting_for_sdk_key_value_checkpoint_after_an_earlier_key_deletion() {
    let mut d = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&d.inspect().unwrap()["info"])[0];
    for t in [1, 4, 6, 8] {
        d.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":t,"value":t*10})).unwrap();
    }
    let f = Fixture::from_bytes(&d.save(None).unwrap(), Default::default());
    f.inspect().await;
    let groups = f.ok("query", json!({"objects":[layer],"follow":"props.children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|o| o["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let props = f.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    let property = props["objects"].as_array().unwrap().iter().find(|o| o["fields"]["meta"]["match"] == "opacity").unwrap();
    let key = property["fields"]["keys"][1].clone();
    f.ok("query", json!({"objects":[key]})).await;
    let (_, _, basis) = f.inspect().await;
    rewrite(&f.file(), |project| {
        opacity_prop(project).unwrap()["keys"].as_array_mut().unwrap().remove(0);
    });
    let current = f.ok("query", json!({"objects":[key]})).await;
    assert_eq!(
        current["objects"][0]["fields"]["time"],
        json!((4_i64 * 254_016_000_000).to_string()),
        "the retained id must still name the observed key: {current}"
    );
    assert_eq!(current["objects"][0]["fields"]["value"]["v"], 40, "{current}");
    let answer = f.call("batch", json!({"basis":basis,"ops":[{"op":"set","target":key,"field":"value","value":{"t":"Scalar","v":77}}]})).await;
    assert!(answer.is_ok(), "the observed key's value did not change: {answer:?}");
    assert!(keys_on_disk(&f.file(), layer, OPACITY).iter().any(|(time, value)| *time == 4.0 && value.as_f64() == Some(77.0)));
    f.shutdown().await;
}
