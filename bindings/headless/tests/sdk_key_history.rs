mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn animated() -> (Vec<u8>, u64) {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    for time in [1, 4, 6, 8] {
        document.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":time,"value":time*10})).unwrap();
    }
    (document.save(None).unwrap(), layer)
}

#[tokio::test]
async fn consumed_key_deletion_refuses_while_a_distant_insertion_applies() {
    let (bytes, layer) = animated();
    let steps = [
        json!({"command":"prop.addKey","params":{"layer":layer,"path":OPACITY,"time":4.5}}),
        json!({"command":"prop.set","params":{"layer":layer,"path":OPACITY,"time":4.5,"value":77}}),
        json!({"command":"prop.set","at":{"time":4.5},"params":{"layer":layer,"path":OPACITY,"value":77}}),
        json!({"command":"keys.set","params":{"layer":layer,"path":OPACITY,"time":4.4,"value":77}}),
    ];
    for step in steps {
        for deletion in [true, false] {
            let fixture = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = fixture.inspect().await;
            if deletion {
                fixture.colleague(&[("keys.select", json!({"keys":[{"layer":layer,"path":OPACITY,"time":4}]})), ("keys.delete", json!({}))]);
            } else {
                fixture.colleague(&[("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":9,"value":90}))]);
            }
            let saved = std::fs::read(fixture.file()).unwrap();
            let answer = fixture.call("run", json!({"basis":basis,"steps":[step]})).await;
            if deletion {
                assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
                assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
            } else {
                assert!(answer.is_ok(), "an unconsumed key insertion refused {step}: {answer:?}");
                assert!(keys_on_disk(&fixture.file(), layer, OPACITY).contains(&(9.0, json!(90.0))));
            }
            fixture.shutdown().await;
        }
    }
}

#[tokio::test]
async fn renamed_effect_name_is_relocated_while_unrelated_layer_content_applies() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    let effect = document.execute("effect.apply", json!({"layer":layer,"effect":"Fill"})).unwrap()["effects"][0].clone();
    let bytes = document.save(None).unwrap();
    for rename in [true, false] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[if rename { ("prop.renameGroup", json!({"layer":layer,"prop":effect,"name":"Renamed Fill"})) } else { opacity(layer, 23.0) }]);
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"effect.remove","params":{"layer":layer,"effect":"Fill"}}]})).await;
        if rename {
            assert_conflict(&answer.unwrap_err(), &layer.to_string(), "relocated");
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
        }
        fixture.shutdown().await;
    }
}
