//! Explicit call context (task card C4-motion): a run step's `at.part` (`{comp: id}`), `at.time`
//! (comp seconds) and `on` (layer ids) set the active comp, its current time and the layer
//! selection for one call through `KitDocument::set_context`, the host puts the editor state back
//! with `restore_editor_state`, none of it is an edit, and an agent's command given the layers,
//! comp or time it acts on leaves the editor as it was.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::kdm::Origin;
use kition_kit::{CallContext, KitDocument};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

/// `Main` (320 x 180, 30 fps, 4 s, open) with `Red` (its layer time starting 1 s into the comp)
/// and `Blue` on top, and `Other` (160 x 90, 25 fps, 2 s) with `Green`.
struct Fixture {
    doc: MotionDocument,
    main: u64,
    other: u64,
    red: u64,
    blue: u64,
    green: u64,
}

fn fixture() -> Fixture {
    let mut doc = MotionDocument::create("Motion.kmotion", 320, 180).unwrap();
    doc.execute("comp.settings", json!({ "duration": 4, "name": "Main" })).unwrap();
    let red = doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.timing", json!({ "layers": [red], "start": 1 })).unwrap();
    let blue = doc.execute("layer.newSolid", json!({ "name": "Blue", "color": "#0000ff" })).unwrap()["layer"].as_u64().unwrap();
    let main = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let other =
        doc.execute("comp.new", json!({ "name": "Other", "width": 160, "height": 90, "frameRate": 25, "duration": 2 })).unwrap()["comp"].as_u64().unwrap();
    let green = doc.execute("layer.newSolid", json!({ "name": "Green", "color": "#00ff00" })).unwrap()["layer"].as_u64().unwrap();
    doc.execute("comp.open", json!({ "comp": main })).unwrap();
    doc.execute("layer.select", json!({ "layers": [] })).unwrap();
    doc.execute("time.set", json!({ "time": 0 })).unwrap();
    doc.mark_saved();
    Fixture { doc, main, other, red, blue, green }
}

fn context(on: &[Value], part: Option<u64>, time: Option<f64>) -> CallContext {
    CallContext { on: on.to_vec(), part: part.map(|comp| json!({ "comp": comp })), time }
}

fn agent(doc: &mut MotionDocument, command: &str, params: Value) -> Result<Value, String> {
    doc.execute_as(command, params, &Origin::agent("agent:test"))
}

