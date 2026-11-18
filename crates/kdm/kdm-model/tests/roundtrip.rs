//! Nothing is lost on a round trip: every `.kmotion` sample in the repository, every one under
//! `KITION_MOTION_CORPUS` (a folder, searched recursively), and a synthetic project that uses
//! every kind of reference the model holds, imported into a replica, loaded back and exported,
//! are the very project they were (ids included: engine ids are the objects' own data), the id
//! counter aside.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::{add_layer, assert_same_content, demo, layer_mut, new_replica, read, set_key};
use kdm_model::MotionModel;
use kition_kit::session::{KitModel, MemoryBlobs};
use kition_motion_project::props::{Expression, GroupKind, Node, PropGroup, Property};
use kition_motion_project::render_queue::RenderQueueItem;
use kition_motion_project::{Footage, ItemKind, LayerSource, MatteKind, Project, TrackMatte, Value};
use serde_json::json;

fn collect(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 16 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, depth + 1, out);
        } else if matches!(path.extension().and_then(|e| e.to_str()), Some("kmotion" | "ecproj")) {
            out.push(path);
        }
    }
}

fn samples() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut found = Vec::new();
    collect(&root.join("web/public/sample"), 0, &mut found);
    found.sort();
    found
}

/// Import -> load -> export of `text`, compared with what `Project::from_json` reads.
fn round_trip(name: &str, text: &str) {
    let original = Project::from_json(text).unwrap_or_else(|error| panic!("{name}: not a project: {error}"));
    let mut blobs = MemoryBlobs::default();
    let replica = MotionModel::import(name, text.as_bytes(), &mut blobs, common::actor(1)).unwrap_or_else(|error| panic!("{name}: import: {error}"));
    let model = MotionModel::load(&replica, &blobs).unwrap_or_else(|error| panic!("{name}: load: {error}"));
    assert_same_content(&original, &model.project, &format!("{name}: loaded"));
    let exported = model.export(&replica, name, &blobs).unwrap_or_else(|error| panic!("{name}: export: {error}"));
    let reread = Project::from_json(std::str::from_utf8(&exported).unwrap_or_default()).unwrap_or_else(|error| panic!("{name}: exported file: {error}"));
    assert_same_content(&original, &reread, &format!("{name}: exported"));
    // A peer opening the replica's snapshot reads the same project too.
    let snapshot = replica.snapshot().unwrap_or_else(|error| panic!("{name}: snapshot: {error}"));
    let peer = kition_kit::kdm::Replica::load(MotionModel::schema(), &snapshot, common::actor(2)).unwrap_or_else(|error| panic!("{name}: reload: {error}"));
    assert_same_content(&original, &read(&peer), &format!("{name}: a peer's replica"));
}

#[test]
fn every_sample_round_trips() {
    let samples = samples();
    assert!(!samples.is_empty(), "no .kmotion samples found to round-trip");
    for path in samples {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        round_trip(&path.display().to_string(), &text);
    }
}

/// `KITION_MOTION_CORPUS=/path/to/projects cargo test -p kdm-model --test roundtrip corpus`.
#[test]
fn the_corpus_round_trips() {
    let Some(dir) = std::env::var_os("KITION_MOTION_CORPUS") else { return };
    let mut found = Vec::new();
    collect(Path::new(&dir), 0, &mut found);
    found.sort();
    assert!(!found.is_empty(), "KITION_MOTION_CORPUS names no folder of .kmotion (or the engine's own .ecproj) files");
    let mut checked = 0;
    for path in found {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        // Files the engine itself cannot open are not this crate's business.
        if Project::from_json(&text).is_err() {
            continue;
        }
        round_trip(&path.display().to_string(), &text);
        checked += 1;
    }
    eprintln!("round-tripped {checked} corpus projects");
}

