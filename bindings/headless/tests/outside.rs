//! The premise guard and receipts when the `.kmotion` file itself changes (task card C8-motion): a
//! colleague's program, an engine of this kit that shares nothing with the server under test,
//! opens the project, edits it and writes it back, so the server sees only a changed file. The
//! agent works through the kits' MCP server's tools (`kition_kits::tools::call_as`, the library
//! `kition-kits-mcp` serves) on the `kition-motion` executable, holding the ids and the `revision`
//! of an inspect made before the colleague's edit.
//!
//! Motion's position premise is a layer's index in its comp's layer list (its stack position, `1`
//! the top): a layer put above the target, or taken out above it, relocates it. A layer's times
//! (`startTime`, `inPoint`, `outPoint`) and its keyframes are fields of the layer: a colleague who
//! shifts or retimes the target changes it.
//!
//! With the SDK's C8-fix-6 and the kit-side declarations of its migration (the Layer's `startTime`,
//! `inPoint` and `outPoint` marked `"position": true`, the Node's `"pathName": "meta.match"`, the
//! commands' index spelling `"#"` and `KitDocument::under` for it), the guard names a change made
//! in the file by the layer (a load answers the field `id`, the tool id's field), refuses a write
//! whose target others changed, deleted or moved, a write by stack position whose layer moved, and
//! a marker edit on a comp others changed, and `others` says where in the commands' words
//! (`transform/opacity`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::time::Instant;

use common::*;
use kition_kits::tools::{CallOptions, Guard};
use serde_json::{Value, json};

const LAYERS: usize = 8;

/// The colleague's new solid: it goes on top of the comp, so every layer's stack position grows
/// by one.
fn new_solid_on_top() -> (&'static str, Value) {
    ("layer.newSolid", json!({ "name": "Inserted", "color": "#ffffff" }))
}

// ---- 1. An unrelated layer's property ----

/// The colleague changes another layer's opacity; the agent's write with `basis` applies, and
/// `others` names the colleague's layer alone, its edit in the commands' words
/// (`transform/opacity`). C8-fix-6: was counted as an unnamed `Node`.
#[tokio::test(flavor = "multi_thread")]
async fn s1_a_colleagues_change_of_another_layer_applies_and_others_names_that_layer() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let layers = layer_ids(&info);
    let (mine, theirs) = (layers[2], layers[5]);
    f.colleague(&[opacity(theirs, 30.0)]);
    let ran = f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_eq!(ids(&ran["changes"]["changed"]), [mine.to_string()], "{ran}");
    let others = &ran["others"];
    assert_eq!(others["by"], json!(["outside"]), "{ran}");
    assert_eq!(ids(&others["changed"]), [theirs.to_string()], "{others}");
    for list in ["created", "moved", "deleted"] {
        assert!(ids(&others[list]).is_empty(), "{others}");
    }
    assert!(others.get("unnamed").is_none(), "{others}");
    let edit = &others["changed"][0]["edits"][0];
    assert!(edit["named"].as_str().is_some_and(|named| named.starts_with("transform/opacity")), "{others}");
    println!("s1: others for one unrelated opacity change: {} bytes: {others}", others.to_string().len());
    assert_eq!((opacity_on_disk(&f.file(), mine), opacity_on_disk(&f.file(), theirs)), (50.0, 30.0));
    f.shutdown().await;
}

// ---- 2. The target's same property ----

/// The colleague changes the target layer's opacity from 100 to 30, then the agent sets it to 50
/// with the `basis` of an inspect that showed 100: refused as `changed` on the layer, nothing
/// written; the retry after an inspect applies. C8-fix-6: was applied over the colleague's 30.
#[tokio::test(flavor = "multi_thread")]
async fn s2_a_colleagues_change_of_the_target_is_refused_as_changed_and_the_retry_applies() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[opacity(mine, 30.0)]);
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_conflict(&refused, &mine.to_string(), "changed");
    assert_eq!(ids(&refused["others"]["changed"]), [mine.to_string()], "{refused}");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    println!("s2: refused changed; others {} bytes", refused["others"].to_string().len());
    let (_, _, basis) = f.inspect().await;
    f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_eq!(opacity_on_disk(&f.file(), mine), 50.0);
    f.shutdown().await;
}

/// The same between two agents of one server: another agent sets the target's opacity, then this
/// agent's write with the `basis` from before it is refused as `changed` on the layer, nothing
/// written; the retry after an inspect applies. Here the guard reads change sets from the kit's
/// replica, where items and layers have their tool ids.
#[tokio::test(flavor = "multi_thread")]
async fn s2b_another_agents_change_of_the_target_is_refused_as_changed_and_the_retry_applies() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.call_by("agent-other", "run", json!({ "steps": [set_opacity(mine, 30.0)] })).await;
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_conflict(&refused, &mine.to_string(), "changed");
    assert_eq!(ids(&refused["others"]["changed"]), [mine.to_string()], "{refused}");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    println!("s2b: refusal's others {} bytes: {}", refused["others"].to_string().len(), refused["others"]);
    let (_, _, basis) = f.inspect().await;
    f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_eq!(opacity_on_disk(&f.file(), mine), 50.0);
    f.shutdown().await;
}

// ---- 3. A layer inserted above the target ----

/// Whether file `after` is file `before` with layer `layer` of comp `comp` shifted one second (its
/// start, In and Out) and nothing else changed in the items: the merge of a colleague's edit (in
/// `before`) and the agent's `layer.timing {delta: 1}`. Answers what differs otherwise.
fn shifted_by_one_second(before: &[u8], after: &[u8], comp: u64, layer: u64) -> &'static str {
    let mut want: Value = serde_json::from_slice(before).unwrap();
    let now: Value = serde_json::from_slice(after).unwrap();
    let Some(target) = want["items"][comp.to_string()]["kind"]["layers"].as_array_mut().unwrap().iter_mut().find(|l| l["id"] == layer) else {
        return "the layer is gone";
    };
    for name in ["start_time", "in_point", "out_point"] {
        target[name] = json!(target[name].as_i64().unwrap() + 254_016_000_000);
    }
    if want["items"] == now["items"] { "the merge of both edits" } else { "another result" }
}

