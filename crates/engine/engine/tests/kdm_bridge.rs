//! The command bridge to the project's KDM replica: every mutating command replicates as one
//! transaction and a replica that imports it holds the same project; undo after a file reread
//! keeps the file's change; a coalesced drag is one undo step; the playhead and selections write
//! nothing; a history jump replicates as one change; ids a draft and the document mint never
//! collide, and a draft's merge is repaired; random commands, undos and redos keep the replica
//! holding the project; a replica that refuses a write is left (a visible reset) and a fresh one
//! is made on the next request.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use kition_motion_engine::{Origin, Services, Session};
use kition_motion_project::LayerId;
use serde_json::json;

/// A three-layer composition (Red over Green over Blue), Red's opacity animated, with its replica.
fn new_session() -> Session {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Main", "width": 320, "height": 180, "frameRate": 30, "duration": 4})).unwrap();
    for (name, color) in [("Blue", "#0000ff"), ("Green", "#00ff00"), ("Red", "#ff0000")] {
        s.execute("layer.newSolid", json!({"name": name, "color": color})).unwrap();
    }
    let red = layer(&s, "Red");
    s.execute("prop.addKey", json!({"layer": red, "path": "transform/opacity", "time": 0, "value": 0})).unwrap();
    s.execute("prop.addKey", json!({"layer": red, "path": "transform/opacity", "time": 1, "value": 100})).unwrap();
    assert!(s.ensure_kdm().is_some(), "the replica is made on request");
    s
}

/// A second session holding a replica of `a`'s project (open on its composition): a reader that
/// takes `a`'s later changes in order, or a draft of `a`.
fn peer(a: &mut Session) -> Session {
    let mut b = Session::default();
    b.open_snapshot(&a.snapshot().unwrap()).unwrap();
    assert!(b.active_comp().is_some(), "the peer opens the composition");
    b
}

fn send(from: &mut Session, to: &mut Session) {
    let since = to.version().unwrap();
    let bytes = from.export_changes(&since).unwrap();
    to.import_changes(&bytes).unwrap();
}

/// Takes `draft`'s changes since `base` into `doc` as the person's one step (`mergeChanges`).
fn merge(doc: &mut Session, draft: &mut Session, base: &kition_motion_engine::Version) {
    let bytes = draft.export_changes(base).unwrap();
    doc.merge_draft(&bytes, &kition_motion_engine::kdm_bridge::local_person()).unwrap();
}

/// `a` and `b` hold the same replicated state and the same project (id counters aside).
fn assert_same(a: &mut Session, b: &mut Session, what: &str) {
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap(), "{what}: replicas differ");
    let left = serde_json::to_value(kdm_model::normalize(&a.project)).unwrap();
    let right = serde_json::to_value(kdm_model::normalize(&b.project)).unwrap();
    assert!(left == right, "{what}: projects differ");
}

/// The project as read back fresh from the session's replica.
fn replica_project(s: &Session) -> kition_motion_project::Project {
    kdm_model::read_document(s.kdm().expect("a replica").replica()).unwrap().0
}

fn holds(s: &Session) -> bool {
    kdm_model::same_content(&replica_project(s), &s.project)
}

fn layer(s: &Session, name: &str) -> u64 {
    s.active_comp().unwrap().layers.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no layer {name}")).id.0
}

fn names(s: &Session) -> Vec<String> {
    s.active_comp().unwrap().layers.iter().map(|l| l.name.clone()).collect()
}

fn opacity_keys(s: &Session, id: u64) -> Vec<f64> {
    let l = s.active_comp().unwrap().layer(LayerId(id)).unwrap();
    l.props.prop("transform/opacity").unwrap().keys.iter().map(|k| k.value.components().first().copied().unwrap_or(f64::NAN)).collect()
}

fn top_steps(s: &Session) -> u32 {
    let state = s.history.undo.last().map(|e| e.1.clone()).unwrap();
    s.history.kdm.undo_steps(&state)
}

