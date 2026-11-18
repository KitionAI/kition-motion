//! C8-motion-read: static mask path writes checked against an independent saved-file writer.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn fixture() -> (Vec<u8>, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    let mask = doc.execute("mask.new", json!({"layer":layer,"vertices":[[0,0],[20,0],[20,20]],"closed":false})).unwrap()["mask"].as_u64().unwrap();
    (doc.save(None).unwrap(), layer, mask)
}

#[tokio::test]
async fn static_closed_assignment_holds_path_but_allows_layer_opacity() {
    let (bytes, layer, mask) = fixture();
    for relevant in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if relevant { ("mask.setVertex", json!({"layer":layer,"mask":mask,"index":0,"point":[3,7]})) } else { opacity(layer, 23.0) }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"mask.setClosed","params":{"layer":layer,"mask":mask,"closed":true}}]})).await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "static mask.setClosed refused unrelated opacity: {answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn static_closed_assignment_does_not_evaluate_path_expression() {
    let (bytes, layer, mask) = fixture();
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("prop.setExpression", json!({"layer":layer,"path":"masks/#1/path","expression":"value"}))]);
    let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"mask.setClosed","params":{"layer":layer,"mask":mask,"closed":true}}]})).await;
    assert!(answer.is_ok(), "mask::edit_path_counted calls value_at, which does not evaluate the expression: {answer:?}");
    f.shutdown().await;
}

#[tokio::test]
async fn becoming_animated_after_basis_refuses_a_static_closed_assignment() {
    let (bytes, layer, mask) = fixture();
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("prop.addKey", json!({"layer":layer,"path":"masks/#1/path","time":0}))]);
    let refused = f.refused("run", json!({"basis":basis,"steps":[{"command":"mask.setClosed","params":{"layer":layer,"mask":mask,"closed":true}}]})).await;
    assert_conflict(&refused, &layer.to_string(), "changed");
    f.shutdown().await;
}
