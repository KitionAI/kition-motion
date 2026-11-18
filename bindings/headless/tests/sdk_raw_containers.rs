mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn raw_layer_moves_keep_the_recipient_and_gap_across_an_outside_reread() {
    // Raw `list.move` takes `to` in pre-removal coordinates (kition-sdk call-context.md): moving
    // layer 1 behind layer 3 is `from:1,to:4`, a gap whose neighbours are layers 3 and 4. The
    // later deletion is layer 5, a member after the gap but not next to it.
    for change in ["before", "after", "opacity"] {
        let fixture = Fixture::new(6);
        let (info, comp, basis) = fixture.inspect().await;
        let layers = layer_ids(&info);
        let held = fixture.ok("query", json!({"objects":[comp]})).await["objects"][0]["fields"]["comp"].clone();
        fixture.colleague(&[match change {
            "before" => ("edit.clear", json!({"layers":[layers[0]]})),
            "after" => ("edit.clear", json!({"layers":[layers[5]]})),
            _ => opacity(layers[1], 23.0),
        }]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("batch", json!({"basis":basis,"ops":[{"op":"list.move","target":held,"field":"layers","from":1,"to":4}]})).await;
        if change == "before" {
            let error = answer.expect_err("an earlier deletion redirects the raw moving recipient");
            assert_eq!(error["error"]["code"], "conflict", "{error}");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{change}: {answer:?}");
            let expected = if change == "after" {
                vec![layers[0], layers[2], layers[3], layers[1], layers[4]]
            } else {
                vec![layers[0], layers[2], layers[3], layers[1], layers[4], layers[5]]
            };
            assert_eq!(layers_on_disk(&fixture.file()), expected);
            if change == "opacity" {
                assert_eq!(opacity_on_disk(&fixture.file(), layers[1]), 23.0);
            }
        }
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn raw_folder_and_property_group_creation_preserve_an_unrelated_edit() {
    for kind in ["folder", "group"] {
        let fixture = Fixture::new(1);
        let (info, _, basis) = fixture.inspect().await;
        let layer = layer_ids(&info)[0];
        let properties = fixture.ok("query", json!({"objects":[layer]})).await["objects"][0]["fields"]["props"].clone();
        let object = fixture.ok("query", json!({"objects":[properties]})).await;
        let mut meta = object["objects"][0]["fields"]["meta"].clone();
        meta["name"] = json!("Independent group");
        meta["match"] = json!("independentGroup");
        fixture.colleague(&[opacity(layer, 23.0)]);
        let operation = if kind == "folder" {
            json!({"op":"create","type":"Item","in":"root.items","at":0,"fields":{"kind":"folder","name":"Independent folder"}})
        } else {
            json!({"op":"create","type":"Node","in":format!("{}.children",properties.as_str().unwrap()),"at":0,"fields":{"uid":999999,"node":"group","meta":meta,"enabled":true}})
        };
        let answer = fixture.call("batch", json!({"basis":basis,"ops":[operation]})).await;
        assert!(answer.is_ok(), "{kind}: {answer:?}");
        assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
        assert!(std::fs::read_to_string(fixture.file()).unwrap().contains(if kind == "folder" { "Independent folder" } else { "Independent group" }));
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn raw_property_group_creation_refuses_an_occupied_observed_gap() {
    let fixture = Fixture::new(1);
    let (info, _, basis) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    let properties = fixture.ok("query", json!({"objects":[layer]})).await["objects"][0]["fields"]["props"].clone();
    let object = fixture.ok("query", json!({"objects":[properties]})).await;
    let mut meta = object["objects"][0]["fields"]["meta"].clone();
    meta["name"] = json!("New group");
    meta["match"] = json!("newGroup");
    let operation = |uid| json!({"op":"create","type":"Node","in":format!("{}.children",properties.as_str().unwrap()),"at":0,"fields":{"uid":uid,"node":"group","meta":meta,"enabled":true}});
    fixture.call_by("other-agent", "batch", json!({"ops":[operation(999998)]})).await;
    let saved = std::fs::read(fixture.file()).unwrap();
    let answer = fixture.call("batch", json!({"basis":basis,"ops":[operation(999999)]})).await;
    assert_eq!(answer.expect_err("another group occupied the observed insertion gap")["error"]["code"], "conflict");
    assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
    fixture.shutdown().await;
}

#[tokio::test]
async fn raw_folder_creation_refuses_a_deleted_parent() {
    let fixture = Fixture::new(1);
    let (_, _, basis) = fixture.inspect().await;
    let objects = fixture.ok("query", json!({"follow":"items"})).await;
    let parent = objects["objects"].as_array().unwrap().iter().find(|object| object["fields"]["kind"] == "folder").unwrap()["id"].clone();
    fixture.colleague(&[("project.delete", json!({"items":[parent]}))]);
    let saved = std::fs::read(fixture.file()).unwrap();
    let answer = fixture
        .call(
            "batch",
            json!({"basis":basis,"ops":[{"op":"create","type":"Item","in":"root.items","at":0,"fields":{"kind":"folder","name":"Child","parent":parent}}]}),
        )
        .await;
    assert!(answer.is_err(), "a deleted parent must not silently become a root-folder creation: {answer:?}");
    assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
    fixture.shutdown().await;
}

#[tokio::test]
async fn raw_property_creation_refuses_a_property_as_a_group_owner() {
    let fixture = Fixture::new(1);
    let (info, _, _) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    let groups = fixture.ok("query", json!({"objects":[layer],"follow":"props.children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let props = fixture.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    let property = props["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "opacity").unwrap()["id"].clone();
    let saved = std::fs::read(fixture.file()).unwrap();
    let answer = fixture.call("batch",json!({"ops":[{"op":"create","type":"Node","in":format!("{}.children",property.as_str().unwrap()),"at":0,"fields":{"uid":999999,"node":"group","meta":{"kind":{"kind":"Plain"},"name":"Must not disappear","match":"invalidOwner"},"enabled":true}}]})).await;
    assert!(answer.is_err(), "a property cannot own a group that the native file silently omits: {answer:?}");
    assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
    fixture.shutdown().await;
}

#[tokio::test]
async fn raw_key_creation_refuses_a_group_as_a_key_owner() {
    let mut document = kition_motion_kit::MotionDocument::open(FILE, &project(1)).unwrap();
    use kition_kit::KitDocument;
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    document.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":1,"value":20})).unwrap();
    let fixture = Fixture::from_bytes(&document.save(None).unwrap(), Default::default());
    fixture.inspect().await;
    let root = fixture.ok("query", json!({"objects":[layer]})).await["objects"][0]["fields"]["props"].clone();
    let groups = fixture.ok("query", json!({"objects":[root],"follow":"children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let props = fixture.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    let key = props["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "opacity").unwrap()["fields"]["keys"][0].clone();
    let data = fixture.ok("query", json!({"objects":[key]})).await["objects"][0]["fields"]["data"].clone();
    let saved = std::fs::read(fixture.file()).unwrap();
    let answer = fixture.call("batch",json!({"ops":[{"op":"create","type":"Key","in":format!("{}.keys",root.as_str().unwrap()),"at":0,"fields":{"time":"0","value":{"t":"Scalar","v":99},"data":data}}]})).await;
    assert!(answer.is_err(), "a group cannot own a key that the native file silently omits: {answer:?}");
    assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
    fixture.shutdown().await;
}
