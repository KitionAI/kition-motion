//! An agent gives shapes and text their colours the ways the engine reads them: `#rrggbb`,
//! `[r, g, b]`, `false` for no fill or stroke, a paint type by name, and a shape's size as
//! `[w, h]` or one number. The parameter schemas once typed `stroke` as a whole number, `fill` as
//! one of the two colour forms, `fillType` as a number and `size` as a number, so the strict
//! checks refused every stroke colour and the engine's own forms (khp-bench r3, V05 and V06).

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
fn the_agent_paints_shapes_and_text_in_every_form_the_engine_reads() {
    let root = std::env::temp_dir().join(format!("kition-motion-paint-params-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("countdown.kmotion");
    let doc = MotionDocument::create("countdown.kmotion", 64, 36).unwrap();
    std::fs::write(&file, doc.save(None).unwrap()).unwrap();

    let mut kit = Headless::<MotionDocument>::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], &[]).unwrap());
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("countdown.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let calls = [
        ("layer.newShape", json!({ "kind": "ellipse", "size": 520, "fill": false, "stroke": "#2f7bff", "strokeWidth": 14 })),
        ("layer.newShape", json!({ "kind": "rounded", "size": [472, 350], "fill": [1, 1, 1], "fillType": "solid", "stroke": [0, 0, 0], "strokeWidth": 2 })),
        ("layer.newShape", json!({ "kind": "rect", "fillType": "radial", "strokeType": "linear", "stroke": "#ffffff", "strokeWidth": 4 })),
        ("shape.newShape", json!({ "kind": "rect", "size": 100, "fill": "#ff0000", "fillType": "linear", "stroke": false })),
        ("shape.toolOptions", json!({ "fill": "#00ff00", "stroke": [0, 0, 1] })),
        ("layer.newText", json!({ "text": "3", "fill": [1, 1, 1], "stroke": "#000000", "strokeWidth": 2 })),
    ];
    for (command, given) in calls {
        if let Err(error) = kit.execute(&id, command, params(given.clone()), Origin::Agent) {
            panic!("{command} {given} was refused: {} {:?}", error.message, error.data);
        }
    }
    let text = kit.execute(&id, "layer.newText", params(json!({ "text": "Go" })), Origin::Agent).unwrap().result.unwrap()["layer"].clone();
    kit.execute(&id, "layer.setText", params(json!({ "layer": text, "fill": "#ff00ff", "stroke": [0, 0, 0] })), Origin::Agent).unwrap();
    let refused = kit.execute(&id, "layer.newShape", params(json!({ "size": "big" })), Origin::Agent).unwrap_err();
    assert!(refused.message.contains("size") || refused.data.is_some_and(|data| data.to_string().contains("size")), "a size no form fits is refused");
    let _ = std::fs::remove_dir_all(root);
}
