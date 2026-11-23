//! What a call needs of the layer it acts on (task card C8-time-3, kition-sdk C8-fix-7
//! `Under::premises`, the kit's `place::premise_of`): for each command family whose premise the
//! kit narrows, a colleague's change inside the premise refuses the agent's write with `basis`,
//! and one outside it does not. The fixture is `outside.rs`'s: an 8-layer comp of solids `L0`
//! (bottom) to `L7` (top), and a colleague's program that edits the `.kmotion` file directly.
//!
//! Tests whose names say `wrongly` pin an outcome that is wrong today (what is wanted is in their
//! comment), so that fixing it fails them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use serde_json::{Value, json};

const LAYERS: usize = 8;
const ROTATION: &str = "transform/rotation";

/// Steps giving layer `layer` an opacity key at 1, 2, 3 and 4 s (layer time; the solids start at
/// 0), valued 10, 20, 30, 40.
fn keys(layer: u64) -> Value {
    Value::Array(
        [1.0, 2.0, 3.0, 4.0]
            .iter()
            .map(|t| json!({ "command": "prop.addKey", "params": { "layer": layer, "path": OPACITY, "time": t, "value": t * 10.0 } }))
            .collect(),
    )
}

/// The agent's write: opacity 77 at comp time 2 s (`at.time`), where its inspect showed the second
/// key, on `layer`.
fn key_at_2(layer: impl Into<Value>) -> Value {
    json!({ "command": "prop.addKey", "params": { "layer": layer.into(), "path": OPACITY, "value": 77 }, "at": { "time": 2.0 } })
}

/// The value of `layer`'s opacity key at comp time `t` on disk.
fn key_at(file: &std::path::Path, layer: u64, t: f64) -> Option<f64> {
    keys_on_disk(file, layer, OPACITY).into_iter().find(|(at, _)| (at - t).abs() < 1e-6).and_then(|(_, value)| value.as_f64())
}

/// The colleague changes the key set of the property the agent keys at 2 s (by id, `at.time`, with
/// `basis`), the key at 2 s being 20 when the agent looked. The kit names the key by its time
/// (`transform/opacity.keys[time=…]`, kition-sdk C8-fix-9 selectors), followed across saves:
/// - its value set to 25, then, in a second save, a key added at 1.5 s (the key at 2 s moves from
///   index 1 to 2): refused as `changed`;
/// - its value set to 25: refused as `changed`;
/// - a key added at 1.5 s or at 3.5 s (the key at 2 s still 20): applied, the key at 2 s now 77.
///   (Before: refused, the kit holding the whole property for want of this form.)
#[tokio::test(flavor = "multi_thread")]
async fn a_key_at_a_time_is_held_by_its_time() {
    for change in ["its value, then a key before", "its value", "a key before", "a key after"] {
        let f = Fixture::new(LAYERS);
        let (info, _, _) = f.inspect().await;
        let mine = layer_ids(&info)[2];
        f.ok("run", json!({ "steps": keys(mine) })).await;
        let (_, _, basis) = f.inspect().await;
        let value = ("keys.set", json!({ "layer": mine, "path": OPACITY, "time": 2.0, "value": 25 }));
        let before = ("prop.addKey", json!({ "layer": mine, "path": OPACITY, "time": 1.5, "value": 15 }));
        match change {
            "its value, then a key before" => {
                f.colleague(&[value]);
                f.ok("inspect", json!({})).await;
                f.colleague(&[before]);
            }
            "its value" => {
                f.colleague(&[value]);
            }
            "a key before" => {
                f.colleague(&[before]);
            }
            _ => {
                f.colleague(&[("prop.addKey", json!({ "layer": mine, "path": OPACITY, "time": 3.5, "value": 35 }))]);
            }
        }
        let answer = f.call("run", json!({ "basis": basis, "steps": [key_at_2(mine)] })).await;
        if change.starts_with("its value") {
            let refused = match answer {
                Ok(ran) => panic!("{change}: applied, the key at 2 s now {:?}: {ran}", key_at(&f.file(), mine, 2.0)),
                Err(refused) => refused,
            };
            assert_conflict(&refused, &mine.to_string(), "changed");
            assert_eq!(key_at(&f.file(), mine, 2.0), Some(25.0), "{change}: the colleague's value kept");
        } else {
            answer.unwrap_or_else(|error| panic!("{change}: {error}"));
            assert_eq!(key_at(&f.file(), mine, 2.0), Some(77.0), "{change}");
        }
        f.shutdown().await;
    }
}

