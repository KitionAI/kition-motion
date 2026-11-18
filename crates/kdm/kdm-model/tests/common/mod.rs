//! Shared test helpers: a small project to edit, and a two-replica harness over
//! `KitSession<MotionModel>`. Included by several `tests/*.rs` binaries, each of which uses only
//! some of these.
#![allow(dead_code)]

use std::sync::Arc;

use kdm_model::MotionModel;
use kition_kit::kdm::{ActorId, DocId, Origin, Replica};
use kition_kit::session::{KitModel, KitSession, MemoryBlobs};
use kition_motion_color::Label;
use kition_motion_project::{Comp, ItemId, ItemKind, Keyframe, Layer, LayerId, LayerSource, Project, Solid, Value, build};
use kition_motion_time::{FrameRate, Tick};

pub type Session = KitSession<MotionModel>;

pub fn seconds(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

/// A solid item of `color`.
pub fn solid(project: &mut Project, name: &str, color: [f32; 3]) -> ItemId {
    project.add_item(name, Label::None, None, ItemKind::Solid(Solid { color, width: 640, height: 360, pixel_aspect: 1.0 }))
}

/// A new composition item, 640 × 360 at 30 fps, `duration` seconds long.
pub fn comp(project: &mut Project, name: &str, duration: f64) -> ItemId {
    project.add_item(name, Label::None, None, ItemKind::Comp(Arc::new(Comp::new(640, 360, FrameRate::FPS_30, seconds(duration)))))
}

/// Appends a layer showing `source` to composition `cid` (at the bottom of its stack).
pub fn add_layer(project: &mut Project, cid: ItemId, name: &str, source: LayerSource) -> LayerId {
    let comp = project.comp(cid).unwrap_or_else(|| panic!("no comp {cid:?}")).clone();
    let layer = build::layer(project, &comp, name, source, (640, 360), None);
    let id = layer.id;
    project.comp_mut(cid).unwrap_or_else(|| panic!("no comp")).layers.push(layer);
    id
}

pub fn layer_mut(project: &mut Project, cid: ItemId, id: LayerId) -> &mut Layer {
    project.comp_mut(cid).and_then(|c| c.layer_mut(id)).unwrap_or_else(|| panic!("no layer {id:?}"))
}

pub fn layer(project: &Project, cid: ItemId, id: LayerId) -> &Layer {
    project.comp(cid).and_then(|c| c.layer(id)).unwrap_or_else(|| panic!("no layer {id:?}"))
}

/// Adds (or replaces) a key of `path` at `time` with a scalar or vector `value`.
pub fn set_key(project: &mut Project, cid: ItemId, id: LayerId, path: &str, time: f64, value: Value) {
    let prop = layer_mut(project, cid, id).props.prop_mut(path).unwrap_or_else(|| panic!("no property {path}"));
    kition_motion_project::keyframe::set_key(&mut prop.keys, Keyframe::new(seconds(time), value));
}

pub fn keys(project: &Project, cid: ItemId, id: LayerId, path: &str) -> Vec<Keyframe> {
    layer(project, cid, id).props.prop(path).unwrap_or_else(|| panic!("no property {path}")).keys.clone()
}

/// A project with a main composition of three solid layers (Red over Green over Blue), Red's
/// opacity and Green's position animated, and a precomposition `Inner` used by no layer yet.
pub struct Demo {
    pub project: Project,
    pub main: ItemId,
    pub inner: ItemId,
    pub red: LayerId,
    pub green: LayerId,
    pub blue: LayerId,
}

pub fn demo() -> Demo {
    let mut project = Project::default();
    let main = comp(&mut project, "Main", 4.0);
    let inner = comp(&mut project, "Inner", 2.0);
    let red_solid = solid(&mut project, "Red Solid", [1.0, 0.0, 0.0]);
    let green_solid = solid(&mut project, "Green Solid", [0.0, 1.0, 0.0]);
    let blue_solid = solid(&mut project, "Blue Solid", [0.0, 0.0, 1.0]);
    let red = add_layer(&mut project, main, "Red", LayerSource::Solid { item: red_solid });
    let green = add_layer(&mut project, main, "Green", LayerSource::Solid { item: green_solid });
    let blue = add_layer(&mut project, main, "Blue", LayerSource::Solid { item: blue_solid });
    set_key(&mut project, main, red, "transform/opacity", 0.0, Value::Scalar(0.0));
    set_key(&mut project, main, red, "transform/opacity", 1.0, Value::Scalar(100.0));
    set_key(&mut project, main, green, "transform/position", 0.0, Value::Vec3([0.0, 0.0, 0.0]));
    set_key(&mut project, main, green, "transform/position", 2.0, Value::Vec3([640.0, 360.0, 0.0]));
    Demo { project, main, inner, red, green, blue }
}

pub fn actor(value: u64) -> ActorId {
    ActorId::from_u64(value).unwrap_or_else(|error| panic!("actor {value}: {error}"))
}

pub fn person(name: &str) -> Origin {
    Origin::person(format!("user:{name}"))
}

/// A fresh replica holding `project`, with the ids its write minted.
pub fn new_replica(project: &Project) -> (Replica, kdm_model::Ids) {
    let mut ids = kdm_model::Ids::default();
    let (replica, ()) = Replica::create_with(MotionModel::schema(), DocId::new(), actor(1), &Origin::system("create"), |tx| {
        kdm_model::write_document(tx, project, &mut ids).map(|_| ())
    })
    .unwrap_or_else(|error| panic!("create: {error}"));
    (replica, ids)
}

/// A fresh replica + session holding `project`.
pub fn first(project: &Project) -> Session {
    let (replica, _) = new_replica(project);
    KitSession::from_replica(replica, Box::new(MemoryBlobs::default())).unwrap_or_else(|error| panic!("attach: {error}"))
}

/// A peer session starting from a snapshot of `of`, under a different actor.
pub fn peer(of: &Session, value: u64) -> Session {
    let snapshot = of.snapshot().unwrap_or_else(|error| panic!("snapshot: {error}"));
    KitSession::open_snapshot(&snapshot, Box::new(MemoryBlobs::default()), actor(value)).unwrap_or_else(|error| panic!("open_snapshot: {error}"))
}

/// Exchanges changes until both replicas converge (or panics after a generous number of rounds).
pub fn sync(a: &mut Session, b: &mut Session) {
    for _ in 0..8 {
        a.import_changes(&b.export_changes(&a.version()).unwrap_or_else(|error| panic!("export: {error}"))).unwrap_or_else(|error| panic!("import: {error}"));
        b.import_changes(&a.export_changes(&b.version()).unwrap_or_else(|error| panic!("export: {error}"))).unwrap_or_else(|error| panic!("import: {error}"));
        if a.version() == b.version() {
            return;
        }
    }
    panic!("the replicas did not converge");
}

/// Asserts `a` and `b` hold the same content, id counters aside (the first difference shown).
pub fn assert_same_content(a: &Project, b: &Project, what: &str) {
    let left = serde_json::to_value(kdm_model::normalize(a)).unwrap_or_else(|error| panic!("serializing a: {error}"));
    let right = serde_json::to_value(kdm_model::normalize(b)).unwrap_or_else(|error| panic!("serializing b: {error}"));
    if left != right {
        panic!("{what}: the projects differ: {:#?}", differences(&left, &right));
    }
}

/// The first few paths at which two JSON values differ.
pub fn differences(a: &serde_json::Value, b: &serde_json::Value) -> Vec<String> {
    use serde_json::Value as J;
    fn walk(a: &J, b: &J, path: String, out: &mut Vec<String>) {
        if out.len() >= 5 || a == b {
            return;
        }
        match (a, b) {
            (J::Object(x), J::Object(y)) => {
                for key in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                    walk(x.get(key).unwrap_or(&J::Null), y.get(key).unwrap_or(&J::Null), format!("{path}.{key}"), out);
                }
            }
            (J::Array(x), J::Array(y)) if x.len() == y.len() => {
                for (i, (p, q)) in x.iter().zip(y).enumerate() {
                    walk(p, q, format!("{path}[{i}]"), out);
                }
            }
            _ => {
                let short = |v: &J| v.to_string().chars().take(300).collect::<String>();
                out.push(format!("{path}: {} vs {}", short(a), short(b)));
            }
        }
    }
    let mut out = Vec::new();
    walk(a, b, String::new(), &mut out);
    out
}

/// Writes the diff between the session's current project and `after` as one transaction, then
/// makes `after` the session's view (as the engine's command bridge does).
pub fn record(session: &mut Session, origin: &Origin, after: &Project) {
    let mut ids = session.model().ids().clone();
    let before = session.model().project.clone();
    session
        .record(origin, |tx| {
            kdm_model::write_changes(tx, &before, after, &mut ids).map(|_| ()).map_err(|error| kition_kit::session::KitError::new(error.to_string()))
        })
        .unwrap_or_else(|error| panic!("record: {error}"));
    *session.model_mut() = MotionModel::from_parts(after.clone(), ids);
}

/// The project as the replica holds it, read back fresh.
pub fn read(replica: &Replica) -> Project {
    kdm_model::read_document(replica).unwrap_or_else(|error| panic!("read: {error}")).0
}
