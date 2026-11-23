//! C8-time-5: expectations are declared before the independent writer changes the saved file.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

const SECOND: i64 = 254_016_000_000;
const CHANGES: [&str; 30] = [
    "insert-before",
    "insert-at",
    "insert-after",
    "delete-before",
    "delete-at",
    "delete-after",
    "shift-keys",
    "layer-start",
    "stretch",
    "value-at",
    "ease-left",
    "ease-right",
    "layer-insert-above",
    "layer-delete-above",
    "layer-reorder",
    "precomp",
    "rename",
    "trim-in",
    "trim-out",
    "unrelated",
    "work-area",
    "marker-before",
    "marker-at",
    "marker-after",
    "effect-before",
    "effect-at",
    "effect-after",
    "layer-insert-below",
    "layer-delete-below",
    "layer-reorder-below",
];
const EXTRA: [&str; 5] = ["value-after", "ease-after", "precomp-below", "rename-below", "effect-rename"];
const ROWS: [&str; 22] = [
    "key-time",
    "key-nearest",
    "point-add",
    "point-add-sampled",
    "point-set",
    "current-time",
    "key-move",
    "layer-id",
    "layer-index",
    "layer-name",
    "layer-timing",
    "work-area",
    "comp-marker-index",
    "comp-marker-time",
    "layer-marker-index",
    "layer-marker-time",
    "effect-id",
    "effect-index",
    "effect-name",
    "range-delete",
    "range-shift",
    "range-scale",
];

async fn observed(bytes: &[u8]) -> (Fixture, u64, String) {
    for attempt in 0..3 {
        let f = Fixture::from_bytes(bytes, Default::default());
        match f.call("inspect", json!({})).await {
            Ok(seen) => return (f, seen["details"]["comp"]["id"].as_u64().unwrap(), revision(&seen)),
            Err(error) => assert!(attempt < 2 && error["error"]["code"] == "unavailable" && error.to_string().contains("private folder"), "{error}"),
        }
        f.shutdown().await;
    }
    panic!("private folder creation failed three times")
}

// A apply; R relocated; C changed. Key/marker ids and key command indexes do not exist.
// This is a specification over address forms and independent edits, not over observed errors.
fn expected(row: &str, change: &str) -> char {
    if row == "work-area" {
        return if change == "work-area" { 'C' } else { 'A' };
    }
    // C8-time-policy: the necessity rule. A marker named by its index is the marker at that
    // index (an insert before it names another one; one after it does not); one named by its time
    // is the marker at that time (inserts elsewhere do not touch it). Both write its fields.
    if row.starts_with("comp-marker") {
        return match change {
            "marker-at" => 'C',
            "marker-before" if row.ends_with("index") => 'C',
            _ => 'A',
        };
    }
    if change == "precomp" {
        return 'R';
    }
    if row.starts_with("layer-marker") {
        // By index, `markers.set {comment}` reads neither the layer's start nor its stretch (the
        // marker is stored in layer time and only its comment is written); by comp time it
        // converts that time to layer time through both.
        let by_time = row.ends_with("time");
        return match change {
            "marker-at" => 'C',
            "marker-before" if !by_time => 'C',
            "stretch" if by_time => 'C',
            "layer-start" if by_time => 'R',
            _ => 'A',
        };
    }
    if row.starts_with("layer-") && row != "layer-timing" {
        return match change {
            "rename" => {
                if row == "layer-name" {
                    'R'
                } else {
                    'C'
                }
            }
            // `"#3"` is read as a stack position; a name is the first layer from the top with it,
            // which a layer put or taken above, or the target moved, leaves the same.
            "layer-insert-above" | "layer-delete-above" | "layer-reorder" if row == "layer-index" => 'R',
            _ => 'A',
        };
    }
    if row == "layer-timing" {
        return match change {
            // `layer.timing {delta}` shifts the layer's start, In and Out by the delta: it reads
            // them, never the layer's place in the stack.
            "layer-start" | "trim-in" | "trim-out" | "stretch" => 'R',
            _ => 'A',
        };
    }
    if row.starts_with("effect-") {
        return match change {
            "effect-at" => 'C',
            "effect-before" if row == "effect-index" => 'R',
            _ => 'A',
        };
    }
    if change == "layer-start" {
        return 'R';
    }
    if change == "stretch" {
        return 'C';
    }
    let changed = match row {
        "key-time" | "key-nearest" => matches!(change, "insert-at" | "delete-at" | "shift-keys" | "value-at" | "ease-left"),
        "point-add" => matches!(change, "insert-at"),
        "point-set" | "current-time" | "point-add-sampled" => {
            matches!(change, "insert-at" | "delete-at" | "shift-keys" | "value-at" | "ease-left" | "ease-right")
        }
        "key-move" => matches!(change, "insert-at" | "delete-at" | "shift-keys" | "value-at" | "ease-left"),
        "range-delete" | "range-shift" | "range-scale" => matches!(change, "insert-at" | "delete-at" | "shift-keys" | "value-at" | "ease-left" | "ease-right"),
        _ => panic!("missing specification for {row}"),
    };
    if changed { 'C' } else { 'A' }
}