/// A key at a time needs the layer's start and stretch: after the colleague time-stretched the
/// target (200 %, its keys now at twice their comp times), the agent's key at 2 s is refused.
#[tokio::test(flavor = "multi_thread")]
async fn a_key_at_a_time_after_a_time_stretch_of_its_layer_is_refused() {
    let f = Fixture::new(LAYERS);
    let (info, _, _) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.ok("run", json!({ "steps": keys(mine) })).await;
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("layer.timeStretch", json!({ "layers": [mine], "percent": 200 }))]);
    let refused = f.refused("run", json!({ "basis": basis, "steps": [key_at_2(mine)] })).await;
    assert_eq!(refused["error"]["code"], "conflict", "{refused}");
    assert!(refused["conflicts"].as_array().unwrap().iter().any(|c| c["id"] == json!(mine)), "{refused}");
    f.shutdown().await;
}

/// `prop.set` of a property with no keys needs that property and no position: after the colleague
/// changed another property of the layer (its rotation) it applies; after the colleague changed
/// the same property it is refused as `changed`.
#[tokio::test(flavor = "multi_thread")]
async fn a_property_with_no_keys_needs_that_property_alone() {
    for change in ["rotation", "opacity"] {
        let f = Fixture::new(LAYERS);
        let (info, _, basis) = f.inspect().await;
        let mine = layer_ids(&info)[2];
        f.colleague(&[match change {
            "rotation" => ("prop.set", json!({ "layer": mine, "path": ROTATION, "value": 45 })),
            _ => opacity(mine, 30.0),
        }]);
        let answer = f.call("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
        match change {
            "rotation" => {
                answer.unwrap_or_else(|error| panic!("{change}: {error}"));
                assert_eq!(opacity_on_disk(&f.file(), mine), 50.0);
            }
            _ => {
                assert_conflict(&answer.expect_err(change), &mine.to_string(), "changed");
                assert_eq!(opacity_on_disk(&f.file(), mine), 30.0);
            }
        }
        f.shutdown().await;
    }
}

/// `prop.set` with `selected: true` writes the same property of the other selected layers too:
/// no premise is narrowed (every field of the layer): after the colleague changed the layer's
/// rotation it is refused.
#[tokio::test(flavor = "multi_thread")]
async fn a_property_set_on_the_selected_layers_too_keeps_every_field() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[("prop.set", json!({ "layer": mine, "path": ROTATION, "value": 45 }))]);
    let step = json!({ "command": "prop.set", "params": { "layer": mine, "path": OPACITY, "value": 50, "selected": true } });
    let refused = f.refused("run", json!({ "basis": basis, "steps": [step] })).await;
    assert_conflict(&refused, &mine.to_string(), "changed");
    f.shutdown().await;
}

/// `layer.rename` needs the name alone: after the colleague changed the layer's opacity it
/// applies; after the colleague renamed it, refused as `changed`.
#[tokio::test(flavor = "multi_thread")]
async fn a_rename_needs_the_name_alone() {
    for change in ["opacity", "rename"] {
        let f = Fixture::new(LAYERS);
        let (info, _, basis) = f.inspect().await;
        let mine = layer_ids(&info)[2];
        f.colleague(&[match change {
            "opacity" => opacity(mine, 30.0),
            _ => ("layer.rename", json!({ "layer": mine, "name": "Theirs" })),
        }]);
        let answer = f.call("run", json!({ "basis": basis, "steps": [{ "command": "layer.rename", "params": { "layer": mine, "name": "Mine" } }] })).await;
        match change {
            "opacity" => {
                answer.unwrap_or_else(|error| panic!("{change}: {error}"));
            }
            _ => {
                assert_conflict(&answer.expect_err(change), &mine.to_string(), "changed");
            }
        }
        f.shutdown().await;
    }
}

