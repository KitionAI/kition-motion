//! A draft's merge through `KitSession<MotionModel>` (data model 6.6, rules D1 to D9): a draft
//! forked from the document at version v, the document edited meanwhile, the draft's changes taken
//! in as one step of the person. Edits of different things all survive; what the merge leaves
//! that the engine cannot hold (a layer using a deleted precomposition, a matte naming a deleted
//! layer, a parenting or nesting cycle, two keys at one time, two Render Queue items with one id)
//! is repaired inside the merge's one undo step. An edit the rules refuse (the draft changing a layer the document deleted)
//! changes nothing.

mod common;

use common::{add_layer, assert_same_content, assert_view_is_replica, demo, draft, first, keys, layer, layer_mut, merge, person, record, set_key, try_merge};
use kition_motion_project::{LayerSource, Value};

#[test]
fn edits_of_different_layers_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);

    let mut after_a = a.model().project.clone();
    layer_mut(&mut after_a, d.main, d.red).name = "Crimson".into();
    set_key(&mut after_a, d.main, d.red, "transform/opacity", 3.0, Value::Scalar(10.0));
    record(&mut a, &person("a"), &after_a);

    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.blue).props.prop_mut("transform/scale").unwrap().value = Value::Vec3([50.0, 50.0, 100.0]);
    layer_mut(&mut after_b, d.main, d.blue).out_point = common::seconds(2.0);
    record(&mut b, &person("b"), &after_b);

    merge(&mut a, &b, &base);
    let p = &a.model().project;
    assert_eq!(layer(p, d.main, d.red).name, "Crimson");
    assert_eq!(keys(p, d.main, d.red, "transform/opacity").len(), 3);
    assert_eq!(layer(p, d.main, d.blue).props.prop("transform/scale").unwrap().value, Value::Vec3([50.0, 50.0, 100.0]));
    assert_eq!(layer(p, d.main, d.blue).out_point, common::seconds(2.0));
}

#[test]
fn a_keyframe_added_and_a_keyframe_moved_on_another_property_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);

    let mut after_a = a.model().project.clone();
    set_key(&mut after_a, d.main, d.red, "transform/opacity", 0.5, Value::Scalar(25.0));
    record(&mut a, &person("a"), &after_a);

    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.green).props.prop_mut("transform/position").unwrap().keys[1].time = common::seconds(3.0);
    record(&mut b, &person("b"), &after_b);

    merge(&mut a, &b, &base);
    let p = &a.model().project;
    let opacity: Vec<_> = keys(p, d.main, d.red, "transform/opacity").iter().map(|k| k.time).collect();
    assert_eq!(opacity, vec![common::seconds(0.0), common::seconds(0.5), common::seconds(1.0)], "a's key stayed, in time order");
    let position: Vec<_> = keys(p, d.main, d.green, "transform/position").iter().map(|k| k.time).collect();
    assert_eq!(position, vec![common::seconds(0.0), common::seconds(3.0)], "b's move merged");
}

#[test]
fn edits_of_two_keys_of_one_property_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after_a = a.model().project.clone();
    layer_mut(&mut after_a, d.main, d.red).props.prop_mut("transform/opacity").unwrap().keys[0].value = Value::Scalar(5.0);
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.red).props.prop_mut("transform/opacity").unwrap().keys[1].value = Value::Scalar(95.0);
    record(&mut b, &person("b"), &after_b);
    merge(&mut a, &b, &base);
    let values: Vec<_> = keys(&a.model().project, d.main, d.red, "transform/opacity").iter().map(|k| k.value.clone()).collect();
    assert_eq!(values, vec![Value::Scalar(5.0), Value::Scalar(95.0)]);
}

#[test]
fn deleting_a_precomp_a_draft_uses_removes_the_layer_using_it() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);

    // The draft places the (unused) precomposition in the main composition and parents Red to it.
    let mut after_b = b.model().project.clone();
    let nested = add_layer(&mut after_b, d.main, "Inner", LayerSource::Comp { item: d.inner });
    layer_mut(&mut after_b, d.main, d.red).parent = Some(nested);
    record(&mut b, &person("b"), &after_b);

    // The document, meanwhile, deletes the precomposition.
    let mut after_a = a.model().project.clone();
    after_a.items.remove(&d.inner);
    record(&mut a, &person("a"), &after_a);
    let (state, project) = (a.state_hash().unwrap(), a.model().project.clone());

    merge(&mut a, &b, &base);
    let p = &a.model().project;
    assert!(p.item(d.inner).is_none(), "the precomposition stays deleted");
    let main = p.comp(d.main).unwrap();
    assert!(main.layer(nested).is_none(), "the layer that used it is gone");
    assert_eq!(main.layers.len(), 3);
    assert_eq!(main.layer(d.red).unwrap().parent, None, "nothing is parented to the removed layer");
    // The repair is part of the merge's step: one Undo of the person gives back exactly the
    // document's state before the merge.
    assert!(a.undo_in(kition_kit::kdm::UndoScope::Person).unwrap().0);
    assert_eq!(a.state_hash().unwrap(), state, "exactly the state before the merge");
    assert_same_content(&a.model().project, &project, "the view before the merge");
}

