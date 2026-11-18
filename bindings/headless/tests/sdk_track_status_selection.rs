//! `track.status` without `layer` reports (and applies the finished analysis of) the Current
//! Track, or else the first selected layer that has a tracker (`commands/track.rs` `current`):
//! it reads the layer selection. A person selecting another layer after the agent's basis must
//! refuse the agent's write; with no selection change, or with `on` naming the layer, it applies.
mod common;

use common::*;
use kition_kit::KitDocument;
use kition_kits::Caller;
use kition_motion_kit::MotionDocument;
use serde_json::{Map, Value, json};

/// Two solids, each with a raw tracker; no Current Track. Answers the bytes and the layer ids.
fn tracked() -> (Vec<u8>, Vec<u64>) {
    let mut document = MotionDocument::open(FILE, &project(2)).unwrap();
    let layers = layer_ids(&document.inspect().unwrap()["info"]);
    for layer in &layers {
        document.execute("track.new", json!({"layer":layer,"kind":"raw","target":null})).unwrap();
    }
    (document.save(None).unwrap(), layers)
}

/// The person, in the document the server holds, selects `layer`.
async fn person_selects(fixture: &Fixture, layer: u64) {
    let view = fixture.kits.open(Caller::User, FILE).await.unwrap();
    let params: Map<String, Value> = json!({"layers":[layer]}).as_object().cloned().unwrap();
    fixture.kits.execute(Caller::User, view.kit_id, &view.document_id, "layer.select", params).await.unwrap();
}

fn status(comp: u64, on: Option<u64>) -> Value {
    let mut step = json!({"command":"track.status","params":{},"at":{"part":{"comp":comp},"time":0}});
    if let Some(layer) = on {
        step["on"] = json!([layer]);
    }
    step
}

#[tokio::test]
async fn a_person_selecting_another_layer_refuses_track_status_without_a_layer() {
    let (bytes, layers) = tracked();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    person_selects(&fixture, layers[1]).await;
    let refused = fixture.call("run", json!({"basis":basis,"steps":[status(comp, None)]})).await.unwrap_err();
    assert_eq!(refused["error"]["code"], "conflict", "the selection track.status reads is no premise: {refused}");
    // The kit's editor state names a `selection` member (`state_members`), so the guard refuses
    // on exactly that read, as `changed`.
    let conflicts = refused["conflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1, "{refused}");
    assert_eq!((&conflicts[0]["now"]["state"], &conflicts[0]["reason"]), (&json!("selection"), &json!("changed")), "{refused}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn track_status_without_a_selection_change_applies() {
    let (bytes, _) = tracked();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    let answer = fixture.call("run", json!({"basis":basis,"steps":[status(comp, None)]})).await;
    assert!(answer.is_ok(), "nothing changed, yet track.status was refused: {answer:?}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn track_status_on_a_layer_applies_after_a_person_selects_another() {
    let (bytes, layers) = tracked();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    person_selects(&fixture, layers[1]).await;
    let answer = fixture.call("run", json!({"basis":basis,"steps":[status(comp, Some(layers[0]))]})).await;
    let answer = answer.unwrap_or_else(|error| panic!("`on` names the layer, so the person's selection is no premise: {error}"));
    assert_eq!(answer["completed"][0]["result"]["layer"], layers[0], "track.status reported another layer than `on` names: {answer}");
    fixture.shutdown().await;
}
