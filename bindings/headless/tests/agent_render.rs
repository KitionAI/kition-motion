//! The agent's view of a motion graphics project through `kition-motion` served headless: the
//! preview is the kit's render command (`agent.render` in kit.json), run as the agent, and what
//! `view.describe` answers (the `motion_inspect` details) is the document alone: moving the current
//! time or selecting layers, which are the editor's state, changes neither, and rendering changes
//! nothing. Keyframe times, which are the document's, stay in the details.

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit::KitDocument;
use kition_kit_server::protocol::Origin;
use kition_kit_server::{KitBackend, OpenRequest};
use kition_motion_kit::MotionDocument;
use serde_json::{Map, Value, json};

const MANIFEST: &str = include_str!("../../../kit.json");

fn params(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// A 320 x 180, two-second comp, red for a second and blue for the next, the blue layer fading in
/// (opacity keys at 1 s and 1.5 s), saved in a fresh workspace.
fn workspace(name: &str) -> (std::path::PathBuf, HostFolder) {
    let root = std::env::temp_dir().join(format!("kition-motion-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let mut doc = MotionDocument::create("Motion.kmotion", 320, 180).unwrap();
    doc.execute("comp.settings", json!({ "duration": 2 })).unwrap();
    for (color, from) in [("#ff0000", 0), ("#0000ff", 1)] {
        let layer = doc.execute("layer.newSolid", json!({ "color": color })).unwrap()["layer"].clone();
        doc.execute("layer.timing", json!({ "layers": [layer], "in": from, "out": from + 1 })).unwrap();
        if from == 1 {
            for (time, value) in [(1.0, 0.0), (1.5, 100.0)] {
                doc.execute("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": time, "value": value, "timeBase": "comp" })).unwrap();
            }
        }
    }
    std::fs::write(root.join("Motion.kmotion"), doc.save(None).unwrap()).unwrap();
    let folder = HostFolder::new(&root).unwrap();
    (root, folder)
}

/// The headless kit as the `kition-motion` executable makes it.
fn kit() -> Headless<MotionDocument> {
    let info = KitInfo::from_manifest(MANIFEST, &["file.import"], kition_motion_kit::EDITOR_STATE).unwrap();
    Headless::<MotionDocument>::with_replica_port(info)
}

fn run(kit: &mut Headless<MotionDocument>, id: &str, command: &str, value: Value, origin: Origin) -> Value {
    kit.execute(id, command, params(value), origin).unwrap_or_else(|error| panic!("{command}: {error:?}")).result.unwrap_or(Value::Null)
}

#[test]
fn the_manifest_names_the_render_command() {
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    assert_eq!(manifest["agent"]["render"], json!("file.render"));
}

#[test]
fn the_agent_renders_a_time_and_changes_nothing() {
    let (root, folder) = workspace("agent-render");
    let mut kit = kit();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("Motion.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let described = kit.describe(&id).unwrap().state;
    let done = kit.execute(&id, "file.render", params(json!({ "target": { "time": 1.5 }, "scale": 1, "maxSize": 1024 })), Origin::Agent).unwrap();
    assert!(!done.changed, "rendering is no edit");
    let answer = done.result.unwrap();
    assert!(answer["png"].as_str().is_some_and(|png| !png.is_empty()), "{answer}");
    assert_eq!((answer["width"].as_u64(), answer["height"].as_u64()), (Some(320), Some(180)));
    assert!(!kit.document(&id).unwrap().dirty, "rendering leaves the document clean");
    assert_eq!(kit.describe(&id).unwrap().state, described);
    // Out of range, the agent is told the range, as invalid parameters.
    let error = kit.execute(&id, "file.render", params(json!({ "target": { "time": 9 } })), Origin::Agent).unwrap_err();
    let shown = format!("{error:?}");
    assert!(shown.contains("invalid_params") && shown.contains("/target/time") && shown.contains("from 0 to 2 s"), "{shown}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_current_time_and_the_selection_are_no_part_of_the_description() {
    let (root, folder) = workspace("describe-time");
    let mut kit = kit();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("Motion.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let before = kit.describe(&id).unwrap().state;
    let blue = before["info"]["layers"][0]["id"].clone();
    // The keyframe times are the document's: the session keys kition-headless leaves out
    // (`time`, `selection`, `playhead`) take none of them away.
    let animated = &before["info"]["layers"][0]["animated"][0];
    assert_eq!((animated["name"].as_str(), animated["times"].clone()), (Some("Opacity"), json!([1.0, 1.5])), "{before:?}");
    run(&mut kit, &id, "time.set", json!({ "time": 1.25 }), Origin::User);
    run(&mut kit, &id, "layer.select", json!({ "layers": [blue] }), Origin::User);
    let after = kit.describe(&id).unwrap().state;
    assert_eq!(after, before, "the editor's current time and selection changed what the agent reads of the document");
    let text = serde_json::to_string(&after).unwrap();
    for key in kition_motion_kit::EDITOR_STATE {
        assert!(!text.contains(&format!("\"{key}\"")), "{key} is the editor's, not the document's");
    }
    let _ = std::fs::remove_dir_all(root);
}