/// The colleague adds a solid on top of the comp: the target keeps its id and moves from stack
/// position 3 to 4. C8-fix-7: the kit states what `prop.set` of a property with no keys needs (that
/// property, no position), so setting the target's opacity by id applies, `others` saying the
/// insert. `layer.timing {delta}` reads the layer's start, In and Out, never its place in the
/// stack: it applies too, and the saved file is the merge of both edits (C8-time-policy; it was
/// refused as `relocated` while the kit held the stack position). (C8-fix-6 pinned the opacity
/// write refused as `relocated`.)
#[tokio::test(flavor = "multi_thread")]
async fn s3a_a_layer_inserted_above_the_target_refuses_only_a_write_by_id_that_depends_on_the_stack() {
    let f = Fixture::new(LAYERS);
    let (info, comp, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[new_solid_on_top()]);
    assert_eq!(layers_on_disk(&f.file())[3], mine, "the target is fourth from the top now");
    let before = std::fs::read(f.file()).unwrap();
    let timing = json!({ "command": "layer.timing", "params": { "layers": [mine], "delta": 1.0 } });
    let timed = f.ok("run", json!({ "basis": basis, "steps": [timing] })).await;
    assert_eq!(shifted_by_one_second(&before, &std::fs::read(f.file()).unwrap(), comp, mine), "the merge of both edits");
    assert_eq!(layers_on_disk(&f.file())[3], mine, "the colleague's solid kept on top");
    let ran = f.ok("run", json!({ "basis": revision(&timed), "steps": [set_opacity(mine, 50.0)] })).await;
    assert_eq!(opacity_on_disk(&f.file(), mine), 50.0);
    println!("s3a: timing applied after the insert above (merged); opacity applied, others {} bytes", ran["others"].to_string().len());
    f.shutdown().await;
}

/// The same insert, and a write naming the target by its stack position, `layer: "#3"`, where the
/// agent's inspect showed it: refused as `relocated`, naming the layer `#3` named at `basis` (the
/// target, now at index 3) and the one it names now; the saved file proves nothing was written.
/// C8-fix-6: was applied to the layer now third.
#[tokio::test(flavor = "multi_thread")]
async fn s3b_a_write_by_stack_position_after_an_insert_above_is_refused_as_relocated() {
    let f = Fixture::new(LAYERS);
    let (info, comp, basis) = f.inspect().await;
    let layers = layer_ids(&info);
    let (mine, above) = (layers[2], layers[1]);
    f.colleague(&[new_solid_on_top()]);
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({ "basis": basis, "steps": [set_opacity("#3", 50.0)] })).await;
    let conflict = assert_conflict(&refused, &mine.to_string(), "relocated");
    assert_eq!((conflict["now"]["parent"].clone(), conflict["now"]["index"].clone()), (json!(comp), json!(3)), "{refused}");
    assert_eq!(conflict["place"]["under"], json!([above]), "what #3 names now: {refused}");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    assert_eq!((opacity_on_disk(&f.file(), mine), opacity_on_disk(&f.file(), above)), (100.0, 100.0), "the saved file");
    println!("s3b: refused relocated: {}", refused["conflicts"]);
    f.shutdown().await;
}

/// The same through `on`: `on` takes ids, and the SDK refuses `"#3"` there as `invalid-params`
/// (the kit declares its index spelling), before anything runs. C8-fix-6: the host put `"#3"` into
/// `prop.set`'s `layer` and the write reached the layer now third.
#[tokio::test(flavor = "multi_thread")]
async fn s3c_on_with_a_stack_position_is_refused_as_invalid_params() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let above = layer_ids(&info)[1];
    f.colleague(&[new_solid_on_top()]);
    let before = std::fs::read(f.file()).unwrap();
    let refused =
        f.refused("run", json!({ "basis": basis, "steps": [{ "command": "prop.set", "on": ["#3"], "params": { "path": OPACITY, "value": 50 } }] })).await;
    assert_eq!(refused["error"]["code"], "invalid-params", "{refused}");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    assert_eq!(opacity_on_disk(&f.file(), above), 100.0);
    f.shutdown().await;
}

/// The same through another agent of the same server adding a solid on top: this agent's
/// `layer.timing` of the target by id applies, the saved file the merge of both edits
/// (C8-time-policy: the stack position is not read; it was refused as `relocated`), and its
/// opacity by id applies (C8-fix-7: the kit states no position for it; C8-fix-6 pinned it refused).
#[tokio::test(flavor = "multi_thread")]
async fn s3d_another_agents_layer_above_the_target_refuses_only_a_write_by_id_that_depends_on_the_stack() {
    let f = Fixture::new(LAYERS);
    let (info, comp, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    let (command, params) = new_solid_on_top();
    f.call_by("agent-other", "run", json!({ "steps": [{ "command": command, "params": params, "at": { "part": { "comp": comp } } }] })).await;
    let before = std::fs::read(f.file()).unwrap();
    let timing = json!({ "command": "layer.timing", "params": { "layers": [mine], "delta": 1.0 } });
    let timed = f.ok("run", json!({ "basis": basis, "steps": [timing] })).await;
    assert_eq!(shifted_by_one_second(&before, &std::fs::read(f.file()).unwrap(), comp, mine), "the merge of both edits");
    f.ok("run", json!({ "basis": revision(&timed), "steps": [set_opacity(mine, 50.0)] })).await;
    assert_eq!(opacity_on_disk(&f.file(), mine), 50.0);
    f.shutdown().await;
}

/// Another agent of the same server deletes the target: the write is refused as `deleted`.
#[tokio::test(flavor = "multi_thread")]
async fn s4b_another_agent_deleting_the_target_is_refused_as_deleted() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.call_by("agent-other", "run", json!({ "steps": [{ "command": "edit.clear", "params": { "layers": [mine] } }] })).await;
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_conflict(&refused, &mine.to_string(), "deleted");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    f.shutdown().await;
}

// ---- 4. The target deleted ----

/// The colleague deletes the target: the agent's write is refused as a `conflict` naming the layer
/// `deleted`, nothing written. C8-fix-6: was the command failing on a layer the comp lacks.
#[tokio::test(flavor = "multi_thread")]
async fn s4_a_colleague_deleting_the_target_is_refused_as_deleted() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[("edit.clear", json!({ "layers": [mine] }))]);
    assert!(!layers_on_disk(&f.file()).contains(&mine));
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_conflict(&refused, &mine.to_string(), "deleted");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    f.shutdown().await;
}

// ---- 5. A write at a time after a retime ----

