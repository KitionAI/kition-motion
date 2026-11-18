mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn tracked() -> (Vec<u8>, u64, u64) {
    let mut document = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    let tracker = document.execute("track.new", json!({"layer":layer,"kind":"raw","target":null})).unwrap()["tracker"].as_u64().unwrap();
    (document.save(None).unwrap(), layer, tracker)
}

#[tokio::test]
async fn tracker_option_assignment_preserves_other_edits_and_refuses_the_assigned_field() {
    let (bytes, layer, tracker) = tracked();
    for (change, conflict) in [
        (("prop.set", json!({"layer":layer,"path":"transform/opacity","value":23})), false),
        (("track.options", json!({"layer":layer,"tracker":tracker,"threshold":21})), false),
        (("track.options", json!({"layer":layer,"tracker":tracker,"blur":3})), true),
    ] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(std::slice::from_ref(&change));
        let answer =
            fixture.call("run", json!({"basis":basis,"steps":[{"command":"track.options","params":{"layer":layer,"tracker":tracker,"blur":7}}]})).await;
        if conflict {
            assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "unrelated edit {change:?} was refused: {answer:?}");
            let mut document = on_disk(&fixture.file());
            let info = document.execute("track.status", json!({"layer":layer,"tracker":tracker})).unwrap();
            assert_eq!(info["tracker"]["settings"]["options"]["blur"], 7.0);
            if change.0 == "prop.set" {
                assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
            }
            if change.0 == "track.options" {
                assert_eq!(info["tracker"]["settings"]["options"]["threshold"], 21.0);
            }
        }
        fixture.shutdown().await;
    }
}
