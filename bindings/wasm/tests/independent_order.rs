//! Source-complete declarations must not depend on which API is called first.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_engine::Session;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[test]
fn asking_under_first_preserves_the_typed_object_declarations() {
    let mut session = Session::default();
    session.execute("comp.new", json!({"name":"Main"})).unwrap();
    let layer = session.execute("layer.newSolid", json!({"name":"Layer"})).unwrap()["layer"].as_u64().unwrap();
    let doc = MotionDocument::open("Order.kmotion", session.project.to_json().as_bytes()).unwrap();
    doc.under("layer.rename", &json!({"layer":layer,"name":"Renamed"})).unwrap();
    let commands = doc.commands();
    let command = commands.iter().find(|command| command.id == "track.new").unwrap();
    assert!(command.objects.contains(&"target"), "target is resolved as a layer even if under was called first: {:?}", command.objects);
}
