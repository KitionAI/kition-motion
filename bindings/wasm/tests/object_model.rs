//! The object model is reachable from its root (KHP A16): `model.describe` names
//! `Project.items`, an agent's `query` with `follow` walks from the root to a composition, a layer
//! of it and the layer's name, and a `batch` that renames the layer reads back through `query`
//! and survives save and reopen. This runs the object model the way `kition-motion` serves it
//! headless (`kition_kit::session::model::Exchange` over the document's change exchange); `follow`
//! is the agent tools' own walk (a field path from the root, positions counted from 0).
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;

use kition_kit::KitDocument;
use kition_kit::kdm::{ObjectId, Replica};
use kition_kit::session::model::Exchange;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

const BASIC: &[u8] = include_bytes!("../../../conformance/basic.kmotion");

fn load(exchange: &mut Exchange, doc: &mut MotionDocument, target: &str, fields: Option<&[&str]>) -> Value {
    let entry = match fields {
        Some(fields) => json!({ "target": target, "fields": fields }),
        None => json!({ "target": target }),
    };
    let answer = exchange.batch(doc, &json!({ "ops": [], "load": [entry] }), false).unwrap_or_else(|error| panic!("load {target}: {error}"));
    answer["loaded"][0].clone()
}

/// The ids a loaded field names: a list's items or a ref.
fn ids_of(value: &Value) -> Vec<String> {
    match value {
        Value::String(id) => vec![id.clone()],
        Value::Array(items) => items.iter().map(|item| item.as_str().unwrap_or_else(|| panic!("{value} lists ids")).to_owned()).collect(),
        other => panic!("{other} names no objects"),
    }
}

/// `query { follow }`: from the root, field names and list positions joined with dots.
fn follow(exchange: &mut Exchange, doc: &mut MotionDocument, path: &str) -> Vec<String> {
    let mut current = vec!["root".to_owned()];
    for segment in path.split('.') {
        if let Ok(position) = segment.parse::<usize>() {
            current = vec![current.get(position).cloned().unwrap_or_else(|| panic!("`{path}`: no item {position} of {current:?}"))];
            continue;
        }
        let [object] = current.as_slice() else { panic!("`{path}`: `{segment}` follows {} objects", current.len()) };
        let loaded = load(exchange, doc, object, Some(&[segment]));
        current = ids_of(&loaded["fields"][segment]);
    }
    current
}

