#![allow(clippy::unwrap_used, clippy::panic)]

use kition_motion_engine::Session;
use kition_motion_project::ItemId;
use serde_json::json;

#[test]
fn invalid_explicit_project_item_does_not_rename_the_selection() {
    let mut session = Session::default();
    let selected = session.execute("project.newFolder", json!({"name":"Selected"})).unwrap()["item"].as_u64().unwrap();
    for item in [json!(999999), json!("Missing item"), json!(null)] {
        let answer = session.execute("project.rename", json!({"item":item,"name":"Wrong target"}));
        assert!(answer.is_err(), "invalid explicit item fell back to the selection: {answer:?}");
        assert_eq!(session.project.item(ItemId(selected)).unwrap().name, "Selected");
    }
    session.execute("project.rename", json!({"name":"Selection fallback"})).unwrap();
    assert_eq!(session.project.item(ItemId(selected)).unwrap().name, "Selection fallback");
    session.execute("project.rename", json!({"item":selected,"name":"Explicit target"})).unwrap();
    assert_eq!(session.project.item(ItemId(selected)).unwrap().name, "Explicit target");
}