fn seed() -> (Vec<u8>, u64, Vec<u64>) {
    let mut doc = MotionDocument::open(FILE, &project(5)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[2];
    for t in [1, 4, 6, 8] {
        doc.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":t,"value":t*10})).unwrap();
    }
    for t in [1, 4, 8] {
        doc.execute("comp.addMarker", json!({"time":t,"comment":format!("C{t}")})).unwrap();
        doc.execute("layer.addMarker", json!({"layer":layer,"time":t,"comment":format!("L{t}")})).unwrap();
    }
    let effects = ["Gaussian Blur", "Fill", "Gaussian Blur"]
        .into_iter()
        .map(|effect| doc.execute("effect.apply", json!({"layer":layer,"effect":effect})).unwrap()["effects"][0].as_u64().unwrap())
        .collect();
    (doc.save(None).unwrap(), layer, effects)
}

fn prop_mut<'a>(node: &'a mut Value, name: &str) -> &'a mut Value {
    if node["match"] == name {
        return node;
    }
    let children = node["children"].as_array_mut().unwrap();
    let i = children.iter().position(|n| n["match"] == name || n["children"].as_array().is_some_and(|cs| cs.iter().any(|n| n["match"] == name))).unwrap();
    prop_mut(&mut children[i], name)
}

fn change(f: &Fixture, comp: u64, layer: u64, effects: &[u64], row: &str, change: &str) {
    if change == "precomp-below" {
        let below = *layers_on_disk(&f.file()).last().unwrap();
        f.colleague(&[("layer.precompose", json!({"layers":[below],"name":"Below","open":false}))]);
        return;
    }
    if change == "precomp" {
        f.colleague(&[("layer.precompose", json!({"layers":[layer],"name":"Inner","open":false}))]);
        return;
    }
    rewrite(&f.file(), |p| {
        if change == "unrelated" {
            p["items"][comp.to_string()]["comment"] = json!("Colleague");
            return;
        }
        let c = &mut p["items"][comp.to_string()]["kind"];
        if change == "work-area" {
            c["work_area"] = json!([SECOND, 9 * SECOND]);
            return;
        }
        let layers = c["layers"].as_array_mut().unwrap();
        let i = layers.iter().position(|l| l["id"] == layer).unwrap();
        match change {
            "rename-below" => {
                layers.last_mut().unwrap()["name"] = json!("Renamed below");
                return;
            }
            "layer-insert-above" => {
                let mut new = layers[0].clone();
                new["id"] = json!(900_000);
                new["name"] = json!("New");
                layers.insert(0, new);
                return;
            }
            "layer-delete-above" => {
                layers.remove(0);
                return;
            }
            "layer-reorder" => {
                let target = layers.remove(i);
                layers.push(target);
                return;
            }
            "layer-insert-below" => {
                let mut new = layers[0].clone();
                new["id"] = json!(900_000);
                new["name"] = json!("New");
                layers.push(new);
                return;
            }
            "layer-delete-below" => {
                layers.pop().unwrap();
                return;
            }
            "layer-reorder-below" => {
                let n = layers.len();
                layers.swap(n - 1, n - 2);
                return;
            }
            _ => {}
        }
        if change.starts_with("marker-") {
            {
                let owner: &mut Value = c;
                let markers = owner["markers"].as_array_mut().unwrap();
                if change == "marker-at" {
                    markers[1]["comment"] = json!("Theirs");
                } else {
                    let mut m = markers[1].clone();
                    m["time"] = json!(if change == "marker-before" { 0 } else { 9 * SECOND });
                    markers.push(m);
                    markers.sort_by_key(|m| m["time"].as_i64().unwrap());
                }
            }
            let l = &mut c["layers"][i];
            let markers = l["markers"].as_array_mut().unwrap();
            if change == "marker-at" {
                markers[1]["comment"] = json!("Theirs");
            } else {
                let mut m = markers[1].clone();
                m["time"] = json!(if change == "marker-before" { 0 } else { 9 * SECOND });
                markers.push(m);
                markers.sort_by_key(|m| m["time"].as_i64().unwrap());
            }
            return;
        }
        let l = &mut c["layers"][i];
        match change {
            "layer-start" => {
                for name in ["start_time", "in_point", "out_point"] {
                    l[name] = json!(l[name].as_i64().unwrap() + SECOND);
                }
                return;
            }
            "stretch" => {
                l["stretch"] = json!(200);
                l["out_point"] = json!(20 * SECOND);
                return;
            }
            "rename" => {
                l["name"] = json!("Renamed");
                return;
            }
            "trim-in" => {
                l["in_point"] = json!(2 * SECOND);
                return;
            }
            "trim-out" => {
                l["out_point"] = json!(9 * SECOND);
                return;
            }
            _ => {}
        }
        if change.starts_with("effect-") {
            let children = prop_mut(&mut l["props"], "effects")["children"].as_array_mut().unwrap();
            if change == "effect-rename" {
                children[1]["name"] = json!("Renamed Fill");
            } else if change == "effect-at" {
                children[1]["enabled"] = json!(false);
            } else {
                let target = effects[if change == "effect-before" { 0 } else { 2 }];
                children.retain(|n| n["uid"] != target);
            }
            return;
        }
        let keys = prop_mut(&mut l["props"], "opacity")["keys"].as_array_mut().unwrap();
        match change {
            "insert-before" | "insert-at" | "insert-after" => {
                let mut k = keys[1].clone();
                let t = if change == "insert-before" {
                    SECOND / 2
                } else if change == "insert-after" {
                    9 * SECOND
                } else if matches!(row, "point-add" | "point-add-sampled" | "point-set" | "current-time") {
                    SECOND * 9 / 2
                } else {
                    4 * SECOND
                };
                k["time"] = json!(t);
                k["value"]["v"] = json!(33);
                keys.retain(|k| k["time"] != t);
                keys.push(k);
                keys.sort_by_key(|k| k["time"].as_i64().unwrap());
            }
            "delete-before" | "delete-at" | "delete-after" => {
                // The last key: in the four-key seed it is key 3 (8 s).
                let last = keys.len() - 1;
                keys.remove(if change == "delete-before" {
                    0
                } else if change == "delete-at" {
                    1
                } else {
                    last
                });
            }
            "shift-keys" => {
                for k in keys {
                    k["time"] = json!(k["time"].as_i64().unwrap() + SECOND);
                }
            }
            "value-at" => keys[1]["value"]["v"] = json!(33),
            "value-after" => keys[3]["value"]["v"] = json!(33),
            "ease-after" => keys[3]["out_interp"] = json!("Hold"),
            "ease-left" => keys[1]["out_interp"] = json!("Hold"),
            "ease-right" => {
                keys[2]["in_interp"] = json!("Bezier");
                keys[2]["in_ease"] = json!([{"speed":10,"influence":0.7}]);
            }
            _ => panic!("missing writer for {change}"),
        }
    });
}