fn layer_names(doc: &MotionDocument) -> Vec<String> {
    doc.inspect().unwrap()["info"]["layers"].as_array().unwrap().iter().map(|layer| layer["name"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn describe_names_the_root_items_list() {
    let mut doc = MotionDocument::open("basic.kmotion", BASIC).unwrap();
    let described = Exchange::new().describe(&mut doc).unwrap();
    assert_eq!(described["schemaVersion"], 1);
    assert_eq!(described["root"], "Project");
    let items = &described["types"]["Project"]["fields"]["items"];
    assert_eq!(items["kind"], "list", "{items}");
    assert_eq!(items["of"], "Item", "{items}");
    assert_eq!(items["sample"], false, "membership is a query view");
}

#[test]
fn follow_from_the_root_reaches_a_comp_a_layer_and_its_name() {
    let mut doc = MotionDocument::open("basic.kmotion", BASIC).unwrap();
    let mut exchange = Exchange::new();
    let items = follow(&mut exchange, &mut doc, "items");
    assert_eq!(items.len(), 4, "Comp 1, the Solids folder and two solids");
    let comp = load(&mut exchange, &mut doc, &items[0], None);
    assert_eq!(comp["type"], "Item");
    assert_eq!((comp["fields"]["kind"].as_str(), comp["fields"]["name"].as_str()), (Some("comp"), Some("Comp 1")));

    let layers = follow(&mut exchange, &mut doc, "items.0.comp.layers");
    assert_eq!(layers.len(), 2);
    let top = load(&mut exchange, &mut doc, &layers[0], None);
    assert_eq!(top["type"], "Layer");
    assert_eq!(top["fields"]["name"], "Band", "the top layer, a string field of its own");

    // Further down: the layer's property tree and a property of it.
    let groups = follow(&mut exchange, &mut doc, "items.0.comp.layers.0.props.children");
    assert!(!groups.is_empty());
    let render = follow(&mut exchange, &mut doc, "renderQueue");
    assert!(render.is_empty(), "basic.kmotion has no Render Queue entry");
}

#[test]
fn a_batch_rename_reads_back_and_survives_save_and_reopen() {
    let mut doc = MotionDocument::open("basic.kmotion", BASIC).unwrap();
    let mut exchange = Exchange::new();
    let layer = follow(&mut exchange, &mut doc, "items.0.comp.layers.0").remove(0);
    let item = follow(&mut exchange, &mut doc, "items.0").remove(0);
    let ops = json!({ "ops": [
        { "op": "set", "target": layer, "field": "name", "value": "Headline band" },
        { "op": "set", "target": item, "field": "name", "value": "Opening" }
    ] });
    exchange.batch(&mut doc, &ops, false).unwrap();

    assert_eq!(load(&mut exchange, &mut doc, &layer, Some(&["name"]))["fields"]["name"], "Headline band");
    assert_eq!(load(&mut Exchange::new(), &mut doc, &layer, Some(&["name"]))["fields"]["name"], "Headline band", "a fresh reader too");
    assert_eq!(layer_names(&doc), ["Headline band", "Background"], "the engine has it");

    let saved = doc.save(None).unwrap();
    let mut reopened = MotionDocument::open("basic.kmotion", &saved).unwrap();
    assert_eq!(layer_names(&reopened), ["Headline band", "Background"]);
    let mut exchange = Exchange::new();
    let layer = follow(&mut exchange, &mut reopened, "items.0.comp.layers.0").remove(0);
    assert_eq!(load(&mut exchange, &mut reopened, &layer, Some(&["name"]))["fields"]["name"], "Headline band");
    let item = follow(&mut exchange, &mut reopened, "items.0").remove(0);
    assert_eq!(load(&mut exchange, &mut reopened, &item, Some(&["name"]))["fields"]["name"], "Opening");
}

#[test]
fn deleting_an_item_leaves_no_dangling_entry() {
    let mut doc = MotionDocument::open("basic.kmotion", BASIC).unwrap();
    let mut exchange = Exchange::new();
    let items = follow(&mut exchange, &mut doc, "items");
    // Through the object model: the Band solid (the top layer's source).
    let band = items.iter().find(|id| load(&mut exchange, &mut doc, id, Some(&["name"]))["fields"]["name"] == "Band").unwrap().clone();
    exchange.batch(&mut doc, &json!({ "ops": [{ "op": "delete", "target": band }] }), false).unwrap();
    let after = follow(&mut exchange, &mut doc, "items");
    assert_eq!(after.len(), 3);
    assert!(!after.contains(&band));
    assert_eq!(layer_names(&doc), ["Background"], "the layer using the deleted item went with it");

    // Through a command: the Solids folder.
    let folder = after.iter().find(|id| load(&mut exchange, &mut doc, id, Some(&["kind"]))["fields"]["kind"] == "folder").unwrap().clone();
    doc.execute("project.delete", json!({ "items": [2] })).unwrap();
    let after = follow(&mut exchange, &mut doc, "items");
    assert!(!after.contains(&folder), "{after:?}");
    for id in &after {
        assert_eq!(load(&mut exchange, &mut doc, id, Some(&["kind"]))["type"], "Item", "{id} is live");
    }
}

/// Every project the repository's cases and conformance checks open still opens, and its replica
/// lists each of its items once, from the root.
#[test]
fn every_fixture_opens_with_every_item_listed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let mut dirs = vec![root.join("conformance"), root.join("cases")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "kmotion") {
                files.push(path);
            }
        }
    }
    assert!(files.len() >= 20, "{files:?}");
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let mut doc = MotionDocument::open(&name, &bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let snapshot = doc.snapshot().unwrap();
        let schema = <kdm_model::MotionModel as kition_kit::session::KitModel>::schema();
        let replica = Replica::load(schema, &snapshot, kition_kit::kdm::ActorId::from_u64(9).unwrap()).unwrap();
        let listed = replica.list(&ObjectId::from_string("project"), "items").unwrap();
        let project: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(listed.len(), project["items"].as_object().unwrap().len(), "{}", path.display());
        assert_eq!(listed.len(), replica.objects_of_type("Item").len(), "{}", path.display());
    }
}
