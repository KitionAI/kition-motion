//! One id space (task card C8-motion): the object model, receipts and `others` name a project's
//! items (compositions, footage, solids, folders) and layers by the numbers `motion_inspect` lists
//! and commands take (KitObjectModel OM9 tool ids, no prefix: the project hands items, layers and
//! property uids out from one counter), stored in the `.kmotion` file, so they survive saving,
//! reopening, and another program rewriting the file. A change inside a layer (a property, a
//! keyframe, an effect, a mask) is told as a change of the layer at a path. Commands and `on` take
//! an id as `motion_inspect` lists it (`51`) and as receipts print it (`"51"`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use kition_kits::tools::CallOptions;
use serde_json::{Value, json};

/// One object as `motion_query` answers it.
async fn load(f: &Fixture, id: &str) -> Value {
    let answer = f.ok("query", json!({ "objects": [id] })).await;
    answer["objects"][0].clone()
}

/// The object model's ids of the open comp's layers, read by following the comp's item (its tool
/// id is the id `motion_inspect` lists) to its composition and that composition's layers.
async fn model_layer_ids(f: &Fixture) -> Vec<String> {
    let (_, comp, _) = f.inspect().await;
    let item = load(f, &comp.to_string()).await;
    let held = load(f, item["fields"]["comp"].as_str().unwrap()).await;
    // A layer's integer tool id is a number in `_query` (SDK C8-fix-6 item 6).
    held["fields"]["layers"].as_array().unwrap().iter().map(|id| id_text(id).unwrap()).collect()
}

/// The ids `motion_inspect` lists are the ids `motion_query` answers for the same objects: a layer
/// `51` is the object `51`, of type `Layer`; the comp's item likewise.
#[tokio::test(flavor = "multi_thread")]
async fn the_object_model_names_items_and_layers_by_the_ids_inspect_lists() {
    let f = Fixture::new(3);
    let (info, comp, _) = f.inspect().await;
    let layers: Vec<String> = layer_ids(&info).iter().map(u64::to_string).collect();
    assert_eq!(model_layer_ids(&f).await, layers);
    let loaded = load(&f, &layers[1]).await;
    assert_eq!((loaded["id"].clone(), loaded["type"].as_str()), (json!(layers[1].parse::<u64>().unwrap()), Some("Layer")), "{loaded}");
    let loaded = load(&f, &comp.to_string()).await;
    assert_eq!((loaded["id"].clone(), loaded["type"].as_str()), (json!(comp), Some("Item")), "{loaded}");
    f.shutdown().await;
}

/// The ids survive saving and reopening the file, and another program rewriting it.
#[tokio::test(flavor = "multi_thread")]
async fn ids_survive_a_save_a_reopen_and_another_program_rewriting_the_file() {
    let f = Fixture::new(3);
    let before = model_layer_ids(&f).await;
    let (info, _, _) = f.inspect().await;
    f.ok("run", json!({ "steps": [set_opacity(layer_ids(&info)[1], 40.0)] })).await;
    let bytes = std::fs::read(f.file()).unwrap();
    f.shutdown().await;
    let g = Fixture::from_bytes(&bytes, CallOptions::default());
    assert_eq!(model_layer_ids(&g).await, before, "after a reopen");
    rewrite(&g.file(), |project| project["settings"]["comment"] = json!("renamed elsewhere"));
    assert_eq!(model_layer_ids(&g).await, before, "after another program rewrote the file");
    g.shutdown().await;
}

/// A receipt names a property change under its layer, by the layer's id, at the path from the
/// layer to the property's value; the layer's own fields are not touched.
#[tokio::test(flavor = "multi_thread")]
async fn a_receipt_names_a_property_change_under_its_layer() {
    let f = Fixture::new(3);
    let (info, _, _) = f.inspect().await;
    let layer = layer_ids(&info)[1];
    let ran = f.ok("run", json!({ "steps": [set_opacity(layer, 50.0)] })).await;
    let changed = &ran["changes"]["changed"];
    assert_eq!(ids(changed), [layer.to_string()], "{ran}");
    let fields = entry(changed, &layer.to_string())["fields"].clone();
    let fields: Vec<&str> = fields.as_array().unwrap().iter().map(|field| field.as_str().unwrap()).collect();
    assert_eq!(fields.len(), 1, "{ran}");
    // A json value's change is told at its changed leaf (SDK C8-fix-6 item 5): `….value.v`, and in
    // the commands' words (`named`).
    assert!(fields[0].starts_with("props.children.") && fields[0].ends_with(".value.v"), "{ran}");
    assert_eq!(entry(changed, &layer.to_string())["edits"][0]["named"], "transform/opacity.value.v", "{ran}");
    println!("a property change: {}", ran["changes"]);
    f.shutdown().await;
}

