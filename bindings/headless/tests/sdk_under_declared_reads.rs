//! Commands whose code reads editor state their declarations (`bindings/wasm/src/reads.json`)
//! missed, found by `bindings/wasm/tests/acts.rs` (task acts-known-differences, case
//! regressions/issue-acts-under-declared-reads):
//!
//! - `learn.step` with `showMe` runs the tutorial step's commands (learn.rs `step_cmd`), and
//!   `layer.newText` there types into the active composition: it reads `part`. Undeclared, the
//!   person opening another composition after the agent's basis was no premise of the step (one
//!   that gives its time and layers failed only because its layers are not in the person's
//!   composition); declared, the step is refused as `changed` on `part`, and applies when it gives
//!   its composition.
//! - `mediaBrowser.import` with `addToComp` adds the files to the active composition above its
//!   first selected layer, at its current time (panels_cmds.rs `browser_import` → file.rs
//!   `add_to_comp` → layer.rs `add_item`; the engine test `media_browser_reads.rs` shows it): it
//!   reads `selection` and `time`. An agent cannot run it (commands that name files are the
//!   host's), so the declaration is checked here and the reading in the engine test.
//! - The script window and panel commands (`scriptWindow.click`, `.set`, `.close`,
//!   `window.scriptPanel`, `window.restoreScriptPanels`) run JavaScript that reads `part`,
//!   `selection` and `time`; the kit has no scripting engine, so their declarations are checked.
mod common;

use common::*;
use kition_kit::KitDocument;
use kition_kits::Caller;
use serde_json::{Map, Value, json};

/// The person, in the document the server holds, runs `command`.
async fn person(fixture: &Fixture, command: &str, params: Value) {
    let view = fixture.kits.open(Caller::User, FILE).await.unwrap();
    let params: Map<String, Value> = params.as_object().cloned().unwrap();
    fixture.kits.execute(Caller::User, view.kit_id, &view.document_id, command, params).await.unwrap();
}

/// `Main` (open, with two solids) and an empty `Other`.
fn two_comps() -> Vec<u8> {
    let mut doc = kition_motion_kit::MotionDocument::open(FILE, &project(2)).unwrap();
    doc.execute("comp.new", json!({"name": "Other", "width": 64, "height": 64, "duration": 2, "open": false})).unwrap();
    doc.save(None).unwrap()
}

fn assert_changed(refused: &Value, state: &str) {
    assert_eq!(refused["error"]["code"], "conflict", "the {state} the step reads is no premise: {refused}");
    let conflicts = refused["conflicts"].as_array().unwrap();
    assert!(conflicts.iter().any(|conflict| conflict["now"]["state"] == state && conflict["reason"] == "changed"), "not `changed` on `{state}`: {refused}");
}

#[test]
fn the_commands_declare_what_their_code_reads() {
    // The script window and panel commands run JavaScript (script_windows.rs `dispatch`,
    // file_more.rs `run_js`) whose object model reads the active comp, the selected layers and
    // the time (crates/engine/script/src/model.rs), as script.run and file.runScript declare.
    let scripts =
        ["scriptWindow.click", "scriptWindow.set", "scriptWindow.close", "window.scriptPanel", "window.restoreScriptPanels", "script.run", "file.runScript"];
    for id in ["learn.step", "mediaBrowser.import"].into_iter().chain(scripts) {
        let declared: Vec<&str> = kition_motion_kit::acts::reads(id).iter().map(|state| state.as_str()).collect();
        for kind in ["part", "selection", "time"] {
            assert!(declared.contains(&kind), "{id} reads `{kind}` but declares {declared:?}");
        }
    }
}

/// The agent's tutorial at its second step, "Type a title", whose Show Me types into the active
/// composition, with nothing selected (so that opening another composition changes only the
/// active one); the basis after it.
async fn at_type_a_title(fixture: &Fixture) -> (u64, String, u64) {
    let steps = json!([
        {"command":"learn.start","params":{"id":"animate-title"}},
        {"command":"learn.step","params":{"action":"next"}},
        {"command":"edit.deselectAll","params":{}}
    ]);
    fixture.ok("run", json!({ "steps": steps })).await;
    let (info, comp, basis) = fixture.inspect().await;
    (comp, basis, layer_ids(&info)[0])
}

/// Show Me with its time and selection given (`at.time`, `on`: Main's `layer`), so that only its
/// composition is left to the person's editor.
fn show_me(layer: u64, at: Value) -> Value {
    json!({"command":"learn.step","params":{"action":"showMe"},"on":[layer],"at":at})
}

/// The title layers of the file's compositions `Main` and `Other`.
fn titles(fixture: &Fixture) -> (usize, usize) {
    let mut doc = on_disk(&fixture.file());
    let mut count = |comp: &str| {
        doc.execute("comp.open", json!({ "comp": comp })).unwrap();
        doc.inspect().unwrap()["info"]["layers"].as_array().unwrap().iter().filter(|layer| layer.to_string().contains("Hello, motion")).count()
    };
    (count("Main"), count("Other"))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_person_opening_another_comp_refuses_show_me_as_changed_part() {
    let fixture = Fixture::from_bytes(&two_comps(), Default::default());
    let (_, basis, layer) = at_type_a_title(&fixture).await;
    person(&fixture, "comp.open", json!({"comp": "Other"})).await;
    let answer = fixture.call("run", json!({"basis":basis,"steps":[show_me(layer, json!({"time": 0}))]})).await;
    let refused = answer.expect_err("Show Me would type into the composition the person opened");
    assert_changed(&refused, "part");
    assert!(refused["conflicts"].as_array().unwrap().iter().all(|conflict| conflict["now"]["state"] == "part"), "{refused}");
    assert_eq!(titles(&fixture), (0, 0), "nothing was typed: {refused}");
    fixture.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn show_me_applies_in_the_comp_its_step_gives() {
    let fixture = Fixture::from_bytes(&two_comps(), Default::default());
    let (main, basis, layer) = at_type_a_title(&fixture).await;
    person(&fixture, "comp.open", json!({"comp": "Other"})).await;
    let answer = fixture.call("run", json!({"basis":basis,"steps":[show_me(layer, json!({"part": {"comp": main}, "time": 0}))]})).await;
    assert!(answer.is_ok(), "the step gives its composition, yet the person's comp.open refused it: {answer:?}");
    assert_eq!(titles(&fixture), (1, 0), "the title is typed into Main, the composition the step gave: {answer:?}");
    fixture.shutdown().await;
}
