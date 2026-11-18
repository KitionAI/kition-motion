use kition_kit::{EditorState, KitDocument};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn text_fixture() -> (MotionDocument, Value) {
    let mut document = MotionDocument::create("Typing.kmotion", 64, 64).unwrap();
    let layer = document.execute("layer.newText", json!({"text":"abcdef","size":60})).unwrap()["layer"].clone();
    (document, layer)
}

fn agent(document: &mut MotionDocument, command: &str, params: Value) {
    document.execute_as(command, params, &kition_kit::kdm::Origin::agent("typing-agent")).unwrap();
}

fn text(document: &mut MotionDocument, layer: &Value) -> Value {
    document.execute("prop.get", json!({"layer":layer,"path":"text/sourceText"})).unwrap()["value"].clone()
}

#[test]
fn text_session_commands_declare_the_mode_that_selects_their_target_or_range() {
    let document = MotionDocument::create("Text.kmotion", 8, 8).unwrap();
    let commands = document.commands();
    let missing: Vec<_> = [
        "edit.pasteTextFormattingOnly",
        "edit.pasteTextMatchFormatting",
        "text.delete",
        "text.edit",
        "text.endEdit",
        "text.insert",
        "text.moveCaret",
        "text.setSelection",
    ]
    .into_iter()
    .filter(|id| !commands.iter().find(|command| command.id == *id).unwrap().reads.contains(&EditorState::Mode))
    .collect();
    assert!(missing.is_empty(), "text-edit session inputs are undeclared: {missing:?}");
}

#[test]
fn font_features_declare_the_text_mode_used_for_the_implicit_font() {
    let document = MotionDocument::create("Fonts.kmotion", 8, 8).unwrap();
    let commands = document.commands();
    let command = commands.iter().find(|command| command.id == "text.fontFeatures").unwrap();
    assert!(command.reads.contains(&EditorState::Mode), "text_edit.rs:476-480 chooses the font at the edited range: {:?}", command.reads);
}

#[test]
fn named_agent_text_selection_and_insertions_keep_the_caret_between_steps() {
    let (mut document, layer) = text_fixture();
    let before_run = document.editor_state();
    agent(&mut document, "text.setSelection", json!({"layer":layer,"start":1,"end":4}));
    agent(&mut document, "text.insert", json!({"layer":layer,"text":"Q"}));
    agent(&mut document, "text.insert", json!({"layer":layer,"text":"R"}));
    assert_eq!(text(&mut document, &layer), "aQRef");
    document.restore_editor_state(&before_run).unwrap();
    assert!(document.editor_state()["textEdit"].is_null(), "the outer run must still restore the person's mode");
}

#[test]
fn named_agent_text_delete_keeps_the_replacement_caret_for_the_next_step() {
    let (mut document, layer) = text_fixture();
    document.execute("text.edit", json!({"layer":layer,"select":[1,4]})).unwrap();
    agent(&mut document, "text.delete", json!({"layer":layer}));
    agent(&mut document, "text.insert", json!({"layer":layer,"text":"Q"}));
    assert_eq!(text(&mut document, &layer), "aQef");
}

#[test]
fn named_agent_text_edit_caret_motion_and_pending_style_survive_until_typing() {
    let (mut document, layer) = text_fixture();
    agent(&mut document, "text.edit", json!({"layer":layer,"caret":1}));
    agent(&mut document, "text.moveCaret", json!({"layer":layer,"to":"right"}));
    agent(&mut document, "layer.setText", json!({"layer":layer,"range":[2,2],"size":12}));
    assert_eq!(document.editor_state()["textEdit"]["pending"]["size"], 12.0);
    agent(&mut document, "text.insert", json!({"layer":layer,"text":"Q"}));
    assert_eq!(text(&mut document, &layer), "abQcdef");
}

#[test]
fn named_agent_matching_text_paste_keeps_the_caret_for_the_next_step() {
    let (mut document, layer) = text_fixture();
    document.execute("text.edit", json!({"layer":layer,"select":[1,4]})).unwrap();
    agent(&mut document, "edit.pasteTextMatchFormatting", json!({"text":"Q"}));
    agent(&mut document, "text.insert", json!({"layer":layer,"text":"R"}));
    assert_eq!(text(&mut document, &layer), "aQRef");
}

#[test]
fn a_failed_named_agent_caret_move_restores_the_previous_mode() {
    let (mut document, layer) = text_fixture();
    let before = document.editor_state();
    assert!(document.execute_as("text.moveCaret", json!({"layer":layer,"to":"invalid-target"}), &kition_kit::kdm::Origin::agent("typing-agent")).is_err());
    assert_eq!(document.editor_state(), before);
}