/// A layer named by its name in `layer` (`"L5"`): the name is a premise, and so is its place in
/// the stack (the first layer from the top with that name). After the colleague renamed the
/// target and gave its name to another layer, the opacity write is refused (it would land on the
/// other layer); after the colleague changed the target's rotation, it applies.
#[tokio::test(flavor = "multi_thread")]
async fn a_layer_named_by_its_name_needs_its_name() {
    for change in ["name moved", "rotation"] {
        let f = Fixture::new(LAYERS);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        let (mine, other) = (layers[2], layers[4]);
        let theirs: Vec<(&str, Value)> = match change {
            "name moved" => vec![("layer.rename", json!({ "layer": mine, "name": "Was L5" })), ("layer.rename", json!({ "layer": other, "name": "L5" }))],
            _ => vec![("prop.set", json!({ "layer": mine, "path": ROTATION, "value": 45 }))],
        };
        f.colleague(&theirs);
        let answer = f.call("run", json!({ "basis": basis, "steps": [set_opacity("L5", 50.0)] })).await;
        match change {
            "name moved" => {
                let refused = answer.expect_err(change);
                assert_eq!(refused["error"]["code"], "conflict", "{refused}");
                assert_eq!(opacity_on_disk(&f.file(), other), 100.0, "the other layer untouched");
            }
            _ => {
                answer.unwrap_or_else(|error| panic!("{change}: {error}"));
                assert_eq!(opacity_on_disk(&f.file(), mine), 50.0);
            }
        }
        f.shutdown().await;
    }
}

/// A whole number in `layer` that is no layer's id is a stack position (`resolve_layer`,
/// `KitCommand::index_numbers`): after the colleague's solid on top, the number names the layer
/// above the one the agent saw there, and the write is refused as `relocated`; after a change of
/// another layer it applies.
#[tokio::test(flavor = "multi_thread")]
async fn a_number_that_is_no_layer_id_is_a_stack_position() {
    for change in ["insert above", "another layer"] {
        let f = Fixture::new(LAYERS);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        // A position that names no object at all (the comp's item and the solids' items have ids
        // of the same counter).
        let mut position = 0;
        for n in 1..=LAYERS as u64 {
            let found = f.call("query", json!({ "objects": [n] })).await;
            let names_one = found.as_ref().is_ok_and(|answer| answer["objects"][0]["id"].is_number() || answer["objects"][0]["id"].is_string());
            if !layers.contains(&n) && !names_one {
                position = n;
                break;
            }
        }
        assert!(position > 0, "every position names an object: {layers:?}");
        let seen = layers[usize::try_from(position).unwrap() - 1];
        f.colleague(&[match change {
            "insert above" => ("layer.newSolid", json!({ "name": "Inserted", "color": "#ffffff" })),
            _ => opacity(layers[(usize::try_from(position).unwrap() + 2) % LAYERS], 30.0),
        }]);
        let answer = f.call("run", json!({ "basis": basis, "steps": [set_opacity(position, 50.0)] })).await;
        match change {
            "insert above" => {
                let refused = answer.expect_err(change);
                assert_eq!(refused["error"]["code"], "conflict", "{refused}");
                assert!(refused["conflicts"].as_array().unwrap().iter().all(|c| c["reason"] == "relocated"), "{refused}");
                assert_eq!(opacity_on_disk(&f.file(), seen), 100.0);
            }
            _ => {
                answer.unwrap_or_else(|error| panic!("{change}: {error}"));
                assert_eq!(opacity_on_disk(&f.file(), seen), 50.0);
            }
        }
        f.shutdown().await;
    }
}