/// The colleague shifts the target layer one second later. The schema marks a layer's
/// `startTime`, `inPoint` and `outPoint` as its position (`"position": true`, C8-fix-6 kit-side):
/// the shift is a move, and `prop.addKey` (not position-independent: where its key lands in layer
/// time depends on the start) by id at `at.time` 2 s is refused as `relocated`, `now` giving the
/// layer's new start; nothing written. C8-fix-6: was applied unchecked.
#[tokio::test(flavor = "multi_thread")]
async fn s5a_a_key_at_a_time_on_a_layer_shifted_in_time_is_refused_as_relocated() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[("layer.timing", json!({ "layers": [mine], "delta": 1.0 }))]);
    let before = std::fs::read(f.file()).unwrap();
    let key = json!({ "command": "prop.addKey", "params": { "layer": mine, "path": OPACITY, "value": 20 }, "at": { "time": 2.0 } });
    let refused = f.refused("run", json!({ "basis": basis, "steps": [key] })).await;
    let conflict = assert_conflict(&refused, &mine.to_string(), "relocated");
    assert!(conflict["now"].get("startTime").is_some(), "{refused}");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    assert!(keys_on_disk(&f.file(), mine, OPACITY).is_empty());
    println!("s5a: refused relocated, now {}", conflict["now"]);
    f.shutdown().await;
}

/// The comp's markers, as `(comp seconds, comment)`, in the file.
fn comp_markers_on_disk(f: &Fixture, comp: u64) -> Vec<(f64, String)> {
    let project: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
    let markers = project["items"][comp.to_string()]["kind"]["markers"].as_array().cloned().unwrap_or_default();
    markers.iter().map(|m| (m["time"].as_i64().unwrap() as f64 / 254_016_000_000.0, m["comment"].as_str().unwrap_or_default().to_owned())).collect()
}

/// A command that addresses by time alone: `markers.set {at}` edits the comp marker at a comp time,
/// with `at.part {comp}` and no other target, so it acts on the comp (C8-fix-6). The agent saw
/// marker `A` at 2 s and `B` at 3 s; the colleague moves both a second earlier; the agent's rename
/// of the marker at 2 s (meaning `A`) is refused as `changed` on the comp's item, nothing
/// written. C8-fix-6: was applied to `B`.
#[tokio::test(flavor = "multi_thread")]
async fn s5b_a_marker_edit_by_time_after_a_retime_is_refused() {
    let f = Fixture::new(LAYERS);
    let (_, comp, _) = f.inspect().await;
    let setup = json!([
        { "command": "markers.set", "params": { "new": true, "time": 2.0, "comment": "A" }, "at": { "part": { "comp": comp } } },
        { "command": "markers.set", "params": { "new": true, "time": 3.0, "comment": "B" }, "at": { "part": { "comp": comp } } },
    ]);
    f.ok("run", json!({ "steps": setup })).await;
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("markers.set", json!({ "comp": comp, "at": 2.0, "time": 1.0 })), ("markers.set", json!({ "comp": comp, "at": 3.0, "time": 2.0 }))]);
    assert_eq!(comp_markers_on_disk(&f, comp), [(1.0, "A".to_owned()), (2.0, "B".to_owned())]);
    let step = json!({ "command": "markers.set", "params": { "at": 2.0, "comment": "agent" }, "at": { "part": { "comp": comp } } });
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({ "basis": basis, "steps": [step] })).await;
    assert_conflict(&refused, &comp.to_string(), "changed");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    println!("s5b: refused; others {} bytes: {}", refused["others"].to_string().len(), refused["others"]["changed"]);
    f.shutdown().await;
}

/// The same marker edit between two agents of one server: refused as `changed` on the comp's item,
/// nothing written. C8-fix-6: was applied to `B` (the part was checked for deletion alone).
#[tokio::test(flavor = "multi_thread")]
async fn s5c_a_marker_edit_by_time_after_another_agents_retime_is_refused() {
    let f = Fixture::new(LAYERS);
    let (_, comp, _) = f.inspect().await;
    let setup = json!([
        { "command": "markers.set", "params": { "new": true, "time": 2.0, "comment": "A" }, "at": { "part": { "comp": comp } } },
        { "command": "markers.set", "params": { "new": true, "time": 3.0, "comment": "B" }, "at": { "part": { "comp": comp } } },
    ]);
    f.ok("run", json!({ "steps": setup })).await;
    let (_, _, basis) = f.inspect().await;
    let moves = json!([
        { "command": "markers.set", "params": { "comp": comp, "at": 2.0, "time": 1.0 } },
        { "command": "markers.set", "params": { "comp": comp, "at": 3.0, "time": 2.0 } },
    ]);
    f.call_by("agent-other", "run", json!({ "steps": moves })).await;
    let step = json!({ "command": "markers.set", "params": { "at": 2.0, "comment": "agent" }, "at": { "part": { "comp": comp } } });
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({ "basis": basis, "steps": [step] })).await;
    assert_conflict(&refused, &comp.to_string(), "changed");
    let changed = entry(&refused["others"]["changed"], &comp.to_string());
    println!("s5c: refused; others {} bytes, fields {}", refused["others"].to_string().len(), changed["fields"]);
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    assert_eq!(comp_markers_on_disk(&f, comp), [(1.0, "A".to_owned()), (2.0, "B".to_owned())]);
    f.shutdown().await;
}

// ---- 6. Guard modes ----

/// `--guard epoch` refuses scenario 1 (any change by others: it needs no names); `--guard off`
/// applies scenario 2, overwriting the colleague's opacity, and `others` names the layer (C8-fix-6:
/// was counted unnamed).
#[tokio::test(flavor = "multi_thread")]
async fn s6_epoch_refuses_an_unrelated_change_and_off_overwrites_the_targets() {
    let f = Fixture::with(LAYERS, CallOptions { guard: Guard::Epoch, ..CallOptions::default() });
    let (info, _, basis) = f.inspect().await;
    let layers = layer_ids(&info);
    f.colleague(&[opacity(layers[5], 30.0)]);
    let refused = f.refused("run", json!({ "basis": basis, "steps": [set_opacity(layers[2], 50.0)] })).await;
    assert_eq!(refused["error"]["code"], "conflict", "{refused}");
    assert_eq!(opacity_on_disk(&f.file(), layers[2]), 100.0, "nothing written");
    f.shutdown().await;

    let f = Fixture::with(LAYERS, CallOptions { guard: Guard::Off, ..CallOptions::default() });
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[opacity(mine, 30.0)]);
    let ran = f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
    assert_eq!(ids(&ran["others"]["changed"]), [mine.to_string()], "{ran}");
    assert_eq!(opacity_on_disk(&f.file(), mine), 50.0, "the colleague's 30 overwritten");
    f.shutdown().await;
}

