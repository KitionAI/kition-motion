mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn outside_reread_keeps_agent_undo_and_the_colleagues_edit() {
    let fixture = Fixture::new(2);
    let (info, _, _) = fixture.inspect().await;
    let layers = layer_ids(&info);
    fixture.ok("run", json!({"steps":[{"command":"prop.set","params":{"layer":layers[0],"path":OPACITY,"value":37}}]})).await;
    fixture.colleague(&[opacity(layers[1], 23.0)]);
    let reread = fixture.ok("inspect", json!({})).await;
    fixture.ok("run", json!({"basis":revision(&reread),"steps":[{"command":"edit.undo","params":{}}]})).await;
    assert_eq!(opacity_on_disk(&fixture.file(), layers[0]), 100.0);
    assert_eq!(opacity_on_disk(&fixture.file(), layers[1]), 23.0);
    fixture.shutdown().await;
}

#[tokio::test]
async fn outside_reread_keeps_property_references_by_persistent_uid() {
    let fixture = Fixture::new(1);
    let (info, _, _) = fixture.inspect().await;
    let layer = layer_ids(&info)[0];
    let groups = fixture.ok("query", json!({"objects":[layer],"follow":"props.children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|object| object["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let before = fixture.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    assert!(!before["objects"].as_array().unwrap().is_empty());
    fixture.colleague(&[("layer.setComment", json!({"layers":[layer],"comment":"outside"}))]);
    let after = fixture.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    assert_eq!(after["objects"], before["objects"], "an unchanged property retains its imported reference and fields");
    fixture.shutdown().await;
}

#[test]
fn premise_fork_changes_only_its_copy() {
    let document = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    let before = document.save(None).unwrap();
    let mut copy = document.fork_for_premises().unwrap();
    copy.execute("prop.set", json!({"layer":layer,"path":OPACITY,"value":31})).unwrap();
    assert_eq!(document.save(None).unwrap(), before);
    assert_ne!(copy.save(None).unwrap(), before);
}

#[test]
fn premise_fork_keeps_the_clipboard_consumed_by_paste() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    document.execute("edit.copy", json!({"layers":[layer]})).unwrap();
    let before = document.save(None).unwrap();
    let mut copy = document.fork_for_premises().unwrap();
    copy.execute("edit.paste", json!({})).unwrap();
    assert_eq!(layer_ids(&copy.inspect().unwrap()["info"]).len(), 2);
    assert_eq!(document.save(None).unwrap(), before);
}

#[test]
fn unresolved_reread_editor_state_keeps_the_previous_document() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    document.execute("comp.new", json!({"name":"Active"})).unwrap();
    let saved = document.editor_state();
    let before = document.save(None).unwrap();
    let mut fresh = MotionDocument::open(FILE, &project(1)).unwrap();
    fresh.restore_editor_state(&saved).unwrap();
    let error = document.reconcile_reread(&fresh).unwrap_err();
    assert!(error.contains("editor"), "{error}");
    assert_eq!(document.save(None).unwrap(), before);
    assert_eq!(document.editor_state(), saved);
}