/// A whole number in `layer` that is the comp item's id (an object of another type, kition-sdk
/// C8-fix-9 `object_types`) is a stack position as the engine reads it: after the colleague changed
/// another layer it applies, after the colleague's solid on top it is refused as `relocated`.
/// (Before: the number was taken as the comp item, refused "changed" after any layer change, and
/// the stack position was not followed.)
#[tokio::test(flavor = "multi_thread")]
async fn a_number_that_is_the_comps_item_id_is_a_stack_position() {
    for change in ["insert above", "another layer"] {
        let f = Fixture::new(LAYERS);
        let (info, comp, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        let position = comp;
        assert!(position >= 1 && position <= LAYERS as u64 && !layers.contains(&position), "the comp's item id {comp} is no stack position here: {layers:?}");
        let seen = layers[usize::try_from(position).unwrap() - 1];
        f.colleague(&[match change {
            "insert above" => ("layer.newSolid", json!({ "name": "Inserted", "color": "#ffffff" })),
            _ => opacity(*layers.iter().find(|layer| **layer != seen).unwrap(), 30.0),
        }]);
        let answer = f.call("run", json!({ "basis": basis, "steps": [set_opacity(position, 50.0)] })).await;
        match change {
            "insert above" => {
                let refused = answer.expect_err(change);
                assert!(refused["conflicts"].as_array().unwrap().iter().all(|c| c["reason"] == "relocated"), "{refused}");
                assert_eq!(opacity_on_disk(&f.file(), seen), 100.0);
            }
            _ => {
                answer.unwrap_or_else(|error| panic!("{change}: {error}"));
                assert_eq!(opacity_on_disk(&f.file(), seen), 50.0);
            }
        }
        f.shutdown().await;
    }
}

/// A comp marker has no tool id: a marker edit by `at` or `index` needs the comp's markers
/// (`comp.markers` of the comp's item), with no `at.part` too (where kition-sdk's premise
/// on the part needs one). After the colleague changed the marker's comment it is refused as
/// `changed`; after the colleague renamed a project item that is no comp, it applies.
#[tokio::test(flavor = "multi_thread")]
async fn a_comp_marker_edit_needs_the_comp_markers() {
    for by in ["at", "index"] {
        for change in ["comment", "another item"] {
            let f = Fixture::new(LAYERS);
            let (_, comp, _) = f.inspect().await;
            f.colleague(&[("markers.set", json!({ "new": true, "time": 1.0, "comment": "first" }))]);
            let (_, _, basis) = f.inspect().await;
            f.colleague(&[match change {
                "comment" => ("markers.set", json!({ "at": 1.0, "comment": "theirs" })),
                _ => ("project.rename", json!({ "item": "L0", "name": "Elsewhere" })),
            }]);
            let step = match by {
                "at" => json!({ "command": "markers.set", "params": { "at": 1.0, "comment": "mine" } }),
                _ => json!({ "command": "markers.set", "params": { "index": 0, "comment": "mine" } }),
            };
            let answer = f.call("run", json!({ "basis": basis, "steps": [step] })).await;
            match change {
                "comment" => {
                    assert_conflict(&answer.expect_err(change), &comp.to_string(), "changed");
                }
                _ => {
                    answer.unwrap_or_else(|error| panic!("{by} after {change}: {error}"));
                }
            }
            f.shutdown().await;
        }
    }
}

/// A step that names its layer by id and gives `at.time` is asked at that time too (kition-sdk
/// C8-fix-9). A layer marker added by id at 2 s, a command the kit states no premise for, applies
/// after the colleague created a project folder (a create elsewhere): its time names no layer, only
/// the layer it names. (Answering nothing there, the write was refused as `relocated`, "the kit
/// cannot say what this place names", after any create, delete or move by others.)
#[tokio::test(flavor = "multi_thread")]
async fn a_write_by_id_at_a_time_applies_after_a_create_elsewhere() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[("project.newFolder", json!({ "name": "Theirs" }))]);
    let step = json!({ "command": "layer.addMarker", "params": { "layer": mine, "comment": "mine" }, "at": { "time": 2.0 } });
    let ran = f.ok("run", json!({ "basis": basis, "steps": [step] })).await;
    assert_eq!(ran["changes"]["changed"][0]["id"], json!(mine), "{ran}");
    f.shutdown().await;
}

/// A command whose targets are `layers` that is given one layer in `layer` (`layer.addMarker`)
/// holds that layer (kition-sdk `KitCommand::objects`, C8-fix-8: a whole number names an object
/// only where the kit marks it). After the colleague deleted it the write is refused as `deleted`,
/// nothing run. (Unmarked, the number named nothing for the guard, and the run reached the kit.)
#[tokio::test(flavor = "multi_thread")]
async fn a_layer_given_in_the_single_form_is_held() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[("edit.clear", json!({ "layers": [mine] }))]);
    let step = json!({ "command": "layer.addMarker", "params": { "layer": mine, "comment": "mine" }, "at": { "time": 2.0 } });
    let refused = f.refused("run", json!({ "basis": basis, "steps": [step] })).await;
    assert_conflict(&refused, &mine.to_string(), "deleted");
    f.shutdown().await;
}
