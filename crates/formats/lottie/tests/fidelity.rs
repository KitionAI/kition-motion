//! Format fidelity scorecard (roadmap phase 7).
//!
//! `KITION_MOTION_CORPUS=<dir>`: every project (`.kmotion`, and the engine's own `.ecproj`)
//! and every Lottie document (`.json`, `.lottie`) in that directory is read, written and read
//! again (a project as a project, a Lottie document as Lottie), and each composition of a project
//! that no other composition uses is also exported to Lottie and read back (an entry
//! `<file> › <comp> (Lottie)`: what File ▸ Export ▸ Lottie keeps, as File ▸ Import ▸ Lottie
//! reads it). The test reports
//!
//! - per feature class (compositions, layers, transform, effects, masks, text, shapes, keyframes
//!   and easing, expressions, markers, assets, project settings, …; the Lottie leg's classes are
//!   named `lottie …` and compare the exported composition and what it uses, with ids replaced by
//!   positions), which files keep it through import → export → import (the two projects' leaves
//!   of that class are compared as multisets of `path=value`, so a moved layer is not a loss but
//!   a dropped property is), and
//! - what the written file lost against the original file: its JSON keys counted by path (array
//!   indices dropped, numeric keys folded; the values of type tags counted with them) per part
//!   kind (the top-level key, and the item type of a project's items). The Lottie export of a
//!   project has no original file to compare with and counts no keys.
//!
//! `KITION_FIDELITY_OUT=<dir>` writes the full report (`effect-fidelity.txt`, per file and
//! aggregated) and the aggregate scorecard (`effect-fidelity.json`, no file names) there.
//! `KITION_FIDELITY_BASELINE=<file>` fails when the corpus scores below such a scorecard: fewer
//! files keeping a class, more leaves lost, or less of the file retained (the non-regression
//! gate). Skipped when the corpus variable is unset. The KDM leg of the product's path (import
//! into a replica, read back) is covered by `kdm-model`'s `tests/roundtrip.rs`.
//!
//! The scorer below the kit-specific part is the same as in Kition Word, Slide and Sheet (the
//! package features are read from JSON instead of a zip package, so `features` is kit-specific
//! here, as in Kition Vector and CAD, and a corpus file may be scored as more than one entry).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::{Value, json};

use kition_motion_lottie::{ExportOptions, ImportedImage};
use kition_motion_project::{ItemId, ItemKind, LayerSource, Project};

// ---- Kit-specific -------------------------------------------------------------------------------

const KIT: &str = "effect";
const CORPUS_VAR: &str = "KITION_MOTION_CORPUS";
const EXTENSIONS: &[&str] = &["kmotion", "ecproj", "json", "lottie"];
/// A class the self-test's document must have.
const PRIMARY_CLASS: &str = "layers";

/// A project, and the composition it was read into when it came from a Lottie document (saved
/// back as Lottie).
struct Model {
    project: Project,
    lottie: Option<ItemId>,
}

/// Images a Lottie import stored, by the name it gave them, so that the export reads them back.
static IMAGES: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::new(BTreeMap::new());

fn is_project(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(bytes.get(..bytes.len().min(4096)).unwrap_or(bytes)).into_owned();
    head.contains("\"schema\"") && head.contains("\"items\"")
}

/// An entry scoring the Lottie export of composition `<id>` of the project that follows.
const LOTTIE_EXPORT: &str = "lottie-export:";

fn import(bytes: &[u8]) -> Result<Model, String> {
    if let Some(rest) = bytes.strip_prefix(LOTTIE_EXPORT.as_bytes()) {
        let text = std::str::from_utf8(rest).map_err(|error| error.to_string())?;
        let (id, json) = text.split_once('\n').ok_or("no composition id")?;
        let comp = ItemId(id.parse().map_err(|_| "bad composition id")?);
        return Ok(Model { project: Project::from_json(json).map_err(|error| error.to_string())?, lottie: Some(comp) });
    }
    if is_project(bytes) {
        let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        return Ok(Model { project: Project::from_json(text).map_err(|error| error.to_string())?, lottie: None });
    }
    let mut project = Project::default();
    let mut store = |image: &ImportedImage| -> Option<String> {
        let name = format!("image:{}.{}", digest(&image.bytes), image.ext);
        IMAGES.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(name.clone(), image.bytes.clone());
        Some(name)
    };
    let result = kition_motion_lottie::import(&mut project, bytes, "Lottie", "", &mut store).map_err(|error| error.to_string())?;
    Ok(Model { project, lottie: Some(result.comp) })
}

