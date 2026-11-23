//! Exact comparisons use literal model paths. Untranslated paths remain unresolved, never equal.

use super::{Row, families};
use kition_kit::{KitDocument, Under};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod animation;
mod composition;
mod markers;
mod objects;
mod state;
mod styles;

struct Probe {
    params: Value,
    expected: BTreeSet<String>,
}

fn premise(id: Value, fields: &[&str]) -> BTreeSet<String> {
    let mut facts = BTreeSet::from([format!("exists:{id}"), format!("positions:{id}:[]")]);
    facts.extend(fields.iter().map(|field| format!("field:{id}:{field}")));
    facts
}

fn facts(answer: Option<Under>) -> BTreeSet<String> {
    let Some(answer) = answer else { return BTreeSet::from(["unanswered".to_owned()]) };
    let mut facts = extended_facts(&serde_json::to_value(&answer).unwrap());
    for object in answer.objects {
        facts.insert(format!("object:{object}"));
    }
    for held in answer.premises {
        facts.insert(format!("exists:{}", held.id));
        match held.fields {
            Some(fields) => facts.extend(fields.into_iter().map(|field| format!("field:{}:{field}", held.id))),
            None => {
                facts.insert(format!("fields:{}:*", held.id));
            }
        }
        match held.positions {
            Some(mut positions) => {
                positions.sort();
                facts.insert(format!("positions:{}:{}", held.id, json!(positions)));
            }
            None => {
                facts.insert(format!("positions:{}:*", held.id));
            }
        }
        for (path, identity) in held.identities {
            facts.insert(format!("identity:{}:{path}:{identity}", held.id));
        }
    }
    for window in answer.within {
        facts.insert(format!("within:{}", serde_json::to_string(&window).unwrap()));
    }
    if let Some(cover) = answer.covers {
        facts.insert(format!("cover:{}", serde_json::to_string(&cover).unwrap()));
    }
    facts
}

// The pinned SDK currently serializes none of these extensions. Keep extraction explicit
// so a later SDK addition is visible to the comparison rather than silently discarded.
fn extended_facts(answer: &Value) -> BTreeSet<String> {
    let mut facts = BTreeSet::new();
    for order in answer.get("orders").and_then(Value::as_array).into_iter().flatten() {
        let mut order = order.clone();
        if let Some(objects) = order.get_mut("objects").and_then(Value::as_array_mut) {
            objects.sort_by_key(Value::to_string);
        }
        facts.insert(format!("order:{}", serde_json::to_string(&order).unwrap()));
    }
    for content in answer.get("content").and_then(Value::as_array).into_iter().flatten() {
        facts.insert(format!("content:{}", serde_json::to_string(&content).unwrap()));
    }
    for input in answer.get("inputs").and_then(Value::as_array).into_iter().flatten() {
        if let Some(input) = input.as_str() {
            facts.insert(format!("input:{input}"));
        }
    }
    facts
}

fn probe(row: &Row, comp: u64, layer: u64) -> Option<Probe> {
    let (params, expected) = match row.command {
        "face.model.select" | "roto.model.select" => (json!({"id":"classical","wait":true}), BTreeSet::new()),
        "face.model.install" | "roto.model.install" => (json!({"path":"weights.bin"}), BTreeSet::new()),
        "face.model.download" | "face.model.remove" => (json!({"id":"mediapipe-face"}), BTreeSet::new()),
        "roto.model.download" | "roto.model.remove" => (json!({"id":"mobilesam"}), BTreeSet::new()),
        "project.setProjectComment" => (json!({"comment":"replacement"}), premise(json!("project"), &["attrs.settings.comment"])),
        "renderQueue.setNotify" => (json!({"notify":true}), premise(json!("project"), &["attrs.render_prefs.notify"])),
        "renderQueue.setOverflowFolders" => (json!({"folders":null}), premise(json!("project"), &["attrs.render_prefs.overflow_folders"])),
        "view.clearGuides" => (json!({"comp":comp}), premise(json!(comp), &["comp.guides"])),
        "view.moveGuide" => (json!({"comp":comp,"index":0,"position":24}), premise(json!(comp), &["comp.guides.0"])),
        "view.removeGuide" => (json!({"comp":comp,"index":0}), premise(json!(comp), &["comp.guides.0"])),
        "layer.rename" => (json!({"layer":layer,"name":"replacement"}), premise(json!(layer), &["name"])),
        "layer.setComment" => (json!({"layer":layer,"comment":"replacement"}), premise(json!(layer), &["attrs.comment"])),
        "layer.setBlendMode" => (
            if row.path == "step" { json!({"layer":layer,"step":1}) } else { json!({"layer":layer,"mode":"normal"}) },
            premise(json!(layer), &["attrs.blend_mode"]),
        ),
        "layer.quality" => (json!({"layer":layer,"quality":"best"}), premise(json!(layer), &["switches.quality"])),
        "layer.sampling" => (json!({"layer":layer,"sampling":"bilinear"}), premise(json!(layer), &["switches.sampling"])),
        "layer.frameBlending" => (json!({"layer":layer,"mode":"off"}), premise(json!(layer), &["switches.frame_blend"])),
        "comp.workArea" => (
            match row.path {
                "set begin at CTI" => json!({"comp":comp,"set":"begin"}),
                "set end at CTI" => json!({"comp":comp,"set":"end"}),
                "explicit start/end or normalization only" => json!({"comp":comp,"start":0,"end":1}),
                _ => return None,
            },
            premise(json!(comp), &["comp.attrs.work_area", "comp.attrs.frame_rate", "comp.attrs.duration"]),
        ),
        _ => return None,
    };
    Some(Probe { params, expected })
}