fn steps(row: &str, layer: u64, effects: &[u64]) -> Vec<Value> {
    let command = |id: &str, params: Value| json!({"command":id,"params":params});
    let key = |id: &str, t: f64| command(id, json!({"layer":layer,"path":OPACITY,"time":t,"value":77}));
    match row {
        "key-time" => vec![key("keys.set", 4.0)],
        "key-nearest" => vec![key("keys.set", 4.4)],
        "point-add" => vec![key("prop.addKey", 4.5)],
        "point-add-sampled" => vec![command("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":4.5}))],
        "point-set" => vec![key("prop.set", 4.5)],
        "current-time" => vec![json!({"command":"prop.set","at":{"time":4.5},"params":{"layer":layer,"path":OPACITY,"value":77}})],
        "key-move" => vec![command("keys.set", json!({"layer":layer,"path":OPACITY,"time":4,"newTime":5}))],
        "layer-id" | "layer-index" | "layer-name" => vec![command(
            "layer.rename",
            json!({"layer":if row=="layer-id" {json!(layer)} else if row=="layer-index" {json!("#3")} else {json!("L2")},"name":"Mine"}),
        )],
        "layer-timing" => vec![command("layer.timing", json!({"layers":[layer],"delta":0.5}))],
        "work-area" => vec![command("comp.workArea", json!({"start":2,"end":7}))],
        "comp-marker-index" | "comp-marker-time" | "layer-marker-index" | "layer-marker-time" => {
            let mut p = json!({"comment":"Mine"});
            if row.starts_with("layer-") {
                p["layer"] = json!(layer);
            }
            p[if row.ends_with("index") { "index" } else { "at" }] = if row.ends_with("index") { json!(1) } else { json!(4) };
            vec![command("markers.set", p)]
        }
        "effect-id" | "effect-index" | "effect-name" => vec![command(
            "effect.remove",
            json!({"layer":layer,"effect":if row=="effect-id" {json!(effects[1])} else if row=="effect-index" {json!(2)} else {json!("Fill")}}),
        )],
        "range-delete" | "range-shift" | "range-scale" => vec![
            command("keys.select", json!({"keys":[{"layer":layer,"path":OPACITY,"time":4},{"layer":layer,"path":OPACITY,"time":6}]})),
            if row == "range-delete" {
                command("keys.delete", json!({}))
            } else if row == "range-shift" {
                command("keys.move", json!({"delta":0.25}))
            } else {
                command("keys.transform", json!({"timeScale":1.1,"timeAnchor":4}))
            },
        ],
        _ => panic!("missing action {row}"),
    }
}