/// Commands and `on` take a layer's and a comp's id as `motion_inspect` lists it (a number) and as
/// receipts print it (a string): `layer: "51"`, `layers: ["51"]`, `on: ["51"]`, `comp: "1"` and
/// `at.part: {"comp": "1"}` name the same layer and comp as their numbers.
#[tokio::test(flavor = "multi_thread")]
async fn ids_are_taken_as_numbers_and_as_the_strings_receipts_print() {
    let f = Fixture::new(3);
    let (info, comp, _) = f.inspect().await;
    let layer = layer_ids(&info)[1];
    let text = layer.to_string();
    let cases = [
        json!({ "command": "prop.set", "params": { "layer": text, "path": OPACITY, "value": 11 } }),
        json!({ "command": "prop.set", "params": { "layer": text, "comp": comp.to_string(), "path": OPACITY, "value": 12 } }),
        json!({ "command": "prop.set", "on": [text], "params": { "path": OPACITY, "value": 13 } }),
        json!({ "command": "layer.setSwitch", "params": { "layers": [text], "switch": "shy", "value": true } }),
        json!({ "command": "prop.set", "on": [text], "at": { "part": { "comp": comp.to_string() } }, "params": { "path": OPACITY, "value": 14 } }),
    ];
    for (n, step) in cases.into_iter().enumerate() {
        let ran = f.call("run", json!({ "steps": [step.clone()] })).await.unwrap_or_else(|error| panic!("case {n} {step}: {error}"));
        assert_eq!(ids(&ran["changes"]["changed"]), std::slice::from_ref(&text), "case {n}: {ran}");
    }
    assert_eq!(opacity_on_disk(&f.file(), layer), 14.0);
    f.shutdown().await;
}

/// A digit string that is no id of the comp's layers but a layer's name still names that layer;
/// one that is neither is refused, listing the layers.
#[tokio::test(flavor = "multi_thread")]
async fn a_digit_string_that_is_no_id_is_still_a_name() {
    let f = Fixture::new(2);
    let (info, _, _) = f.inspect().await;
    let layer = layer_ids(&info)[0];
    f.ok("run", json!({ "steps": [{ "command": "layer.rename", "params": { "layer": layer, "name": "777" } }] })).await;
    let ran = f.ok("run", json!({ "steps": [set_opacity("777", 33.0)] })).await;
    assert_eq!(ids(&ran["changes"]["changed"]), [layer.to_string()], "{ran}");
    let refused = f.refused("run", json!({ "steps": [set_opacity("778", 33.0)] })).await;
    assert!(refused.to_string().contains("no layer"), "{refused}");
    f.shutdown().await;
}

// ---- Receipt sizes ----

/// Prints the receipt (`changes`) of a step, in bytes, and answers it.
async fn receipt(f: &Fixture, what: &str, steps: Value) -> Value {
    let ran = f.ok("run", json!({ "steps": steps, "changes": "all" })).await;
    let changes = ran["changes"].clone();
    println!("receipt: {what}: {} bytes: {}", changes.to_string().len(), short(&changes));
    changes
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.len() > 600 { format!("{}… ({} bytes)", &text[..600], text.len()) } else { text }
}

/// `n` opacity keys on layer `layer`, one a frame from 0 (layer time).
fn keys(layer: u64, n: usize) -> Value {
    Value::Array(
        (0..n)
            .map(|k| json!({ "command": "prop.addKey", "params": { "layer": layer, "path": OPACITY, "time": k as f64 / 30.0, "value": (k % 100) as f64 } }))
            .collect(),
    )
}

