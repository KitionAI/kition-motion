//! `kition-motion` as `kition-headless` serves it (`run_manifest_model`): a parameter of the wrong JSON
//! type is refused by the SDK's schema check (KHP A5, A18), for the agent and the person alike,
//! and the project stays as it was. The engine's readers would otherwise take the value as absent
//! and fall back to a default (`layer.newSolid {width: "wide"}` made a default-size solid).

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit::KitDocument;
use kition_kit_server::protocol::Origin;
use kition_kit_server::{ErrorReason, KitBackend, OpenRequest};
use kition_motion_kit::MotionDocument;
use serde_json::{Map, Value, json};

fn params(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

#[test]
fn a_value_of_the_wrong_type_is_refused_and_changes_nothing() {
    let root = std::env::temp_dir().join(format!("kition-motion-params-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("Title.kmotion");
    std::fs::write(&file, MotionDocument::create("Title.kmotion", 320, 180).unwrap().save(None).unwrap()).unwrap();
    let info = KitInfo::from_manifest(include_str!("../../../kit.json"), &["file.import"], kition_motion_kit::EDITOR_STATE).unwrap();
    let mut kit = Headless::<MotionDocument>::with_replica_port(info);
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("Title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let bad = params(json!({ "name": { "khp-check": "not a valid value" }, "color": "#ff0000" }));
    for origin in [Origin::Agent, Origin::User] {
        let error = kit.execute(&id, "layer.newSolid", bad.clone(), origin).unwrap_err();
        assert_eq!(error.code, ErrorReason::InvalidParams.code());
        let detail = error.data.unwrap()["detail"].as_str().unwrap().to_owned();
        assert!(detail.contains("`name`: expected a string"), "{detail}");
        assert!(!kit.document(&id).unwrap().dirty, "nothing changed");
    }
    let made = kit.execute(&id, "layer.newSolid", params(json!({ "name": "Red", "color": "#ff0000" })), Origin::Agent).unwrap();
    assert!(made.changed, "a right call runs");
    let _ = std::fs::remove_dir_all(root);
}
