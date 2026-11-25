use kition_kit::{CallContext, EditorState, KitDocument};
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[test]
fn recent_preset_declares_the_time_used_by_its_delegated_application() {
    let document = MotionDocument::create("Recent.kmotion", 8, 8).unwrap();
    let commands = document.commands();
    let command = commands.iter().find(|command| command.id == "animation.applyRecentPreset").unwrap();
    assert!(command.reads.contains(&EditorState::Time), "recent preset application shifts keys by CTI: {:?}", command.reads);
}

#[test]
fn recent_preset_key_placement_uses_the_call_time_and_restoring_preserves_the_editor_time() {
    let mut document = MotionDocument::create("Recent.kmotion", 8, 8).unwrap();
    let source = document.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].clone();
    for (time, value) in [(0, 0), (1, 100)] {
        document.execute("prop.addKey", json!({"layer":source,"path":"transform/opacity","time":time,"value":value})).unwrap();
    }
    document.execute("prop.select", json!({"layer":source,"path":"transform/opacity"})).unwrap();
    document.execute("animation.savePreset", json!({"path":"fade.ecpreset"})).unwrap();
    document.execute("animation.applyPreset", json!({"path":"fade.ecpreset","layers":[source]})).unwrap();
    let target = document.execute("layer.newSolid", json!({"name":"Target"})).unwrap()["layer"].clone();
    let saved = document.editor_state();
    document.set_context(&CallContext { time: Some(3.0), ..Default::default() }).unwrap();
    document.execute("animation.applyRecentPreset", json!({"index":0,"layers":[target]})).unwrap();
    document.restore_editor_state(&saved).unwrap();
    assert_eq!(document.editor_state(), saved);
    let property = document.execute("prop.get", json!({"layer":target,"path":"transform/opacity","time":0})).unwrap();
    let times: Vec<_> = property["keys"].as_array().unwrap().iter().map(|key| key["compTime"].as_f64().unwrap()).collect();
    assert_eq!(times, [3.0, 4.0]);
}