/// Receipt sizes for the edits that move things in time: one keyframe's value; one keyframe
/// added; a layer's in-point moved; a comp marker moved; a layer shifted in time and time-stretched with 5 and 200
/// keyframes (its keys are in layer time, so neither touches them); every keyframe of a layer
/// moved (5 and 200); a layer moved from the bottom to the top of a 5-layer and a 200-layer comp.
#[tokio::test(flavor = "multi_thread")]
async fn receipt_sizes_of_edits_in_time() {
    for n in [5usize, 200] {
        let f = Fixture::new(3);
        let (info, _, _) = f.inspect().await;
        let layer = layer_ids(&info)[1];
        f.ok("run", json!({ "steps": keys(layer, n) })).await;
        let id = layer.to_string();
        if n == 5 {
            let one = receipt(
                &f,
                "one keyframe's value",
                json!([{ "command": "keyframe.set", "params": { "layer": layer, "path": OPACITY, "time": 2.0 / 30.0, "value": 77 } }]),
            )
            .await;
            assert_eq!(ids(&one["changed"]), std::slice::from_ref(&id));
            let added = receipt(
                &f,
                "one keyframe added",
                json!([{ "command": "prop.addKey", "params": { "layer": layer, "path": OPACITY, "time": 3.0, "value": 5 } }]),
            )
            .await;
            assert_eq!(ids(&added["changed"]), std::slice::from_ref(&id));
            let trimmed = receipt(&f, "in-point moved", json!([{ "command": "layer.timing", "params": { "layers": [layer], "in": 1.0 } }])).await;
            assert_eq!(ids(&trimmed["changed"]), std::slice::from_ref(&id));
            let (_, comp, _) = f.inspect().await;
            f.ok("run", json!({ "steps": [{ "command": "marker.set", "params": { "comp": comp, "new": true, "time": 2.0, "comment": "A" } }] })).await;
            let marker = receipt(&f, "a comp marker moved", json!([{ "command": "marker.set", "params": { "comp": comp, "at": 2.0, "time": 3.0 } }])).await;
            assert_eq!(ids(&marker["changed"]), [comp.to_string()], "the comp's item holds its markers: {marker}");
        }
        let shifted =
            receipt(&f, &format!("layer shifted 1 s, {n} keys"), json!([{ "command": "layer.timing", "params": { "layers": [layer], "delta": 1.0 } }])).await;
        assert_eq!(ids(&shifted["changed"]), std::slice::from_ref(&id));
        let stretched = receipt(
            &f,
            &format!("layer time-stretched 200 %, {n} keys"),
            json!([{ "command": "layer.timeStretch", "params": { "layers": [layer], "percent": 200 } }]),
        )
        .await;
        assert_eq!(ids(&stretched["changed"]), std::slice::from_ref(&id));
        let moved = receipt(
            &f,
            &format!("every keyframe moved 0.5 s, {n} keys"),
            json!([{ "command": "keyframe.selectAll", "on": [layer] }, { "command": "keyframe.move", "on": [layer], "params": { "delta": 0.5 } }]),
        )
        .await;
        assert_eq!(ids(&moved["changed"]), std::slice::from_ref(&id));
        f.shutdown().await;
    }
    for n in [5usize, 200] {
        let f = Fixture::new(n);
        let (info, comp, _) = f.inspect().await;
        let bottom = *layer_ids(&info).last().unwrap();
        let moved =
            receipt(&f, &format!("bottom layer to the top of {n}"), json!([{ "command": "layer.arrange", "params": { "layers": [bottom], "to": "front" } }]))
                .await;
        assert_eq!(ids(&moved["moved"]), [bottom.to_string()], "one layer moved; the others' index shift is not told");
        assert_eq!(entry(&moved["changed"], &comp.to_string())["fields"], json!(["comp.layers"]), "the comp's item holds the list: {moved}");
        assert_eq!(ids(&moved["changed"]), [comp.to_string()], "{moved}");
        f.shutdown().await;
    }
}

// ---- Named paths ----

