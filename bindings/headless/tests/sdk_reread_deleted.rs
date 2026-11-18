mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn deleted_recipient_reason_survives_an_intervening_read() {
    for reread_before_write in [false, true] {
        let f = Fixture::new(2);
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        f.colleague(&[("edit.clear", json!({"layers":[layer]}))]);
        if reread_before_write {
            f.inspect().await;
        }
        let saved = std::fs::read(f.file()).unwrap();
        let error = f
            .call("run", json!({"basis":basis,"steps":[{"command":"layer.rename","params":{"layer":layer,"name":"Mine"}}]}))
            .await
            .expect_err("an earlier read must not forget a deleted recipient");
        assert_conflict(&error, &layer.to_string(), "deleted");
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        f.shutdown().await;
    }
}

#[tokio::test]
async fn replayed_key_creation_keeps_its_returned_reference() {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    doc.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":1,"value":10})).unwrap();
    let f = Fixture::from_bytes(&doc.save(None).unwrap(), Default::default());
    f.inspect().await;
    let groups = f.ok("query", json!({"objects":[layer],"follow":"props.children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let properties = f.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    let property = properties["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "opacity").unwrap();
    let owner = property["id"].clone();
    let old = f.ok("query", json!({"objects":[property["fields"]["keys"][0]]})).await;
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("comp.settings", json!({"background":"#ff0000"}))]);
    let result = f.ok("batch", json!({"basis":basis,"ops":[
        {"op":"create","type":"Key","ref":"$new","fields":{"time":(2_i64*254_016_000_000).to_string(),"value":{"t":"Scalar","v":20},"data":old["objects"][0]["fields"]["data"]}},
        {"op":"list.insert","target":owner,"field":"keys","index":1,"item":"$new"},
        {"op":"set","target":"$new","field":"value","value":{"t":"Scalar","v":77}}
    ]})).await;
    let created = result["refs"]["$new"].clone();
    assert!(created.is_string(), "{result}");
    let current = f.ok("query", json!({"objects":[owner]})).await;
    assert!(current["objects"][0]["fields"]["keys"].as_array().unwrap().contains(&created), "{result}: {current}");
    let queried = f.ok("query", json!({"objects":[created]})).await;
    assert_eq!(queried["objects"][0]["fields"]["value"]["v"], 77);
    assert!(keys_on_disk(&f.file(), layer, OPACITY).contains(&(2.0, json!(77.0))));
    f.shutdown().await;
}