// ---- 7. Saved again unchanged ----

/// The file saved again with no change, by the colleague's engine and by a bare rewrite (pretty
/// JSON): no notice, no refusal, nothing in `others`, with or without `basis`.
#[tokio::test(flavor = "multi_thread")]
async fn s7_a_file_saved_again_unchanged_is_no_change() {
    for how in ["engine", "rewrite"] {
        let f = Fixture::new(LAYERS);
        let (info, _, basis) = f.inspect().await;
        let mine = layer_ids(&info)[2];
        let bytes = std::fs::read(f.file()).unwrap();
        match how {
            "engine" => {
                f.colleague(&[]);
            }
            _ => rewrite(&f.file(), |_| {}),
        }
        println!("s7 {how}: same bytes {}", std::fs::read(f.file()).unwrap() == bytes);
        let ran = f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
        assert!(ran.get("others").is_none_or(Value::is_null), "{how}: {ran}");
        if how == "engine" {
            f.colleague(&[]);
        } else {
            rewrite(&f.file(), |_| {});
        }
        let seen = f.ok("inspect", json!({})).await;
        assert!(seen.get("changedOutside").is_none_or(|told| told.is_null() || told == false), "{how}: {seen}");
        f.ok("run", json!({ "steps": [set_opacity(mine, 60.0)] })).await;
        f.shutdown().await;
    }
}

// ---- 8. False refusals ----

/// Fifty rounds on a 24-layer comp: the colleague changes the opacity of a layer picked at random
/// (a fixed seed) among the 23 other than the agent's target, then the agent writes its target
/// with the revision of its last answer. Under the premise guard no write is refused; under
/// `--guard epoch` every one is. Prints both counts. (C8-fix-6: the premise guard now names each
/// change; before, it refused none because it saw no change named at all.)
#[tokio::test(flavor = "multi_thread")]
async fn s8_fifty_unrelated_file_changes_refuse_no_write_under_premise_and_every_write_under_epoch() {
    const ROUNDS: u64 = 50;
    let mut counts = Vec::new();
    for (guard, name) in [(Guard::Premise, "premise"), (Guard::Epoch, "epoch")] {
        let f = Fixture::with(24, CallOptions { guard, ..CallOptions::default() });
        let (info, _, mut basis) = f.inspect().await;
        let mut layers = layer_ids(&info);
        let mine = layers.remove(7);
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut refused = 0;
        for round in 0..ROUNDS {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let theirs = layers[usize::try_from(seed % 23).unwrap()];
            f.colleague(&[opacity(theirs, f64::from(u32::try_from(round).unwrap()))]);
            match f.call("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0 + f64::from(u32::try_from(round % 2).unwrap()))] })).await {
                Ok(ran) => {
                    assert_eq!(ids(&ran["others"]["changed"]), [theirs.to_string()], "{theirs}: {ran}");
                    assert!(ran["others"].get("unnamed").is_none(), "{ran}");
                    basis = revision(&ran);
                }
                Err(error) => {
                    assert_eq!(error["error"]["code"], "conflict", "{error}");
                    refused += 1;
                    basis = revision(&error);
                }
            }
        }
        println!("s8: --guard {name}: {refused} of {ROUNDS} writes refused after an unrelated file change");
        counts.push(refused);
        f.shutdown().await;
    }
    assert_eq!(counts, [0, ROUNDS], "premise refuses none, epoch every one");
}

// ---- 9. Position-independent commands ----

/// A layer inserted above the target, and the target shifted a second later: a write by id with a
/// command declared position-independent (`layer.rename`) applies after both, `others` saying
/// what moved; one that is not (`prop.set`) is refused as `relocated` after both (C8-motion-2).
#[tokio::test(flavor = "multi_thread")]
async fn s9_a_move_refuses_a_write_by_id_only_when_the_call_depends_on_position() {
    for (move_, theirs) in [("insert above", new_solid_on_top()), ("shift in time", ("layer.timing", json!({ "layers": ["#3"], "delta": 1.0 })))] {
        let f = Fixture::new(LAYERS);
        let (info, _, basis) = f.inspect().await;
        let mine = layer_ids(&info)[2];
        f.colleague(std::slice::from_ref(&theirs));
        let ran = f.ok("run", json!({ "basis": basis, "steps": [{ "command": "layer.rename", "params": { "layer": mine, "name": "Mine" } }] })).await;
        assert_eq!(ids(&ran["changes"]["changed"]), [mine.to_string()], "{move_}: {ran}");
        assert!(ran["others"].is_object(), "{move_}: others says the move: {ran}");
        println!("s9 {move_}: rename applied; others {}", ran["others"]);
        // C8-fix-7: the kit states that an opacity with no keys needs no position: applied
        // (C8-motion-2 pinned it refused as `relocated`). A relative timing change reads the
        // layer's start, In and Out, not its stack place (C8-time-policy): refused after the
        // shift, applied after the insert above with the result the merge of both edits.
        let before = std::fs::read(f.file()).unwrap();
        let timing = json!({ "basis": basis, "steps": [{ "command": "layer.timing", "params": { "layers": [mine], "delta": 0.5 } }] });
        if move_ == "shift in time" {
            let refused = f.refused("run", timing).await;
            assert_conflict(&refused, &mine.to_string(), "relocated");
        } else {
            f.ok("run", timing).await;
            let mut want: Value = serde_json::from_slice(&before).unwrap();
            let now: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
            for item in want["items"].as_object_mut().unwrap().values_mut() {
                let layers = item.get_mut("kind").and_then(|kind| kind.get_mut("layers")).and_then(Value::as_array_mut);
                for layer in layers.into_iter().flatten().filter(|l| l["id"] == mine) {
                    for name in ["start_time", "in_point", "out_point"] {
                        layer[name] = json!(layer[name].as_i64().unwrap() + 127_008_000_000);
                    }
                }
            }
            assert_eq!(want["items"], now["items"], "{move_}: the merge of both edits");
        }
        f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, 50.0)] })).await;
        assert_eq!(opacity_on_disk(&f.file(), mine), 50.0, "{move_}");
        f.shutdown().await;
    }
}

