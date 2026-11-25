//! C8-motion-read: source-dependent writes checked against an independent saved-file writer.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn source_item(bytes: &[u8], layer: u64) -> u64 {
    let project: Value = serde_json::from_slice(bytes).unwrap();
    project["items"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|item| item["kind"]["layers"].as_array())
        .flatten()
        .find(|item| item["id"] == layer)
        .unwrap()["source"]["item"]
        .as_u64()
        .unwrap()
}

fn effects() -> (Vec<u8>, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    doc.execute("effect.apply", json!({"layers":[layer],"effect":"Gaussian Blur"})).unwrap();
    let bytes = doc.save(None).unwrap();
    let item = source_item(&bytes, layer);
    (bytes, layer, item)
}

#[tokio::test]
async fn effect_point_defaults_hold_source_dimensions_without_layer_values() {
    let (bytes, layer, item) = effects();
    for change in 0..3 {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[match change {
            0 => ("layer.settings", json!({"layer":layer,"width":640})),
            1 => opacity(layer, 23.0),
            _ => ("prop.set", json!({"layer":layer,"path":"effects/#1/blurriness","value":13})),
        }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"effect.apply","params":{"layers":[layer],"effect":"Gradient Ramp"}}]})).await;
        if change == 0 {
            assert_conflict(&answer.unwrap_err(), &item.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "effect defaults refused unrelated existing layer values: {answer:?}");
            if change == 1 {
                assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
            } else {
                assert_eq!(value_on_disk(&f.file(), layer, "effects/#1/blurriness"), json!(13.0));
            }
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn effect_append_holds_its_stack_membership() {
    let (bytes, layer, _) = effects();
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("effect.apply", json!({"layers":[layer],"effect":"Gradient Ramp"}))]);
    let refused = f.refused("run", json!({"basis":basis,"steps":[{"command":"effect.apply","params":{"layers":[layer],"effect":"Gradient Ramp"}}]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "a new matching effect changed the next instance's name and append gap: {refused}");
    f.shutdown().await;
}

#[tokio::test]
async fn effect_without_dimension_defaults_ignores_source_size() {
    let (bytes, layer, _) = effects();
    for params in [json!({"effect":"Gaussian Blur"}), json!({"effect":"Gradient Ramp","params":{"start":[1,2],"end":[3,4]}})] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("layer.settings", json!({"layer":layer,"width":640}))]);
        let mut params = params;
        params["layers"] = json!([layer]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"effect.apply","params":params}]})).await;
        assert!(answer.is_ok(), "an effect without dimension-derived defaults refused source size: {answer:?}");
        f.shutdown().await;
    }
}

fn shared_solid() -> (Vec<u8>, u64, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    let item = source_item(&doc.save(None).unwrap(), layer);
    let other = doc.execute("layer.addItem", json!({"item":item})).unwrap()["layer"].as_u64().unwrap();
    (doc.save(None).unwrap(), layer, other, item)
}

#[tokio::test]
async fn shared_solid_settings_hold_other_source_users_without_their_values() {
    let (bytes, layer, other, _) = shared_solid();
    for change in 0..3 {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[match change {
            0 => ("layer.settings", json!({"layer":other,"color":"#112233","affectAll":false})),
            1 => opacity(layer, 23.0),
            _ => opacity(other, 23.0),
        }]);
        let answer =
            f.call("run", json!({"basis":basis,"steps":[{"command":"layer.settings","params":{"layer":layer,"color":"#445566","affectAll":false}}]})).await;
        if change == 0 {
            assert_conflict(&answer.unwrap_err(), &other.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "shared-solid settings refused opacity: {answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), if change == 1 { layer } else { other }), 23.0);
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn shared_solid_settings_copy_source_data_without_copying_comments() {
    let (bytes, layer, other, item) = shared_solid();
    for relevant in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if relevant {
            ("layer.settings", json!({"layer":other,"width":640}))
        } else {
            ("project.setComment", json!({"items":[item],"comment":"Colleague source comment"}))
        }]);
        let answer =
            f.call("run", json!({"basis":basis,"steps":[{"command":"layer.settings","params":{"layer":layer,"color":"#445566","affectAll":false}}]})).await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &item.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "cloning a solid does not copy its source comment: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn shared_solid_settings_hold_layer_membership() {
    let (bytes, layer, other, item) = shared_solid();
    for remove in [true, false] {
        let bytes = if remove {
            bytes.clone()
        } else {
            let mut doc = MotionDocument::open(FILE, &bytes).unwrap();
            doc.execute("edit.delete", json!({"layers":[other]})).unwrap();
            doc.save(None).unwrap()
        };
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if remove { ("edit.delete", json!({"layers":[other]})) } else { ("layer.addItem", json!({"item":item})) }]);
        let refused =
            f.refused("run", json!({"basis":basis,"steps":[{"command":"layer.settings","params":{"layer":layer,"color":"#445566","affectAll":false}}]})).await;
        assert_eq!(refused["error"]["code"], "conflict", "the set of source users changed: {refused}");
        f.shutdown().await;
    }
}