#[test]
fn one_command_one_transaction_reproduces_in_a_peer() {
    let cases: Vec<(&str, fn(&mut Session))> = vec![
        ("prop.set on an animated property (a keyframe edit)", |s| {
            let red = layer(s, "Red");
            s.execute("prop.set", json!({"layer": red, "path": "transform/opacity", "value": 40, "time": 1})).unwrap();
        }),
        ("prop.addKey", |s| {
            let green = layer(s, "Green");
            s.execute("prop.addKey", json!({"layer": green, "path": "transform/rotation", "time": 2, "value": 90})).unwrap();
        }),
        ("layer.arrange (a layer move)", |s| {
            let blue = layer(s, "Blue");
            s.execute("layer.arrange", json!({"layers": [blue], "to": "front"})).unwrap();
        }),
        ("layer.rename", |s| {
            let green = layer(s, "Green");
            s.execute("layer.rename", json!({"layer": green, "name": "Lime"})).unwrap();
        }),
        ("edit.delete (a layer deleted)", |s| {
            let green = layer(s, "Green");
            s.execute("edit.delete", json!({"layers": [green]})).unwrap();
        }),
        ("layer.setParent", |s| {
            let (red, blue) = (layer(s, "Red"), layer(s, "Blue"));
            s.execute("layer.setParent", json!({"layers": [red], "parent": blue})).unwrap();
        }),
        ("layer.newSolid", |s| {
            s.execute("layer.newSolid", json!({"name": "White", "color": "#ffffff"})).unwrap();
        }),
        ("engine.batch", |s| {
            s.execute(
                "engine.batch",
                json!({"steps": [
                    {"command": "layer.newNull", "params": {}},
                    {"command": "prop.set", "params": {"layer": "$1.layer", "path": "transform/opacity", "value": 40}}
                ]}),
            )
            .unwrap();
        }),
    ];
    for (name, run) in cases {
        let mut a = new_session();
        let mut b = peer(&mut a);
        let version = a.version().unwrap();
        run(&mut a);
        assert_ne!(a.version().unwrap(), version, "{name}: wrote a change");
        assert_eq!(top_steps(&a), 1, "{name}: exactly one replica undo step");
        assert!(holds(&a), "{name}: the replica holds the project");
        send(&mut a, &mut b);
        assert_same(&mut a, &mut b, name);
    }
}

#[test]
fn ids_are_the_same_on_every_replica() {
    let mut a = new_session();
    let b = peer(&mut a);
    let ids = |s: &Session| s.active_comp().unwrap().layers.iter().map(|l| (l.id, l.props.prop("transform/opacity").unwrap().uid)).collect::<Vec<_>>();
    assert_eq!(ids(&a), ids(&b), "layer ids and property uids are the replica's own data");
}

/// A file changed outside and read again (`reconcile_reread`: an `import` change, no undo step)
/// between the person's edit and their undo: the undo takes back only the person's edit, and the
/// file's change stays; a reader of the replica follows each step.
#[test]
fn undo_after_a_file_reread_keeps_it_and_reverts_only_the_local_one() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let (red, green) = (layer(&a, "Red"), layer(&a, "Green"));
    a.execute("prop.set", json!({"layer": red, "path": "transform/opacity", "value": 30, "time": 1})).unwrap();
    let mut file = (*a.project).clone();
    let main = *file.comps().find(|(_, c)| c.layer(LayerId(green)).is_some()).unwrap().0;
    file.comp_mut(main).unwrap().layer_mut(LayerId(green)).unwrap().name = "Outside".into();
    a.reconcile_reread(&file).unwrap();
    assert!(names(&a).contains(&"Outside".to_string()), "the file's rename arrived");
    send(&mut a, &mut b);
    assert_same(&mut a, &mut b, "read");

    assert!(a.undo(), "the local edit undoes");
    assert_eq!(opacity_keys(&a, red), vec![0.0, 100.0], "the local key edit is gone");
    assert!(names(&a).contains(&"Outside".to_string()), "the file's rename survives the local undo");
    assert!(holds(&a));
    send(&mut a, &mut b);
    assert_same(&mut a, &mut b, "after the undo");
    assert!(a.redo());
    assert_eq!(opacity_keys(&a, red), vec![0.0, 30.0]);
    send(&mut a, &mut b);
    assert_same(&mut a, &mut b, "after the redo");
}

#[test]
fn single_user_undo_keeps_the_engines_own_states() {
    let mut s = new_session();
    let saved = s.project.clone();
    s.mark_saved();
    let red = layer(&s, "Red");
    s.execute("layer.rename", json!({"layer": red, "name": "Ruby"})).unwrap();
    assert!(s.is_dirty());
    assert!(s.undo());
    assert!(Arc::ptr_eq(&s.project, &saved), "the very state the session saved");
    assert!(!s.is_dirty(), "back at the saved state");
    assert!(holds(&s));
}

