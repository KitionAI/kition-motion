mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

async fn property(f: &Fixture, layer: u64) -> Value {
    let groups = f.ok("query", json!({"objects":[layer],"follow":"props.children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|o| o["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let props = f.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    props["objects"].as_array().unwrap().iter().find(|o| o["fields"]["meta"]["match"] == "opacity").unwrap().clone()
}

async fn structural(form: &str) -> Vec<Value> {
    let mut d = MotionDocument::open(FILE, &project(2)).unwrap();
    let layers = layer_ids(&d.inspect().unwrap()["info"]);
    for t in [1, 4, 6, 8] {
        d.execute("prop.addKey", json!({"layer":layers[0],"path":OPACITY,"time":t,"value":t*10})).unwrap();
    }
    let bytes = d.save(None).unwrap();
    let mut receipts = Vec::new();
    // Check the dangerous controls first: refusal must remain even after enabling a contract.
    for change in ["same", "before", "deleted", "precomp", "unrelated", "none"] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        f.inspect().await;
        let p = property(&f, layers[0]).await;
        let owner = p["id"].as_str().unwrap();
        let loaded = f.ok("query", json!({"objects":[p["fields"]["keys"][1]]})).await;
        let fields = json!({"time":(254_016_000_000_i64*9/2).to_string(),"value":{"t":"Scalar","v":77},"data":loaded["objects"][0]["fields"]["data"]});
        let (_, _, basis) = f.inspect().await;
        if change != "none" {
            f.colleague(&[match change {
                "same" => ("prop.addKey", json!({"layer":layers[0],"path":OPACITY,"time":4,"value":43})),
                "before" => ("prop.addKey", json!({"layer":layers[0],"path":OPACITY,"time":0.5,"value":5})),
                "deleted" => ("keys.select", json!({"keys":[{"layer":layers[0],"path":OPACITY,"time":4}]})),
                "precomp" => ("layer.precompose", json!({"layers":[layers[0]],"name":"Moved","mode":"move"})),
                _ => opacity(layers[1], 23.0),
            }]);
            if change == "deleted" {
                f.colleague(&[("keys.select", json!({"keys":[{"layer":layers[0],"path":OPACITY,"time":4}]})), ("keys.delete", json!({}))]);
            }
        }
        let ops = match form {
            "remove" => json!([{"op":"list.remove","target":owner,"field":"keys","index":1}]),
            "create" => json!([{"op":"create","type":"Key","in":format!("{owner}.keys"),"at":2,"fields":fields}]),
            "insert" => {
                json!([{"op":"create","type":"Key","ref":"$new","fields":fields},{"op":"list.insert","target":owner,"field":"keys","index":2,"item":"$new"}])
            }
            "move" => json!([{"op":"list.move","target":owner,"field":"keys","from":1,"to":3}]),
            _ => panic!("unknown structural class"),
        };
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("batch", json!({"basis":basis,"ops":ops})).await;
        // Conflict places describe their predicates; private fact values belong in neither
        // the write receipt nor the public report of intervening changes.
        let public = match &answer {
            Ok(value) | Err(value) => json!({"changes":value.get("changes"),"others":value.get("others")}),
        };
        receipts.push(public);
        if matches!(change, "none" | "unrelated") {
            assert!(answer.is_ok(), "{form}/{change}: {answer:?}");
            if change == "unrelated" {
                assert_eq!(opacity_on_disk(&f.file(), layers[1]), 23.0);
            }
        } else {
            let error = answer.expect_err(&format!("{form}/{change}: a structural operation must not overwrite or redirect an observed key"));
            assert_eq!(error["error"]["code"], "conflict", "{form}/{change}: {error}");
            let reason = if change == "same" { "changed" } else { "relocated" };
            assert!(error["conflicts"].as_array().unwrap().iter().any(|c| c["reason"] == reason), "{form}/{change}: {error}");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        }
        f.shutdown().await;
        println!("RAW CONTRACT {form}/{change} PASS");
    }
    receipts
}

#[tokio::test]
async fn raw_remove_keeps_necessary_refusals() {
    structural("remove").await;
}
#[tokio::test]
async fn raw_create_keeps_necessary_refusals() {
    structural("create").await;
}
#[tokio::test]
async fn raw_insert_keeps_necessary_refusals() {
    structural("insert").await;
}
#[tokio::test]
async fn raw_move_keeps_necessary_refusals() {
    structural("move").await;
}

#[tokio::test]
async fn waiting_for_sdk_private_facts_absent_from_outside_created_field_lists() {
    for form in ["remove", "create", "insert", "move"] {
        let receipts = structural(form).await;
        for fact in ["~keyOrder", "~container", "~rawValue", "~rawRecord"] {
            assert!(receipts.iter().all(|public| !public.to_string().contains(fact)), "private key facts leaked into a public receipt: {receipts:?}");
        }
    }
}
