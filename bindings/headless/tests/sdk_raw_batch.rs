mod common;

use common::*;
use kition_kit::KitDocument;
use serde_json::{Value, json};

async fn opacity_model(fixture: &Fixture, layer: u64) -> Value {
    let groups = fixture.ok("query", json!({"objects":[layer],"follow":"props.children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let properties = fixture.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    properties["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "opacity").unwrap().clone()
}

#[tokio::test]
async fn raw_key_value_holds_its_field_and_retains_its_identity_after_outside_reread() {
    let mut document = kition_motion_kit::MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    for time in [1, 4, 8] {
        document.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":time,"value":time*10})).unwrap();
    }
    let bytes = document.save(None).unwrap();
    for change in ["same", "before", "after", "ease", "unrelated"] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        fixture.inspect().await;
        let property = opacity_model(&fixture, layer).await;
        let key = property["fields"]["keys"][1].clone();
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[match change {
            "same" => ("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":4,"value":43})),
            "before" => ("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":0.5,"value":5})),
            "after" => ("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":9,"value":90})),
            "ease" => ("keys.setEase", json!({"layer":layer,"path":OPACITY,"time":4,"side":"out","speed":3})),
            _ => ("layer.rename", json!({"layer":layer,"name":"Colleague"})),
        }]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("batch", json!({"basis":basis,"ops":[{"op":"set","target":key,"field":"value","value":{"t":"Scalar","v":77}}]})).await;
        if change == "same" {
            let error = answer.unwrap_err();
            assert_conflict(&error, key.as_str().unwrap(), "changed");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{change}: {answer:?}");
            assert!(
                keys_on_disk(&fixture.file(), layer, OPACITY).iter().any(|(time, value)| *time == 4.0 && value.as_f64() == Some(77.0)),
                "{change}: the retained key must contain the assigned scalar"
            );
            if change == "before" || change == "after" {
                assert_eq!(keys_on_disk(&fixture.file(), layer, OPACITY).len(), 4);
            }
        }
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn raw_assignment_refuses_the_same_field_and_accepts_an_unrelated_outside_edit() {
    for same_field in [true, false] {
        let fixture = Fixture::new(2);
        let (info, _, basis) = fixture.inspect().await;
        let layers = layer_ids(&info);
        fixture.colleague(&[if same_field { ("layer.rename", json!({"layer":layers[0],"name":"Colleague"})) } else { opacity(layers[1], 23.0) }]);
        let answer = fixture.call("batch", json!({"basis":basis,"ops":[{"op":"set","target":layers[0],"field":"name","value":"Mine"}]})).await;
        if same_field {
            assert_conflict(&answer.unwrap_err(), &layers[0].to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&fixture.file(), layers[1]), 23.0);
            assert_eq!(on_disk(&fixture.file()).inspect().unwrap()["info"]["layers"][0]["name"], "Mine");
        }
        fixture.shutdown().await;
    }
}

/// A composition marker is an object of its own (schema 1): a raw write of one of its fields
/// holds that marker's field. Another field of it, or another marker added before or after it,
/// changed by a colleague, applies; the same field refuses.
#[tokio::test]
async fn raw_comp_marker_field_holds_its_field_and_preserves_other_edits() {
    let mut document = kition_motion_kit::MotionDocument::open(FILE, &project(1)).unwrap();
    for time in [1, 4, 8] {
        document.execute("comp.addMarker", json!({"time":time,"comment":format!("C{time}")})).unwrap();
    }
    let bytes = document.save(None).unwrap();
    for change in ["before", "same", "other-field", "after", "unrelated"] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (info, comp, _) = fixture.inspect().await;
        let layer = layer_ids(&info)[0];
        let item = fixture.ok("query", json!({"objects":[comp]})).await;
        let owner = item["objects"][0]["fields"]["comp"].clone();
        let loaded = fixture.ok("query", json!({"objects":[owner]})).await;
        let marker = loaded["objects"][0]["fields"]["markers"][1].clone();
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[match change {
            "before" => ("comp.addMarker", json!({"time":0.5,"comment":"Before"})),
            "same" => ("markers.set", json!({"index":1,"chapter":"Colleague"})),
            "other-field" => ("markers.set", json!({"index":1,"comment":"Colleague"})),
            "after" => ("comp.addMarker", json!({"time":9,"comment":"After"})),
            _ => opacity(layer, 23.0),
        }]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("batch", json!({"basis":basis,"ops":[{"op":"set","target":marker,"field":"chapter","value":"Mine"}]})).await;
        if change == "same" {
            assert_eq!(answer.as_ref().err().unwrap()["error"]["code"], "conflict", "{answer:?}");
            assert!(answer.unwrap_err()["conflicts"].as_array().unwrap().iter().any(|conflict| conflict["reason"] == "changed"));
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{change}: {answer:?}");
            let markers = on_disk(&fixture.file()).execute("markers.list", json!({})).unwrap();
            let markers = markers.as_array().unwrap();
            assert!(markers.iter().any(|m| m["time"] == json!(4.0) && m["chapter"] == "Mine"), "{change}: {markers:?}");
            if change == "other-field" {
                assert!(markers.iter().any(|m| m["time"] == json!(4.0) && m["comment"] == "Colleague"), "both fields: {markers:?}");
            }
            if change == "before" || change == "after" {
                assert_eq!(markers.len(), 4, "{markers:?}");
            }
            if change == "unrelated" {
                assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
            }
        }
        fixture.shutdown().await;
    }
}