#[test]
fn a_coalesced_drag_is_one_undo_step() {
    let mut s = new_session();
    let red = layer(&s, "Red");
    let past = s.history.undo.len();
    for v in [90.0, 70.0, 50.0, 30.0] {
        s.execute("prop.set", json!({"layer": red, "path": "transform/rotation", "value": v, "merge": "drag"})).unwrap();
    }
    assert_eq!(s.history.undo.len(), past + 1, "one history step");
    assert_eq!(top_steps(&s), 1, "one replica undo step for the whole drag");
    assert!(holds(&s));
    assert!(s.undo());
    let rotation = |s: &Session| s.active_comp().unwrap().layer(LayerId(red)).unwrap().props.prop("transform/rotation").unwrap().value.components()[0];
    assert_eq!(rotation(&s), 0.0, "one undo takes the whole drag back");
    assert!(holds(&s));
    assert!(s.redo());
    assert_eq!(rotation(&s), 30.0);
    assert!(holds(&s));
}

#[test]
fn playhead_and_selection_changes_write_nothing() {
    let mut a = new_session();
    let b = peer(&mut a);
    let version = a.version().unwrap();
    a.execute("time.set", json!({"time": 2.0})).unwrap();
    let red = layer(&a, "Red");
    a.execute("layer.select", json!({"layers": [red]})).unwrap();
    assert_eq!(a.version().unwrap(), version, "no transaction");
    assert!(!a.state.selected_layers.is_empty());
    assert!(b.state.selected_layers.is_empty(), "the peer's own state is untouched");
}

/// Kition platform KitModel 1.1 M12a: on one replica, the person's undo and an agent's undo each
/// take back only their own steps and keep the other's.
#[test]
fn agent_edits_undo_in_their_own_scope() {
    let mut s = new_session();
    let agent = Origin::agent("test");
    let red = layer(&s, "Red");
    s.execute_as("layer.rename", json!({"layer": red, "name": "Agent"}), &agent).unwrap();
    assert_eq!(s.origin(), kition_motion_engine::kdm_bridge::local_person(), "the origin is restored after the call");
    assert!(names(&s).contains(&"Agent".to_string()));
    s.undo();
    assert!(names(&s).contains(&"Agent".to_string()), "the person's undo keeps the agent's edit");
    s.execute_as("edit.undo", json!({}), &agent).unwrap();
    assert!(!names(&s).contains(&"Agent".to_string()), "the agent's undo takes it back");
    assert!(holds(&s));
}

/// M12a: an agent's undo of a step under the person's later edit keeps the person's edit, and
/// `edit.undo`'s `scope` (engine rule E14a) names whose step to undo.
#[test]
fn an_agent_undo_under_the_persons_later_step_keeps_the_persons_edit() {
    let mut s = new_session();
    let agent = Origin::agent("test");
    let red = layer(&s, "Red");
    s.execute_as("layer.rename", json!({"layer": red, "name": "Agent"}), &agent).unwrap();
    s.execute("layer.newSolid", json!({"name": "Person"})).unwrap();
    s.execute("edit.undo", json!({"scope": "agent"})).unwrap();
    assert!(!names(&s).contains(&"Agent".to_string()), "the agent's step is undone: {:?}", names(&s));
    assert!(names(&s).contains(&"Person".to_string()), "the person's later edit stays: {:?}", names(&s));
    s.execute_as("edit.redo", json!({}), &agent).unwrap();
    assert!(names(&s).contains(&"Agent".to_string()) && names(&s).contains(&"Person".to_string()), "{:?}", names(&s));
    assert!(s.execute("edit.undo", json!({"scope": "other"})).is_err(), "an unknown scope is refused");
    assert!(holds(&s));
}

#[test]
fn layers_a_draft_and_the_document_add_both_survive_under_distinct_ids() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let base = a.version().unwrap();
    let x = a.execute("layer.newSolid", json!({"name": "FromA"})).unwrap()["layer"].as_u64().unwrap();
    let y = b.execute("layer.newSolid", json!({"name": "FromB"})).unwrap()["layer"].as_u64().unwrap();
    assert_ne!(x, y, "each actor mints from its own slot");
    merge(&mut a, &mut b, &base);
    assert!(holds(&a));
    assert!(names(&a).contains(&"FromA".into()) && names(&a).contains(&"FromB".into()), "{:?}", names(&a));
    assert!(a.undo(), "the merge is one undo step");
    assert!(names(&a).contains(&"FromA".into()) && !names(&a).contains(&"FromB".into()), "{:?}", names(&a));
    assert!(holds(&a));
}

