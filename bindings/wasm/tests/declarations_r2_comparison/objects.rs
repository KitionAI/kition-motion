//! Object-class comparison of every frozen r2 path against what the guard holds for that path.
//!
//! `objects.json` translates each frozen row (never the declarations) into the classes of objects
//! its result depends on: the objects its own parameters name (`target`: none, fields or whole),
//! the composition's own attributes (`comp`), other layers, other project items, project settings
//! and the render queue. Host inputs (files, models, workers) are recorded but never compared: no
//! premise can hold them. A path is run on one fixture with the translated parameters; what the
//! guard holds is the targets and object parameters it names (whole unless `under` narrows them),
//! the part when the step names no object, and every premise, window and object `under` answers.
//!
//! The comparison is coarse on purpose: a class required and not held at all is held LESS, a class
//! held and not required (or a whole target where the path reads fields) is held MORE. Two paths
//! equal at class level are NOT certified equal field by field; they are counted separately.
use super::super::families;
use kition_kit::{CallContext, KitDocument, Under};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const COMP: u64 = 1;
const COMP2: u64 = 219;
const ITEMS: &[u64] = &[2, 3, 195, COMP2];

fn id(answer: &Value, key: &str) -> u64 {
    answer[key].as_u64().unwrap_or_else(|| panic!("{key} in {answer}"))
}

/// The fixture `objects.json` names by placeholder; its ids are asserted, not assumed.
pub(super) fn fixture() -> Vec<u8> {
    let mut d = MotionDocument::create("Fixture.kmotion", 320, 180).unwrap();
    d.execute("comp.settings", json!({"name":"Main","duration":10,"frameRate":30})).unwrap();
    let a = id(&d.execute("layer.newSolid", json!({"name":"Solid A","color":"#ff0000"})).unwrap(), "layer");
    let effect = d.execute("effect.apply", json!({"effect":"Gaussian Blur","layers":[a]})).unwrap()["effects"][0].as_u64().unwrap();
    let mask = id(&d.execute("mask.new", json!({"layer":a,"vertices":[[0,0],[100,0],[100,100]],"closed":true})).unwrap(), "mask");
    for (time, value) in [(0, 10), (1, 50), (2, 90)] {
        d.execute("prop.addKey", json!({"layer":a,"path":"transform/opacity","time":time,"value":value})).unwrap();
    }
    let b = id(&d.execute("layer.newText", json!({"text":"Title"})).unwrap(), "layer");
    let n = id(&d.execute("layer.newNull", json!({"name":"Null"})).unwrap(), "layer");
    d.execute("layer.setParent", json!({"layers":[b],"parent":n})).unwrap();
    let s = id(&d.execute("layer.newShape", json!({"kind":"rect","name":"Shape"})).unwrap(), "layer");
    let k = id(&d.execute("layer.newCamera", json!({"name":"Camera"})).unwrap(), "layer");
    let g = id(&d.execute("layer.newLight", json!({"name":"Light"})).unwrap(), "layer");
    d.execute("comp.addMarker", json!({"time":1,"comment":"Beat"})).unwrap();
    let render = id(&d.execute("renderQueue.add", json!({"comp":COMP})).unwrap(), "id");
    let inner = id(&d.execute("layer.newSolid", json!({"name":"Inner"})).unwrap(), "layer");
    let pre = d.execute("layer.precompose", json!({"layers":[inner],"name":"Pre","mode":"move"})).unwrap();
    assert_eq!((a, effect, mask, b, n, s, k, g, render), (4, 27, 31, 36, 75, 87, 133, 165, 194));
    assert_eq!((pre["comp"].as_u64(), pre["layer"].as_u64()), (Some(COMP2), Some(220)));
    let bytes = d.save(None).unwrap();
    let saved: Value = serde_json::from_slice(&bytes).unwrap();
    // Solid A's Masks, Effects and Transform groups by uid and place, and the title's Animators
    // group, which the field translations name (`{masks}`, `props.children.1`, `{animators}`).
    let layer_of = |id: u64| saved["items"][COMP.to_string()]["kind"]["layers"].as_array().unwrap().iter().find(|layer| layer["id"] == id).unwrap().clone();
    let solid = layer_of(a);
    let children = solid["props"]["children"].as_array().unwrap();
    for (index, (name, uid)) in [("masks", MASKS), ("effects", EFFECTS), ("transform", TRANSFORM)].into_iter().enumerate() {
        assert_eq!((children[index]["match"].as_str(), children[index]["uid"].as_u64()), (Some(name), Some(uid)), "Solid A's group {index}");
    }
    let title = layer_of(b);
    let text = &title["props"]["children"][0];
    let animators = text["children"].as_array().unwrap().iter().find(|node| node["match"] == "animators").unwrap();
    assert_eq!((text["match"].as_str(), animators["uid"].as_u64()), (Some("text"), Some(ANIMATORS)), "the title's Animators group");
    bytes
}