/// Merges `draft`'s changes since `base` into `doc`, which the repair takes back entirely: no
/// step, the version and the state stay, and the view is still the replica's.
fn merge_changing_nothing(doc: &mut common::Session, draft: &common::Session, base: &kition_kit::kdm::Version) {
    let (version, state) = (doc.version(), doc.state_hash().unwrap());
    let (scope, _) = try_merge(doc, draft, base).unwrap_or_else(|error| panic!("merge: {error}"));
    assert_eq!(scope, None, "a merge that changes nothing records no step");
    assert_eq!(doc.version(), version, "the version stays");
    assert_eq!(doc.state_hash().unwrap(), state, "the state stays");
    assert_view_is_replica(doc);
}

#[test]
fn deleting_a_layer_a_draft_makes_a_matte_merges_cleanly() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after_a = a.model().project.clone();
    after_a.comp_mut(d.main).unwrap().layers.retain(|l| l.id != d.green);
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.blue).track_matte =
        Some(kition_motion_project::TrackMatte { layer: d.green, kind: kition_motion_project::MatteKind::Alpha });
    record(&mut b, &person("b"), &after_b);
    // The repair clears the matte the draft named the deleted layer in, which is all the draft
    // did: the merge changes nothing and records nothing (data model 6.6 D9).
    merge_changing_nothing(&mut a, &b, &base);
    let main = a.model().project.comp(d.main).unwrap().clone();
    assert!(main.layer(d.green).is_none(), "the deleted layer stays deleted");
    assert_eq!(main.layer(d.blue).unwrap().track_matte, None, "no matte names it");
}

/// Rule D8: a draft that keys a layer the document deleted is refused as a whole, and the
/// document is as it was.
#[test]
fn a_draft_editing_a_layer_the_document_deleted_is_refused() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after_a = a.model().project.clone();
    after_a.comp_mut(d.main).unwrap().layers.retain(|l| l.id != d.green);
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    set_key(&mut after_b, d.main, d.green, "transform/position", 1.0, Value::Vec3([10.0, 10.0, 0.0]));
    record(&mut b, &person("b"), &after_b);
    let (version, state) = (a.version(), a.state_hash().unwrap());
    assert!(try_merge(&mut a, &b, &base).is_err(), "the draft changes what the document deleted");
    assert_eq!(a.version(), version, "nothing changed");
    assert_eq!(a.state_hash().unwrap(), state);
    assert_view_is_replica(&a);
}

#[test]
fn parenting_into_a_cycle_on_both_sides_is_broken() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after_a = a.model().project.clone();
    layer_mut(&mut after_a, d.main, d.red).parent = Some(d.green);
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.green).parent = Some(d.red);
    record(&mut b, &person("b"), &after_b);
    merge(&mut a, &b, &base);
    let p = &a.model().project;
    let (red, green) = (layer(p, d.main, d.red).parent, layer(p, d.main, d.green).parent);
    assert!(red.is_none() || green.is_none(), "no cycle survives: {red:?} {green:?}");
    assert!(red.is_some() || green.is_some(), "only one of the two parentings is undone");
}

#[test]
fn keys_at_one_time_on_both_sides_keep_one() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    for (session, value, name) in [(&mut a, 30.0, "a"), (&mut b, 70.0, "b")] {
        let mut after = session.model().project.clone();
        set_key(&mut after, d.main, d.red, "transform/opacity", 2.0, Value::Scalar(value));
        record(session, &person(name), &after);
    }
    // The repair keeps the document's key at 2 s, the one the draft's would have been beside:
    // all the draft did is taken back, so the merge changes nothing and records nothing.
    merge_changing_nothing(&mut a, &b, &base);
    let times: Vec<_> = keys(&a.model().project, d.main, d.red, "transform/opacity").iter().map(|k| k.time).collect();
    assert_eq!(times, vec![common::seconds(0.0), common::seconds(1.0), common::seconds(2.0)], "one key at 2 s");
}

#[test]
fn nesting_two_comps_in_each_other_on_both_sides_is_broken() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after_a = a.model().project.clone();
    add_layer(&mut after_a, d.main, "Inner", LayerSource::Comp { item: d.inner });
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    add_layer(&mut after_b, d.inner, "Main", LayerSource::Comp { item: d.main });
    record(&mut b, &person("b"), &after_b);
    merge(&mut a, &b, &base);
    let p = &a.model().project;
    assert!(!(p.comp_contains(d.main, d.inner) && p.comp_contains(d.inner, d.main)), "no precomposition cycle survives");
}

#[test]
fn render_queue_adds_on_both_sides_keep_both_with_distinct_ids() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    for (session, name) in [(&mut a, "a"), (&mut b, "b")] {
        let mut after = session.model().project.clone();
        let id = after.render_queue.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        after.render_queue.push(kition_motion_project::render_queue::RenderQueueItem::new(id, d.main));
        record(session, &person(name), &after);
    }
    merge(&mut a, &b, &base);
    let queue = &a.model().project.render_queue;
    assert_eq!(queue.len(), 2);
    assert_ne!(queue[0].id, queue[1].id);
}