/// A layer marker likewise; the layer moved into another composition relocates it.
#[tokio::test]
async fn raw_layer_marker_field_holds_its_field_and_its_layer() {
    let mut document = kition_motion_kit::MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    for time in [1, 4, 8] {
        document.execute("layer.addMarker", json!({"layer":layer,"time":time,"comment":format!("L{time}")})).unwrap();
    }
    let bytes = document.save(None).unwrap();
    for change in ["same", "before", "after", "precomp", "unrelated"] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        fixture.inspect().await;
        let loaded = fixture.ok("query", json!({"objects":[layer]})).await;
        let marker = loaded["objects"][0]["fields"]["markers"][1].clone();
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[match change {
            "same" => ("markers.set", json!({"layer":layer,"index":1,"chapter":"Colleague"})),
            "before" => ("layer.addMarker", json!({"layer":layer,"time":0.5,"comment":"Before"})),
            "after" => ("layer.addMarker", json!({"layer":layer,"time":9,"comment":"After"})),
            "precomp" => ("layer.precompose", json!({"layers":[layer],"name":"Moved","mode":"move"})),
            _ => opacity(layer, 23.0),
        }]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("batch", json!({"basis":basis,"ops":[{"op":"set","target":marker,"field":"chapter","value":"Mine"}]})).await;
        match change {
            "same" | "precomp" => {
                let refused = answer.unwrap_err();
                assert_eq!(refused["error"]["code"], "conflict", "{refused}");
                let reason = if change == "precomp" { "relocated" } else { "changed" };
                assert!(refused["conflicts"].as_array().unwrap().iter().any(|conflict| conflict["reason"] == reason), "{refused}");
                assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
            }
            _ => {
                assert!(answer.is_ok(), "{change}: {answer:?}");
                let markers = on_disk(&fixture.file()).execute("markers.list", json!({"layer":layer})).unwrap();
                let markers = markers.as_array().unwrap();
                assert!(markers.iter().any(|m| m["chapter"] == "Mine"), "{change}: {markers:?}");
                if change == "unrelated" {
                    assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
                }
            }
        }
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn composition_membership_and_resize_inputs_stay_private() {
    let fixture = Fixture::new(1);
    let (info, _, basis) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    for step in [json!({"command":"comp.settings","params":{"width":480}}), json!({"command":"layer.newNull","params":{"name":"Other"}})] {
        let receipt = fixture.call_by("other", "run", json!({"steps":[step]})).await;
        for name in ["~layerMembers", "~resizeInputs"] {
            assert!(!receipt.to_string().contains(name), "private input leaked into the receipt: {receipt}");
        }
    }
    let answer = fixture.ok("run", json!({"basis":basis,"steps":[{"command":"layer.rename","params":{"layer":layer,"name":"Mine"}}]})).await;
    for name in ["~layerMembers", "~resizeInputs"] {
        assert!(!answer.to_string().contains(name), "private input leaked into others: {answer}");
    }
    fixture.shutdown().await;
}