const MASKS: u64 = 6;
const EFFECTS: u64 = 7;
const TRANSFORM: u64 = 16;
const ANIMATORS: u64 = 53;

const PLACEHOLDERS: &[(&str, u64)] = &[
    ("{comp}", COMP),
    ("{comp2}", COMP2),
    ("{folder}", 2),
    ("{solidItem}", 3),
    ("{A}", 4),
    ("{B}", 36),
    ("{N}", 75),
    ("{S}", 87),
    ("{K}", 133),
    ("{G}", 165),
    ("{P}", 220),
    ("{effect}", 27),
    ("{mask}", 31),
    ("{opacity}", 15),
    ("{position}", 9),
    ("{render}", 194),
    ("{masks}", MASKS),
    ("{effects}", EFFECTS),
    ("{animators}", ANIMATORS),
];

/// A whole-string placeholder becomes its number; a placeholder inside a longer string its digits.
fn substitute(value: &Value) -> Value {
    match value {
        Value::String(text) => {
            if let Some((_, number)) = PLACEHOLDERS.iter().find(|(name, _)| name == text) {
                return json!(number);
            }
            let mut text = text.clone();
            for (name, number) in PLACEHOLDERS {
                text = text.replace(name, &number.to_string());
            }
            Value::String(text)
        }
        Value::Array(items) => Value::Array(items.iter().map(substitute).collect()),
        Value::Object(map) => Value::Object(map.iter().map(|(key, value)| (key.clone(), substitute(value))).collect()),
        other => other.clone(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Depth {
    None,
    Fields,
    Whole,
}

fn depth(text: &str) -> Option<Depth> {
    match text {
        "none" => Some(Depth::None),
        "fields" => Some(Depth::Fields),
        "whole" => Some(Depth::Whole),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct Held {
    /// The step names an object the guard reads as one (a target, an object parameter, a place).
    named: bool,
    /// The step's object parameters name the composition / a project item.
    names_comp: bool,
    names_item: bool,
    target: Option<Depth>,
    comp: Option<Depth>,
    classes: BTreeSet<&'static str>,
    /// The fields premises on a target name (a field-level hold), for the field comparison.
    target_fields: BTreeSet<String>,
}

fn numbers(value: Option<&Value>, out: &mut BTreeSet<u64>) {
    match value {
        Some(Value::Number(number)) => out.extend(number.as_u64()),
        Some(Value::Array(items)) => items.iter().for_each(|item| numbers(Some(item), out)),
        _ => {}
    }
}

thread_local! {
    /// Project items a probe's setup created (a placeholder footage, a template's folder): ids
    /// the fixed `ITEMS` list cannot name, classed as items rather than as other layers.
    static PROBE_ITEMS: std::cell::RefCell<BTreeSet<u64>> = const { std::cell::RefCell::new(BTreeSet::new()) };
}

fn class_of(id: &Value, targets: &BTreeSet<u64>) -> &'static str {
    if id.as_str().is_some_and(|text| text == "project") {
        return "project";
    }
    if id.as_str().is_some_and(|text| text.starts_with("render:")) {
        return "queue";
    }
    let Some(number) = id.as_u64().or_else(|| id.as_str()?.parse().ok()) else { return "unknown" };
    // The composition and project items keep their own class when a parameter names them.
    if number == COMP {
        "comp"
    } else if ITEMS.contains(&number) || PROBE_ITEMS.with(|items| items.borrow().contains(&number)) {
        "items"
    } else if targets.contains(&number) {
        "target"
    } else {
        "layers"
    }
}

fn deeper(slot: &mut Option<Depth>, depth: Depth) {
    *slot = Some(slot.map_or(depth, |old| old.max(depth)));
}

/// What the guard holds for one step: its named targets and object parameters (whole unless a
/// premise narrows them), the part when it names no object, and `under`'s answer.
fn held(command: &kition_kit::KitCommand, params: &Value, selection: &[u64], answer: &Under) -> Held {
    let mut targets = BTreeSet::new();
    if let Some(name) = command.targets {
        match params.get(name) {
            Some(value) => numbers(Some(value), &mut targets),
            // An agent acts on a selection by `on`, which the host writes into this parameter.
            None => targets.extend(selection.iter().copied()),
        }
    }
    for name in command.objects {
        let name = name.trim_end_matches("[]");
        if let Some((list, member)) = name.split_once("[].") {
            for item in params.get(list).and_then(Value::as_array).into_iter().flatten() {
                numbers(item.get(member), &mut targets);
            }
        } else {
            numbers(params.get(name), &mut targets);
        }
    }
    targets.extend(answer.objects.iter().filter_map(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok())));
    let mut held = Held {
        named: targets.iter().any(|id| *id != COMP && !ITEMS.contains(id)),
        names_comp: targets.contains(&COMP),
        names_item: targets.iter().any(|id| ITEMS.contains(id)),
        ..Default::default()
    };
    let mut narrowed = BTreeSet::new();
    for premise in &answer.premises {
        let class = class_of(&premise.id, &targets);
        let fields: Option<Vec<&String>> = premise.fields.as_ref().map(|fields| fields.iter().collect());
        if let Some(number) = premise.id.as_u64().or_else(|| premise.id.as_str()?.parse().ok()) {
            narrowed.insert(number);
        }
        match class {
            "target" => {
                deeper(&mut held.target, if fields.is_some() { Depth::Fields } else { Depth::Whole });
                held.target_fields.extend(fields.iter().flatten().map(|field| (*field).clone()));
            }
            "comp" => {
                let mut derived = false;
                let mut own = false;
                match &fields {
                    None => own = true,
                    Some(fields) => {
                        for field in fields {
                            match field.as_str() {
                                // `Project::comp(id)` is true only for a composition: holding the
                                // kind can refuse nothing but the composition's deletion.
                                "kind" => {}
                                // Item.parent is folder ancestry (project_items.rs:34-50),
                                // rather than an attribute of the composition it owns.
                                "parent" => {
                                    held.classes.insert("items");
                                }
                                "~layerMembers" => derived = true,
                                _ => own = true,
                            }
                        }
                    }
                }
                if derived {
                    held.classes.insert("layers");
                }
                if own {
                    deeper(&mut held.comp, if fields.is_some() { Depth::Fields } else { Depth::Whole });
                }
            }
            other => {
                held.classes.insert(other);
            }
        }
    }
    for number in &targets {
        if narrowed.contains(number) {
            continue;
        }
        match class_of(&json!(number), &targets) {
            "comp" => deeper(&mut held.comp, Depth::Whole),
            "target" => deeper(&mut held.target, Depth::Whole),
            class => {
                held.classes.insert(class);
            }
        }
    }
    if command.reads.contains(&kition_kit::EditorState::Part)
        && targets.is_empty()
        && !answer.premises.iter().any(|premise| class_of(&premise.id, &targets) == "comp")
    {
        // Only a command that reads Part has an implicit part in this probe. There is no
        // explicit run target/at.part here (SDK guard::Premises::of_run, guard.rs:552-570).
        deeper(&mut held.comp, Depth::Whole);
    }
    for window in &answer.within {
        let class = match (class_of(&window.parent, &targets), window.field.as_str()) {
            ("comp", "comp.layers") => "layers",
            ("comp", _) => {
                deeper(&mut held.comp, Depth::Fields);
                continue;
            }
            ("project", "items") => "items",
            ("project", "renderQueue") => "queue",
            ("target", _) => {
                deeper(&mut held.target, Depth::Fields);
                continue;
            }
            (other, _) => other,
        };
        held.classes.insert(class);
    }
    // A content premise reads a region of a field of its object (one effect, one marker).
    for content in &answer.content {
        match class_of(&content.id, &targets) {
            "comp" => deeper(&mut held.comp, Depth::Fields),
            "target" => {
                deeper(&mut held.target, Depth::Fields);
                held.target_fields.insert(content.field.clone());
            }
            class => {
                held.classes.insert(class);
            }
        }
    }
    if answer.covers.is_some() {
        held.classes.insert("layers");
    }
    held
}

const CLASSES: &[&str] = &["layers", "items", "project", "queue"];

#[derive(Default)]
struct Verdict {
    less: Vec<String>,
    more: Vec<String>,
    unresolved: Option<String>,
    /// Field level, for a class-equal path whose row is translated to fields: the required fields
    /// no held field covers, and the held fields no required field relates to.
    fields: Option<(Vec<String>, Vec<String>)>,
}

/// Whether field path `inner` is `outer` or lies inside it (`a.b` and `a.keys[time=0]` in `a`).
fn within(inner: &str, outer: &str) -> bool {
    inner == outer || inner.strip_prefix(outer).is_some_and(|rest| rest.starts_with('.') || rest.starts_with('['))
}

/// The field-level comparison of the target (`fields.target` in `objects.json`): the row's reads
/// and writes of its target, spelled in the object model's paths.
fn compare_fields(requirement: &Value, held: &Held) -> Option<(Vec<String>, Vec<String>)> {
    let required: Vec<String> =
        substitute(requirement.get("fields")?.get("target")?).as_array()?.iter().filter_map(|field| Some(field.as_str()?.to_owned())).collect();
    // A required field is held by itself, an enclosing field, or a selector inside it (the one item
    // the handler reads); a held field enclosing more than any required field is held MORE.
    let less = required.iter().filter(|field| !held.target_fields.iter().any(|held| within(field, held) || within(held, field))).cloned().collect();
    let more = held.target_fields.iter().filter(|held| !required.iter().any(|field| within(held, field))).cloned().collect();
    Some((less, more))
}

fn compare(command: &str, requirement: &Value, held: &Held) -> Verdict {
    let mut verdict = Verdict::default();
    let note = requirement["note"].as_str().unwrap_or_default();
    let (Some(mut target), Some(mut comp)) = (requirement["target"].as_str().and_then(depth), requirement["comp"].as_str().and_then(depth)) else {
        verdict.unresolved = Some("translation gives no target/comp class".to_owned());
        return verdict;
    };
    if requirement["dynamic"].as_bool() == Some(true) {
        verdict.unresolved = Some(format!("dynamic dependency closure (scripts, nested steps, pending jobs): {note}"));
        return verdict;
    }
    if note.to_ascii_lowercase().contains("uncertain") && requirement["resolution"].is_null() {
        verdict.unresolved = Some(format!("translation uncertain: {note}"));
        return verdict;
    }
    let mut requirement = requirement.clone();
    // A step whose parameters name no object the guard reads as one (no declared target or object
    // parameter): what the row calls its target is the object a non-object parameter names, which
    // the guard holds in that object's own class (a named composition or project item, or through
    // `under`). Render queue entries are
    // queue, project items are items, the composition is comp; anything else (selected keys'
    // layers) is other layers. The row's depth carries over to the composition.
    if !held.named && target != Depth::None {
        let class = if held.names_comp {
            None
        } else if held.names_item {
            Some("items")
        } else if command.starts_with("renderQueue.") || command.starts_with("render.") {
            Some("queue")
        } else if ["footage.", "file.", "project."].iter().any(|prefix| command.starts_with(prefix)) {
            Some("items")
        } else if command.starts_with("comp.") || command.starts_with("view.") || command.starts_with("templateProps.") {
            None
        } else {
            Some("layers")
        };
        match class {
            Some(class) => requirement[class] = json!(true),
            None => comp = comp.max(target),
        }
        target = Depth::None;
    }
    for (name, required, actual) in [("target", target, held.target.unwrap_or(Depth::None)), ("comp", comp, held.comp.unwrap_or(Depth::None))] {
        match (required, actual) {
            (Depth::Fields | Depth::Whole, Depth::None) => verdict.less.push(format!("{name}:unheld")),
            (Depth::Whole, Depth::Fields) => verdict.less.push(format!("{name}:fields-of-whole")),
            (Depth::None, Depth::Fields | Depth::Whole) => verdict.more.push(format!("{name}:unread")),
            (Depth::Fields, Depth::Whole) => verdict.more.push(format!("{name}:whole-for-fields")),
            _ => {}
        }
    }
    for class in CLASSES {
        match (requirement[*class].as_bool(), held.classes.contains(class)) {
            (Some(true), false) => verdict.less.push(format!("{class}:unheld")),
            (Some(false), true) => verdict.more.push(format!("{class}:unread")),
            (None, _) => verdict.unresolved = Some(format!("translation gives no {class} class")),
            _ => {}
        }
    }
    verdict
}

pub(super) fn analyze() -> Value {
    let requirements: Vec<Value> = serde_json::from_str(include_str!("objects.json")).unwrap();
    let by_id: BTreeMap<String, &Value> = requirements.iter().map(|value| (value["id"].as_str().unwrap().to_owned(), value)).collect();
    let bytes = fixture();
    let commands: BTreeMap<String, kition_kit::KitCommand> =
        MotionDocument::open("Fixture.kmotion", &bytes).unwrap().commands().into_iter().map(|command| (command.id.clone(), command)).collect();
    let mut report = Vec::new();
    let mut totals = BTreeMap::<&str, usize>::new();
    let mut classes = BTreeMap::<String, BTreeSet<String>>::new();
    let mut differing_commands = BTreeSet::new();
    let mut compared_commands = BTreeSet::new();
    for (family, rows) in families() {
        for (index, row) in rows.iter().enumerate() {
            let id = format!("{family}/{index}");
            let requirement = by_id.get(&id).copied();
            let mut entry = json!({"id":id,"command":row.command,"path":row.path});
            let verdict = match requirement {
                None => Verdict { unresolved: Some("no translation".to_owned()), ..Default::default() },
                Some(requirement) if requirement["command"].as_str() != Some(row.command) || requirement["path"].as_str() != Some(row.path) => {
                    Verdict { unresolved: Some("translation names another row".to_owned()), ..Default::default() }
                }
                Some(requirement) if requirement["params"].is_null() => {
                    Verdict { unresolved: Some(format!("not reachable on the fixture: {}", requirement["note"])), ..Default::default() }
                }
                Some(requirement) => match commands.get(row.command) {
                    None => Verdict { unresolved: Some("not an advertised command".to_owned()), ..Default::default() },
                    Some(command) => probe(&bytes, command, requirement, &mut entry),
                },
            };
            let status = if verdict.unresolved.is_some() {
                "unresolved"
            } else if verdict.less.is_empty() && verdict.more.is_empty() {
                compared_commands.insert(row.command);
                "class-equal"
            } else {
                compared_commands.insert(row.command);
                differing_commands.insert(row.command);
                "differing"
            };
            *totals.entry(status).or_default() += 1;
            for (kind, list) in [("less", &verdict.less), ("more", &verdict.more)] {
                if !list.is_empty() {
                    *totals.entry(if kind == "less" { "less" } else { "more" }).or_default() += 1;
                }
                for class in list {
                    classes.entry(format!("{kind} {class}")).or_default().insert(id.clone());
                }
            }
            if status == "class-equal" {
                if let Some((less, more)) = &verdict.fields {
                    let level = if less.is_empty() && more.is_empty() { "field-equal" } else { "field-differing" };
                    *totals.entry(level).or_default() += 1;
                    if !less.is_empty() {
                        *totals.entry("field-less").or_default() += 1;
                    }
                    entry["fields"] = json!({"less":less,"more":more});
                } else {
                    *totals.entry("field-untranslated").or_default() += 1;
                }
            }
            entry["status"] = json!(status);
            entry["less"] = json!(verdict.less);
            entry["more"] = json!(verdict.more);
            entry["unresolved"] = json!(verdict.unresolved);
            report.push(entry);
        }
    }
    json!({
        "paths":report,"totals":totals,"classes":classes,
        "commands":{"compared":compared_commands.len(),"differing":differing_commands.len()},
        "interpretation":"Class-level only: class-equal paths are not field-level certified."
    })
}

fn probe(bytes: &[u8], command: &kition_kit::KitCommand, requirement: &Value, entry: &mut Value) -> Verdict {
    let mut document = MotionDocument::open("Fixture.kmotion", bytes).unwrap();
    let context = substitute(&requirement["context"]);
    let time = context.get("time").and_then(Value::as_f64).unwrap_or(1.0);
    document.set_context(&CallContext { part: Some(json!({"comp":COMP})), time: Some(time), on: Vec::new() }).unwrap();
    for step in requirement["setup"].as_array().into_iter().flatten() {
        let step = substitute(step);
        if let Err(error) = document.execute(step[0].as_str().unwrap_or_default(), step[1].clone()) {
            return Verdict { unresolved: Some(format!("setup {step} failed: {error}")), ..Default::default() };
        }
    }
    let mut selection = BTreeSet::new();
    numbers(Some(context.get("selection").unwrap_or(&json!([4]))), &mut selection);
    let selection: Vec<u64> = selection.into_iter().collect();
    // A plain layer.select clears the selected keys, properties and vertices a setup step chose
    // (layer.rs:325-330); keep them, so a path acting on the selected keys is probed with them.
    let keeps = requirement["setup"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|step| matches!(step[0].as_str(), Some("keyframe.select" | "prop.select" | "mask.selectVertices")));
    if !selection.is_empty() {
        let _ = document.execute("layer.select", json!({"layers":selection,"add":keeps}));
    }
    let saved: Value = serde_json::from_slice(&document.save(None).unwrap()).unwrap();
    let created: BTreeSet<u64> = saved["items"].as_object().into_iter().flatten().filter_map(|(id, _)| id.parse().ok()).filter(|id| *id > COMP2).collect();
    PROBE_ITEMS.with(|items| *items.borrow_mut() = created);
    let params = substitute(&requirement["params"]);
    entry["params"] = params.clone();
    let answer = match document.under(&command.id, &params) {
        Ok(answer) => answer.unwrap_or_default(),
        Err(error) => return Verdict { unresolved: Some(format!("under refused the probe: {error}")), ..Default::default() },
    };
    entry["under"] = json!(super::facts(Some(answer.clone())));
    let held = held(command, &params, &selection, &answer);
    entry["held"] = json!({"target":format!("{:?}", held.target),"comp":format!("{:?}", held.comp),"classes":held.classes});
    let mut verdict = compare(&command.id, requirement, &held);
    if held.target == Some(Depth::Fields) {
        verdict.fields = compare_fields(requirement, &held);
    }
    verdict
}

#[test]
fn every_frozen_path_is_compared_by_object_class() {
    let report = analyze();
    if let Ok(path) = std::env::var("MOTION_R2_OBJECTS_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("R2 OBJECTS {} commands {}", report["totals"], report["commands"]);
    for (class, ids) in report["classes"].as_object().unwrap() {
        println!("R2 OBJECTS {class}: {} paths", ids.as_array().unwrap().len());
    }
    assert_eq!(report["paths"].as_array().unwrap().len(), 652);
    // Only-decreasing gate: no path may gain a held-LESS or held-MORE class, or become unresolved,
    // that it did not have when the comparison was first measured. Fixing a class removes it.
    let baseline: Value = serde_json::from_str(include_str!("objects_baseline.json")).unwrap();
    let mut grown = Vec::new();
    for path in report["paths"].as_array().unwrap() {
        let id = path["id"].as_str().unwrap();
        for kind in ["less", "more"] {
            let allowed: BTreeSet<&str> = baseline[kind][id].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            for class in path[kind].as_array().unwrap().iter().filter_map(Value::as_str) {
                if !allowed.contains(class) {
                    grown.push(format!("{id} {kind} {class}"));
                }
            }
        }
        if path["status"] == "unresolved" && !baseline["unresolved"].as_array().unwrap().iter().any(|known| known == id) {
            grown.push(format!("{id} unresolved"));
        }
    }
    assert!(grown.is_empty(), "the comparison grew: {grown:?}");
    // Field level: a translated class-equal path holds every field its row reads or assigns.
    let short: Vec<String> = report["paths"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|path| path["fields"]["less"].as_array().is_some_and(|less| !less.is_empty()))
        .map(|path| format!("{} {}", path["id"], path["fields"]["less"]))
        .collect();
    assert!(short.is_empty(), "field-level held LESS: {short:?}");
}

#[test]
fn an_independent_project_guard_does_not_invent_a_composition_read() {
    let command = kition_kit::KitCommand { id: "render.backend".to_owned(), ..Default::default() };
    let answer = Under {
        premises: vec![kition_kit::Premise::on("project").fields(["attrs.settings.gpu_acceleration"]).positions(Vec::<String>::new())],
        ..Default::default()
    };
    let actual = held(&command, &json!({"backend":"gpu"}), &[], &answer);
    assert_eq!(actual.comp, None);
    assert!(actual.classes.contains("project"));
}

#[test]
fn a_composition_items_folder_parent_belongs_to_project_items() {
    let command = kition_kit::KitCommand { id: "project.delete".to_owned(), ..Default::default() };
    let answer = Under { premises: vec![kition_kit::Premise::on(json!(COMP)).fields(["parent"]).positions(Vec::<String>::new())], ..Default::default() };
    let actual = held(&command, &json!({"items":[3]}), &[], &answer);
    assert_eq!(actual.comp, None);
    assert!(actual.classes.contains("items"));
}
