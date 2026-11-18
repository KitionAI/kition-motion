//! What `ids.rs` and `outside.rs` share: a project of solid layers in one comp, the
//! `kition-motion` executable driven through the kits' MCP server's tools (`kition_kits::tools`,
//! the library `kition-kits-mcp` serves) as an agent drives it, and two programs other than that
//! server that change the file: a colleague's (this kit's engine, an instance of its own, opening
//! the file, running commands and saving it) and a bare rewriter (the `.kmotion` JSON edited as
//! JSON, no engine).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use kition_kit::KitDocument;
use kition_kits::tools::{self, CallOptions};
use kition_kits::{KitDefinition, KitManager};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

pub const ME: &str = "agent-me";
pub const FILE: &str = "Motion.kmotion";
pub const OPACITY: &str = "transform/opacity";

pub fn kit() -> &'static KitDefinition {
    KitDefinition::from_manifest_text(include_str!("../../../../kit.json")).unwrap()
}

/// A 320 x 180, 30 fps, 10 s comp `Main` holding `layers` solids, `L0` at the bottom up to
/// `L<layers-1>` at the top (each new solid goes on top).
pub fn project(layers: usize) -> Vec<u8> {
    let mut doc = MotionDocument::create(FILE, 320, 180).unwrap();
    doc.execute("comp.settings", json!({ "duration": 10, "name": "Main" })).unwrap();
    for n in 0..layers {
        let color = format!("#{:02x}{:02x}{:02x}", (n * 37) % 256, (n * 91) % 256, (n * 53) % 256);
        doc.execute("layer.newSolid", json!({ "name": format!("L{n}"), "color": color })).unwrap();
    }
    doc.save(None).unwrap()
}

/// Sets the file's modification time an hour before its own, so that only its content tells the
/// server it changed.
fn backdate(file: &Path, before: std::time::SystemTime) {
    std::fs::File::options().write(true).open(file).unwrap().set_modified(before - Duration::from_secs(3600)).unwrap();
}

/// A colleague's program: this kit's engine, its own instance, opening the file from disk,
/// running `steps` (commands and parameters) and saving the file back.
pub fn colleague(file: &Path, steps: &[(&str, Value)]) -> Vec<Value> {
    let at = std::fs::metadata(file).unwrap().modified().unwrap();
    let mut doc = MotionDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    let answers =
        steps.iter().map(|(command, params)| doc.execute(command, params.clone()).unwrap_or_else(|error| panic!("colleague's {command}: {error}"))).collect();
    std::fs::write(file, doc.save(Some(FILE)).unwrap()).unwrap();
    backdate(file, at);
    answers
}

/// Another program rewriting the file as JSON alone (no engine), pretty-printed, after `change`.
pub fn rewrite(file: &Path, change: impl FnOnce(&mut Value)) {
    let at = std::fs::metadata(file).unwrap().modified().unwrap();
    let mut project: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    change(&mut project);
    std::fs::write(file, serde_json::to_vec_pretty(&project).unwrap()).unwrap();
    backdate(file, at);
}

/// The file's first comp as a new engine reads it.
pub fn on_disk(file: &Path) -> MotionDocument {
    MotionDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap()
}

/// The layer ids of the file's open comp, top first.
pub fn layers_on_disk(file: &Path) -> Vec<u64> {
    layer_ids(&on_disk(file).inspect().unwrap()["info"])
}

/// Layer `id`'s value of property `path` at comp time 0 in the file, as a new engine reads it.
pub fn value_on_disk(file: &Path, id: u64, path: &str) -> Value {
    let mut doc = on_disk(file);
    doc.execute("prop.get", json!({ "layer": id, "path": path, "time": 0 })).unwrap()["value"].clone()
}

pub fn opacity_on_disk(file: &Path, id: u64) -> f64 {
    value_on_disk(file, id, OPACITY).as_f64().unwrap_or_else(|| panic!("layer {id}'s opacity"))
}

/// Layer `id`'s keyframes of `path` in the file, as `(comp seconds, value)`.
pub fn keys_on_disk(file: &Path, id: u64, path: &str) -> Vec<(f64, Value)> {
    let mut doc = on_disk(file);
    let got = doc.execute("prop.get", json!({ "layer": id, "path": path, "time": 0 })).unwrap();
    got["keys"].as_array().map(|keys| keys.iter().map(|key| (key["compTime"].as_f64().unwrap(), key["value"].clone())).collect()).unwrap_or_default()
}

/// The ids of the layers an inspect's `details.info` lists, top first.
pub fn layer_ids(info: &Value) -> Vec<u64> {
    info["layers"].as_array().unwrap().iter().map(|layer| layer["id"].as_u64().unwrap()).collect()
}

pub struct Fixture {
    pub kits: KitManager,
    pub options: CallOptions,
    pub workspace: tempfile::TempDir,
}

impl Fixture {
    pub fn new(layers: usize) -> Self {
        Self::with(layers, CallOptions::default())
    }