// Coverage cannot trade one established path for another at the same count.
const COMPARED_FLOOR: &[(&str, &str)] = &[
    ("project.setProjectComment", "absolute project comment assignment"),
    ("renderQueue.setNotify", "absolute bool or toggle"),
    ("renderQueue.setOverflowFolders", "folder list or null/absent clear"),
    ("view.clearGuides", "all guides"),
    ("view.moveGuide", "guide by zero-based index"),
    ("view.removeGuide", "guide by zero-based index"),
    ("layer.rename", "absolute assignment"),
    ("layer.setComment", "absolute assignment"),
    ("layer.setBlendMode", "absolute mode"),
    ("layer.setBlendMode", "step"),
    ("layer.quality", "assign"),
    ("layer.sampling", "assign"),
    ("layer.frameBlending", "assign"),
    ("comp.workArea", "set begin at CTI"),
    ("comp.workArea", "set end at CTI"),
    ("comp.workArea", "explicit start/end or normalization only"),
    ("face.model.select", "classical or published model preference; optional synchronous activation"),
    ("roto.model.select", "classical or published model preference; optional synchronous activation"),
    ("face.model.install", "kit has no settings/models folder: error before filesystem read"),
    ("roto.model.install", "kit has no settings/models folder: error before filesystem read"),
    ("face.model.download", "kit has no model folder: error before download"),
    ("roto.model.download", "kit has no model folder: error before download"),
    ("face.model.remove", "absent model returns false; desktop deletes installed weights"),
    ("roto.model.remove", "absent model returns false; desktop deletes installed weights"),
];

#[test]
fn every_frozen_path_has_an_explicit_comparison_status() {
    let mut document = MotionDocument::create("r2-comparison", 64, 64).unwrap();
    let comp = document.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let layer = document.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
    document.execute("view.addGuide", json!({"position":12})).unwrap();
    let writes: BTreeSet<_> = document.commands().into_iter().filter(|command| command.mutates).map(|command| command.id).collect();
    let mut report = Vec::new();
    let mut unresolved = BTreeSet::new();
    let mut differing = BTreeSet::new();
    let mut commands = BTreeSet::new();
    let mut compared_paths = BTreeSet::new();
    let mut family_counts = BTreeMap::<&str, (usize, usize)>::new();
    for (family, rows) in families() {
        for (index, row) in rows.iter().enumerate() {
            let id = format!("{family}/{index}:{}:{}", row.command, row.path);
            let advertised = writes.contains(row.command);
            if advertised {
                commands.insert(row.command);
            }
            let counts = family_counts.entry(family).or_default();
            counts.0 += 1;
            let comparison = if let Some(probe) = probe(row, comp, layer) {
                compared_paths.insert((row.command, row.path));
                counts.1 += 1;
                let actual = facts(document.under(row.command, &probe.params).unwrap());
                let less: Vec<_> = probe.expected.difference(&actual).cloned().collect();
                let more: Vec<_> = actual.difference(&probe.expected).cloned().collect();
                if advertised && (!less.is_empty() || !more.is_empty()) {
                    differing.insert(row.command);
                }
                json!({"params":probe.params,"required":probe.expected,"actual":actual,"less":less,"more":more})
            } else {
                unresolved.insert(id.clone());
                json!({"unresolved":"No executable path fixture and complete literal dependency oracle yet."})
            };
            report.push(json!({
                "id":id,"family":family,"command":row.command,"path":row.path,
                "advertised":advertised,"pinned":family.starts_with("pinned_r2_"),
                "resolves":row.resolves,"reads":row.reads,"checks":row.checks,"writes":row.writes,"state":row.state,
                "source":row.source,"comparison":comparison
            }));
        }
    }
    println!(
        "R2 COMPARISON: {} paths; {} advertised commands; {} literal probes; {} unresolved paths; {} literal-probe mismatches (not an exhaustive rate)",
        report.len(),
        commands.len(),
        report.len() - unresolved.len(),
        unresolved.len(),
        differing.len()
    );
    for (family, (total, compared)) in family_counts {
        println!("R2 FAMILY {family}: {compared}/{total} compared");
    }
    if let Ok(path) = std::env::var("MOTION_R2_COMPARISON_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"exhaustive":unresolved.is_empty(),"paths":report,"unresolved":unresolved,"differing_literal_probes":differing,"editor_state":state::audit(&document),"composition_projection":composition::analyze()})).unwrap()).unwrap();
    }
    assert_eq!(report.len(), 652, "the frozen path inventory changed");
    assert_eq!(commands.len(), 486, "the advertised command inventory changed");
    // This is an upper bound on unfinished work, not permission to count it as a match.
    for path in COMPARED_FLOOR {
        assert!(compared_paths.contains(path), "an established r2 comparison path disappeared: {path:?}");
    }
    assert!(unresolved.len() <= 628, "r2 comparison coverage regressed: {} unresolved", unresolved.len());
    assert!(differing.is_empty(), "a compared path differs: {differing:?}");
}

#[test]
fn relative_order_content_and_external_inputs_are_not_invisible_to_the_comparison() {
    let under = json!({
        "objects":[],
        "orders":[{"parent":1,"field":"comp.layers","objects":[3,2]}],
        "content":[{"id":1,"field":"comp.guides","region":{"gap":0}}],
        "inputs":["footage:2"]
    });
    let actual = extended_facts(&under);
    assert!(actual.contains("order:{\"field\":\"comp.layers\",\"objects\":[2,3],\"parent\":1}"), "{actual:?}");
    assert!(actual.contains("content:{\"field\":\"comp.guides\",\"id\":1,\"region\":{\"gap\":0}}"), "{actual:?}");
    assert!(actual.contains("input:footage:2"), "{actual:?}");
}
