mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn source_markers_hold_precomp_markers_but_ignore_its_background() {
    let mut d = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&d.inspect().unwrap()["info"])[0];
    let pre = d.execute("layer.precompose", json!({"layers":[layer],"name":"Inner","mode":"move"})).unwrap();
    let parent = pre["layer"].as_u64().unwrap();
    let source = pre["comp"].as_u64().unwrap();
    d.execute("comp.addMarker", json!({"comp":source,"time":1,"comment":"Source"})).unwrap();
    let bytes = d.save(None).unwrap();
    for markers in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if markers {
            ("comp.addMarker", json!({"comp":source,"time":2,"comment":"New"}))
        } else {
            ("comp.settings", json!({"comp":source,"background":"#ff0000"}))
        }]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.updateMarkersFromSource","params":{"layers":[parent]}}]})).await;
        if markers {
            assert_eq!(answer.expect_err("source markers changed")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}
