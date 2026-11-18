use kition_kit::{EditorState, KitDocument};
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[test]
fn view_switch_commands_declare_the_current_and_last_view_mode_they_read() {
    let document = MotionDocument::create("Views.kmotion", 8, 8).unwrap();
    let commands = document.commands();
    let missing: Vec<_> = [
        "view.3d.activeCamera",
        "view.3d.back",
        "view.3d.bottom",
        "view.3d.custom1",
        "view.3d.custom2",
        "view.3d.custom3",
        "view.3d.default",
        "view.3d.front",
        "view.3d.last",
        "view.3d.left",
        "view.3d.right",
        "view.3d.top",
        "view.set3DView",
    ]
    .into_iter()
    .filter(|id| !commands.iter().find(|command| command.id == *id).unwrap().reads.contains(&EditorState::Mode))
    .collect();
    assert!(missing.is_empty(), "three_d.rs:308-315,349: switching views reads current/last mode: {missing:?}");
}

#[test]
fn restoring_view_context_restores_the_input_of_the_last_view_command() {
    let mut document = MotionDocument::create("Views.kmotion", 8, 8).unwrap();
    document.execute("view.3d.front", json!({})).unwrap();
    let saved = document.editor_state();
    document.execute("view.3d.left", json!({})).unwrap();
    document.restore_editor_state(&saved).unwrap();
    assert_eq!(document.execute("view.3d.last", json!({})).unwrap()["view"], "activeCamera");
    assert_eq!(document.execute("view.3d.last", json!({})).unwrap()["view"], "front");
}