/// The property a receipt's `named` path (up to its first `.`, where the value's fields begin)
/// names when given back to `prop.get` on `layer`: its uid.
async fn uid_at(f: &Fixture, layer: u64, named: &str) -> Option<u64> {
    let path = named.split('.').next().unwrap();
    let ran = f.call("run", json!({ "steps": [{ "command": "prop.get", "params": { "layer": layer, "path": path, "time": 0 } }] })).await.ok()?;
    ran["completed"][0]["result"]["uid"].as_u64()
}

/// Sets property `uid` of `layer` (named by uid, not by path) and answers the receipt's `named`.
async fn named_after_setting(f: &Fixture, layer: u64, uid: u64, value: Value) -> String {
    let ran = f.ok("run", json!({ "steps": [{ "command": "prop.set", "params": { "layer": layer, "prop": uid, "value": value } }] })).await;
    let edit = &entry(&ran["changes"]["changed"], &layer.to_string())["edits"][0];
    edit["named"].as_str().unwrap_or_else(|| panic!("no named: {ran}")).to_owned()
}

/// A receipt's `named` (kition-sdk's rule over a Node's `meta.match`, the Node's `pathName`) is
/// the path `prop.*` commands take for the same property: the second of two Gaussian Blurs
/// (`effects/#2/…`: an effect's match id holds `.`, so the SDK names it by its place) and the
/// second of two rectangles (`contents/rect#2/size`). WRONG where a shape group's display name is
/// a sibling's match id: with a group renamed `rect` first in `contents`, `named` still says
/// `contents/rect#2/size` for the second rectangle, and motion reads that path as the first
/// rectangle's size: its path reading (`kition-motion-project` `props.rs`, `child_index`) counts
/// a sibling whose display name matches, case-insensitively, as one of that match id; the SDK
/// counts match ids only (`kition-kdm` `tool_ids.rs`, `path_names`), as motion's own
/// `match_path_of` does. Wanted: one rule.
#[tokio::test(flavor = "multi_thread")]
async fn named_paths_are_the_paths_prop_commands_take_but_wrongly_not_beside_a_group_named_as_a_match_id() {
    let f = Fixture::new(1);
    f.ok("run", json!({ "steps": [{ "command": "layer.newShape", "params": { "kind": "none", "name": "S" } }] })).await;
    let (info, _, _) = f.inspect().await;
    let layer = layer_ids(&info)[0];
    // Two Gaussian Blurs: the second one's blurriness.
    for _ in 0..2 {
        f.ok("run", json!({ "steps": [{ "command": "effect.apply", "params": { "effect": "Gaussian Blur", "layers": [layer] } }] })).await;
    }
    let second_blur = uid_at(&f, layer, "effects/#2/blurriness").await.unwrap();
    let named = named_after_setting(&f, layer, second_blur, json!(7)).await;
    println!("named: second Gaussian Blur's blurriness: {named}");
    assert_eq!(uid_at(&f, layer, &named).await, Some(second_blur), "{named}");
    // Two rectangles: the second one's size.
    let mut rects = Vec::new();
    for _ in 0..2 {
        let ran = f.ok("run", json!({ "steps": [{ "command": "layer.addShapeItem", "params": { "layer": layer, "kind": "rect" } }] })).await;
        rects.push(ran["completed"][0]["result"].clone());
    }
    println!("named: rectangles added: {rects:?}");
    let second_rect = rects.iter().find(|rect| rect["path"].as_str().is_some_and(|path| path.ends_with("rect#2"))).unwrap_or_else(|| panic!("{rects:?}"));
    let size = uid_at(&f, layer, &format!("{}/size", second_rect["path"].as_str().unwrap())).await.unwrap();
    let named = named_after_setting(&f, layer, size, json!([40, 40])).await;
    println!("named: second rectangle's size: {named}");
    assert_eq!(uid_at(&f, layer, &named).await, Some(size), "{named}");
    // A group renamed `rect`, moved first in `contents`.
    let ran = f.ok("run", json!({ "steps": [{ "command": "layer.addShapeItem", "params": { "layer": layer, "kind": "group" } }] })).await;
    let group = ran["completed"][0]["result"]["uid"].as_u64().unwrap();
    f.ok(
        "run",
        json!({ "steps": [
        { "command": "prop.renameGroup", "params": { "layer": layer, "prop": group, "name": "rect" } },
        { "command": "prop.moveGroup", "params": { "layer": layer, "prop": group, "index": 1 } },
    ] }),
    )
    .await;
    let named = named_after_setting(&f, layer, size, json!([50, 50])).await;
    let read = uid_at(&f, layer, &named).await;
    println!("named: second rectangle's size beside a group named `rect`: {named}; motion reads it as property {read:?}, the size is {size}");
    assert_eq!(named.split('.').next(), Some("contents/rect#2/size"), "{named}");
    assert_ne!(read, Some(size), "motion reads the path as another property");
    f.shutdown().await;
}

