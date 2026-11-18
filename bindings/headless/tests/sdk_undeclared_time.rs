//! `edit.selectAll` (like `edit.copy` and `learn.step`) reads the current time in its code
//! (`tests/acts.rs`), so it declares `time`: a person moving only the current time
//! after the agent's basis refuses the agent's step that gives its part but not its time, as
//! `changed` on `time`; with no change, or with `at.time` given, it applies.
mod common;

use common::*;
use kition_kits::Caller;
use serde_json::{Map, Value, json};

/// The person, in the document the server holds, moves the current time.
async fn person_moves_time(fixture: &Fixture) {
    let view = fixture.kits.open(Caller::User, FILE).await.unwrap();
    let params: Map<String, Value> = json!({"time": 2}).as_object().cloned().unwrap();
    fixture.kits.execute(Caller::User, view.kit_id, &view.document_id, "time.set", params).await.unwrap();
}

fn select_all(comp: u64, time: Option<f64>) -> Value {
    let mut step = json!({"command":"edit.selectAll","params":{},"at":{"part":{"comp":comp}}});
    if let Some(time) = time {
        step["at"]["time"] = json!(time);
    }
    step
}

#[test]
fn the_time_readers_declare_time() {
    for id in ["edit.copy", "edit.selectAll", "learn.step"] {
        assert!(kition_motion_kit::acts::reads(id).iter().any(|state| state.as_str() == "time"), "{id} reads the current time but does not declare it");
    }
}

#[tokio::test]
async fn a_person_moving_the_time_refuses_select_all_as_changed_time() {
    let fixture = Fixture::from_bytes(&project(2), Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    person_moves_time(&fixture).await;
    let refused = fixture.call("run", json!({"basis":basis,"steps":[select_all(comp, None)]})).await.unwrap_err();
    assert_eq!(refused["error"]["code"], "conflict", "the time edit.selectAll reads is no premise: {refused}");
    let conflicts = refused["conflicts"].as_array().unwrap();
    assert!(
        conflicts.iter().any(|conflict| conflict["now"]["state"] == "time" && conflict["reason"] == "changed"),
        "the refusal is not `changed` on `time`: {refused}"
    );
    assert!(conflicts.iter().all(|conflict| conflict["reason"] == "changed"), "{refused}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn select_all_applies_with_no_change_or_with_its_time_given() {
    let fixture = Fixture::from_bytes(&project(2), Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    let answer = fixture.call("run", json!({"basis":basis,"steps":[select_all(comp, None)]})).await;
    assert!(answer.is_ok(), "nothing changed, yet edit.selectAll was refused: {answer:?}");
    fixture.shutdown().await;

    let fixture = Fixture::from_bytes(&project(2), Default::default());
    let (_, comp, basis) = fixture.inspect().await;
    person_moves_time(&fixture).await;
    let answer = fixture.call("run", json!({"basis":basis,"steps":[select_all(comp, Some(0.0))]})).await;
    assert!(answer.is_ok(), "the step gives its own time, yet a time change refused it: {answer:?}");
    fixture.shutdown().await;
}