// ---- 10. A keyframe at a time after each kind of change ----

/// Comp time of the target's keys before any change (the layer starts at 0, so layer time).
const KEY_TIMES: [f64; 4] = [1.0, 2.0, 3.0, 4.0];
/// The time the agent keys at (`at.time`): where its inspect showed the second key.
const T: f64 = 2.0;

/// Steps giving layer `layer` an opacity key at each of [`KEY_TIMES`], valued 10, 20, 30, 40.
fn target_keys(layer: impl Into<Value> + Clone) -> Value {
    Value::Array(
        KEY_TIMES
            .iter()
            .map(|t| json!({ "command": "prop.addKey", "params": { "layer": layer.clone().into(), "path": OPACITY, "time": t, "value": t * 10.0 } }))
            .collect(),
    )
}

/// The agent's keyframe write: opacity 77 at comp time [`T`] (`at.time`) on `layer`, by id or `"#n"`.
fn key_at_t(layer: impl Into<Value>) -> Value {
    json!({ "command": "prop.addKey", "params": { "layer": layer.into(), "path": OPACITY, "value": 77 }, "at": { "time": T } })
}

/// The target's opacity key at comp time `t` in the file, if any: its value.
fn key_on_disk_at(file: &std::path::Path, layer: u64, t: f64) -> Option<f64> {
    keys_on_disk(file, layer, OPACITY).into_iter().find(|(at, _)| (at - t).abs() < 1e-6).and_then(|(_, value)| value.as_f64())
}

/// The colleague changes, after the agent's inspect, (a) the target's time (shifted 1 s), (b) the
/// key at `T` (moved to 2.5 s), (c) another key of the same property (the one at 3 s, to 99), (d)
/// another property of the target (rotation 45), (e) another layer (its opacity), or nothing; the
/// agent then keys the target's opacity at `at.time` `T` with `basis`, naming the layer by id and,
/// on a fresh project, by its stack position `"#3"`. A refusal is necessary when what the write
/// acts on changed: the key at `T` (its presence or value: (a) moves the key at 1 s to 2 s, (b)
/// moves the key at 2 s away). Outcomes asserted, and printed as a table:
///
/// | change | by id | by `#3` | necessary |
/// |---|---|---|---|
/// | (a) shift | relocated | relocated | yes |
/// | (b) the key at T moved | changed | changed | yes |
/// | (c) another key | applied | applied | no |
/// | (d) another property | applied | applied | no |
/// | (e) another layer | applied | applied | no |
/// | nothing | applied | applied | no |
///
/// C8-fix-7, C8-time-3, kition-sdk C8-fix-9: the kit states what a key at a time needs: the key at
/// that time, named by its time (`transform/opacity.keys[time=…]`, followed across others' saves),
/// the layer's `startTime` and stretch; the layer `"#3"` names carries the same premise as by id.
/// So (c) applies: the key at `T` did not change. (Before: refused, the whole property held.)
#[tokio::test(flavor = "multi_thread")]
async fn s10_a_key_at_a_time_is_refused_after_a_change_of_its_property_or_the_layers_start() {
    let changes: [(&str, bool); 6] = [
        ("(a) shift", true),
        ("(b) the key at T moved", true),
        ("(c) another key", false),
        ("(d) another property", false),
        ("(e) another layer", false),
        ("nothing", false),
    ];
    println!("s10: change | by | answer | necessary | key at T on disk");
    for (change, necessary) in changes {
        for by in ["id", "#3"] {
            let f = Fixture::new(LAYERS);
            let (info, _, _) = f.inspect().await;
            let layers = layer_ids(&info);
            let (mine, other) = (layers[2], layers[5]);
            f.ok("run", json!({ "steps": target_keys(mine) })).await;
            let (_, _, basis) = f.inspect().await;
            let theirs: Vec<(&str, Value)> = match change {
                "(a) shift" => vec![("layer.timing", json!({ "layers": [mine], "delta": 1.0 }))],
                "(b) the key at T moved" => vec![("keys.set", json!({ "layer": mine, "path": OPACITY, "time": T, "newTime": 2.5 }))],
                "(c) another key" => vec![("keys.set", json!({ "layer": mine, "path": OPACITY, "time": 3.0, "value": 99 }))],
                "(d) another property" => vec![("prop.set", json!({ "layer": mine, "path": "transform/rotation", "value": 45 }))],
                "(e) another layer" => vec![opacity(other, 30.0)],
                _ => vec![],
            };
            f.colleague(&theirs);
            let seen = key_on_disk_at(&f.file(), mine, T);
            assert_eq!(seen != Some(20.0), necessary, "{change}: the key at T on disk is {seen:?}");
            let target = if by == "id" { json!(mine) } else { json!("#3") };
            let answer = f.call("run", json!({ "basis": basis, "steps": [key_at_t(target)] })).await;
            let told = match &answer {
                Ok(_) => "applied".to_owned(),
                Err(refused) => {
                    assert_eq!(refused["error"]["code"], "conflict", "{change} by {by}: {refused}");
                    refused["conflicts"].as_array().unwrap().iter().map(|c| c["reason"].as_str().unwrap().to_owned()).collect::<Vec<_>>().join("+")
                }
            };
            let wanted = match change {
                "(a) shift" => "relocated",
                "(b) the key at T moved" => "changed",
                _ => "applied",
            };
            println!("s10: {change} | {by} | {told} | {necessary} | {:?}", key_on_disk_at(&f.file(), mine, T));
            assert_eq!(told, wanted, "{change} by {by}: {answer:?}");
            if told == "applied" {
                assert_eq!(key_on_disk_at(&f.file(), mine, T), Some(77.0), "{change} by {by}");
            }
            f.shutdown().await;
        }
    }
}