/// Every integer parameter whose name or description says it is an id is a target, an object
/// parameter or a place (kition-sdk `testing::check_id_params`): since kition-sdk C8-fix-8 a whole
/// number elsewhere names no object, and a write over a colleague's change of it would apply.
#[test]
fn every_integer_id_parameter_is_marked() {
    let doc = <kition_motion_kit::MotionDocument as kition_kit::KitDocument>::open(FILE, &project(2)).unwrap();
    let commands = kition_kit::KitDocument::commands(&doc);
    if let Err(problems) = kition_kit::testing::check_id_params(&commands, ALLOWED_IDS) {
        panic!("{} unmarked:\n{}", problems.len(), problems.join("\n"));
    }
}

/// Integer parameters whose words say id but that name no object of the object model, each with why.
const ALLOWED_IDS: &[(&str, &str)] = &[
    // A step of the undo history: the session's, not in the file.
    ("edit.history.goto", "id"),
    ("edit.history.goto", "index"),
    // A Graphics Templates group or control: an entry of the comp's json, no tool id, no object to hold.
    ("templateProps.addProperty", "group"),
    ("templateProps.move", "group"),
    ("templateProps.set", "control"),
    // A script window or widget: a script's panel in the session, not in the file.
    ("scriptWindow.get", "window"),
    ("scriptWindow.click", "widget"),
    ("scriptWindow.click", "window"),
    // A downloadable machine-learning model (roto, face tracking): the app's, not in the file.
    ("roto.model.install", "id"),
    ("roto.model.download", "id"),
    ("roto.model.remove", "id"),
    ("face.model.install", "id"),
    ("face.model.download", "id"),
    ("face.model.remove", "id"),
    // A built-in tutorial: the app's, not in the file.
    ("learn.start", "id"),
    // A saved project template: the app's, not in the file.
    ("template.thumbnail", "id"),
    ("template.add", "id"),
    // A render queue entry: an item of the project's `renderQueue` with no tool id, no object to hold.
    ("renderQueue.remove", "item"),
];

/// The object model's schema gives kition-sdk's kit-env (and any reader) the kit's words
/// (kition-sdk C8-fix-9): a layer's times and a key's time are shown as times (`"display": {
/// "kind": "time" }` in ticks per second), and fields whose change remakes or renumbers what holds
/// them are no fields to sample (`"sample": false`): an item's `kind`, a property node's `uid`,
/// `node` (property or group) and `meta`.
#[tokio::test(flavor = "multi_thread")]
async fn the_object_model_says_how_its_values_are_shown_and_sampled() {
    let f = Fixture::new(2);
    let described = f.ok("query", json!({ "describe": "*" })).await;
    let types = &described["describe"]["types"];
    let text = types.to_string();
    let time = json!({ "kind": "time", "perSecond": 254_016_000_000_u64 });
    for field in ["startTime", "inPoint", "outPoint"] {
        assert_eq!(types["Layer"]["fields"][field]["display"], time, "{field}: {text}");
    }
    assert_eq!(types["Key"]["fields"]["time"]["display"], time, "{text}");
    assert_eq!(types["Item"]["fields"]["kind"]["sample"], false, "{text}");
    for field in ["uid", "node", "meta"] {
        assert_eq!(types["Node"]["fields"][field]["sample"], false, "{field}: {text}");
    }
    f.shutdown().await;
}
