use kition_motion_engine::{Session, commands::text_edit::layer_doc};
use kition_motion_project::LayerId;
use serde_json::json;

fn fixture() -> (Session, LayerId) {
    let mut session = Session::default();
    session.execute("comp.new", json!({"name":"Text","width":64,"height":64,"duration":4})).unwrap();
    let layer = session.execute("layer.newText", json!({"text":"abcdef","size":60})).unwrap()["layer"].as_u64().unwrap();
    (session, LayerId(layer))
}

#[test]
fn restored_text_mode_replaces_the_saved_range() {
    let (mut session, layer) = fixture();
    session.execute("text.edit", json!({"layer":layer.0,"select":[1,4]})).unwrap();
    let saved = session.call_state();
    session.execute("text.setSelection", json!({"start":5,"end":6})).unwrap();
    session.restore_call_state(&saved);
    session.execute("text.insert", json!({"text":"Q"})).unwrap();
    assert_eq!(layer_doc(&session, layer).unwrap().text, "aQef");
}

#[test]
fn restored_text_mode_preserves_pending_character_style() {
    let (mut session, layer) = fixture();
    session.execute("text.edit", json!({"layer":layer.0,"caret":2})).unwrap();
    session.execute("layer.setText", json!({"layer":layer.0,"range":[2,2],"size":12,"fill":"#00ff00"})).unwrap();
    let saved = session.call_state();
    session.execute("text.moveCaret", json!({"to":"end"})).unwrap();
    session.restore_call_state(&saved);
    session.execute("text.insert", json!({"text":"Q"})).unwrap();
    let document = layer_doc(&session, layer).unwrap();
    assert_eq!(document.text, "abQcdef");
    assert_eq!(document.style_at(2).size, 12.0);
    assert_eq!(document.style_at(2).fill, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(document.style_at(3).size, 60.0);
}

#[test]
fn restoring_no_text_mode_clears_a_new_session() {
    let (mut session, layer) = fixture();
    let saved = session.call_state();
    session.execute("text.edit", json!({"layer":layer.0,"caret":1})).unwrap();
    session.restore_call_state(&saved);
    session.execute("text.insert", json!({"layer":layer.0,"text":"Q"})).unwrap();
    assert_eq!(layer_doc(&session, layer).unwrap().text, "abcdefQ");
}

#[test]
fn restored_text_mode_is_sanitized_against_the_current_project() {
    let (mut session, layer) = fixture();
    session.execute("text.edit", json!({"layer":layer.0,"select":[4,6]})).unwrap();
    let saved = session.call_state();
    session.execute("text.endEdit", json!({})).unwrap();
    session.execute("layer.setText", json!({"layer":layer.0,"text":"ab"})).unwrap();
    session.restore_call_state(&saved);
    let edit = session.state.text_edit.as_ref().unwrap();
    assert_eq!((edit.anchor, edit.caret), (2, 2));
    session.execute("edit.delete", json!({"layers":[layer.0]})).unwrap();
    session.restore_call_state(&saved);
    assert!(session.state.text_edit.is_none(), "restoring a deleted editing target must drop the session");
}