/// The colleague sets the target's opacity to 30; the agent, by stack position `"#3"` with
/// `basis`, sets it to 50. WRONG: applied over the colleague's 30. A place's premise is that it
/// names the same layer and that the layer was not moved (kition-sdk `tools/guard.rs`, the place
/// steps of `conflicts`: created, moved, position fields; `Premises::params` leaves a value in the
/// index spelling out of the targets), not the layer's content. Wanted: refused as `changed`, as
/// by id (s2).
#[tokio::test(flavor = "multi_thread")]
async fn s10b_a_write_by_stack_position_after_a_colleagues_change_of_that_layer_is_refused_as_changed() {
    let f = Fixture::new(LAYERS);
    let (info, _, basis) = f.inspect().await;
    let mine = layer_ids(&info)[2];
    f.colleague(&[opacity(mine, 30.0)]);
    let refused = f.refused("run", json!({ "basis": basis, "steps": [set_opacity("#3", 50.0)] })).await;
    assert_conflict(&refused, &mine.to_string(), "changed");
    assert_eq!(opacity_on_disk(&f.file(), mine), 30.0, "the colleague's 30 kept");
    f.shutdown().await;
}

// ---- 11. False refusals by place ----

/// Fifty rounds as s8, each write by a place: (#) `prop.set` on `"#8"`, the target's stack
/// position; (T) a keyframe on the target by id at `at.time` 2 s; (M) a comp marker added at
/// `at.time` 2 s (`comp.addMarker`, no layer: a write by time alone, which the kit cannot answer
/// `under` for, markers having no ids). The colleague changes the opacity of another layer each
/// round. Premise refuses none of either; epoch every one.
#[tokio::test(flavor = "multi_thread")]
async fn s11_fifty_unrelated_file_changes_refuse_no_write_by_place_under_premise_and_every_one_under_epoch() {
    const ROUNDS: u64 = 50;
    for (write, name) in [('#', "by stack position"), ('T', "a key at a time"), ('M', "a marker at a time")] {
        let mut counts = Vec::new();
        for (guard, mode) in [(Guard::Premise, "premise"), (Guard::Epoch, "epoch")] {
            let f = Fixture::with(24, CallOptions { guard, ..CallOptions::default() });
            let (info, comp, mut basis) = f.inspect().await;
            let mut layers = layer_ids(&info);
            let mine = layers.remove(7);
            let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
            let mut refused = 0;
            for round in 0..ROUNDS {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let theirs = layers[usize::try_from(seed % 23).unwrap()];
                f.colleague(&[opacity(theirs, f64::from(u32::try_from(round).unwrap()))]);
                let value = 50.0 + f64::from(u32::try_from(round % 2).unwrap());
                let step = match write {
                    '#' => set_opacity("#8", value),
                    'T' => json!({ "command": "prop.addKey", "params": { "layer": mine, "path": OPACITY, "value": value }, "at": { "time": 2.0 } }),
                    _ => json!({ "command": "comp.addMarker", "params": { "comment": format!("m{round}") }, "at": { "part": { "comp": comp }, "time": 2.0 } }),
                };
                match f.call("run", json!({ "basis": basis, "steps": [step] })).await {
                    Ok(ran) => basis = revision(&ran),
                    Err(error) => {
                        assert_eq!(error["error"]["code"], "conflict", "{error}");
                        refused += 1;
                        basis = revision(&error);
                    }
                }
            }
            println!("s11: {name}: --guard {mode}: {refused} of {ROUNDS} writes refused after an unrelated file change");
            counts.push(refused);
            f.shutdown().await;
        }
        assert_eq!(counts, [0, ROUNDS], "{name}: premise refuses none, epoch every one");
    }
}

// ---- 12. Mixed editing ----

/// What the colleague does in a round of s12.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Theirs {
    /// Another layer's opacity.
    Property,
    /// A solid inserted on top of the comp (above the target).
    InsertAbove,
    /// A solid inserted at the bottom of the comp (below the target).
    InsertBelow,
    /// Another layer deleted (above or below the target).
    Delete,
    /// Another layer shifted 1 s later.
    ShiftOther,
    /// The target shifted 1 s later.
    ShiftTarget,
    /// One of the target's opacity keys moved 0.5 s later.
    MoveKey,
}