fn read_image(name: &str) -> Option<Vec<u8>> {
    IMAGES.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(name).cloned()
}

fn lottie(project: &Project, comp: ItemId) -> Result<Vec<u8>, String> {
    let result = kition_motion_lottie::export_comp(project, comp, &ExportOptions::default(), &read_image).map_err(|error| error.to_string())?;
    Ok(result.to_string_compact().into_bytes())
}

fn export(model: &Model) -> Result<Vec<u8>, String> {
    match model.lottie {
        Some(comp) => lottie(&model.project, comp),
        None => model.project.to_file_json().map(String::into_bytes).map_err(|error| error.to_string()),
    }
}

/// What a corpus file is scored by: a project or a Lottie document as it is, and for a project
/// also each composition no other composition uses, exported to Lottie (the project marked with
/// [`LOTTIE_EXPORT`] and the composition's id).
fn sources(name: &str, bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut out = vec![(name.to_owned(), bytes.to_vec())];
    let Ok(Model { project, lottie: None }) = import(bytes) else { return out };
    let nested: Vec<ItemId> = project
        .comps()
        .flat_map(|(_, comp)| comp.layers.iter().filter_map(|layer| if let LayerSource::Comp { item } = layer.source { Some(item) } else { None }))
        .collect();
    for (id, _) in project.comps() {
        if nested.contains(id) {
            continue;
        }
        let comp = project.item(*id).map(|item| item.name.clone()).unwrap_or_default();
        let mut entry = format!("{LOTTIE_EXPORT}{}\n", id.0).into_bytes();
        entry.extend_from_slice(bytes);
        out.push((format!("{name} › {comp} (Lottie)"), entry));
    }
    out
}

/// A layer's property tree as a list: each group and property with its path of match names
/// (`transform/position`), keyed by the part of the layer it belongs to (`transform`, `effects`,
/// `masks`, `text`, `contents`, …: the root group's child it is under).
fn flatten_props(node: &Value, path: &str, part: &str, out: &mut Vec<Value>) {
    let Some(map) = node.as_object() else { return };
    let name = map.get("match").and_then(Value::as_str).unwrap_or("");
    let here = if path.is_empty() { name.to_owned() } else { format!("{path}/{name}") };
    let mut fields = map.clone();
    fields.remove("children");
    // An animated property's static value is not used (turning the stopwatch off keeps the
    // value at the current time instead), and Lottie has no place for it.
    if fields.get("keys").and_then(Value::as_array).is_some_and(|keys| !keys.is_empty()) {
        fields.remove("value");
    }
    split_numbers(&mut fields);
    fields.insert("path".into(), json!(here));
    let group = map.get("node").and_then(Value::as_str) != Some("Prop") && map.contains_key("children");
    out.push(json!({ format!("{part}{}", if group { ":group" } else { "" }): fields }));
    if let Some(children) = map.get("children").and_then(Value::as_array) {
        for child in children {
            let part = if part == "root" { child.get("match").and_then(Value::as_str).unwrap_or("other") } else { part };
            flatten_props(child, &here, part, out);
        }
    }
}

/// Points, colours and vectors (`[x, y]`, `[r, g, b, a]`) as one leaf per number, so that each is
/// compared within [`close`]'s tolerance.
fn split_numbers(map: &mut serde_json::Map<String, Value>) {
    fn split(value: &mut Value) {
        match value {
            Value::Array(items) if (2..=4).contains(&items.len()) && items.iter().all(Value::is_number) => {
                const NAMES: [&str; 4] = ["x", "y", "z", "w"];
                *value = Value::Object(NAMES.iter().zip(items.iter()).map(|(k, v)| ((*k).to_owned(), v.clone())).collect());
            }
            Value::Array(items) => items.iter_mut().for_each(split),
            Value::Object(map) => map.values_mut().for_each(split),
            _ => {}
        }
    }
    map.values_mut().for_each(split);
}

fn layer_values(layers: &mut [Value]) {
    for layer in layers.iter_mut() {
        let Some(map) = layer.as_object_mut() else { continue };
        let mut props = Vec::new();
        if let Some(tree) = map.remove("props") {
            flatten_props(&tree, "", "root", &mut props);
        }
        map.insert("props".into(), Value::Array(props));
    }
}

