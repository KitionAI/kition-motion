//! C8-land-11t: resolver declarations and retained premises.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::*;
use kition_kit::KitDocument;
use serde_json::{Value, json};

#[test]
fn resolver_declarations() {
    let doc = kition_motion_kit::MotionDocument::open(FILE, &project(2)).unwrap();
    let commands = doc.commands();
    if std::env::var_os("C8_DECLARATIONS").is_some() {
        for c in &commands {
            println!("{}: target={:?} objects={:?} schema={}", c.id, c.targets, c.objects, c.schema.as_ref().unwrap_or(&Value::Null));
        }
    }
    let command = |id: &str| commands.iter().find(|c| c.id == id).unwrap();
    assert!(command("keys.select").objects.contains(&"keys[].layer"));
    assert!(command("paint.options").objects.contains(&"cloneSource"));
    assert!(!command("file.replaceWithLayeredComp").name_params.contains(&("item", "name")));
    assert!(command("prop.pickWhip").name_params.contains(&("target.layer", "name")));
    // The ordinary declaration marks scalar layer references; points remain coordinates.
    assert!(command("transition.iris").objects.contains(&"center"));
    assert!(command("transition.iris").name_params.contains(&("center", "name")));
    let mut marks = 0;
    for c in &commands {
        marks += c.objects.len() + usize::from(c.targets.is_some());
        if c.targets.is_some() {
            assert_eq!(c.name_field, Some("name"), "{}", c.id);
        }
        for (path, _) in c.object_types {
            if c.targets == Some(path)
                || *path == "cloneSource"
                || (c.id == "project.newFolder" && *path == "parent")
                || (c.id == "file.replaceWithLayeredComp" && *path == "item")
            {
                continue;
            }
            let names = if c.schema.as_ref().and_then(|schema| schema.get(*path)).and_then(|node| node.get("type")).and_then(Value::as_str) == Some("array") {
                format!("{path}[]")
            } else {
                (*path).to_owned()
            };
            assert!(c.name_params.contains(&(names.as_str(), "name")), "{}: name-taking resolver at {path} is undeclared", c.id);
        }
    }
    println!("object marks: {marks}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_secondary_numeric_stack_reference_holds_the_layer() {
    for changed in [true, false] {
        let f = Fixture::new(3);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        f.colleague(&[("layer.rename", json!({"layer": if changed {layers[0]} else {layers[1]}, "name": "Theirs"}))]);
        let answer = f
            .call(
                "run",
                json!({"basis": basis, "steps": [{"command": "layer.setParent", "params": {"layers": [layers[2]], "parent": 1, "compensate": false}}]}),
            )
            .await;
        if changed {
            assert_conflict(&answer.expect_err("indexed parent changed"), &layers[0].to_string(), "changed");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_named_parent_keeps_the_observed_name_target() {
    for changed in [true, false] {
        let f = Fixture::new(3);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        f.colleague(&[("layer.rename", json!({"layer": layers[1], "name": if changed {"Gone"} else {"L1"}}))]);
        let answer = f.call("run", json!({"basis": basis, "steps": [{"command": "layer.setParent", "params": {"layers": [layers[0]], "parent": "L1"}}]})).await;
        if changed {
            let error = answer.expect_err("renamed parent");
            assert_eq!(error["error"]["code"], "conflict", "{error}");
            assert_conflict(&error, &layers[1].to_string(), "relocated");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn nested_key_references_hold_the_layer() {
    let f = Fixture::new(2);
    let (info, _, _) = f.inspect().await;
    let layer = layer_ids(&info)[0];
    f.ok("run", json!({"steps": [{"command": "prop.addKey", "params": {"layer": layer, "path": OPACITY, "time": 1, "value": 20}}]})).await;
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("prop.set", json!({"layer": layer, "path": OPACITY, "time": 1, "value": 30}))]);
    let error = f.refused("run", json!({"basis": basis, "steps": [{"command": "keys.select", "params": {"keys": [{"layer": layer, "path": OPACITY, "time": 1}]}}, {"command": "keys.delete"}]})).await;
    assert_conflict(&error, &layer.to_string(), "changed");
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn nested_stack_references_hold_what_the_index_names() {
    for changed in [true, false] {
        let f = Fixture::new(2);
        let (info, _, _) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        f.ok("run", json!({"steps": [{"command": "prop.addKey", "params": {"layer": layer, "path": OPACITY, "time": 1, "value": 20}}]})).await;
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("prop.set", json!({"layer": if changed {layer} else {layer_ids(&info)[1]}, "path": OPACITY, "time": 1, "value": 30}))]);
        let answer = f.call("run", json!({"basis": basis, "steps": [{"command": "keys.select", "params": {"keys": [{"layer": 1, "path": OPACITY, "time": 1}]}}, {"command": "keys.delete"}]})).await;
        if changed {
            assert_conflict(&answer.expect_err("indexed layer changed"), &layer.to_string(), "changed");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn compensation_keeps_the_old_parent_transform() {
    for changed in [true, false] {
        let f = Fixture::new(3);
        let (info, _, _) = f.inspect().await;
        let layers = layer_ids(&info);
        f.ok("run", json!({"steps": [{"command": "layer.setParent", "params": {"layers": [layers[0]], "parent": layers[1]}}]})).await;
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if changed {
            ("prop.set", json!({"layer": layers[1], "path": "transform/position", "value": [40, 50, 0]}))
        } else {
            ("layer.rename", json!({"layer": layers[1], "name": "Other name"}))
        }]);
        let answer =
            f.call("run", json!({"basis": basis, "steps": [{"command": "layer.setParent", "params": {"layers": [layers[0]], "parent": layers[2]}}]})).await;
        if changed {
            assert_conflict(&answer.expect_err("old parent changed"), &layers[1].to_string(), "changed");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn stored_float_precision_is_in_the_receipt() {
    let f = Fixture::new(1);
    f.inspect().await;
    let loaded = f.ok("query", json!({"objects": [1]})).await;
    let held = &loaded["objects"][0]["fields"]["comp"];
    let ran = f
        .ok(
            "batch",
            json!({"ops": [{"op": "map.set", "target": held, "field": "attrs", "key": "background", "value": [0.123456789, 0.2, 0.3]}], "changes": "all"}),
        )
        .await;
    let after = &entry(&ran["changes"]["changed"], "1")["edits"][0]["after"];
    let file: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
    let comp = &file["items"]["1"]["kind"];
    let stored: [f32; 3] = serde_json::from_value(comp["background"].clone()).unwrap();
    assert_eq!(*after, serde_json::to_value(stored).unwrap(), "receipt must name the f32 stored value");
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn iris_center_holds_a_layer_but_not_point_coordinates() {
    for center_is_layer in [true, false] {
        let f = Fixture::new(3);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        f.colleague(&[("prop.set", json!({"layer": layers[1], "path": "transform/position", "value": [10, 20, 0]}))]);
        let center = if center_is_layer { json!(layers[1]) } else { json!([layers[1], 30]) };
        let answer = f.call("run", json!({"basis": basis, "steps": [{"command": "transition.iris", "params": {"layer": layers[0], "center": center}}]})).await;
        if center_is_layer {
            assert_conflict(&answer.expect_err("center changed"), &layers[1].to_string(), "changed");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_layer_control_value_holds_its_referenced_layer() {
    for changed in [true, false] {
        let f = Fixture::new(2);
        let (info, _, _) = f.inspect().await;
        let layers = layer_ids(&info);
        f.ok("run", json!({"steps": [{"command": "layer.newLight", "params": {"kind": "Environment", "name": "Environment"}}]})).await;
        let (info, _, basis) = f.inspect().await;
        let light = layer_ids(&info)[0];
        f.colleague(&[("layer.rename", json!({"layer": if changed {layers[0]} else {layers[1]}, "name": "Theirs"}))]);
        let answer = f
            .call(
                "run",
                json!({"basis": basis, "steps": [{"command": "prop.set", "params": {"layer": light, "path": "lightOptions/source", "value": layers[0]}}]}),
            )
            .await;
        if changed {
            assert_conflict(&answer.expect_err("referenced layer changed"), &layers[0].to_string(), "changed");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[test]
fn sdk_numeric_members_describe_tagged_property_values() {
    let schema = kition_kit::kdm::Schema::parse(
        r#"{"kdm":1,"kit":"kit.motion","schemaVersion":1,"root":"Node","types":{"Node":{"fields":{"value":{"kind":"json","members":{"v":{"minimum":0,"maximum":100}}}}}}}"#,
    )
    .unwrap();
    assert!(schema.check_value("Node", "value", &json!({"t":"Scalar","v":100})).is_ok());
    assert!(schema.check_value("Node", "value", &json!({"t":"Scalar","v":101})).is_err());
}