#[test]
fn deleting_a_precomp_a_draft_uses_removes_its_layer() {
    let mut a = new_session();
    let inner = a.execute("comp.new", json!({"name": "Inner", "width": 64, "height": 64, "frameRate": 30, "duration": 1, "open": false})).unwrap()["comp"]
        .as_u64()
        .unwrap();
    let main = a.project.comps().find(|(_, c)| c.layers.len() == 3).map(|(id, _)| id.0).unwrap();
    a.execute("comp.open", json!({"comp": main})).unwrap();
    let mut b = peer(&mut a);
    let base = a.version().unwrap();
    b.execute("comp.open", json!({"comp": main})).unwrap();
    b.execute("layer.addItem", json!({"item": inner})).unwrap();
    a.execute("project.delete", json!({"items": [inner]})).unwrap();
    merge(&mut a, &mut b, &base);
    assert!(holds(&a));
    assert!(a.project.item(kition_motion_project::ItemId(inner)).is_none());
    let uses = |s: &Session| s.project.comps().any(|(_, c)| c.layers.iter().any(|l| l.source.item() == Some(kition_motion_project::ItemId(inner))));
    assert!(!uses(&a), "no layer uses the deleted precomposition");
}

#[test]
fn a_history_jump_replicates_as_one_change_and_its_line_undoes() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let red = layer(&a, "Red");
    a.execute("layer.rename", json!({"layer": red, "name": "One"})).unwrap();
    a.execute("layer.rename", json!({"layer": red, "name": "Two"})).unwrap();
    // Undo, then a new edit: "Two" is kept as a branch of the history.
    assert!(a.undo());
    a.execute("layer.rename", json!({"layer": red, "name": "Three"})).unwrap();
    let list = a.execute("edit.history.list", json!({})).unwrap();
    let two =
        list["states"].as_array().unwrap().iter().find(|n| n["label"] == "Rename Layer" && n["line"] == false).map(|n| n["id"].as_str().unwrap().to_owned());
    let Some(two) = two else { return };
    let version = a.version().unwrap();
    a.execute("edit.history.goto", json!({"id": two})).unwrap();
    assert_ne!(a.version().unwrap(), version, "the jump wrote a change");
    assert!(holds(&a));
    send(&mut a, &mut b);
    assert_same(&mut a, &mut b, "after the jump");
    // Undo along the new line writes its change too (nothing arrived from b meanwhile).
    let version = a.version().unwrap();
    assert!(a.undo());
    assert_ne!(a.version().unwrap(), version, "the undo on the new line was replicated");
    assert!(holds(&a));
    send(&mut a, &mut b);
    assert_same(&mut a, &mut b, "after undoing on the new line");
}

#[test]
fn a_step_taken_before_the_replica_existed_undoes_onto_it() {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Main", "width": 64, "height": 64, "frameRate": 30, "duration": 1})).unwrap();
    s.execute("layer.newSolid", json!({"name": "Early"})).unwrap();
    assert!(s.kdm().is_none(), "no replica until a host asks");
    let mut b = {
        let mut b = Session::default();
        b.open_snapshot(&s.snapshot().unwrap()).unwrap();
        b
    };
    assert!(s.undo(), "the solid's step undoes");
    assert!(holds(&s), "and the replica follows");
    send(&mut s, &mut b);
    assert_same(&mut s, &mut b, "a peer gets the undo");
}

#[test]
fn a_direct_write_outside_a_command_is_replicated_by_the_next_one() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    // A host (or a job) changes the project outside any command.
    let mut p = (*a.project).clone();
    p.items.values_mut().for_each(|i| {
        if i.name == "Main" {
            i.comment = "direct".into();
        }
    });
    a.project = Arc::new(p);
    a.execute("time.set", json!({"time": 1.0})).unwrap();
    assert!(holds(&a), "replicated before the next command ran");
    send(&mut a, &mut b);
    assert_same(&mut a, &mut b, "the peer has it");
}

/// Files in memory, for `open_with_replica`.
#[derive(Default)]
struct Memory(Mutex<HashMap<String, Vec<u8>>>);

impl Services for Memory {
    fn read_file(&self, path: &str) -> std::io::Result<Vec<u8>> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, path.to_owned()))
    }
    fn write_file(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(path.to_owned(), data.to_vec());
        Ok(())
    }
    fn exists(&self, path: &str) -> bool {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).contains_key(path)
    }
}