/// The project as JSON: items as a list, each composition's layers with their property trees
/// flattened ([`flatten_props`]).
fn project_value(project: &Project) -> Value {
    let mut value = serde_json::to_value(project).unwrap_or(Value::Null);
    if let Some(items) = value.get_mut("items").and_then(Value::as_object_mut) {
        let mut list = Vec::new();
        for (_, mut item) in std::mem::take(items) {
            if let Some(layers) = item.pointer_mut("/kind/layers").and_then(Value::as_array_mut) {
                layer_values(layers);
            }
            list.push(item);
        }
        value["items"] = Value::Array(list);
    }
    value
}

/// Composition `root` and the items its layers use (nested compositions' too), as Lottie carries
/// them: in the order they are reached, with item and layer ids replaced by their positions (a
/// Lottie import numbers them anew) and without folders.
fn exchange_value(project: &Project, root: ItemId) -> Value {
    let mut order = vec![root];
    let mut at = 0;
    while let Some(id) = order.get(at).copied() {
        at += 1;
        let Some(comp) = project.comp(id) else { continue };
        for layer in &comp.layers {
            if let LayerSource::Footage { item } | LayerSource::Comp { item } | LayerSource::Solid { item } = layer.source
                && !order.contains(&item)
            {
                order.push(item);
            }
        }
    }
    let item_at = |id: u64| order.iter().position(|item| item.0 == id).map_or(Value::Null, |i| json!(i));
    let mut items = Vec::new();
    for id in &order {
        let Some(item) = project.item(*id) else { continue };
        let mut value = serde_json::to_value(item).unwrap_or(Value::Null);
        let Some(map) = value.as_object_mut() else { continue };
        map.remove("id");
        map.remove("parent");
        if let (Some(comp), Some(layers)) = (project.comp(*id), value.pointer_mut("/kind/layers").and_then(Value::as_array_mut)) {
            let layer_at = |id: u64| comp.layers.iter().position(|layer| layer.id.0 == id).map_or(Value::Null, |i| json!(i));
            for layer in layers.iter_mut() {
                for pointer in ["/id", "/parent", "/track_matte/layer"] {
                    if let Some(v) = layer.pointer_mut(pointer)
                        && let Some(id) = v.as_u64()
                    {
                        *v = layer_at(id);
                    }
                }
                if let Some(v) = layer.pointer_mut("/source/item")
                    && let Some(id) = v.as_u64()
                {
                    *v = item_at(id);
                }
            }
            layer_values(layers);
        }
        items.push(value);
    }
    json!({ "lottie": { "items": items } })
}

/// The project as JSON ([`project_value`]); for a Lottie document, or a project scored by its
/// Lottie export, the composition Lottie carries ([`exchange_value`]).
fn model_value(model: &Model) -> Value {
    match model.lottie {
        Some(comp) => exchange_value(&model.project, comp),
        None => project_value(&model.project),
    }
}

/// Keys whose values are ids or caches a re-import may renumber without losing anything.
fn ignored(key: &str) -> bool {
    matches!(key, "uid" | "next_id" | "savedBy")
}

/// Whether two numbers of `class` are the same after a round trip: exact for a project file up
/// to float printing; through Lottie (frames instead of ticks, normalized easing and colours) as
/// in the other kits.
fn close(class: &str, a: f64, b: f64) -> bool {
    if class.starts_with("lottie") {
        return (a - b).abs() <= 0.01_f64.max(a.abs() * 1e-4);
    }
    (a - b).abs() <= 1e-9_f64.max(a.abs() * 1e-12)
}

/// The feature class of a leaf, by its path (`.key` segments, `[]` for array items, `#` for
/// numeric map keys). Property tree entries are `….props[].<part>[:group].…`.
fn classify(path: &str) -> &'static str {
    let (lottie, path) = match path.strip_prefix(".lottie") {
        Some(rest) => (true, rest),
        None => (false, path),
    };
    let class = native_class(path);
    if !lottie {
        return class;
    }
    match class {
        "compositions" => "lottie compositions",
        "layers" => "lottie layers",
        "transform" => "lottie transform",
        "effects" => "lottie effects",
        "masks" => "lottie masks",
        "text" => "lottie text",
        "shapes" => "lottie shapes",
        "layer styles" => "lottie layer styles",
        "other properties" => "lottie other properties",
        "keyframes and easing" => "lottie keyframes and easing",
        "expressions" => "lottie expressions",
        "markers" => "lottie markers",
        "graphics templates" => "lottie graphics templates",
        "assets" => "lottie assets",
        "render queue" => "lottie render queue",
        _ => "lottie project settings",
    }
}