/// Fifty rounds of mixed editing, each on a fresh 12-layer comp whose sixth layer from the top is
/// the target, with opacity keys at 1, 2, 3, 4 s: the agent inspects; the colleague makes one
/// change ([`Theirs`], picked by a fixed, printed seed); the agent, each with the revision of its
/// inspect, (A) renames the target by id (`layer.rename`, declared position-independent), (B)
/// sets the rotation of `"#6"` (the target's stack position), (C) keys the target's opacity by id
/// at `at.time` 2 s (where its second key was). A refusal is necessary when what the write acts on
/// changed: (A) the target deleted or renamed (never in this mix); (B) `"#6"` naming another layer
/// than the target, or the target's rotation changed; (C) the target's opacity key at comp time
/// 2 s gone or changed (a shift of the target or a move of that key). Prints the table (their
/// change, write, rounds, applied, refused necessary, refused needlessly, applied wrongly, the
/// reasons); asserts no write is applied wrongly.
#[tokio::test(flavor = "multi_thread")]
async fn s12_fifty_rounds_of_mixed_editing() {
    use std::collections::BTreeMap;
    const ROUNDS: usize = 50;
    const N: usize = 12;
    const MINE: usize = 5;
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;
    let kinds = [Theirs::Property, Theirs::InsertAbove, Theirs::InsertBelow, Theirs::Delete, Theirs::ShiftOther, Theirs::ShiftTarget, Theirs::MoveKey];
    println!("s12: seed {SEED:#x}");
    let mut table: BTreeMap<(Theirs, char), (usize, usize, usize, usize, usize, BTreeMap<String, usize>)> = BTreeMap::new();
    let mut seed = SEED;
    let mut next = |below: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        usize::try_from(seed % u64::try_from(below).unwrap()).unwrap()
    };
    for _ in 0..ROUNDS {
        let kind = kinds[next(kinds.len())];
        let f = Fixture::new(N);
        let (info, _, _) = f.inspect().await;
        let layers = layer_ids(&info);
        let mine = layers[MINE];
        f.ok("run", json!({ "steps": target_keys(mine) })).await;
        let (_, _, basis) = f.inspect().await;
        let other = {
            let k = next(N - 1);
            layers[if k >= MINE { k + 1 } else { k }]
        };
        let theirs: Vec<(&str, Value)> = match kind {
            Theirs::Property => vec![opacity(other, 30.0)],
            Theirs::InsertAbove => vec![new_solid_on_top()],
            Theirs::InsertBelow => {
                vec![("layer.newSolid", json!({ "name": "Below", "color": "#000000" })), ("layer.arrange", json!({ "layers": ["Below"], "to": "back" }))]
            }
            Theirs::Delete => vec![("edit.clear", json!({ "layers": [other] }))],
            Theirs::ShiftOther => vec![("layer.timing", json!({ "layers": [other], "delta": 1.0 }))],
            Theirs::ShiftTarget => vec![("layer.timing", json!({ "layers": [mine], "delta": 1.0 }))],
            Theirs::MoveKey => {
                let t = KEY_TIMES[next(KEY_TIMES.len())];
                vec![("keys.set", json!({ "layer": mine, "path": OPACITY, "time": t, "newTime": t + 0.5 }))]
            }
        };
        let rotation = |file: &std::path::Path| value_on_disk(file, mine, "transform/rotation");
        let (seen_key, seen_rotation) = (key_on_disk_at(&f.file(), mine, T), rotation(&f.file()));
        f.colleague(&theirs);
        let now_sixth = layers_on_disk(&f.file()).get(MINE).copied();
        let (now_key, now_rotation) = (key_on_disk_at(&f.file(), mine, T), rotation(&f.file()));
        let a = f.call("run", json!({ "basis": basis, "steps": [{ "command": "layer.rename", "params": { "layer": mine, "name": "A" } }] })).await;
        let b = f
            .call(
                "run",
                json!({ "basis": basis, "steps": [{ "command": "prop.set", "params": { "layer": "#6", "path": "transform/rotation", "value": 15 } }] }),
            )
            .await;
        let c = f.call("run", json!({ "basis": basis, "steps": [key_at_t(mine)] })).await;
        for (write, answer, necessary) in [('A', a, false), ('B', b, now_sixth != Some(mine) || seen_rotation != now_rotation), ('C', c, seen_key != now_key)] {
            let cell = table.entry((kind, write)).or_default();
            cell.0 += 1;
            match answer {
                Ok(_) if necessary => {
                    cell.4 += 1;
                    println!("s12: {write} applied wrongly after {theirs:?}");
                }
                Ok(_) => cell.1 += 1,
                Err(error) => {
                    assert_eq!(error["error"]["code"], "conflict", "{theirs:?}: {error}");
                    let reasons: Vec<String> = error["conflicts"].as_array().unwrap().iter().map(|c| c["reason"].as_str().unwrap().to_owned()).collect();
                    *cell.5.entry(reasons.join("+")).or_default() += 1;
                    if necessary {
                        cell.2 += 1;
                    } else {
                        cell.3 += 1;
                        println!("s12: needless refusal of {write} after {theirs:?}: {}", error["conflicts"]);
                    }
                }
            }
        }
        f.shutdown().await;
    }
    println!("s12: their change | write | rounds | applied | refused, necessary | refused, needless | applied, wrongly | reasons");
    let mut wrong = 0;
    for ((kind, write), (rounds, applied, needed, needless, wrongly, reasons)) in &table {
        println!("s12: {kind:?} | {write} | {rounds} | {applied} | {needed} | {needless} | {wrongly} | {reasons:?}");
        wrong += wrongly;
    }
    assert_eq!(wrong, 0, "no write applied whose premise changed");
}

// ---- Timings ----

/// The median time of an agent's write (dev profile, ten runs each): plain (no `basis`), with
/// `basis` and nothing changed, after an unrelated change made in the file, and refused after a
/// change of its target made in the file (s2), on the 8-layer project and on the repository's
/// largest project (`conformance/check-truncated.kmotion`, 103 layers, 1.6 MB). Beside them, where
/// the time after a change made in the file goes: an inspect after such a change (the server reads
/// the file again and compares it, no guard), and a write with `basis` right after that inspect
/// (the guard with the change already read), a plain inspect, and one layer loaded through
/// `motion_query` (what each of the guard's reads of the document costs). Printed, not asserted.
#[tokio::test(flavor = "multi_thread")]
async fn timings_of_a_write() {
    let large = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/check-truncated.kmotion")).unwrap();
    for (name, bytes) in [("8 layers", project(LAYERS)), ("check-truncated.kmotion", large)] {
        let f = Fixture::from_bytes(&bytes, CallOptions::default());
        let (info, _, _) = f.inspect().await;
        let layers = layer_ids(&info);
        let (mine, theirs) = (layers[2], layers[5]);
        let [mut plain, mut based, mut unrelated, mut refused, mut reread, mut after_reread, mut inspect, mut load] =
            std::array::from_fn::<Vec<f64>, 8, _>(|_| Vec::new());
        for round in 0..10u32 {
            let value = f64::from(round);
            let start = Instant::now();
            f.ok("run", json!({ "steps": [set_opacity(mine, value)] })).await;
            plain.push(start.elapsed().as_secs_f64() * 1000.0);
            let (_, _, basis) = f.inspect().await;
            let start = Instant::now();
            let ran = f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, value + 0.5)] })).await;
            based.push(start.elapsed().as_secs_f64() * 1000.0);
            f.colleague(&[opacity(theirs, value)]);
            let start = Instant::now();
            let ran = f.ok("run", json!({ "basis": revision(&ran), "steps": [set_opacity(mine, value + 0.25)] })).await;
            unrelated.push(start.elapsed().as_secs_f64() * 1000.0);
            f.colleague(&[opacity(mine, value + 0.75)]);
            let start = Instant::now();
            let answer = f.call("run", json!({ "basis": revision(&ran), "steps": [set_opacity(mine, value)] })).await;
            refused.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(answer.as_ref().err().map(|error| error["error"]["code"].clone()), Some(json!("conflict")), "{name}: {answer:?}");
            let basis = revision(&answer.unwrap_err());
            f.colleague(&[opacity(theirs, value + 0.5)]);
            let start = Instant::now();
            f.ok("inspect", json!({})).await;
            reread.push(start.elapsed().as_secs_f64() * 1000.0);
            let start = Instant::now();
            f.ok("run", json!({ "basis": basis, "steps": [set_opacity(mine, value + 0.125)] })).await;
            after_reread.push(start.elapsed().as_secs_f64() * 1000.0);
            let start = Instant::now();
            f.ok("inspect", json!({})).await;
            inspect.push(start.elapsed().as_secs_f64() * 1000.0);
            let start = Instant::now();
            f.ok("query", json!({ "objects": [mine] })).await;
            load.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        println!(
            "timings ({name}): plain {:.1} ms, with basis {:.1} ms, after an unrelated file change {:.1} ms, refused after a change of the target in the file {:.1} ms; an inspect after an unrelated file change {:.1} ms, then a write with basis {:.1} ms; an inspect {:.1} ms; one layer loaded (motion_query) {:.1} ms",
            median(plain),
            median(based),
            median(unrelated),
            median(refused),
            median(reread),
            median(after_reread),
            median(inspect),
            median(load)
        );
        f.shutdown().await;
    }
}

