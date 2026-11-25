//! An agent sets property values of every shape the engine takes: a number (opacity), a vector
//! (position) and keyframes of either. The schemas once typed `value` as a string, so the strict
//! parameter checks refused `prop.addKey {value: 100}` and no number could be keyed.

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit::KitDocument;
use kition_kit_server::protocol::Origin;
use kition_kit_server::{KitBackend, OpenRequest};
use kition_motion_kit::MotionDocument;
use serde_json::{Map, Value, json};

fn params(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

#[test]
fn the_agent_keys_and_sets_numbers_and_vectors() {
    let root = std::env::temp_dir().join(format!("kition-motion-property-values-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("title.kmotion");
    let mut doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Band", "color": "#ff0000" })).unwrap();
    std::fs::write(&file, doc.save(None).unwrap()).unwrap();

    let mut kit =
        Headless::<MotionDocument>::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_motion_kit::EDITOR_STATE).unwrap());
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let layer = kit.describe(&id).unwrap().state["info"]["layers"][0]["id"].clone();
    let calls = [
        ("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 0, "value": 100 })),
        ("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 2, "value": 0 })),
        ("prop.set", json!({ "layer": layer, "path": "transform/position", "value": [10, 20] })),
        ("layer.setTransform", json!({ "layers": [layer], "prop": "rotation", "value": 45 })),
    ];
    for (command, given) in calls {
        if let Err(error) = kit.execute(&id, command, params(given.clone()), Origin::Agent) {
            panic!("{command} {given} was refused: {}", error.message);
        }
    }
    let animated = kit.describe(&id).unwrap().state["info"]["layers"][0]["animated"].clone();
    assert_eq!(animated, json!([{ "path": "transform/opacity", "name": "Opacity", "times": [0.0, 2.0] }]), "opacity is keyed at 0 s and 2 s");
    let _ = std::fs::remove_dir_all(root);
}

/// Each `animated` entry of what the agent reads names its property by the path `prop.*` commands
/// take: passed back unchanged as `prop.addKey`'s `path`, it keys that property and no other, a
/// transform property as well as one of two effects whose properties share their display names.
#[test]
fn an_animated_path_keys_the_same_property_again() {
    let root = std::env::temp_dir().join(format!("kition-motion-animated-paths-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("title.kmotion");
    let mut doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Band", "color": "#ff0000" })).unwrap();
    std::fs::write(&file, doc.save(None).unwrap()).unwrap();

    let mut kit =
        Headless::<MotionDocument>::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_motion_kit::EDITOR_STATE).unwrap());
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let layer = kit.describe(&id).unwrap().state["info"]["layers"][0]["id"].clone();
    let calls = [
        ("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 0, "value": 100 })),
        ("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 2, "value": 0 })),
        ("effect.apply", json!({ "layer": layer, "effect": "Gaussian Blur" })),
        ("effect.apply", json!({ "layer": layer, "effect": "Gaussian Blur" })),
        ("prop.addKey", json!({ "layer": layer, "path": "effects/#2/blurriness", "time": 1, "value": 4 })),
    ];
    for (command, given) in calls {
        if let Err(error) = kit.execute(&id, command, params(given.clone()), Origin::Agent) {
            panic!("{command} {given} was refused: {}", error.message);
        }
    }
    let animated = |kit: &mut Headless<MotionDocument>| kit.describe(&id).unwrap().state["info"]["layers"][0]["animated"].as_array().cloned().unwrap();
    let before = animated(&mut kit);
    assert_eq!(before.len(), 2, "the second blur's blurriness and opacity are keyed: {before:?}");
    for (i, entry) in before.iter().enumerate() {
        let path = entry["path"].clone();
        let given = json!({ "layer": layer, "path": path, "time": 3 });
        if let Err(error) = kit.execute(&id, "prop.addKey", params(given.clone()), Origin::Agent) {
            panic!("prop.addKey {given} was refused: {}", error.message);
        }
        let after = animated(&mut kit);
        for (j, other) in after.iter().enumerate() {
            let (was, now) = (before[j]["times"].as_array().unwrap().len(), other["times"].as_array().unwrap().len());
            let grown = if j <= i { was + 1 } else { was };
            assert_eq!(now, grown, "keying {path} keyed {} (entry {j}): {after:?}", other["path"]);
        }
    }
    let paths: Vec<Value> = animated(&mut kit).iter().map(|entry| entry["path"].clone()).collect();
    assert_eq!(paths, [json!("effects/#2/blurriness"), json!("transform/opacity")], "readable paths, as prop.* take them");
    let _ = std::fs::remove_dir_all(root);
}

/// khp-bench formal runs: a checkbox property takes `true` / `false` (the schema allowed only
/// numbers, strings, vectors and objects, though the engine reads booleans), and a keyframe's
/// speed is one number for a one-dimensional property (the schema wanted `[0]` for `0`).
#[test]
fn the_agent_sets_checkboxes_and_one_number_speeds() {
    let root = std::env::temp_dir().join(format!("kition-motion-checkbox-values-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("title.kmotion");
    let mut doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Band", "color": "#ff0000" })).unwrap();
    std::fs::write(&file, doc.save(None).unwrap()).unwrap();

    let mut kit =
        Headless::<MotionDocument>::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_motion_kit::EDITOR_STATE).unwrap());
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let layer = kit.describe(&id).unwrap().state["info"]["layers"][0]["id"].clone();
    let calls = [
        ("prop.set", json!({ "layer": layer, "path": "materialOptions/acceptsLights", "value": false })),
        ("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 0, "value": 0 })),
        ("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 1, "value": 100 })),
        ("keyframe.select", json!({ "keys": [{ "layer": layer, "path": "transform/opacity", "time": 1 }] })),
        ("keyframe.velocity", json!({ "outSpeed": 0, "inSpeed": 0 })),
    ];
    for (command, given) in calls {
        if let Err(error) = kit.execute(&id, command, params(given.clone()), Origin::Agent) {
            panic!("{command} {given} was refused: {} {:?}", error.message, error.data);
        }
    }
    let read = kit.execute(&id, "prop.get", params(json!({ "layer": layer, "path": "materialOptions/acceptsLights" })), Origin::Agent).unwrap();
    assert_eq!(read.result.unwrap()["value"], json!(false), "the checkbox is off");
    let _ = std::fs::remove_dir_all(root);
}