fn native_class(path: &str) -> &'static str {
    if path.contains(".keys") {
        return "keyframes and easing";
    }
    if path.contains(".expr") {
        return "expressions";
    }
    if path.contains(".markers") {
        return "markers";
    }
    if path.contains(".graphics_template") {
        return "graphics templates";
    }
    if let Some(at) = path.find(".props[].") {
        let rest = &path[at + ".props[].".len()..];
        let part = rest.split(['.', ':']).next().unwrap_or("");
        return match part {
            "transform" => "transform",
            "effects" => "effects",
            "masks" => "masks",
            "text" => "text",
            "contents" => "shapes",
            "layerStyles" | "styles" => "layer styles",
            _ => "other properties",
        };
    }
    if path.contains(".layers[]") {
        return "layers";
    }
    if path.starts_with(".items[].kind.") && !path.starts_with(".items[].kind.type=") {
        // A composition's own settings; footage, solids and folders are assets.
        return if path.contains("layers")
            || path.contains("work_area")
            || path.contains("frame_rate")
            || path.contains("shutter")
            || path.contains("renderer")
            || path.contains("global_light")
            || path.contains("guides")
        {
            "compositions"
        } else {
            "assets"
        };
    }
    if path.starts_with(".items[]") {
        return "assets";
    }
    if path.starts_with(".render") {
        return "render queue";
    }
    "project settings"
}

/// JSON keys of a project or a Lottie document by path (array indices dropped, numeric keys
/// folded), with the values of type tags; the part kind is the top-level key (a project's items
/// by their type).
fn features(file: &[u8]) -> Result<Features, String> {
    if file.starts_with(LOTTIE_EXPORT.as_bytes()) {
        return Err("a project scored by its Lottie export has no Lottie original".into());
    }
    fn walk(value: &Value, path: &str, kind: &str, out: &mut Features) {
        match value {
            Value::Object(map) => {
                for (key, item) in map {
                    let key = if !key.is_empty() && key.chars().all(|c| c.is_ascii_digit()) { "#" } else { key.as_str() };
                    let here = format!("{path}.{key}");
                    let counts = out.entry(kind.to_owned()).or_default();
                    *counts.entry(here.clone()).or_default() += 1;
                    // Type tags: names, and Lottie's numbered layer types (`t` is a value's type in a
                    // project but a keyframe's time in Lottie, so only its names count).
                    if matches!(key, "type" | "t" | "node" | "kind" | "ty" | "match") && (item.is_string() || (key == "ty" && item.is_number())) {
                        *counts.entry(format!("{here}={item}")).or_default() += 1;
                    }
                    walk(item, &here, kind, out);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| walk(item, &format!("{path}[]"), kind, out)),
            _ => {}
        }
    }
    let text: Value = if file.starts_with(b"PK") {
        // A `.lottie` archive: its first animation.
        let entries = kition_motion_lottie::zip::read_stored(file);
        let json = entries.iter().find(|(name, _)| name.starts_with("animations/") && name.ends_with(".json")).ok_or("no animation in the archive")?;
        serde_json::from_slice(&json.1).map_err(|error| error.to_string())?
    } else {
        serde_json::from_slice(file).map_err(|error| error.to_string())?
    };
    let mut out = Features::new();
    let Value::Object(map) = &text else { return Err("not a JSON object".into()) };
    for (key, value) in map {
        if key == "items"
            && let Value::Object(items) = value
        {
            for item in items.values() {
                let ty = item.pointer("/kind/type").and_then(Value::as_str).unwrap_or("item");
                walk(item, ".items.#", &format!("items/{ty}"), &mut out);
            }
            continue;
        }
        *out.entry("(root)".into()).or_default().entry(key.clone()).or_default() += 1;
        walk(value, &format!(".{key}"), key, &mut out);
    }
    Ok(out)
}

/// A small project exercising most classes, for the self-test: a composition with a solid layer
/// and its solid, an animated, eased position.
fn synthetic() -> Model {
    use kition_motion_color::Label;
    use kition_motion_project::build;
    use kition_motion_project::{Comp, Keyframe, Solid, Value as PropValue};
    use kition_motion_time::{FrameRate, Tick};
    let mut project = Project::default();
    let comp = Comp::new(320, 180, FrameRate::FPS_30, Tick::from_seconds_f64(2.0));
    let cid = project.add_item("Main", Label::Sandstone, None, ItemKind::Comp(comp.into()));
    let solid = project.add_item("Red", Label::Red, None, ItemKind::Solid(Solid { color: [1.0, 0.0, 0.0], width: 64, height: 36, pixel_aspect: 1.0 }));
    let Some(c) = project.comp(cid).cloned() else { panic!("no comp") };
    let mut layer = build::layer(&mut project, &c, "Red", LayerSource::Solid { item: solid }, (64, 36), None);
    if let Some(position) = layer.props.prop_mut("transform/position") {
        position.keys.push(Keyframe::new(Tick::from_seconds_f64(0.0), PropValue::Vec2([20.0, 20.0])).eased());
        position.keys.push(Keyframe::new(Tick::from_seconds_f64(1.0), PropValue::Vec2([200.0, 100.0])).eased());
    }
    if let Some(c) = project.comp_mut(cid) {
        c.layers.push(layer);
    }
    Model { project, lottie: None }
}

// ---- Shared scorer (as in kition-word, kition-slide, kition-sheet) -------------------------------

/// Leaves of a model by class: `path=value` → count.
type Leaves = BTreeMap<&'static str, BTreeMap<String, u64>>;
/// Features of a package by part kind: feature → count.
type Features = BTreeMap<String, BTreeMap<String, u64>>;

fn digest(bytes: &[u8]) -> String {
    // FNV-1a: enough to tell two media items apart in a report.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:016x}/{}", hash, bytes.len())
}

fn fold_digits(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut in_digits = false;
    for c in name.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('#');
            }
            in_digits = true;
        } else {
            out.push(c);
            in_digits = false;
        }
    }
    out
}

