//! Kition Motion against the Kition platform convergence suite (L6): two to five replicas of a
//! project, edited through the engine's own commands (contract 1 `execute`, the contract 2 change
//! exchange, `open_with_replica` to join). The suite checks after every scenario that every replica
//! holds the same state and that the schema's references are valid: every `list` (now including
//! `Project.items`) holds live objects once each, and every `ref` names a live object.
//!
//! What the suite's steps are in a project:
//! - the probe is the active composition's top layer; `SetField` slots 0 and 1 are its name
//!   (`layer.rename`, the layer's `name` field) and its label (`edit.label`, a key of its `attrs`);
//! - `Create`, `Delete`, `Duplicate` are layers (`layer.newSolid`, `edit.clear`, `edit.duplicate`);
//!   `Reorder` is the stacking order (`layer.arrange`); `Reparent` is layer parenting: `which` 0
//!   parents the second layer to the third, 1 the third to the second (`layer.setParent`), so the
//!   two at once make a cycle the repair breaks;
//! - the random edits also add and delete project items and folders, rename items, and animate.
//!
//! A project has no text the suite types into and no blobs (footage is referenced by path), so
//! those scenarios are skipped as not applicable.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kdm_model::MotionModel;
use kition_convergence::{Blobs, KitAdapter, Options, Result, Rng, Script};
use kition_kit::KitDocument;
use kition_kit::kdm::{ActorId, Replica};
use kition_kit::session::KitModel;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

const NAME: &str = "Shared.kmotion";
const SCHEMA: &str = include_str!("../../../crates/kdm/kdm-model/model/schema.json");

struct MotionKit;

fn exec(doc: &mut MotionDocument, command: &str, params: Value) -> Result<Value> {
    doc.execute(command, params.clone()).map_err(|error| format!("{command} {params}: {error}"))
}

/// Runs a command; a refusal because of the project's state is no edit (`false`).
fn attempt(doc: &mut MotionDocument, command: &str, params: Value) -> bool {
    doc.execute(command, params).is_ok()
}

/// The active composition's layer ids, top first: the same on every replica with the same state.
fn layers(doc: &MotionDocument) -> Vec<u64> {
    doc.inspect()
        .map(|info| info["info"]["layers"].as_array().map(|all| all.iter().filter_map(|l| l["id"].as_u64()).collect()).unwrap_or_default())
        .unwrap_or_default()
}

/// The project's item ids of `kind` (`Solid`, `Folder`, …), from `project.summary`.
fn items(doc: &mut MotionDocument, kind: &str) -> Vec<u64> {
    let Ok(summary) = doc.execute("project.summary", json!({})) else { return Vec::new() };
    let mut out = Vec::new();
    collect_items(&summary, kind, &mut out, 0);
    out.sort_unstable();
    out.dedup();
    out
}

fn collect_items(value: &Value, kind: &str, out: &mut Vec<u64>, depth: usize) {
    if depth > 16 {
        return;
    }
    match value {
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some(kind)
                && let Some(id) = map.get("id").and_then(Value::as_u64)
            {
                out.push(id);
            }
            for child in map.values() {
                collect_items(child, kind, out, depth + 1);
            }
        }
        Value::Array(list) => list.iter().for_each(|child| collect_items(child, kind, out, depth + 1)),
        _ => {}
    }
}

fn color(value: u64) -> String {
    format!("#{:06x}", value.wrapping_mul(0x9E37_79B9) & 0xff_ffff)
}

const LABELS: [&str; 6] = ["Red", "Yellow", "Aqua", "Blue", "Green", "Purple"];

fn probe(doc: &MotionDocument) -> Result<u64> {
    layers(doc).first().copied().ok_or_else(|| "the composition has no layer".to_owned())
}

impl KitAdapter for MotionKit {
    type Doc = MotionDocument;

    fn name(&self) -> &str {
        "kit.motion"
    }

    fn schema(&self) -> &str {
        SCHEMA
    }

    /// One composition with three solid layers (their solids filed in a folder), the top one
    /// animated.
    fn create(&self) -> Result<MotionDocument> {
        let mut doc = MotionDocument::create(NAME, 64, 36)?;
        for (name, fill) in [("Back", "#203040"), ("Middle", "#c04020"), ("Top", "#20c040")] {
            exec(&mut doc, "layer.newSolid", json!({ "name": name, "color": fill }))?;
        }
        let top = probe(&doc)?;
        exec(&mut doc, "prop.addKey", json!({ "layer": top, "path": "transform/opacity", "time": 0, "value": 0 }))?;
        exec(&mut doc, "prop.addKey", json!({ "layer": top, "path": "transform/opacity", "time": 1, "value": 100 }))?;
        exec(&mut doc, "project.newFolder", json!({ "name": "Bin" }))?;
        doc.version()?;
        Ok(doc)
    }

    fn join(&self, snapshot: &[u8], _blobs: &Blobs) -> Result<MotionDocument> {
        MotionDocument::open_with_replica(NAME, &[], snapshot, &[])
    }

    fn version(&self, doc: &mut MotionDocument) -> Result<Vec<u8>> {
        doc.version()
    }

    fn export_changes(&self, doc: &mut MotionDocument, since: &[u8]) -> Result<Vec<u8>> {
        doc.export_changes(since)
    }

    fn import_changes(&self, doc: &mut MotionDocument, bytes: &[u8]) -> Result<()> {
        doc.import_changes(bytes).map(|_| ())
    }