#[test]
fn a_file_newer_than_its_stored_replica_wins_on_open() {
    let mut a = new_session();
    let snapshot = a.snapshot().unwrap();
    let doc = a.kdm().unwrap().replica().doc_id();
    let red = layer(&a, "Red");
    a.execute("layer.rename", json!({"layer": red, "name": "Newer"})).unwrap();
    let file = a.project.to_file_json().unwrap();

    let memory = Arc::new(Memory::default());
    memory.write_file("Motion.kmotion", file.as_bytes()).unwrap();
    let mut b = Session { services: memory.clone(), ..Session::default() };
    b.open_with_replica("Motion.kmotion", &snapshot).unwrap();
    assert!(names(&b).contains(&"Newer".to_string()), "the file's content");
    assert!(holds(&b), "recorded onto the replica");
    assert_eq!(b.kdm().unwrap().replica().doc_id(), doc, "the replica's own identity");
    assert!(!b.is_dirty());
    assert!(b.history.undo.is_empty(), "opening is not an undo step");

    let mut c = Session::default();
    c.open_snapshot(&snapshot).unwrap();
    send(&mut b, &mut c);
    assert!(names(&c).contains(&"Newer".to_string()), "a peer of the older replica gets the file's content");

    // A file its replica does hold opens as the replica, writing nothing.
    let current = b.snapshot().unwrap();
    let version = b.version().unwrap();
    let mut d = Session { services: memory, ..Session::default() };
    d.open_with_replica("Motion.kmotion", &current).unwrap();
    assert_eq!(d.version().unwrap(), version, "nothing written");
}

#[test]
fn a_write_the_replica_refuses_resets_it_and_a_fresh_one_is_made_on_request() {
    let mut s = new_session();
    let red = layer(&s, "Red");
    // A property tree nested past what a replica holds: the write is refused, the project
    // leaves its replica (a reset hosts see), keeping the edit and its undo.
    s.edit("Deep", None, |p, _| {
        let cid = *p.comps().next().unwrap().0;
        let l = p.comp_mut(cid).unwrap().layer_mut(LayerId(red)).unwrap();
        let mut deep = kition_motion_project::PropGroup::new(9_000_000, "deep", "Deep");
        for i in 0..300 {
            let mut outer = kition_motion_project::PropGroup::new(9_000_001 + i, "deep", "Deep");
            outer.children.push(kition_motion_project::Node::Group(deep));
            deep = outer;
        }
        l.props.children.push(kition_motion_project::Node::Group(deep));
        Ok(())
    })
    .unwrap();
    s.execute("time.set", json!({"time": 0.5})).unwrap();
    assert!(s.kdm().is_none(), "the project left its replica");
    assert_eq!(s.kdm_resets(), 1);
    assert!(s.undo(), "undo works without a replica");
    assert!(s.ensure_kdm().is_some(), "a fresh replica on request");
    assert_eq!(s.kdm_resets(), 1);
    let mut b = peer(&mut s);
    assert_same(&mut s, &mut b, "a peer joins the fresh replica");
}

/// Changes made raw on a replica of `a`'s (stored changes that follow it): a layer whose start
/// is not a tick listed in the composition (so the project cannot be read back).
fn changes_the_view_cannot_load(a: &mut Session) -> Vec<u8> {
    use kdm_model::kition_kit::kdm::{ActorId, Origin as KdmOrigin, Replica};
    use kdm_model::kition_kit::session::KitModel;
    let mut raw = Replica::load(kdm_model::MotionModel::schema(), &a.snapshot().unwrap(), ActorId::from_u64(77).unwrap()).unwrap();
    let since = raw.version();
    raw.transact(&KdmOrigin::person("user:collaborator"), |tx| {
        let comp = tx.objects_of_type("Comp").into_iter().next().unwrap();
        let node = tx.create("Node", json!({"uid": 1, "node": "group", "meta": {"match": "layer", "name": "x"}}).as_object().cloned().unwrap())?;
        let layer = json!({"id": 5, "name": "x", "startTime": "not a tick", "inPoint": "0", "outPoint": "1", "props": node.as_str()});
        let layer = tx.create("Layer", layer.as_object().cloned().unwrap())?;
        tx.list_insert(&comp, "layers", 0, &layer)?;
        Ok(())
    })
    .unwrap();
    raw.export_changes(&since).unwrap()
}