fn leaves(value: &Value) -> Leaves {
    fn walk(value: &Value, path: &mut String, out: &mut Leaves) {
        match value {
            Value::Null => {}
            Value::Object(map) => {
                for (key, item) in map {
                    if ignored(key) {
                        continue;
                    }
                    let len = path.len();
                    path.push('.');
                    path.push_str(if key.chars().all(|c| c.is_ascii_digit() || c == '-') { "#" } else { key });
                    walk(item, path, out);
                    path.truncate(len);
                }
            }
            Value::Array(items) if items.iter().any(|v| v.is_object() || v.is_array()) => {
                let len = path.len();
                path.push_str("[]");
                for item in items {
                    walk(item, path, out);
                }
                path.truncate(len);
            }
            scalar => {
                *out.entry(classify(path)).or_default().entry(format!("{path}={scalar}")).or_default() += 1;
            }
        }
    }
    let mut out = Leaves::new();
    walk(value, &mut String::new(), &mut out);
    out
}

#[derive(Default)]
struct ClassScore {
    files: u64,
    files_kept: u64,
    leaves: u64,
    kept: u64,
    added: u64,
}

#[derive(Default)]
struct Scorecard {
    files: u64,
    unreadable: u64,
    unwritable: u64,
    unrereadable: u64,
    classes: BTreeMap<&'static str, ClassScore>,
    /// (class, path with digits folded) → (files, leaves lost, an example: lost leaf → a leaf the
    /// second import has instead on that path, if any)
    lost_leaves: BTreeMap<(&'static str, String), (u64, u64, String)>,
    /// (class, path with digits folded) → (files, leaves added, an example)
    added_leaves: BTreeMap<(&'static str, String), (u64, u64, String)>,
    xml_original: u64,
    xml_kept: u64,
    /// (part kind, feature) → (files, count lost)
    lost_features: BTreeMap<(String, String), (u64, u64)>,
    part_kinds: BTreeMap<String, (u64, u64)>,
    lines: Vec<String>,
}

/// The number a leaf holds, if it holds one.
fn number_of(leaf: &str) -> Option<f64> {
    leaf.split_once('=').and_then(|(_, v)| if v.starts_with('"') { None } else { v.parse().ok() })
}

/// Folded path → (count, an example).
type Paths = BTreeMap<String, (u64, String)>;

/// How many of `left`'s leaves `right` keeps (equal, or a number [`close`] to a spare one on the
/// same path), how many it adds, what it lost and what it added.
fn compare(class: &str, left: &BTreeMap<String, u64>, right: &BTreeMap<String, u64>) -> (u64, u64, Paths, Paths) {
    let mut spare: BTreeMap<String, Vec<(f64, &str)>> = BTreeMap::new();
    let mut added = 0;
    for (leaf, m) in right {
        let extra = m.saturating_sub(left.get(leaf).copied().unwrap_or(0));
        added += extra;
        if let Some(v) = number_of(leaf) {
            for _ in 0..extra {
                spare.entry(path_of(leaf)).or_default().push((v, leaf));
            }
        }
    }
    let mut kept = 0;
    let mut lost = Paths::new();
    for (leaf, n) in left {
        let m = right.get(leaf).copied().unwrap_or(0);
        kept += m.min(*n);
        let path = path_of(leaf);
        for _ in m..*n {
            if let (Some(v), Some(list)) = (number_of(leaf), spare.get_mut(&path))
                && let Some(i) = list.iter().position(|(w, _)| close(class, v, *w))
            {
                list.swap_remove(i);
                kept += 1;
                added -= 1;
                continue;
            }
            let entry = lost.entry(fold_digits(&path)).or_default();
            entry.0 += 1;
            if entry.1.is_empty() {
                let instead = right.keys().find(|r| path_of(r) == path && !left.contains_key(*r)).map(String::as_str).unwrap_or("nothing");
                entry.1 = format!("{} → {}", clip(leaf), clip(instead));
            }
        }
    }
    let mut gained = Paths::new();
    let mut note = |leaf: &str, n: u64| {
        let entry = gained.entry(fold_digits(&path_of(leaf))).or_default();
        entry.0 += n;
        if entry.1.is_empty() {
            entry.1 = clip(leaf);
        }
    };
    for (leaf, m) in right {
        let extra = m.saturating_sub(left.get(leaf).copied().unwrap_or(0));
        if extra > 0 && number_of(leaf).is_none() {
            note(leaf, extra);
        }
    }
    for (_, leaf) in spare.values().flatten() {
        note(leaf, 1);
    }
    (kept, added, lost, gained)
}

/// `leaf`, cut to a readable length.
fn clip(leaf: &str) -> String {
    let mut out: String = leaf.chars().take(160).collect();
    if out.len() < leaf.len() {
        out.push('…');
    }
    out
}

fn path_of(leaf: &str) -> String {
    leaf.split_once('=').map(|(p, _)| p.to_owned()).unwrap_or_default()
}

impl Scorecard {
    fn file(&mut self, name: &str, bytes: &[u8]) {
        self.files += 1;
        let Ok(first) = import(bytes) else {
            self.unreadable += 1;
            self.lines.push(format!("{name}: not importable"));
            return;
        };
        let Ok(written) = export(&first) else {
            self.unwritable += 1;
            self.lines.push(format!("{name}: export failed"));
            return;
        };
        let Ok(second) = import(&written) else {
            self.unrereadable += 1;
            self.lines.push(format!("{name}: export not re-importable"));
            return;
        };
        let (a, b) = (leaves(&model_value(&first)), leaves(&model_value(&second)));
        let mut lost_classes = Vec::new();
        let empty = BTreeMap::new();
        for (class, left) in &a {
            let right = b.get(class).unwrap_or(&empty);
            let score = self.classes.entry(class).or_default();
            let total: u64 = left.values().sum();
            let (kept, added, lost_paths, added_paths) = compare(class, left, right);
            score.files += 1;
            score.leaves += total;
            score.kept += kept;
            score.added += added;
            if kept == total && added == 0 {
                score.files_kept += 1;
            } else {
                lost_classes.push(format!("{class} {kept}/{total}+{added}"));
            }
            for (paths, into) in [(lost_paths, &mut self.lost_leaves), (added_paths, &mut self.added_leaves)] {
                for (path, (n, example)) in paths {
                    let entry = into.entry((class, path)).or_default();
                    entry.0 += 1;
                    entry.1 += n;
                    if entry.2.is_empty() {
                        entry.2 = example;
                    }
                }
            }
        }
        for (class, right) in &b {
            if !a.contains_key(class) {
                let score = self.classes.entry(class).or_default();
                score.files += 1;
                score.added += right.values().sum::<u64>();
                lost_classes.push(format!("{class} 0/0+{}", right.values().sum::<u64>()));
            }
        }
        let (Ok(original), Ok(exported)) = (features(bytes), features(&written)) else {
            self.lines.push(format!("{name}: classes changed: {lost_classes:?}; package not comparable"));
            return;
        };
        let (mut total, mut kept) = (0u64, 0u64);
        for (kind, counts) in &original {
            let theirs = exported.get(kind);
            let kind_total: u64 = counts.values().sum();
            let mut kind_kept = 0;
            for (feature, n) in counts {
                let m = theirs.and_then(|t| t.get(feature)).copied().unwrap_or(0);
                kind_kept += m.min(*n);
                if m < *n {
                    let entry = self.lost_features.entry((kind.clone(), feature.clone())).or_default();
                    entry.0 += 1;
                    entry.1 += n - m;
                }
            }
            let entry = self.part_kinds.entry(kind.clone()).or_default();
            entry.0 += kind_total;
            entry.1 += kind_kept;
            total += kind_total;
            kept += kind_kept;
        }
        self.xml_original += total;
        self.xml_kept += kept;
        self.lines.push(format!("{name}: xml {:.1}%; classes changed: {lost_classes:?}", percent(kept, total)));
    }

    fn json(&self) -> Value {
        let classes: serde_json::Map<String, Value> = self
            .classes
            .iter()
            .map(|(class, s)| {
                ((*class).to_owned(), json!({ "files": s.files, "filesKept": s.files_kept, "leaves": s.leaves, "kept": s.kept, "added": s.added }))
            })
            .collect();
        json!({
            "kit": KIT,
            "files": self.files,
            "unreadable": self.unreadable,
            "unwritable": self.unwritable,
            "unrereadable": self.unrereadable,
            "classes": classes,
            "xml": { "original": self.xml_original, "kept": self.xml_kept },
        })
    }

    fn report(&self) -> String {
        let mut out = String::new();
        let ok = self.files - self.unreadable - self.unwritable - self.unrereadable;
        out.push_str(&format!(
            "{KIT} fidelity: {} files, {ok} round-tripped ({} unreadable, {} export failed, {} export unreadable)\n\n",
            self.files, self.unreadable, self.unwritable, self.unrereadable
        ));
        out.push_str("class                        files kept   leaves kept      added\n");
        for (class, s) in &self.classes {
            out.push_str(&format!("{class:<28} {:>4}/{:<4}  {:>6.2}% of {:<9} {}\n", s.files_kept, s.files, percent(s.kept, s.leaves), s.leaves, s.added));
        }
        out.push_str(&format!("\nXML retained: {:.2}% of {} elements and attributes\n", percent(self.xml_kept, self.xml_original), self.xml_original));
        let mut kinds: Vec<_> = self.part_kinds.iter().collect();
        kinds.sort_by_key(|(_, (total, kept))| std::cmp::Reverse(total - kept));
        out.push_str("\npart kinds by features lost:\n");
        for (kind, (total, kept)) in kinds.iter().take(25) {
            out.push_str(&format!("  {kind:<50} {:>9} lost of {:<9} ({:.1}% kept)\n", total - kept, total, percent(*kept, *total)));
        }
        let mut lost: Vec<_> = self.lost_features.iter().collect();
        lost.sort_by_key(|(_, (files, count))| std::cmp::Reverse((*files, *count)));
        out.push_str("\ntop lost XML features by files (files, count):\n");
        for ((kind, feature), (files, count)) in lost.iter().take(150) {
            out.push_str(&format!("  {files:>3} {count:>8}  {kind} {feature}\n"));
        }
        lost.sort_by_key(|(_, (files, count))| std::cmp::Reverse((*count, *files)));
        out.push_str("\ntop lost XML features by count (files, count):\n");
        for ((kind, feature), (files, count)) in lost.iter().take(150) {
            out.push_str(&format!("  {files:>3} {count:>8}  {kind} {feature}\n"));
        }
        let mut lost: Vec<_> = self.lost_leaves.iter().collect();
        lost.sort_by_key(|(_, (files, count, _))| std::cmp::Reverse((*files, *count)));
        out.push_str("\ntop lost model leaves (files, count, example):\n");
        for ((class, path), (files, count, example)) in lost.iter().take(300) {
            out.push_str(&format!("  {files:>3} {count:>8}  [{class}] {path}\n               e.g. {example}\n"));
        }
        let mut added: Vec<_> = self.added_leaves.iter().collect();
        added.sort_by_key(|(_, (files, count, _))| std::cmp::Reverse((*files, *count)));
        out.push_str("\ntop added model leaves (files, count, example):\n");
        for ((class, path), (files, count, example)) in added.iter().take(100) {
            out.push_str(&format!("  {files:>3} {count:>8}  [{class}] {path}\n               e.g. {example}\n"));
        }
        out.push_str("\nper file:\n");
        for line in &self.lines {
            out.push_str(&format!("  {line}\n"));
        }
        out
    }
}

fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 { 100.0 } else { part as f64 * 100.0 / whole as f64 }
}

/// The regressions of `now` against `baseline`, both [`Scorecard::json`] values.
fn regressions(now: &Value, baseline: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let number = |v: &Value, path: &[&str]| path.iter().fold(v, |v, key| &v[*key]).as_u64().unwrap_or(0);
    for key in ["unreadable", "unwritable", "unrereadable"] {
        if number(now, &[key]) > number(baseline, &[key]) {
            out.push(format!("{key}: {} > {}", number(now, &[key]), number(baseline, &[key])));
        }
    }
    let ratio = |kept: u64, total: u64| if total == 0 { 1.0 } else { kept as f64 / total as f64 };
    if let Some(classes) = baseline["classes"].as_object() {
        for class in classes.keys() {
            let (b, n) = (&baseline["classes"][class], &now["classes"][class]);
            if number(n, &["filesKept"]) < number(b, &["filesKept"]) {
                out.push(format!("{class}: {} files keep it, baseline {}", number(n, &["filesKept"]), number(b, &["filesKept"])));
            }
            // More leaves lost than the class grew by: a model that learns a property may also
            // count it on objects that were lost already, which is not a regression.
            let lost = |v: &Value| number(v, &["leaves"]).saturating_sub(number(v, &["kept"]));
            let grown = number(n, &["leaves"]).saturating_sub(number(b, &["leaves"]));
            if lost(n) > lost(b) + grown {
                out.push(format!("{class}: {} leaves lost, baseline {} (+{grown} leaves)", lost(n), lost(b)));
            }
        }
    }
    let (rb, rn) = (
        ratio(number(baseline, &["xml", "kept"]), number(baseline, &["xml", "original"])),
        ratio(number(now, &["xml", "kept"]), number(now, &["xml", "original"])),
    );
    if rn + 1e-9 < rb {
        out.push(format!("XML retained {:.4}%, baseline {:.4}%", rn * 100.0, rb * 100.0));
    }
    out
}

#[test]
fn corpus_fidelity() {
    let Ok(dir) = std::env::var(CORPUS_VAR) else { return };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{dir}: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e))))
        // macOS keeps resource forks of copied files as `._<name>` beside them: not documents.
        .filter(|path| !path.file_name().is_some_and(|n| n.to_string_lossy().starts_with("._")))
        .collect();
    files.sort();
    let mut card = Scorecard::default();
    for path in &files {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{name}: {error}"));
        for (entry, bytes) in sources(&name, &bytes) {
            card.file(&entry, &bytes);
        }
    }
    let report = card.report();
    let summary: String = report.split("\npart kinds").next().unwrap_or_default().to_owned();
    eprintln!("{summary}");
    let now = card.json();
    if let Ok(out) = std::env::var("KITION_FIDELITY_OUT") {
        let _ = std::fs::create_dir_all(&out);
        std::fs::write(format!("{out}/{KIT}-fidelity.txt"), &report).unwrap_or_else(|error| panic!("{out}: {error}"));
        let pretty = serde_json::to_string_pretty(&now).unwrap_or_default();
        std::fs::write(format!("{out}/{KIT}-fidelity.json"), pretty).unwrap_or_else(|error| panic!("{out}: {error}"));
    }
    if let Ok(path) = std::env::var("KITION_FIDELITY_BASELINE") {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
        let baseline: Value = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{path}: {error}"));
        let regressed = regressions(&now, &baseline);
        assert!(regressed.is_empty(), "fidelity regressed against {path}:\n{}", regressed.join("\n"));
    }
}

#[test]
fn scorer_passes_a_lossless_round_trip_and_flags_a_regression() {
    let doc = synthetic();
    let bytes = export(&doc).unwrap_or_else(|error| panic!("export: {error}"));
    let mut card = Scorecard::default();
    card.file("synthetic", &bytes);
    assert_eq!(card.unreadable + card.unwritable + card.unrereadable, 0, "{}", card.report());
    assert!(card.classes.values().all(|s| s.files_kept == s.files), "{}", card.report());
    assert!(card.classes.contains_key(PRIMARY_CLASS), "{}", card.report());
    assert!(card.xml_original > 0 && card.xml_kept == card.xml_original, "{}", card.report());
    // A baseline of this run accepts it; one that kept more flags it.
    let now = card.json();
    assert!(regressions(&now, &now).is_empty());
    let mut better = now.clone();
    better["xml"]["kept"] = json!(card.xml_kept + 1);
    better["xml"]["original"] = json!(card.xml_original);
    assert_eq!(regressions(&now, &better).len(), 1);
}