    fn snapshot(&self, doc: &mut MotionDocument) -> Result<Vec<u8>> {
        doc.snapshot()
    }

    fn state(&self, doc: &mut MotionDocument) -> Result<Value> {
        let snapshot = doc.snapshot()?;
        let replica = Replica::load(MotionModel::schema(), &snapshot, ActorId::random()).map_err(|error| error.to_string())?;
        replica.state_json().map_err(|error| error.to_string())
    }

    fn undo(&self, doc: &mut MotionDocument) -> Result<bool> {
        doc.undo().map(|(done, _)| done)
    }

    fn redo(&self, doc: &mut MotionDocument) -> Result<bool> {
        doc.redo().map(|(done, _)| done)
    }

    fn random_edit(&self, doc: &mut MotionDocument, rng: &mut Rng) -> Result<Option<String>> {
        let all = layers(doc);
        let target = rng.pick(&all).copied();
        let value = rng.next_u64() % 10_000;
        let (label, done) = match (rng.below(12), target) {
            (0 | 1, _) => ("solid", attempt(doc, "layer.newSolid", json!({ "name": format!("S{value}"), "color": color(value) }))),
            (2, Some(id)) => ("rename", attempt(doc, "layer.rename", json!({ "layer": id, "name": format!("L{value}") }))),
            (3, Some(id)) => ("opacity", attempt(doc, "prop.set", json!({ "layer": id, "path": "transform/opacity", "value": value % 100 }))),
            (4, Some(id)) => {
                let time = (value % 30) as f64 / 10.0;
                ("key", attempt(doc, "prop.addKey", json!({ "layer": id, "path": "transform/rotation", "time": time, "value": value % 360 })))
            }
            (5, Some(id)) => {
                let to = ["front", "forward", "backward", "back"][rng.below(4)];
                ("arrange", attempt(doc, "layer.arrange", json!({ "layers": [id], "to": to })))
            }
            (6, Some(id)) if all.len() > 1 => ("clear", attempt(doc, "edit.clear", json!({ "layers": [id] }))),
            (7, Some(id)) => ("duplicate", attempt(doc, "edit.duplicate", json!({ "layers": [id] }))),
            (8, _) => ("folder", attempt(doc, "project.newFolder", json!({ "name": format!("F{value}") }))),
            (9, _) => {
                let solids = items(doc, "Solid");
                let Some(item) = rng.pick(&solids).copied() else { return Ok(None) };
                ("delete item", attempt(doc, "project.delete", json!({ "items": [item] })))
            }
            (10, _) => {
                let folders = items(doc, "Folder");
                let Some(item) = rng.pick(&folders).copied() else { return Ok(None) };
                ("rename item", attempt(doc, "project.rename", json!({ "item": item, "name": format!("I{value}") })))
            }
            (11, _) => {
                let (folders, solids) = (items(doc, "Folder"), items(doc, "Solid"));
                let (Some(folder), Some(item)) = (rng.pick(&folders).copied(), rng.pick(&solids).copied()) else { return Ok(None) };
                ("file item", attempt(doc, "project.move", json!({ "items": [item], "folder": folder })))
            }
            _ => return Ok(None),
        };
        Ok(done.then(|| label.to_owned()))
    }

    fn script(&self, doc: &mut MotionDocument, step: &Script) -> Result<bool> {
        match step {
            Script::SetField { slot: 0, value } => {
                let id = probe(doc)?;
                exec(doc, "layer.rename", json!({ "layer": id, "name": format!("Probe {value}") }))?;
                Ok(true)
            }
            Script::SetField { slot: 1, value } => {
                let id = probe(doc)?;
                let label = LABELS[*value as usize % LABELS.len()];
                exec(doc, "edit.label", json!({ "layers": [id], "label": label }))?;
                Ok(true)
            }
            Script::Create => {
                exec(doc, "layer.newSolid", json!({ "name": "Made", "color": "#808080" }))?;
                Ok(true)
            }
            Script::Delete => {
                let id = probe(doc)?;
                exec(doc, "edit.clear", json!({ "layers": [id] }))?;
                Ok(true)
            }
            Script::Duplicate => {
                let id = probe(doc)?;
                exec(doc, "edit.duplicate", json!({ "layers": [id] }))?;
                Ok(true)
            }
            Script::Reorder { which } => {
                let all = layers(doc);
                let Some(id) = all.get(usize::from(*which) % all.len().max(1)).copied() else { return Ok(false) };
                exec(doc, "layer.arrange", json!({ "layers": [id], "to": if *which == 0 { "back" } else { "front" } }))?;
                Ok(true)
            }
            Script::Reparent { which } => {
                let all = layers(doc);
                let (Some(second), Some(third)) = (all.get(1).copied(), all.get(2).copied()) else { return Ok(false) };
                let (child, parent) = if *which == 0 { (second, third) } else { (third, second) };
                exec(doc, "layer.setParent", json!({ "layers": [child], "parent": parent, "compensate": false }))?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn export_file(&self, doc: &mut MotionDocument) -> Result<Option<Vec<u8>>> {
        doc.save(None).map(Some)
    }
}

/// Every applicable scenario passes (L6); text and blob scenarios do not apply to a project.
#[test]
fn passes_the_convergence_suite() {
    let report = kition_convergence::run(&MotionKit, &Options::from_env());
    println!("{}", report.render());
    report.save_if_requested().unwrap();
    assert!(report.level6(), "{}", report.render());
}
