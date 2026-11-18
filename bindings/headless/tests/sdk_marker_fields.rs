mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn layer_marker_operations_hold_consumed_fields_and_preserve_opacity() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    document.execute("layer.addMarker", json!({"layer":layer,"time":2,"comment":"Observed"})).unwrap();
    let bytes = document.save(None).unwrap();
    for command in ["layer.markersLock", "layer.deleteAllMarkers"] {
        for dependent in [true, false] {
            let fixture = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = fixture.inspect().await;
            fixture.colleague(&[if dependent { ("layer.markersLock", json!({"layers":[layer],"value":true})) } else { opacity(layer, 23.0) }]);
            let saved = std::fs::read(fixture.file()).unwrap();
            let params = if command == "layer.markersLock" { json!({"layers":[layer],"value":false}) } else { json!({"layers":[layer]}) };
            let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
            if dependent {
                assert_eq!(answer.expect_err("the observed marker-lock field changed")["error"]["code"], "conflict");
                assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
            } else {
                assert!(answer.is_ok(), "unrelated opacity must apply: {command} {answer:?}");
                assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
            }
            fixture.shutdown().await;
        }
    }
}
