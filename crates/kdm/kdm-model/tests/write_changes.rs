//! `write_changes` writes only what an edit changed: one keyframe edited is one object written,
//! a layer moved in the stack is one list move, and objects keep their identity through edits
//! that move them (keys moved in time, layers moved to another composition).

mod common;

use std::collections::BTreeSet;

use common::{demo, keys, layer, layer_mut, new_replica, read};
use kition_kit::kdm::{ObjectId, Origin, Replica};
use kition_motion_project::{Keyframe, LayerId, Value};

fn write(
    replica: &mut Replica,
    before: &kition_motion_project::Project,
    after: &kition_motion_project::Project,
    ids: &mut kdm_model::Ids,
) -> BTreeSet<ObjectId> {
    replica.take_changed();
    let wrote = replica
        .transact(&Origin::person("user:test"), |tx| kdm_model::write_changes(tx, before, after, ids))
        .unwrap_or_else(|error| panic!("write_changes: {error}"));
    assert!(wrote, "the edit wrote something");
    let changed = replica.take_changed();
    common::assert_same_content(after, &read(replica), "the replica holds the edit");
    changed
}

fn uid(project: &kition_motion_project::Project, cid: kition_motion_project::ItemId, id: LayerId, path: &str) -> u64 {
    layer(project, cid, id).props.prop(path).unwrap().uid
}

#[test]
fn editing_one_keyframe_writes_only_that_key() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let opacity = uid(&d.project, d.main, d.red, "transform/opacity");
    let key_objects = ids.keys(d.red, opacity).to_vec();
    assert_eq!(key_objects.len(), 2);

    // The value of the key at 1 s.
    let mut after = d.project.clone();
    layer_mut(&mut after, d.main, d.red).props.prop_mut("transform/opacity").unwrap().keys[1].value = Value::Scalar(60.0);
    let changed = write(&mut replica, &d.project, &after, &mut ids);
    assert_eq!(changed, BTreeSet::from([key_objects[1].clone()]), "only the edited key's object changed");
    assert_eq!(ids.keys(d.red, opacity), key_objects.as_slice(), "both keys keep their objects");

    // Its time (a key dragged later).
    let mut later = after.clone();
    layer_mut(&mut later, d.main, d.red).props.prop_mut("transform/opacity").unwrap().keys[1].time = common::seconds(1.5);
    let changed = write(&mut replica, &after, &later, &mut ids);
    assert_eq!(changed, BTreeSet::from([key_objects[1].clone()]), "a key moved in time is the same object, written once");
    assert_eq!(replica.get(&key_objects[1], "time").unwrap(), serde_json::json!(common::seconds(1.5).0.to_string()));
}

#[test]
fn adding_a_keyframe_creates_one_object_and_keeps_the_others() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let opacity = uid(&d.project, d.main, d.red, "transform/opacity");
    let before_keys = ids.keys(d.red, opacity).to_vec();
    let mut after = d.project.clone();
    common::set_key(&mut after, d.main, d.red, "transform/opacity", 0.5, Value::Scalar(50.0));
    let changed = write(&mut replica, &d.project, &after, &mut ids);
    let now = ids.keys(d.red, opacity).to_vec();
    assert_eq!(now.len(), 3);
    assert_eq!((&now[0], &now[2]), (&before_keys[0], &before_keys[1]), "the existing keys keep their objects, in time order");
    let property = ids.node(d.red, opacity).unwrap().clone();
    assert_eq!(changed, BTreeSet::from([now[1].clone(), property]), "the new key and its property's key list");
}

#[test]
fn moving_a_layer_in_the_stack_is_one_move() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let comp_object = ids.comps[&d.main].clone();
    let objects: Vec<ObjectId> = replica.list(&comp_object, "layers").unwrap();
    let mut after = d.project.clone();
    // Blue to the top.
    let comp = after.comp_mut(d.main).unwrap();
    let blue = comp.layers.remove(2);
    comp.layers.insert(0, blue);
    let changed = write(&mut replica, &d.project, &after, &mut ids);
    assert_eq!(changed, BTreeSet::from([comp_object.clone()]), "only the composition's layer list changed");
    let now = replica.list(&comp_object, "layers").unwrap();
    assert_eq!(now, vec![objects[2].clone(), objects[0].clone(), objects[1].clone()], "the same three layer objects, Blue first");
}

