mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn tracked() -> (Vec<u8>, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    let tracker = doc.execute("track.new", json!({"layer":layer,"kind":"raw","target":null})).unwrap()["tracker"].as_u64().unwrap();
    (doc.save(None).unwrap(), layer, tracker)
}

#[tokio::test]
async fn tracker_default_regions_hold_source_dimensions_and_allow_source_comments() {
    let (bytes, layer, _) = tracked();
    for command in ["track.motion", "track.stabilize", "track.new"] {
        for relevant in [true, false] {
            let f = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = f.inspect().await;
            f.colleague(&[if relevant {
                ("layer.settings", json!({"layer":layer,"width":640}))
            } else {
                ("layer.setComment", json!({"layers":[layer],"comment":"unrelated"}))
            }]);
            let params = if command == "track.new" { json!({"layer":layer,"target":null}) } else { json!({"layer":layer}) };
            let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
            if relevant {
                let error = answer.unwrap_err();
                assert_eq!(error["error"]["code"], "conflict", "{command}: {error}");
                assert!(error["conflicts"].as_array().unwrap().iter().any(|conflict| conflict["reason"] == "changed"), "{command}: {error}");
            } else {
                assert!(answer.is_ok(), "{command} default regions do not read the source-layer comment: {answer:?}");
            }
            f.shutdown().await;
        }
    }
}

#[tokio::test]
async fn tracker_options_protect_assigned_fields_and_allow_unrelated_layer_opacity() {
    let (bytes, layer, tracker) = tracked();
    for relevant in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if relevant { ("track.options", json!({"layer":layer,"tracker":tracker,"blur":3})) } else { opacity(layer, 23.0) }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"track.options","params":{"layer":layer,"tracker":tracker,"blur":7}}]})).await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "tracker options do not read layer opacity: {answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn tracker_default_regions_hold_source_dimensions_without_other_layers() {
    let (bytes, layer, _) = tracked();
    let other = layer_ids(&MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["info"])[1];
    for command in ["track.motion", "track.stabilize", "track.new"] {
        for relevant in [true, false] {
            let f = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = f.inspect().await;
            f.colleague(&[if relevant { ("layer.settings", json!({"layer":layer,"width":640})) } else { opacity(other, 23.0) }]);
            let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":{"layer":layer}}]})).await;
            if relevant {
                let error = answer.unwrap_err();
                assert_eq!(error["error"]["code"], "conflict", "{command}: {error}");
                assert!(error["conflicts"].as_array().unwrap().iter().any(|conflict| conflict["reason"] == "changed"), "{command}: {error}");
            } else {
                assert!(answer.is_ok(), "{command} refused another layer's opacity: {answer:?}");
                assert_eq!(opacity_on_disk(&f.file(), other), 23.0);
            }
            f.shutdown().await;
        }
    }
}
