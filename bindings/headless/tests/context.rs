//! `kition-motion` served headless keeps the editing session (the current time, which layers are
//! selected, the selected layer's transform panel) out of what an agent reads of the project
//! (`view.describe`, the `motion_inspect` details), while keyframe times and the animated
//! properties' paths, which are the document's, stay (task card C4-motion).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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

/// The headless kit as the `kition-motion` executable makes it.
fn kit() -> Headless<MotionDocument> {
    let info = KitInfo::from_manifest(MANIFEST, &["file.import"], kition_motion_kit::EDITOR_STATE).unwrap();
    Headless::<MotionDocument>::with_replica_port(info)
}

/// The project the tests open: `Main` (320 x 180, 30 fps, 4 s, open) with a red solid `Red` and a
/// blue solid `Blue` whose opacity is keyed at 1 s and 1.5 s, and `Other` (160 x 90, 25 fps, 2 s)
/// with a green solid `Green`.
fn project() -> MotionDocument {
    let mut doc = MotionDocument::create("Motion.kmotion", 320, 180).unwrap();
    doc.execute("comp.settings", json!({ "duration": 4, "name": "Main" })).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
    let blue = doc.execute("layer.newSolid", json!({ "name": "Blue", "color": "#0000ff" })).unwrap()["layer"].clone();
    for (time, value) in [(1.0, 0.0), (1.5, 100.0)] {
        doc.execute("prop.addKey", json!({ "layer": blue, "path": "transform/opacity", "time": time, "value": value, "timeBase": "comp" })).unwrap();
    }
    let main = doc.inspect().unwrap()["comp"]["id"].clone();
    doc.execute("comp.new", json!({ "name": "Other", "width": 160, "height": 90, "frameRate": 25, "duration": 2, "open": true })).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Green", "color": "#00ff00" })).unwrap();
    doc.execute("comp.open", json!({ "comp": main })).unwrap();
    doc
}

/// A workspace holding [`project`] as `Motion.kmotion`, opened by the headless kit.
fn open(name: &str) -> (Headless<MotionDocument>, String, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("kition-motion-context-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    std::fs::write(root.join("Motion.kmotion"), project().save(None).unwrap()).unwrap();
    let mut kit = kit();
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("Motion.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    (kit, id, root)
}

fn run(kit: &mut Headless<MotionDocument>, id: &str, command: &str, value: Value, origin: Origin) -> Value {
    kit.execute(id, command, params(value), origin).unwrap_or_else(|error| panic!("{command}: {error:?}")).result.unwrap_or(Value::Null)
}

/// Whether `value` holds the key `name` anywhere.
fn has_key(value: &Value, name: &str) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| key == name || has_key(value, name)),
        Value::Array(items) => items.iter().any(|item| has_key(item, name)),
        _ => false,
    }
}

/// Every place `EDITOR_STATE` names is in what the kit's `inspect()` answers (with a layer selected,
/// so the transform panel shows) and in no part of what an agent reads; a keyframe's time and an
/// animated property's path, which are the document's, are.
#[test]
fn what_an_agent_reads_leaves_out_exactly_the_declared_editor_state() {
    let (mut kit, id, root) = open("declared");
    let described = kit.describe(&id).unwrap().state;
    let mut doc = project();
    let blue = doc.inspect().unwrap()["info"]["layers"][0]["id"].clone();
    doc.execute("layer.select", json!({ "layers": [blue] })).unwrap();
    let full = doc.inspect().unwrap();
    let described = Value::Object(described);
    for key in kition_motion_kit::EDITOR_STATE {
        if key.starts_with('/') {
            assert!(full.pointer(key).is_some(), "{key} names nothing in inspect(): {full}");
            assert!(described.pointer(key).is_none(), "{key} is the editor's, not the document's");
        } else {
            assert!(has_key(&full, key), "{key} is in no part of inspect(): {full}");
            assert!(!has_key(&described, key), "{key} is the editor's, not the document's");
        }
    }
    let animated = &described["info"]["layers"][0]["animated"][0];
    assert_eq!(animated["times"], json!([1.0, 1.5]), "keyframe times are the document's: {animated}");
    assert!(animated["path"].as_str().is_some_and(|path| !path.is_empty()), "an animated property's path is the document's: {animated}");
    let _ = std::fs::remove_dir_all(root);
}