/// A layer's name is a `string` field of its own (schema 2), so an agent renames it with one `set`.
#[test]
fn renaming_a_layer_writes_only_its_name() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let object = ids.layer_by_id(d.green).unwrap().clone();
    let mut after = d.project.clone();
    layer_mut(&mut after, d.main, d.green).name = "Lime".into();
    let changed = write(&mut replica, &d.project, &after, &mut ids);
    assert_eq!(changed, BTreeSet::from([object.clone()]));
    assert_eq!(replica.get(&object, "name").unwrap(), serde_json::json!("Lime"));
    assert!(replica.map_get(&object, "attrs", "name").unwrap().is_null(), "not a key of attrs as well");
}

#[test]
fn a_layer_moved_to_another_composition_keeps_its_objects() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let object = ids.layer_by_id(d.blue).unwrap().clone();
    let nodes_before = ids.layers.values().find(|l| l.object == object).unwrap().nodes.clone();
    let mut after = d.project.clone();
    let blue = after.comp_mut(d.main).unwrap().layers.remove(2);
    after.comp_mut(d.inner).unwrap().layers.push(blue);
    write(&mut replica, &d.project, &after, &mut ids);
    assert!(replica.exists(&object));
    assert_eq!(replica.list(&ids.comps[&d.inner], "layers").unwrap(), vec![object.clone()]);
    assert_eq!(ids.layers.values().find(|l| l.object == object).unwrap().nodes, nodes_before, "its property tree keeps every object");
}

#[test]
fn deleting_a_layer_deletes_its_objects() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let lids = ids.layers.values().find(|l| Some(&l.object) == ids.layer_by_id(d.red)).unwrap().clone();
    let mut after = d.project.clone();
    after.comp_mut(d.main).unwrap().layers.retain(|l| l.id != d.red);
    write(&mut replica, &d.project, &after, &mut ids);
    assert!(!replica.exists(&lids.object));
    assert!(lids.nodes.values().all(|n| !replica.exists(n)), "every node of its tree is gone");
    assert!(lids.keys.values().flatten().all(|k| !replica.exists(k)), "and every key");
}

#[test]
fn an_unchanged_project_writes_nothing() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let version = replica.version();
    let wrote = replica.transact(&Origin::person("user:test"), |tx| kdm_model::write_changes(tx, &d.project, &d.project, &mut ids)).unwrap();
    assert!(!wrote);
    assert_eq!(replica.version(), version);
}

#[test]
fn reordering_keys_by_retiming_them_keeps_every_object() {
    let d = demo();
    let (mut replica, mut ids) = new_replica(&d.project);
    let position = uid(&d.project, d.main, d.green, "transform/position");
    let before = ids.keys(d.green, position).to_vec();
    // Swap the two keys in time (time-reverse): the same two keys, each moved.
    let mut after = d.project.clone();
    {
        let prop = layer_mut(&mut after, d.main, d.green).props.prop_mut("transform/position").unwrap();
        let (a, b) = (prop.keys[0].time, prop.keys[1].time);
        prop.keys[0].time = b;
        prop.keys[1].time = a;
        prop.keys.sort_by_key(|k: &Keyframe| k.time);
    }
    write(&mut replica, &d.project, &after, &mut ids);
    let now = ids.keys(d.green, position).to_vec();
    let mut sorted_before = before.clone();
    sorted_before.sort();
    let mut sorted_now = now.clone();
    sorted_now.sort();
    assert_eq!(sorted_now, sorted_before, "no key object was deleted or created");
    assert_eq!(keys(&read(&replica), d.main, d.green, "transform/position"), keys(&after, d.main, d.green, "transform/position"));
}

#[test]
fn a_parent_held_by_a_repeated_layer_id_survives_deleting_the_other_holder() {
    // A file can hold one layer id twice (here in Main and in Inner). Deleting Main's holder
    // shifts Inner's to the first occurrence of that id; a layer parented to it keeps its parent.
    let mut d = demo();
    let inner_solid = common::solid(&mut d.project, "Inner Solid", [1.0, 1.0, 0.0]);
    let twin = common::add_layer(&mut d.project, d.inner, "Twin", kition_motion_project::LayerSource::Solid { item: inner_solid });
    let child = common::add_layer(&mut d.project, d.inner, "Child", kition_motion_project::LayerSource::Solid { item: inner_solid });
    layer_mut(&mut d.project, d.inner, child).parent = Some(twin);
    layer_mut(&mut d.project, d.main, d.blue).id = twin;
    let (mut replica, mut ids) = new_replica(&d.project);
    common::assert_same_content(&d.project, &read(&replica), "the repeated id is written");
    let mut after = d.project.clone();
    after.comp_mut(d.main).unwrap().layers.retain(|l| l.id != twin);
    write(&mut replica, &d.project, &after, &mut ids);
    assert_eq!(layer(&read(&replica), d.inner, child).parent, Some(twin));
}