/// A project with every kind of reference and every container the model has: folders, footage
/// by path, a precomposition layer, parenting, a track matte, a Layer Control value naming a
/// layer, an expression, markers, guides, Graphics Templates with a link, a Render Queue item, a
/// text layer and a shape layer, an effect group, a hold keyframe and an eased one; and the
/// oddities a file may hold (one uid twice in one tree, one layer id in two compositions).
fn rich_project() -> Project {
    let d = demo();
    let mut p = d.project;
    let (main, inner) = (d.main, d.inner);
    let folder = p.add_item("Footage", kition_motion_color::Label::Aqua, None, ItemKind::Folder);
    let sub = p.add_item("Stills", kition_motion_color::Label::None, Some(folder), ItemKind::Folder);
    let clip = p.add_item(
        "clip.mp4",
        kition_motion_color::Label::Blue,
        Some(sub),
        ItemKind::Footage(Footage { path: "media/clip.mp4".into(), width: 320, height: 180, has_video: true, ..Footage::default() }),
    );
    if let Some(item) = p.item_mut(inner) {
        item.parent = Some(folder);
        item.comment = "nested".into();
    }
    let footage = add_layer(&mut p, inner, "Clip", LayerSource::Footage { item: clip });
    let text = add_layer(&mut p, main, "Title", LayerSource::Text);
    let shape = add_layer(&mut p, main, "Shape", LayerSource::Shape);
    let pre = add_layer(&mut p, main, "Inner", LayerSource::Comp { item: inner });
    let null = add_layer(&mut p, main, "Null", LayerSource::Null);
    layer_mut(&mut p, main, d.red).parent = Some(null);
    layer_mut(&mut p, main, d.green).track_matte = Some(TrackMatte { layer: d.red, kind: MatteKind::LumaInverted });
    layer_mut(&mut p, main, text).markers = vec![serde_json::from_value(json!({ "time": 12_700_800_000i64, "comment": "beat" })).unwrap()];
    set_key(&mut p, main, shape, "transform/rotation", 0.5, Value::Scalar(45.0));
    {
        let opacity = layer_mut(&mut p, main, shape).props.prop_mut("transform/opacity").unwrap();
        opacity.expr = Some(Expression { text: "wiggle(2, 10)".into(), enabled: true });
        let mut held = kition_motion_project::Keyframe::new(common::seconds(1.5), Value::Scalar(20.0)).hold();
        held.label = 3;
        opacity.keys.push(held);
        opacity.keys.push(kition_motion_project::Keyframe::new(common::seconds(2.5), Value::Scalar(80.0)).eased());
    }
    // An effect with a Layer Control naming the null.
    let uid = p.alloc();
    let control_uid = p.alloc();
    {
        let effects = layer_mut(&mut p, main, pre).props.sub_mut("effects").unwrap();
        let mut fx = PropGroup::new(uid, "ec.layerControl", "Layer Control");
        fx.kind = GroupKind::Effect { effect: "ec.layerControl".into() };
        fx.children.push(Node::Prop(Property::new(control_uid, "layer", "Layer", Value::Layer(Some(null.0)))));
        effects.children.push(Node::Group(fx));
    }
    // One uid twice in the footage layer's tree (a file's oddity).
    {
        let tree = &mut layer_mut(&mut p, inner, footage).props;
        let dup = tree.prop("transform/opacity").unwrap().clone();
        tree.children.push(Node::Prop(dup));
    }
    if let Some(c) = p.comp_mut(main) {
        c.guides = vec![kition_motion_project::Guide { vertical: true, position: 120.0 }];
        c.markers = vec![serde_json::from_value(json!({ "time": 0, "comment": "start", "chapter": "One" })).unwrap()];
        let position_uid = c.layer(d.green).and_then(|l| l.props.prop("transform/position")).map(|p| p.uid).unwrap();
        let opacity_uid = c.layer(d.red).and_then(|l| l.props.prop("transform/opacity")).map(|p| p.uid).unwrap();
        c.graphics_template = Some(
            serde_json::from_value(json!({
                "name": "Lower third",
                "controls": [
                    { "id": 9001, "name": "Where", "kind": { "type": "Property", "layer": d.green.0, "prop": position_uid, "links": [{ "layer": d.red.0, "prop": opacity_uid }] } },
                    { "id": 9002, "name": "Note", "kind": { "type": "Comment", "text": "hi" } }
                ]
            }))
            .unwrap(),
        );
    }
    // One layer id in two compositions (another file oddity).
    {
        let mut twin = p.comp(main).and_then(|c| c.layer(null)).unwrap().clone();
        twin.parent = None;
        Arc::make_mut(match &mut p.item_mut(inner).unwrap().kind {
            ItemKind::Comp(c) => c,
            _ => unreachable!(),
        })
        .layers
        .push(twin);
    }
    p.render_queue.push(RenderQueueItem::new(1, main));
    p.fix_next_id();
    p
}

#[test]
fn a_project_using_every_reference_round_trips() {
    let project = rich_project();
    let text = project.to_file_json().unwrap();
    round_trip("rich.kmotion", &text);
}

#[test]
fn write_document_and_read_document_round_trip_exactly() {
    let project = rich_project();
    let (replica, ids) = new_replica(&project);
    let (read_back, read_ids) = kdm_model::read_document(&replica).unwrap();
    assert_same_content(&project, &read_back, "read back");
    assert_eq!(ids.items, read_ids.items, "the reader finds every item's object");
    assert_eq!(ids.comps, read_ids.comps);
    assert_eq!(ids.render, read_ids.render);
    for (key, lids) in &ids.layers {
        let got = read_ids.layers.get(key).unwrap_or_else(|| panic!("layer {key:?} not read"));
        assert_eq!(got.object, lids.object);
        assert_eq!(got.nodes, lids.nodes, "layer {key:?}: every node's object");
        assert_eq!(got.keys, lids.keys, "layer {key:?}: every key's object");
    }
    assert_eq!(ids.layers.len(), read_ids.layers.len());
    // The counter is past every id, in the reading actor's slot.
    assert!(read_back.next_id >= kdm_model::slot_base(replica.actor()));
}