fn other_comp_source_user(shared: bool) -> (Vec<u8>, u64, u64, u64, Option<u64>) {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    let item = source_item(&doc.save(None).unwrap(), layer);
    let other_comp = doc.execute("comp.new", json!({"name":"Other","open":false})).unwrap()["comp"].as_u64().unwrap();
    let other = shared.then(|| doc.execute("layer.addItem", json!({"comp":other_comp,"item":item})).unwrap()["layer"].as_u64().unwrap());
    (doc.save(None).unwrap(), layer, item, other_comp, other)
}

#[tokio::test]
async fn source_user_added_in_another_comp() {
    let (bytes, layer, item, other_comp, _) = other_comp_source_user(false);
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("layer.addItem", json!({"comp":other_comp,"item":item}))]);
    let refused =
        f.refused("run", json!({"basis":basis,"steps":[{"command":"layer.settings","params":{"layer":layer,"color":"#445566","affectAll":false}}]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "a new source user changed the clone decision without moving the target: {refused}");
    f.shutdown().await;
}

#[tokio::test]
async fn source_user_removed_from_another_comp() {
    let (bytes, layer, _, other_comp, other) = other_comp_source_user(true);
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("edit.delete", json!({"comp":other_comp,"layers":[other.unwrap()]}))]);
    let refused =
        f.refused("run", json!({"basis":basis,"steps":[{"command":"layer.settings","params":{"layer":layer,"color":"#445566","affectAll":false}}]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "a deleted source user changed the clone decision without moving the target: {refused}");
    f.shutdown().await;
}

fn source_span(out: f64) -> (Vec<u8>, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(0)).unwrap();
    let item = doc.execute("file.importPlaceholder", json!({"name":"Offline","width":640,"height":360,"duration":6,"frameRate":24})).unwrap()["item"]
        .as_u64()
        .unwrap();
    let layer = doc.execute("layer.addItem", json!({"item":item})).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.timing", json!({"layers":[layer],"out":out})).unwrap();
    (doc.save(None).unwrap(), layer, item)
}

#[tokio::test]
async fn slip_source_span_holds_duration_without_unrelated_values() {
    let (bytes, layer, item) = source_span(2.0);
    for change in 0..3 {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[match change {
            0 => ("file.replaceWithPlaceholder", json!({"item":item,"duration":3})),
            1 => opacity(layer, 23.0),
            _ => ("project.setComment", json!({"items":[item],"comment":"Unrelated source comment"})),
        }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.slip","params":{"layers":[layer],"delta":-4}}]})).await;
        if change == 0 {
            assert_conflict(&answer.unwrap_err(), &item.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "slip refused unrelated layer or source values: {answer:?}");
            if change == 1 {
                assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
            }
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn remap_source_endpoint_holds_duration_without_unrelated_values() {
    let (bytes, layer, item) = source_span(10.0);
    for change in 0..3 {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[match change {
            0 => ("file.replaceWithPlaceholder", json!({"item":item,"duration":3})),
            1 => opacity(layer, 23.0),
            _ => ("project.setComment", json!({"items":[item],"comment":"Unrelated source comment"})),
        }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.enableTimeRemap","params":{"layers":[layer],"value":true}}]})).await;
        if change == 0 {
            assert_conflict(&answer.unwrap_err(), &item.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "remap endpoints refused unrelated layer or source values: {answer:?}");
            if change == 1 {
                assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
            }
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn time_commands_hold_remap_presence() {
    let (bytes, layer, _) = source_span(2.0);
    for (command, params) in [("layer.slip", json!({"layers":[layer],"delta":-1})), ("layer.enableTimeRemap", json!({"layers":[layer],"value":true}))] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("layer.enableTimeRemap", json!({"layers":[layer],"value":true}))]);
        let refused = f.refused("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
        assert_conflict(&refused, &layer.to_string(), "changed");
        f.shutdown().await;
    }
}

#[tokio::test]
async fn remap_removal_holds_values_but_enabled_noop_does_not() {
    let (bytes, layer, _) = source_span(2.0);
    let mut doc = MotionDocument::open(FILE, &bytes).unwrap();
    doc.execute("layer.enableTimeRemap", json!({"layers":[layer],"value":true})).unwrap();
    let bytes = doc.save(None).unwrap();
    for value in [false, true] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("prop.set", json!({"layer":layer,"path":"timeRemap","value":1.5}))]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.enableTimeRemap","params":{"layers":[layer],"value":value}}]})).await;
        if value {
            assert!(answer.is_ok(), "already-enabled remap does not read its values: {answer:?}");
            assert_eq!(value_on_disk(&f.file(), layer, "timeRemap"), json!(1.5));
        } else {
            assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn remap_frame_rate_only_affects_reversed_endpoints() {
    let (bytes, layer, _) = source_span(2.0);
    for reverse in [false, true] {
        let bytes = if reverse {
            let mut doc = MotionDocument::open(FILE, &bytes).unwrap();
            doc.execute("layer.timeReverse", json!({"layers":[layer]})).unwrap();
            doc.save(None).unwrap()
        } else {
            bytes.clone()
        };
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("comp.settings", json!({"frameRate":24}))]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.enableTimeRemap","params":{"layers":[layer],"value":true}}]})).await;
        if reverse {
            assert_eq!(answer.unwrap_err()["error"]["code"], "conflict");
        } else {
            assert!(answer.is_ok(), "forward remap endpoints do not use frame duration: {answer:?}");
        }
        f.shutdown().await;
    }
}