/// A person moving the current time or selecting layers changes nothing an agent reads of the
/// project.
#[test]
fn the_current_time_and_the_selection_change_nothing_an_agent_reads() {
    let (mut kit, id, root) = open("unchanged");
    let before = kit.describe(&id).unwrap().state;
    let blue = before["info"]["layers"][0]["id"].clone();
    run(&mut kit, &id, "time.set", json!({ "time": 2.5 }), Origin::User);
    run(&mut kit, &id, "layer.select", json!({ "layers": [blue] }), Origin::User);
    assert_eq!(kit.describe(&id).unwrap().state, before, "the editor's state changed what the agent reads of the project");
    let _ = std::fs::remove_dir_all(root);
}

/// The ids of the layers named `names` in what `describe` answers, in that order.
fn ids(described: &Map<String, Value>, names: &[&str]) -> Vec<Value> {
    let layers = described["info"]["layers"].as_array().unwrap();
    names.iter().map(|name| layers.iter().find(|layer| layer["name"] == json!(name)).unwrap()["id"].clone()).collect()
}

/// What a person sees: the open comp, its current time, and which of its layers are selected.
fn seen(kit: &mut Headless<MotionDocument>, id: &str) -> (Value, Value) {
    let state = kit.context_get(id).unwrap();
    (state["activeComp"].clone(), json!([state["times"].clone(), state["selectedLayers"].clone()]))
}

/// As kition-kits runs `motion_run`: saves the editor state; for each step writes `on` into the
/// command's `targets` parameter when it declares one (else `on` goes to `context.set`), sets the
/// step's context and runs it as the agent; and puts the state back after the last step or the
/// first failure. Answers each step's result, or the failing step's index and error.
fn motion_run(kit: &mut Headless<MotionDocument>, id: &str, steps: &[(Value, &str, Value)]) -> Result<Vec<Value>, (usize, String)> {
    let commands = MotionDocument::create("a.kmotion", 64, 36).unwrap().commands();
    let saved = kit.context_get(id).unwrap();
    let mut done = Vec::new();
    let mut failed = None;
    for (index, (context, command, params)) in steps.iter().enumerate() {
        let (mut context, mut params) = (params_of(context), params_of(params));
        let targets = commands.iter().find(|known| known.id == *command).and_then(|known| known.targets);
        if let (Some(name), Some(on)) = (targets, context.get("on").cloned()) {
            context.remove("on");
            params.insert(name.to_owned(), if name == "layers" { on } else { on[0].clone() });
        }
        if !context.is_empty()
            && let Err(error) = kit.context_set(id, &context)
        {
            failed = Some((index, format!("{error:?}")));
            break;
        }
        match kit.execute(id, command, params, Origin::Agent) {
            Ok(executed) => done.push(executed.result.unwrap_or(Value::Null)),
            Err(error) => {
                failed = Some((index, format!("{error:?}")));
                break;
            }
        }
    }
    kit.context_restore(id, &saved).unwrap();
    failed.map_or(Ok(done), Err)
}