    pub fn with(layers: usize, options: CallOptions) -> Self {
        Self::from_bytes(&project(layers), options)
    }

    pub fn from_bytes(bytes: &[u8], options: CallOptions) -> Self {
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(workspace.path().join(FILE), bytes).unwrap();
        let kits = KitManager::new(Some(workspace.path().to_path_buf()), vec![(kit(), Some(PathBuf::from(env!("CARGO_BIN_EXE_kition-motion"))))]);
        Self { kits, options, workspace }
    }

    pub fn file(&self) -> PathBuf {
        self.workspace.path().join(FILE)
    }

    pub fn colleague(&self, steps: &[(&str, Value)]) -> Vec<Value> {
        colleague(&self.file(), steps)
    }

    pub async fn call(&self, action: &str, mut input: Value) -> Result<Value, Value> {
        input["path"] = json!(FILE);
        match tools::call_as(&self.kits, &format!("motion_{action}"), &input, self.options, ME).await {
            Some(Ok(output)) => Ok(output.output),
            Some(Err(error)) => Err(error.structured()),
            None => panic!("motion_{action} is no tool"),
        }
    }

    /// A call by another agent of the same server (`other`), as a second MCP connection makes it.
    pub async fn call_by(&self, other: &str, action: &str, mut input: Value) -> Value {
        input["path"] = json!(FILE);
        match tools::call_as(&self.kits, &format!("motion_{action}"), &input, self.options, other).await {
            Some(Ok(output)) => output.output,
            Some(Err(error)) => panic!("{other}'s motion_{action}: {}", error.structured()),
            None => panic!("motion_{action} is no tool"),
        }
    }

    pub async fn ok(&self, action: &str, input: Value) -> Value {
        self.call(action, input).await.unwrap_or_else(|error| panic!("motion_{action}: {error}"))
    }

    pub async fn refused(&self, action: &str, input: Value) -> Value {
        match self.call(action, input).await {
            Err(error) => error,
            Ok(answer) => panic!("motion_{action} was expected to be refused: {answer}"),
        }
    }

    /// The open comp's details as the agent inspects them (`details.info`, the comp's id) and the
    /// answer's `revision`.
    pub async fn inspect(&self) -> (Value, u64, String) {
        let seen = self.ok("inspect", json!({})).await;
        (seen["details"]["info"].clone(), seen["details"]["comp"]["id"].as_u64().unwrap(), revision(&seen))
    }

    pub async fn shutdown(self) {
        self.kits.shutdown().await;
    }
}

pub fn revision(answer: &Value) -> String {
    answer["revision"].as_str().unwrap_or_else(|| panic!("a revision: {answer}")).to_owned()
}

/// An id as an answer gives it, as text: an integer tool id is a number in receipts, `others` and
/// conflicts (SDK C8-fix-5), a string elsewhere.
pub fn id_text(id: &Value) -> Option<String> {
    id.as_str().map(str::to_owned).or_else(|| id.as_u64().map(|id| id.to_string()))
}

/// The ids a change list names.
pub fn ids(list: &Value) -> Vec<String> {
    list.as_array().map(|items| items.iter().filter_map(|item| id_text(&item["id"])).collect()).unwrap_or_default()
}

/// The entry of `list` naming `id`.
pub fn entry<'a>(list: &'a Value, id: &str) -> &'a Value {
    list.as_array().and_then(|items| items.iter().find(|item| id_text(&item["id"]).as_deref() == Some(id))).unwrap_or_else(|| panic!("no {id} in {list}"))
}

/// A step setting layer `layer`'s opacity to `value` (a static value: the layer has no keys).
pub fn set_opacity(layer: impl Into<Value>, value: f64) -> Value {
    json!({ "command": "prop.set", "params": { "layer": layer.into(), "path": OPACITY, "value": value } })
}

/// The colleague's command setting layer `layer`'s opacity to `value`.
pub fn opacity(layer: u64, value: f64) -> (&'static str, Value) {
    ("prop.set", json!({ "layer": layer, "path": OPACITY, "value": value }))
}

/// A refusal of a write whose premises others changed, naming `id` with `reason`.
pub fn assert_conflict<'a>(refused: &'a Value, id: &str, reason: &str) -> &'a Value {
    assert_eq!(refused["error"]["code"], "conflict", "{refused}");
    assert_eq!(refused["error"]["retryable"], true, "{refused}");
    assert!(refused["revision"].is_string(), "{refused}");
    let found = refused["conflicts"].as_array().and_then(|conflicts| conflicts.iter().find(|conflict| id_text(&conflict["id"]).as_deref() == Some(id)));
    let found = found.unwrap_or_else(|| panic!("no conflict on {id}: {refused}"));
    assert_eq!(found["reason"], reason, "{refused}");
    found
}

/// The median of `samples`, in milliseconds.
pub fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples.get(samples.len() / 2).copied().unwrap_or(f64::NAN)
}
