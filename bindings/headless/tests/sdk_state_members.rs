//! Motion's editor state names the members a command's declared reads compare (`part`,
//! `selection`, `time`, `mode`, `active`, `area`; kition-sdk `docs/platform/kits/call-context.md`),
//! so the premise guard compares each read on its own: a person moving only the current time
//! leaves an agent's write that reads the selection (and gives its part and time) applying, and a
//! person selecting another layer refuses it as `changed` on `selection`, never as an editor state
//! the guard cannot compare (`unknown`). `track.status` without `on` reads the part, the layer
//! selection and the time.
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

/// The person, in the document the server holds, runs `command`.
async fn person(fixture: &Fixture, command: &str, params: Value) {
    let view = fixture.kits.open(Caller::User, FILE).await.unwrap();
    let params: Map<String, Value> = params.as_object().cloned().unwrap();
    fixture.kits.execute(Caller::User, view.kit_id, &view.document_id, command, params).await.unwrap();
}

/// `track.status` with no layer, in `comp`, at `time` when given.
fn status(comp: u64, time: Option<f64>) -> Value {
    let mut step = json!({"command":"track.status","params":{},"at":{"part":{"comp":comp}}});
    if let Some(time) = time {
        step["at"]["time"] = json!(time);
    }
    step
}

/// The refusal's conflicts on the editor state: each `(state, reason)`.
fn state_conflicts(refused: &Value) -> Vec<(String, String)> {
    refused["conflicts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|conflict| conflict["now"]["state"].is_string() || conflict["reason"] == "unknown")
        .map(|conflict| (conflict["now"]["state"].as_str().unwrap_or_default().to_owned(), conflict["reason"].as_str().unwrap_or_default().to_owned()))
        .collect()
}

#[tokio::test]
async fn a_person_moving_only_the_time_leaves_a_selection_reader_applying() {
    let (bytes, _) = tracked();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    person(&fixture, "time.set", json!({"time": 2})).await;
    let answer = fixture.call("run", json!({"basis":basis,"steps":[status(comp, Some(0.0))]})).await;
    assert!(answer.is_ok(), "only the current time moved, and the step gives its own time, yet it was refused: {answer:?}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn a_person_selecting_another_layer_refuses_a_selection_reader_as_changed_selection() {
    let (bytes, layers) = tracked();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    person(&fixture, "layer.select", json!({"layers":[layers[1]]})).await;
    let refused = fixture.call("run", json!({"basis":basis,"steps":[status(comp, Some(0.0))]})).await.unwrap_err();
    assert_eq!(refused["error"]["code"], "conflict", "the selection track.status reads is no premise: {refused}");
    assert_eq!(state_conflicts(&refused), [("selection".to_owned(), "changed".to_owned())], "{refused}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn a_person_moving_the_time_refuses_a_time_reader_as_changed_time() {
    let (bytes, _) = tracked();
    let fixture = Fixture::from_bytes(&bytes, Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    person(&fixture, "time.set", json!({"time": 2})).await;
    let refused = fixture.call("run", json!({"basis":basis,"steps":[status(comp, None)]})).await.unwrap_err();
    assert_eq!(refused["error"]["code"], "conflict", "the time track.status reads is no premise: {refused}");
    assert_eq!(state_conflicts(&refused), [("time".to_owned(), "changed".to_owned())], "{refused}");
    fixture.shutdown().await;
}