/// What a person sees of the editor: the open comp, its current time, and which of its layers are
/// selected.
fn seen(doc: &MotionDocument) -> (Value, Value, Vec<u64>) {
    let inspected = doc.inspect().unwrap();
    let selected = inspected["info"]["layers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|layer| layer["selected"] == json!(true))
        .map(|layer| layer["id"].as_u64().unwrap())
        .collect();
    (inspected["comp"]["id"].clone(), inspected["time"].clone(), selected)
}

/// Which layers of comp `comp` are shy.
fn shy(doc: &mut MotionDocument, comp: u64) -> Vec<u64> {
    let info = doc.execute("comp.info", json!({ "comp": comp })).unwrap();
    info["layers"].as_array().unwrap().iter().filter(|layer| layer["switches"]["shy"] == json!(true)).map(|layer| layer["id"].as_u64().unwrap()).collect()
}

/// The comp times of `layer`'s keyframes, as motion_inspect lists them for the open comp.
fn key_times(doc: &MotionDocument, layer: u64) -> Value {
    let inspected = doc.inspect().unwrap();
    let row = inspected["info"]["layers"].as_array().unwrap().iter().find(|row| row["id"] == json!(layer)).unwrap().clone();
    row["animated"][0]["times"].clone()
}

/// kition-sdk's check with contexts that move the editor: layers selected and a time set in the
/// open comp, by id and by name; neither the hooks nor the restore are an edit, and the state after
/// the restore is the state before.
#[test]
fn contexts_that_move_the_editor_pass_the_sdk_check() {
    let mut f = fixture();
    let moving = context(&[json!(f.blue), json!(f.red)], Some(f.main), Some(2.5));
    kition_kit::testing::check_context(&mut f.doc, &moving, kition_motion_kit::EDITOR_STATE).unwrap();
    kition_kit::testing::check_context(&mut f.doc, &context(&[json!("Red")], None, Some(3.99)), kition_motion_kit::EDITOR_STATE).unwrap();
    assert_eq!(seen(&f.doc), (json!(f.main), json!(0.0), vec![]), "restored");
}

/// A context in the other comp makes it active, sets its current time on the start of the frame
/// showing it (as `file.render` draws `target.time`) and selects exactly the layers `on` names;
/// restoring puts back the open comp, both current times and the selection, and no hook is an edit.
#[test]
fn a_context_in_another_comp_moves_the_editor_and_restoring_puts_it_back() {
    let mut f = fixture();
    f.doc.execute("time.set", json!({ "time": 1.5 })).unwrap();
    f.doc.execute("layer.select", json!({ "layers": [f.red] })).unwrap();
    let saved = f.doc.editor_state();
    assert!(!saved.is_null(), "the kit keeps editor state");
    let before = seen(&f.doc);
    let (revision, dirty) = (f.doc.revision(), f.doc.dirty());
    f.doc.set_context(&context(&[json!(f.green)], Some(f.other), Some(1.03))).unwrap();
    // 25 fps: 1.03 s is in frame 25, which starts at 1 s.
    assert_eq!(seen(&f.doc), (json!(f.other), json!(1.0), vec![f.green]));
    let drawn = f.doc.execute("file.render", json!({ "target": { "comp": f.other, "time": 1.03 } })).unwrap();
    assert_eq!(drawn["frames"][0]["time"], json!(1.0), "at.time lands where file.render draws target.time");
    assert_eq!((f.doc.revision(), f.doc.dirty()), (revision, dirty), "setting the context is no edit");
    f.doc.restore_editor_state(&saved).unwrap();
    assert_eq!(f.doc.editor_state(), saved);
    assert_eq!(seen(&f.doc), before);
    assert_eq!((f.doc.revision(), f.doc.dirty()), (revision, dirty), "restoring is no edit");
}

/// `on` and `at.part` drive a command that acts on the selected layers: exactly the layers named
/// change, in the comp named, and afterwards the open comp, the time and the selection are as the
/// person left them.
#[test]
fn on_and_part_drive_a_selection_command() {
    let mut f = fixture();
    f.doc.execute("time.set", json!({ "time": 0.5 })).unwrap();
    f.doc.execute("layer.select", json!({ "layers": [f.red] })).unwrap();
    let before = seen(&f.doc);
    let saved = f.doc.editor_state();
    f.doc.set_context(&context(&[json!(f.blue)], Some(f.main), None)).unwrap();
    agent(&mut f.doc, "layer.setSwitch", json!({ "switch": "shy", "value": true })).unwrap();
    f.doc.restore_editor_state(&saved).unwrap();
    assert_eq!(shy(&mut f.doc, f.main), vec![f.blue], "only the layer `on` names changed");
    assert_eq!(seen(&f.doc), before);
}

/// `at.time` drives commands that default to the current time: the key lands at that comp time on
/// a layer whose own time starts 1 s into the comp (comp time, never layer time), the split at the
/// start of the frame showing it, and the current time is left alone.
#[test]
fn at_time_drives_current_time_commands_in_comp_time() {
    let mut f = fixture();
    f.doc.execute("time.set", json!({ "time": 0.5 })).unwrap();
    let before = seen(&f.doc);
    let saved = f.doc.editor_state();
    f.doc.set_context(&context(&[], None, Some(2.0))).unwrap();
    agent(&mut f.doc, "prop.addKey", json!({ "layer": f.red, "path": "transform/opacity", "value": 50 })).unwrap();
    f.doc.set_context(&context(&[], None, Some(1.25))).unwrap();
    let split = agent(&mut f.doc, "edit.splitLayer", json!({ "layers": [f.blue] })).unwrap();
    f.doc.restore_editor_state(&saved).unwrap();
    assert_eq!(key_times(&f.doc, f.red), json!([2.0]), "the key is at 2 s of comp time");
    let piece = split[0].as_u64().unwrap();
    let info = f.doc.execute("comp.info", json!({ "comp": f.main })).unwrap();
    let piece = info["layers"].as_array().unwrap().iter().find(|row| row["id"] == json!(piece)).unwrap();
    let drawn = f.doc.execute("file.render", json!({ "target": { "comp": f.main, "time": 1.25 } })).unwrap();
    assert_eq!(piece["in"], drawn["frames"][0]["time"], "the split is where file.render draws 1.25 s: {piece}");
    assert_eq!(seen(&f.doc), before, "the current time is the person's");
}

/// Two steps in two comps of one run, each naming its comp and its layer; afterwards the editor is
/// as before the run.
#[test]
fn two_comps_in_one_run() {
    let mut f = fixture();
    f.doc.execute("layer.select", json!({ "layers": [f.red] })).unwrap();
    let before = seen(&f.doc);
    let saved = f.doc.editor_state();
    for (comp, layer) in [(f.main, f.blue), (f.other, f.green)] {
        f.doc.set_context(&context(&[json!(layer)], Some(comp), None)).unwrap();
        agent(&mut f.doc, "layer.setSwitch", json!({ "switch": "shy", "value": true })).unwrap();
    }
    f.doc.restore_editor_state(&saved).unwrap();
    assert_eq!((shy(&mut f.doc, f.main), shy(&mut f.doc, f.other)), (vec![f.blue], vec![f.green]));
    assert_eq!(seen(&f.doc), before);
    assert_eq!(f.doc.editor_state(), saved);
}

/// A comp, layer or time the project lacks, a stack position or `#n` where an id belongs, and a
/// name two layers share are refused, saying what is valid, and nothing changes.
#[test]
fn what_the_project_lacks_is_refused_and_changes_nothing() {
    let mut f = fixture();
    for _ in 0..2 {
        let twin = f.doc.execute("layer.newSolid", json!({})).unwrap()["layer"].clone();
        f.doc.execute("layer.rename", json!({ "layer": twin, "name": "Twin" })).unwrap();
    }
    let saved = f.doc.editor_state();
    let position = (1..20).find(|n| ![f.red, f.blue, f.green].contains(n)).unwrap();
    let cases: Vec<(CallContext, Vec<String>)> = vec![
        (context(&[], Some(999), None), vec!["999".into(), format!("{} \"Main\"", f.main), format!("{} \"Other\"", f.other)]),
        (CallContext { part: Some(json!({ "sequence": f.main })), ..Default::default() }, vec!["{\"comp\": id}".into()]),
        (context(&[], Some(f.main), Some(9.0)), vec!["9 s".into(), "from 0 to 4 s".into(), "3.967".into()]),
        (context(&[], Some(f.other), Some(-1.0)), vec!["-1 s".into(), "from 0 to 2 s".into()]),
        (context(&[json!(999)], None, None), vec!["999".into(), format!("{} \"Red\"", f.red)]),
        (context(&[json!(position)], None, None), vec![format!("{position}"), "stack position".into(), "ids".into()]),
        (context(&[json!("#1")], None, None), vec!["#1".into(), "ids".into()]),
        (context(&[json!("Twin")], None, None), vec!["Twin".into(), "2 layers".into(), "ids".into()]),
        (context(&[json!(f.green)], Some(f.main), None), vec![format!("{}", f.green), "Other".into(), format!("{{\"comp\": {}}}", f.other)]),
    ];
    for (context, says) in cases {
        let refused = f.doc.set_context(&context).expect_err(&format!("{context:?} is refused"));
        for words in &says {
            assert!(refused.contains(words.as_str()), "{context:?}: `{refused}` does not say {words}");
        }
        assert!(!refused.to_ascii_lowercase().contains("disabled") && !refused.contains("not available"), "a refusal is invalid params: {refused}");
        assert_eq!(f.doc.editor_state(), saved, "{context:?} changed the editor");
    }
}

/// An agent's command given the layers it acts on leaves the selections as they were, also when it
/// fails; a person's keeps the editor's own behaviour, and so does an agent's navigation command.
#[test]
fn an_agent_command_given_its_layers_leaves_the_editor_as_it_was() {
    let mut f = fixture();
    f.doc.execute("time.set", json!({ "time": 0.5 })).unwrap();
    f.doc.execute("layer.select", json!({ "layers": [f.red] })).unwrap();
    let saved = f.doc.editor_state();
    agent(&mut f.doc, "effect.apply", json!({ "effect": "Gaussian Blur", "layers": [f.blue] })).unwrap();
    assert_eq!(f.doc.editor_state(), saved, "applying an effect to named layers selected it");
    agent(&mut f.doc, "effect.apply", json!({ "effect": "Gaussian Blur", "layers": [f.blue], "params": { "Nope": 1 } })).unwrap_err();
    assert_eq!(f.doc.editor_state(), saved);
    agent(&mut f.doc, "time.set", json!({ "time": 2 })).unwrap();
    assert_eq!(seen(&f.doc).1, json!(2.0), "an agent's time.set moves the current time");
    f.doc.execute_as("effect.apply", json!({ "effect": "Gaussian Blur", "layers": [f.blue] }), &Origin::person("user:a")).unwrap();
    assert_ne!(f.doc.editor_state()["selectedProps"], saved["selectedProps"], "a person's apply selects the effect");
}

/// Within one run a layer the agent makes is selected, so the next step acts on it.
#[test]
fn a_layer_made_in_a_run_is_what_the_next_step_acts_on() {
    let mut f = fixture();
    let saved = f.doc.editor_state();
    f.doc.set_context(&context(&[], Some(f.other), None)).unwrap();
    let made = agent(&mut f.doc, "layer.newSolid", json!({ "name": "Card", "color": "#ffffff" })).unwrap()["layer"].as_u64().unwrap();
    agent(&mut f.doc, "layer.setSwitch", json!({ "switch": "shy", "value": true })).unwrap();
    f.doc.restore_editor_state(&saved).unwrap();
    assert_eq!(shy(&mut f.doc, f.other), vec![made]);
    assert_eq!(f.doc.editor_state(), saved);
}

const GUIDE: &str = include_str!("../../../agent/guide.md");
/// The project the guide's examples 4 and 5 and the eval case `two-comps-one-run` edit.
const PROMO: &[u8] = include_bytes!("../../../cases/motion.agent.context/two-comps-one-run/input/promo.kmotion");

/// The steps of the `motion_run` calls in the guide section that starts with `heading`, in order.
fn guide_steps(heading: &str) -> Vec<Value> {
    let start = GUIDE.find(heading).unwrap_or_else(|| panic!("no section {heading}"));
    let section = &GUIDE[start..];
    let section = &section[..section[3..].find("\n## ").map_or(section.len(), |n| n + 3)];
    let mut out = vec![];
    let mut rest = section;
    while let Some(at) = rest.find("motion_run {") {
        let json = &rest[at + "motion_run ".len()..];
        let mut values = serde_json::Deserializer::from_str(json).into_iter::<Value>();
        let run = values.next().unwrap().unwrap_or_else(|e| panic!("the guide's motion_run is not JSON: {e}"));
        out.extend(run["steps"].as_array().unwrap().iter().cloned());
        rest = &json[values.byte_offset()..];
    }
    out
}

/// Runs `steps` as kition-kits does: `on` into the command's `targets` (else the selection),
/// `at.part` and `at.time` set before the step, the editor state put back after the run.
fn run_steps(doc: &mut MotionDocument, steps: &[Value]) {
    let commands = doc.commands();
    let saved = doc.editor_state();
    for step in steps {
        let command = step["command"].as_str().unwrap();
        let mut params = step.get("params").cloned().unwrap_or(json!({}));
        let mut on = step.get("on").and_then(Value::as_array).cloned().unwrap_or_default();
        if let Some(name) = commands.iter().find(|known| known.id == command).and_then(|known| known.targets)
            && !on.is_empty()
        {
            params[name] = if name == "layers" { Value::Array(on.clone()) } else { on[0].clone() };
            on.clear();
        }
        let at = step.get("at").cloned().unwrap_or(json!({}));
        doc.set_context(&CallContext { on, part: at.get("part").cloned(), time: at.get("time").and_then(Value::as_f64) }).unwrap();
        agent(doc, command, params).unwrap_or_else(|error| panic!("{step}: {error}"));
    }
    doc.restore_editor_state(&saved).unwrap();
}

/// The guide's examples 4 and 5 run as written on the project they name: the key lands at 2 s of
/// comp time on a layer whose own time starts at 1 s, Outro's Logo is at 50 %, and the comp open
/// and its current time are the person's.
#[test]
fn the_guides_context_examples_run_as_written() {
    for heading in ["## Example 4", "## Example 5"] {
        let mut doc = MotionDocument::open("Promo.kmotion", PROMO).unwrap();
        doc.execute("time.set", json!({ "time": 0.5 })).unwrap();
        let before = seen(&doc);
        run_steps(&mut doc, &guide_steps(heading));
        assert_eq!(seen(&doc), before, "{heading}");
        let title = doc.execute("prop.get", json!({ "comp": 1, "layer": 27, "path": "transform/opacity" })).unwrap();
        assert_eq!((title["keys"][0]["compTime"].clone(), title["keys"][0]["value"].clone()), (json!(2.0), json!(0.0)), "{heading}: {title}");
        if heading.ends_with('5') {
            let logo = doc.execute("prop.get", json!({ "comp": 66, "layer": 68, "path": "transform/opacity" })).unwrap();
            assert_eq!(logo["value"], json!(50.0), "{logo}");
        }
    }
}