fn params_of(value: &Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// A key added at `at.time` 2 s on a layer named by id lands at 2 s of comp time; the person's
/// current time and selection stay, the context requests are no edit, and the details the agent
/// reads change only by the key.
#[test]
fn a_key_at_a_time_leaves_the_current_time_alone() {
    let (mut kit, id, root) = open("key-at-time");
    let described = kit.describe(&id).unwrap().state;
    let [blue, red] = ids(&described, &["Blue", "Red"]).try_into().unwrap();
    run(&mut kit, &id, "time.set", json!({ "time": 0.5 }), Origin::User);
    run(&mut kit, &id, "layer.select", json!({ "layers": [blue] }), Origin::User);
    let before = seen(&mut kit, &id);
    let saved = kit.context_get(&id).unwrap();
    kit.context_set(&id, &params(json!({ "time": 2.0 }))).unwrap();
    assert!(!kit.document(&id).unwrap().dirty, "setting the context is no edit");
    assert_eq!(kit.describe(&id).unwrap().state, described, "setting the context changed what the agent reads");
    let set = kit.context_get(&id).unwrap();
    assert_eq!(set["times"][set["activeComp"].to_string()], json!(2 * 254_016_000_000_i64), "{set}");
    kit.context_restore(&id, &saved).unwrap();
    assert_eq!(seen(&mut kit, &id), before);
    let ran = motion_run(&mut kit, &id, &[(json!({ "time": 2.0 }), "prop.addKey", json!({ "layer": red, "path": "transform/opacity", "value": 40 }))]);
    assert!(ran.is_ok(), "{ran:?}");
    assert_eq!(seen(&mut kit, &id), before, "the current time and the selection are the person's");
    let after = kit.describe(&id).unwrap().state;
    let row = after["info"]["layers"].as_array().unwrap().iter().find(|layer| layer["id"] == red).unwrap().clone();
    assert_eq!(row["animated"][0]["times"], json!([2.0]), "{row}");
    let _ = std::fs::remove_dir_all(root);
}

/// One run changes a layer in each of the two comps, naming each comp with `at.part`; afterwards
/// the open comp, its time and the selection are as before, and a later failing step puts the
/// editor state back as it was before the run.
#[test]
fn two_comps_in_one_run_and_a_failing_step_leave_the_editor_as_it_was() {
    let (mut kit, id, root) = open("two-comps");
    let described = kit.describe(&id).unwrap().state;
    let main = described["comp"]["id"].clone();
    let [blue] = ids(&described, &["Blue"]).try_into().unwrap();
    // Other and its layer: the file is the fixture's, with its ids.
    let info = project().execute("comp.info", json!({ "comp": "Other" })).unwrap();
    let (other, green_id) = (info["id"].clone(), info["layers"][0]["id"].clone());
    assert!(other.is_u64() && green_id.is_u64(), "{info}");
    run(&mut kit, &id, "time.set", json!({ "time": 1.5 }), Origin::User);
    let before = seen(&mut kit, &id);
    let shy = json!({ "switch": "shy", "value": true });
    let ran = motion_run(
        &mut kit,
        &id,
        &[
            (json!({ "part": { "comp": main }, "on": [blue] }), "layer.setSwitch", shy.clone()),
            (json!({ "part": { "comp": other }, "on": [green_id] }), "layer.setSwitch", shy.clone()),
        ],
    );
    assert!(ran.is_ok(), "{ran:?}");
    assert_eq!(seen(&mut kit, &id), before);
    for (comp, layer) in [(&main, &blue), (&other, &green_id)] {
        let info = run(&mut kit, &id, "comp.info", json!({ "comp": comp }), Origin::Agent);
        let row = info["layers"].as_array().unwrap().iter().find(|row| row["id"] == *layer).unwrap().clone();
        assert_eq!(row["switches"]["shy"], json!(true), "{row}");
    }
    let state = kit.context_get(&id).unwrap();
    let failed = motion_run(
        &mut kit,
        &id,
        &[
            (json!({ "part": { "comp": other }, "on": [green_id] }), "layer.setSwitch", json!({ "switch": "solo", "value": true })),
            (json!({ "part": { "comp": main }, "time": 3 }), "effect.apply", json!({ "effect": "No Such Effect", "layers": [blue] })),
        ],
    );
    assert_eq!(failed.map_err(|(index, _)| index), Err(1), "the second step fails");
    assert_eq!(kit.context_get(&id).unwrap(), state, "a failed run leaves the editor state as before it");
    let _ = std::fs::remove_dir_all(root);
}

/// An unknown comp or layer, a time outside the comp, and a stack position or `#n` in `on` are
/// `invalid_params` errors that say what is valid; the editor state does not change.
#[test]
fn what_the_project_lacks_is_invalid_params() {
    let (mut kit, id, root) = open("refused");
    let before = kit.context_get(&id).unwrap();
    for (context, says) in [
        (json!({ "part": { "comp": 999 } }), "no comp of this project"),
        (json!({ "time": 9 }), "from 0 to 4 s"),
        (json!({ "on": [999] }), "no layer id"),
        (json!({ "on": [1] }), "stack position is no id"),
        (json!({ "on": ["#2"] }), "`on` takes layer ids"),
    ] {
        let error = kit.context_set(&id, &params(context.clone())).unwrap_err();
        let shown = format!("{error:?}");
        assert!(shown.contains("invalid_params") && shown.contains(says), "{context}: {shown}");
        assert_eq!(kit.context_get(&id).unwrap(), before, "{context} changed the editor state");
    }
    let _ = std::fs::remove_dir_all(root);
}