#[test]
fn an_import_the_view_cannot_load_leaves_the_replica() {
    let mut a = new_session();
    let before = names(&a);
    let bytes = changes_the_view_cannot_load(&mut a);
    assert!(a.import_changes(&bytes).is_err());
    assert!(a.kdm().is_none(), "the project left its replica");
    assert_eq!(a.kdm_resets(), 1);
    assert_eq!(names(&a), before, "the project is as it was");
    assert!(a.ensure_kdm().is_some(), "a fresh replica on request");
}

/// A tiny deterministic generator (no extra dependency).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

fn random_command(s: &mut Session, rng: &mut Rng) -> String {
    let Some(comp) = s.active_comp() else { return "no comp".into() };
    let ids: Vec<u64> = comp.layers.iter().map(|l| l.id.0).collect();
    let pick = |rng: &mut Rng| ids.get(rng.below(ids.len() as u64) as usize).copied();
    let paths = ["transform/opacity", "transform/rotation"];
    let (id, params) = match rng.below(12) {
        0 | 1 => match pick(rng) {
            Some(l) => ("prop.addKey", json!({"layer": l, "path": paths[rng.below(2) as usize], "time": rng.below(30) as f64 / 10.0, "value": rng.below(100)})),
            None => return "no layer".into(),
        },
        2 | 3 => match pick(rng) {
            Some(l) => ("prop.set", json!({"layer": l, "path": paths[rng.below(2) as usize], "time": rng.below(30) as f64 / 10.0, "value": rng.below(100)})),
            None => return "no layer".into(),
        },
        4 => match pick(rng) {
            Some(l) => {
                let to = ["front", "forward", "backward", "back"][rng.below(4) as usize];
                ("layer.arrange", json!({"layers": [l], "to": to}))
            }
            None => return "no layer".into(),
        },
        5 => match pick(rng) {
            Some(l) => ("layer.rename", json!({"layer": l, "name": format!("L{}", rng.below(1000))})),
            None => return "no layer".into(),
        },
        6 => match pick(rng) {
            Some(l) if ids.len() > 1 => ("edit.delete", json!({"layers": [l]})),
            _ => return "keep one".into(),
        },
        7 => ("layer.newSolid", json!({"name": format!("S{}", rng.below(1000))})),
        8 => match (pick(rng), pick(rng)) {
            (Some(x), Some(y)) if x != y => ("layer.setParent", json!({"layers": [x], "parent": y})),
            _ => return "no pair".into(),
        },
        9 => ("edit.redo", json!({})),
        _ => ("edit.undo", json!({})),
    };
    let r = s.execute(id, params.clone());
    format!("{id} {params} -> {}", if r.is_ok() { "ok" } else { "refused" })
}

/// Random commands, undos and redos with a short undo limit: after every one the replica holds
/// the project, and a reader that takes the changes in order holds the same.
#[test]
fn random_commands_keep_the_replica_holding_the_project() {
    for seed in [3u64, 11, 2026] {
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        let mut a = new_session();
        a.prefs.general.undo_levels = 4;
        let mut b = peer(&mut a);
        let mut log: Vec<String> = Vec::new();
        for step in 0..200 {
            let what = random_command(&mut a, &mut rng);
            log.push(what);
            let context = || format!("seed {seed}, step {step}; last commands:\n{}", log[log.len().saturating_sub(6)..].join("\n"));
            assert_eq!(a.kdm_resets(), 0, "{}", context());
            assert!(a.kdm().is_some(), "no replica: {}", context());
            assert!(holds(&a), "the project is not what its replica holds: {}", context());
            if step % 5 == 4 {
                send(&mut a, &mut b);
                assert_same(&mut a, &mut b, &format!("seed {seed}, step {step}"));
            }
        }
        send(&mut a, &mut b);
        assert_same(&mut a, &mut b, &format!("seed {seed}, final"));
        let mut c = peer(&mut a);
        assert_same(&mut a, &mut c, &format!("seed {seed}: a fresh replica matches"));
    }
}

#[test]
fn the_demo_project_round_trips_through_its_replica() {
    let mut s = Session::default();
    s.execute("file.openDemoProject", json!({})).unwrap();
    s.ensure_kdm().unwrap();
    assert!(holds(&s), "the replica holds the whole demo, exactly");
    let b = peer(&mut s);
    assert!(kdm_model::same_content(&b.project, &s.project), "a peer opens the very same project");
}