/// The paths at which two saved projects differ (at most a few), for a merge assertion's message.
fn differences(a: &Value, b: &Value, at: &str, out: &mut Vec<String>) {
    if out.len() >= 8 || a == b {
        return;
    }
    match (a, b) {
        // The kit writes a whole number the colleague wrote as an integer as a float.
        (Value::Number(x), Value::Number(y)) if x.as_f64() == y.as_f64() => {}
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys().filter(|key| !x.contains_key(*key))) {
                differences(x.get(key).unwrap_or(&Value::Null), y.get(key).unwrap_or(&Value::Null), &format!("{at}.{key}"), out);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (x, y)) in x.iter().zip(y).enumerate() {
                differences(x, y, &format!("{at}.{i}"), out);
            }
        }
        _ => out.push(format!("{at}: {a} != {b}")),
    }
}

/// C8-time-policy item 7: an applied write must be the merge of both edits: the colleague's saved
/// file with the agent's edit (as it saw the document at `basis`) applied, and nothing else.
/// `None` for rows this check does not model.
fn merge_of(row: &str, comp: u64, layer: u64, saved: &[u8], now: &[u8]) -> Option<Result<(), String>> {
    let mut want: Value = serde_json::from_slice(saved).unwrap();
    let now: Value = serde_json::from_slice(now).unwrap();
    let c = &mut want["items"][comp.to_string()]["kind"];
    let target = c["layers"].as_array_mut().unwrap().iter_mut().find(|l| l["id"] == layer)?;
    // The observed marker 1 is at 4 s, comp and layer time alike at `basis` (the layer starts at 0).
    let marker = |markers: &mut Value| {
        let found = markers.as_array_mut().unwrap().iter_mut().find(|m| m["time"] == json!(4 * SECOND));
        found.map(|m| m["comment"] = json!("Mine")).is_some()
    };
    match row {
        "layer-timing" => {
            for name in ["start_time", "in_point", "out_point"] {
                target[name] = json!(target[name].as_i64().unwrap() + SECOND / 2);
            }
        }
        "layer-id" | "layer-index" | "layer-name" => target["name"] = json!("Mine"),
        "layer-marker-index" | "layer-marker-time" => {
            if !marker(&mut target["markers"]) {
                return Some(Err("the observed layer marker is gone".into()));
            }
        }
        "comp-marker-index" | "comp-marker-time" => {
            if !marker(&mut c["markers"]) {
                return Some(Err("the observed comp marker is gone".into()));
            }
        }
        _ => return None,
    }
    let mut found = Vec::new();
    differences(&want["items"], &now["items"], "items", &mut found);
    found.dedup();
    Some(if found.is_empty() { Ok(()) } else { Err(found.join("; ")) })
}