/// The colleague's change of the target of s12 (the sixth layer from the top, opacity keys at 1,
/// 2, 3, 4 s), ten rounds each on a fresh 12-layer comp, with s12's writes: (A) rename by id, (B)
/// rotation of `"#6"`, (C) an opacity key by id at `at.time` 2 s. The changes: its rotation set to
/// 45 (what B writes, C8-fix-7); an opacity key inserted at 1.5 s, before the key at 2 s (C8-time-3);
/// its time stretched to 200 % (its keys at twice their comp times, C8-time-3). A refusal is
/// necessary as in s12: (B) the rotation changed or `"#6"` names another layer; (C) the opacity
/// key at comp time 2 s gone or changed.
#[tokio::test(flavor = "multi_thread")]
async fn s12b_changes_of_the_target_under_its_place() {
    const N: usize = 12;
    const MINE: usize = 5;
    println!("s12b: their change | write | rounds | applied | refused, necessary | refused, needless | applied, wrongly");
    let mut wrong = 0;
    for change in ["RotateUnderPlace", "KeyBefore", "Stretch"] {
        let mut counts = [(0, 0, 0, 0); 3];
        for _ in 0..10 {
            let f = Fixture::new(N);
            let (info, _, _) = f.inspect().await;
            let mine = layer_ids(&info)[MINE];
            f.ok("run", json!({ "steps": target_keys(mine) })).await;
            let (_, _, basis) = f.inspect().await;
            let rotation = |file: &std::path::Path| value_on_disk(file, mine, "transform/rotation");
            let (seen_key, seen_rotation) = (key_on_disk_at(&f.file(), mine, T), rotation(&f.file()));
            f.colleague(&[match change {
                "RotateUnderPlace" => ("prop.set", json!({ "layer": mine, "path": "transform/rotation", "value": 45 })),
                "KeyBefore" => ("prop.addKey", json!({ "layer": mine, "path": OPACITY, "time": 1.5, "value": 15 })),
                _ => ("layer.timeStretch", json!({ "layers": [mine], "percent": 200 })),
            }]);
            let now_sixth = layers_on_disk(&f.file()).get(MINE).copied();
            let (now_key, now_rotation) = (key_on_disk_at(&f.file(), mine, T), rotation(&f.file()));
            let a = f.call("run", json!({ "basis": basis, "steps": [{ "command": "layer.rename", "params": { "layer": mine, "name": "A" } }] })).await;
            let b = f
                .call(
                    "run",
                    json!({ "basis": basis, "steps": [{ "command": "prop.set", "params": { "layer": "#6", "path": "transform/rotation", "value": 15 } }] }),
                )
                .await;
            let c = f.call("run", json!({ "basis": basis, "steps": [key_at_t(mine)] })).await;
            let needed = [false, now_sixth != Some(mine) || seen_rotation != now_rotation, seen_key != now_key];
            for (cell, (answer, necessary)) in counts.iter_mut().zip([a, b, c].into_iter().zip(needed)) {
                match (answer, necessary) {
                    (Ok(_), true) => cell.3 += 1,
                    (Ok(_), false) => cell.0 += 1,
                    (Err(_), true) => cell.1 += 1,
                    (Err(error), false) => {
                        cell.2 += 1;
                        println!("s12b: {change}: needless refusal: {}", error["conflicts"]);
                    }
                }
            }
            f.shutdown().await;
        }
        for (write, (applied, needed, needless, wrongly)) in ['A', 'B', 'C'].iter().zip(counts) {
            println!("s12b: {change} | {write} | 10 | {applied} | {needed} | {needless} | {wrongly}");
            wrong += wrongly;
        }
    }
    assert_eq!(wrong, 0, "no write applied wrongly");
}

/// C8-fix-7 item 2: what `others` says, and its size, for the bottom layer moved to the top in a
/// comp of 5 and of 200 layers, every opacity key of a layer moved 0.5 s later with 5 and with 200
/// keys, and a solid layer created. Printed, not asserted.
#[tokio::test(flavor = "multi_thread")]
async fn zz_receipt_sizes() {
    for count in [5usize, 200] {
        let f = Fixture::new(count);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        f.colleague(&[("layer.arrange", json!({ "layers": [layers[count - 1]], "to": "front" }))]);
        let ran = f.ok("run", json!({ "basis": basis, "steps": [{ "command": "layer.rename", "params": { "layer": layers[1], "name": "x" } }] })).await;
        println!("SIZES move bottom to top of {count}: others {} bytes: {}", ran["others"].to_string().len(), ran["others"]);
        let basis = revision(&ran);
        if count == 5 {
            f.colleague(&[new_solid_on_top()]);
            let ran = f.ok("run", json!({ "basis": basis, "steps": [{ "command": "layer.rename", "params": { "layer": layers[1], "name": "y" } }] })).await;
            println!("SIZES created solid: others {} bytes: {}", ran["others"].to_string().len(), ran["others"]["created"]);
        }
        f.shutdown().await;
    }
    for count in [5usize, 200] {
        let f = Fixture::new(2);
        let (info, _, _) = f.inspect().await;
        let mine = layer_ids(&info)[0];
        let steps: Vec<Value> = (0..count)
            .map(|k| json!({ "command": "prop.addKey", "params": { "layer": mine, "path": OPACITY, "time": k as f64 * 0.04, "value": k % 100 } }))
            .collect();
        f.ok("run", json!({ "steps": steps })).await;
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("keys.selectAll", json!({ "layers": [mine] })), ("keys.move", json!({ "delta": 0.5 }))]);
        let ran =
            f.ok("run", json!({ "basis": basis, "steps": [{ "command": "layer.rename", "params": { "layer": layer_ids(&info)[1], "name": "x" } }] })).await;
        let others = ran["others"].to_string();
        println!("SIZES every key moved 0.5 s, {count} keys: others {} bytes: {}", others.len(), &others[..others.len().min(600)]);
        f.shutdown().await;
    }
}