#[tokio::test(flavor = "multi_thread")]
async fn dense_time_matrix() {
    let (bytes, layer, effects) = seed();
    let mut counts = [0_usize; 4];
    let mut failures = Vec::new();
    for row in ROWS {
        for change_kind in CHANGES {
            let want = expected(row, change_kind);
            println!("EXPECT motion/{row}/{change_kind} {want}");
            let (f, comp, basis) = observed(&bytes).await;
            change(&f, comp, layer, &effects, row, change_kind);
            let saved = std::fs::read(f.file()).unwrap();
            let answer = f.call("run", json!({"basis":basis,"steps":steps(row,layer,&effects)})).await;
            counts[0] += 1;
            let reason = if want == 'R' { "relocated" } else { "changed" };
            let mismatch = match (&answer, want) {
                (Ok(_), 'A') => None,
                (Ok(_), _) => {
                    counts[1] += 1;
                    Some("wrong-write")
                }
                (Err(_), 'A') => {
                    counts[2] += 1;
                    Some("false-refusal")
                }
                (Err(e), _) if e["error"]["code"] == "conflict" && e["conflicts"].as_array().is_some_and(|cs| cs.iter().any(|c| c["reason"] == reason)) => None,
                (Err(_), _) => {
                    counts[3] += 1;
                    Some("wrong-reason")
                }
            };
            if answer.is_err() {
                assert_eq!(std::fs::read(f.file()).unwrap(), saved, "refusal saved {row}/{change_kind}");
            }
            if answer.is_ok()
                && want == 'A'
                && change_kind != "precomp"
                && let Some(merged) = merge_of(row, comp, layer, &saved, &std::fs::read(f.file()).unwrap())
            {
                println!("MERGE motion/{row}/{change_kind} {}", merged.as_ref().map_or_else(|e| e.as_str(), |()| "PASS"));
                if let Err(why) = merged {
                    failures.push(format!("{row}/{change_kind}: applied, not the merge of both edits: {why}"));
                }
            }
            println!("MATRIX motion/{row}/{change_kind} {want} {}", mismatch.unwrap_or("PASS"));
            if mismatch.is_some() {
                println!("DETAIL motion/{row}/{change_kind}: {answer:?}");
            }
            if let Some(why) = mismatch {
                failures.push(format!("{row}/{change_kind}: {why}: {answer:?}"));
            }
            f.shutdown().await;
        }
    }
    println!("COUNTS motion cells/wrong-writes/false-refusals/wrong-reasons {counts:?}");
    assert_eq!(counts, [660, 0, 0, 0], "SDK behavior changed: review the cells");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

async fn key_model(f: &Fixture, layer: u64) -> Value {
    let groups = f.ok("query", json!({"objects":[layer],"follow":"props.children"})).await;
    let transform = groups["objects"].as_array().unwrap().iter().find(|o| o["fields"]["meta"]["match"] == "transform").unwrap()["id"].clone();
    let props = f.ok("query", json!({"objects":[transform],"follow":"children"})).await;
    props["objects"].as_array().unwrap().iter().find(|o| o["fields"]["meta"]["match"] == "opacity").unwrap().clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn model_time_matrix_waiting_for_sdk() {
    // Raw operations use batchOps contracts and isolated prefix state. The necessity rules
    // below are declared before the outside edit. Every cell, structural control included,
    // must match its intended result (no cell waits for the SDK since kition-sdk 45bf9625).
    const FORMS: [(&str, &str); 11] = [
        ("key-id", "...................................."),
        ("key-index", "...................................."),
        ("key-create", "...................................."),
        ("key-insert", "...................................."),
        ("key-move", "...................................."),
        ("comp-markers", "...................................."),
        ("layer-markers", "...................................."),
        ("key-index-current-parent", "...................................."),
        ("key-create-current-parent", "...................................."),
        ("key-insert-current-parent", "...................................."),
        ("key-move-current-parent", "...................................."),
    ];
    let (bytes, layer, effects) = seed();
    // Raw `list.move` takes `to` as the gap before the removal (kition-sdk call-context.md), so
    // `key-move` moves key 1 (4 s) behind key 2 (6 s) with `from:1,to:3`: a gap between keys 2
    // and 3 (8 s), which the move holds. Its `delete-after` must delete a key after that gap but
    // not next to it, so its rows observe the seed with a fifth key (9.5 s) and delete that one.
    let trailing = {
        let mut doc = MotionDocument::open(FILE, &bytes).unwrap();
        doc.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":9.5,"value":95})).unwrap();
        doc.save(None).unwrap()
    };
    let mut counts = [0_usize; 4];
    let mut sdk_changes = Vec::new();
    let mut invalid_controls = Vec::new();
    for (row, pinned) in FORMS {
        assert_eq!(pinned.len(), CHANGES.len() + EXTRA.len() + 1);
        for (column, edit) in std::iter::once("none").chain(CHANGES).chain(EXTRA).enumerate() {
            // A marker is an object of its own (schema 1): a raw write of one of its fields holds
            // that marker and that field. Another marker added before or after it, or another field
            // of it changed (its comment), applies; the layer holding it moved elsewhere relocates it,
            // as it does a key.
            let want = if row.ends_with("markers") {
                match edit {
                    "precomp" if row.starts_with("layer") => 'R',
                    _ => 'A',
                }
            } else if row == "key-id" {
                match edit {
                    "delete-at" => 'D',
                    "insert-at" | "value-at" => 'C',
                    "precomp" => 'R',
                    _ => 'A',
                }
            } else {
                match edit {
                    "insert-before" | "delete-before" | "delete-at" | "precomp" => 'R',
                    "insert-at" | "value-at" | "ease-left" | "shift-keys" => 'C',
                    _ => 'A',
                }
            };
            println!("EXPECT motion/model/{row}/{edit} {want}");
            if row.ends_with("-current-parent") {
                println!("EXPECT motion/model/{row}/{edit}/property-reference stable");
            }
            let (f, comp, _) = observed(if row.starts_with("key-move") { &trailing } else { &bytes }).await;
            let property = key_model(&f, layer).await;
            let comp_loaded = f.ok("query", json!({"objects":[comp]})).await;
            let comp_owner = comp_loaded["objects"][0]["fields"]["comp"].clone();
            let owner = property["id"].as_str().unwrap();
            let key = property["fields"]["keys"][1].clone();
            let loaded = f.ok("query", json!({"objects":[key]})).await;
            let data = loaded["objects"][0]["fields"]["data"].clone();
            let (_, _, basis) = f.inspect().await;
            let fields = json!({"time":(SECOND*9/2).to_string(),"value":{"t":"Scalar","v":77},"data":data});
            let marker_owner = if row == "comp-markers" { comp_owner.clone() } else { json!(layer) };
            let marker = f.ok("query", json!({"objects":[marker_owner]})).await["objects"][0]["fields"]["markers"][1].clone();
            let base_row = row.strip_suffix("-current-parent").unwrap_or(row);
            let mut ops = match base_row {
                "key-id" => json!([{"op":"set","target":key,"field":"value","value":{"t":"Scalar","v":77}}]),
                "key-index" => json!([{"op":"list.remove","target":owner,"field":"keys","index":1}]),
                "key-create" => json!([{"op":"create","type":"Key","in":format!("{owner}.keys"),"at":2,"fields":fields}]),
                "key-insert" => {
                    json!([{"op":"create","type":"Key","ref":"$new","fields":fields},{"op":"list.insert","target":owner,"field":"keys","index":2,"item":"$new"}])
                }
                "key-move" => json!([{"op":"list.move","target":owner,"field":"keys","from":1,"to":3}]),
                _ => json!([{"op":"set","target":marker,"field":"chapter","value":"Mine"}]),
            };
            if edit != "none" {
                change(&f, comp, layer, &effects, if row.starts_with("key-") { "key-time" } else { row }, edit);
            }
            if row.ends_with("-current-parent") {
                let current = key_model(&f, layer).await;
                assert_eq!(current["fields"]["uid"], property["fields"]["uid"]);
                if edit == "none" {
                    assert_eq!(current["id"], property["id"], "a stable property UID retains its imported reference");
                } else {
                    assert_eq!(current["id"], property["id"], "a persistent property UID retains its reference through a file reread");
                }
                for op in ops.as_array_mut().unwrap() {
                    if op.get("target").is_some() {
                        op["target"] = current["id"].clone();
                    }
                    if op.get("in").is_some() {
                        op["in"] = json!(format!("{}.keys", current["id"].as_str().unwrap()));
                    }
                }
            }
            let saved = std::fs::read(f.file()).unwrap();
            let answer = f.call("batch", json!({"basis":basis,"ops":ops})).await;
            if edit == "none" && answer.is_err() {
                invalid_controls.push(format!("invalid model control {row}: {answer:?}"));
            }
            counts[0] += 1;
            let reason = match want {
                'R' => "relocated",
                'D' => "deleted",
                _ => "changed",
            };
            let mismatch = match (&answer, want) {
                (Ok(_), 'A') => None,
                (Ok(_), _) => {
                    counts[1] += 1;
                    Some("wrong-write")
                }
                (Err(_), 'A') => {
                    counts[2] += 1;
                    Some("false-refusal")
                }
                (Err(e), _) if e["error"]["code"] == "conflict" && e["conflicts"].as_array().is_some_and(|cs| cs.iter().any(|c| c["reason"] == reason)) => None,
                (Err(_), _) => {
                    counts[3] += 1;
                    Some("wrong-reason")
                }
            };
            if answer.is_err() {
                assert_eq!(std::fs::read(f.file()).unwrap(), saved);
            }
            println!("MATRIX motion/model/{row}/{edit} {want} {}", mismatch.unwrap_or("PASS"));
            let observed = match mismatch {
                Some("wrong-write") => b'W',
                Some("false-refusal") => b'F',
                Some("wrong-reason") => b'E',
                _ => b'.',
            };
            if pinned.as_bytes()[column] != observed {
                sdk_changes.push(format!("SDK behavior changed at {row}/{edit}: {answer:?}"));
            }
            if mismatch.is_some() {
                assert!(
                    answer.as_ref().err().is_none_or(|e| matches!(e["error"]["code"].as_str(), Some("conflict" | "not-found" | "invalid-value"))),
                    "unexpected batch failure {row}/{edit}: {answer:?}"
                );
                println!("WAITING-SDK motion/model/{row}/{edit}: generic batch premise: {answer:?}");
            }
            f.shutdown().await;
        }
    }
    println!("COUNTS motion/model cells/wrong-writes/false-refusals/wrong-reasons {counts:?}");
    assert!(invalid_controls.is_empty(), "{}", invalid_controls.join("\n"));
    assert_eq!(counts, [396, 0, 0, 0], "SDK batch behavior changed: review the cells");
    assert!(sdk_changes.is_empty(), "{}", sdk_changes.join("\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn remaining_time_and_effect_name_matrix() {
    let (bytes, layer, effects) = seed();
    let mut counts = [0_usize; 4];
    let mut failures = Vec::new();
    for row in ROWS {
        for edit in EXTRA {
            let want = if edit == "effect-rename" && row.starts_with("effect-") { if row == "effect-name" { 'R' } else { 'C' } } else { 'A' };
            println!("EXPECT motion/extra/{row}/{edit} {want}");
            let (f, comp, basis) = observed(&bytes).await;
            change(&f, comp, layer, &effects, row, edit);
            let saved = std::fs::read(f.file()).unwrap();
            let answer = f.call("run", json!({"basis":basis,"steps":steps(row,layer,&effects)})).await;
            counts[0] += 1;
            let reason = if want == 'R' { "relocated" } else { "changed" };
            let mismatch = match (&answer, want) {
                (Ok(_), 'A') => None,
                (Ok(_), _) => {
                    counts[1] += 1;
                    Some("wrong-write")
                }
                (Err(_), 'A') => {
                    counts[2] += 1;
                    Some("false-refusal")
                }
                (Err(e), _) if e["error"]["code"] == "conflict" && e["conflicts"].as_array().is_some_and(|cs| cs.iter().any(|c| c["reason"] == reason)) => None,
                _ => {
                    counts[3] += 1;
                    Some("wrong-reason")
                }
            };
            if answer.is_err() {
                assert_eq!(std::fs::read(f.file()).unwrap(), saved);
            }
            println!("MATRIX motion/extra/{row}/{edit} {want} {}", mismatch.unwrap_or("PASS"));
            if let Some(why) = mismatch {
                failures.push(format!("{row}/{edit}: {why}: {answer:?}"));
            }
            f.shutdown().await;
        }
    }
    println!("COUNTS motion/extra cells/wrong-writes/false-refusals/wrong-reasons {counts:?}");
    assert_eq!(counts, [110, 0, 0, 0]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn declarations_hold_both_selected_keys_and_a_layer_moved_to_another_comp() {
    let (bytes, layer, _) = seed();
    let mut doc = MotionDocument::open(FILE, &bytes).unwrap();
    let named = doc.under("layer.rename", &json!({"layer":"L2","name":"Mine"})).unwrap().unwrap();
    // C8-time-policy: a name is not a stack position; its place in the stack is not held.
    assert!(named.within.iter().all(|window| window.field != "comp.layers"), "{named:?}");
    assert!(named.premises.iter().all(|premise| premise.positions.as_ref().is_some_and(Vec::is_empty)));
    let selected =
        doc.under("keys.select", &json!({"keys":[{"layer":layer,"path":OPACITY,"time":4},{"layer":layer,"path":OPACITY,"time":6}]})).unwrap().unwrap();
    assert_eq!(selected.premises.len(), 1);
    let fields = selected.premises[0].fields.as_ref().unwrap();
    assert!(fields.contains(&format!("transform/opacity.keys[time={}]", 4 * SECOND)));
    assert!(fields.contains(&format!("transform/opacity.keys[time={}]", 6 * SECOND)));
    doc.execute("layer.precompose", json!({"layers":[layer],"name":"Inner","open":false})).unwrap();
    let under = doc.under("layer.rename", &json!({"layer":layer,"name":"Mine"})).unwrap().unwrap();
    assert!(under.within.iter().any(|window| window.field == "comp.layers"));
    assert!(under.premises.iter().any(|p| p.id == layer));
    assert!(under.premises.iter().any(|p| p.id == layer && p.positions.as_ref().is_some_and(|positions| positions.iter().any(|p| p == "comp.layers"))));
}
