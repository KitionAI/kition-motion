//! Two-replica merges through `KitSession<MotionModel>`: concurrent edits of different things all
//! survive; a delete beats a concurrent use of what it deleted and the repair leaves nothing
//! dangling, the same way on both replicas.

mod common;

use common::{add_layer, assert_same_content, demo, first, keys, layer, layer_mut, peer, person, record, set_key, sync};
use kition_motion_project::{LayerSource, Value};

#[test]
fn edits_of_different_layers_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);

    let mut after_a = a.model().project.clone();
    layer_mut(&mut after_a, d.main, d.red).name = "Crimson".into();
    set_key(&mut after_a, d.main, d.red, "transform/opacity", 3.0, Value::Scalar(10.0));
    record(&mut a, &person("a"), &after_a);

    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.blue).props.prop_mut("transform/scale").unwrap().value = Value::Vec3([50.0, 50.0, 100.0]);
    layer_mut(&mut after_b, d.main, d.blue).out_point = common::seconds(2.0);
    record(&mut b, &person("b"), &after_b);

    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let p = &a.model().project;
    assert_eq!(layer(p, d.main, d.red).name, "Crimson");
    assert_eq!(keys(p, d.main, d.red, "transform/opacity").len(), 3);
    assert_eq!(layer(p, d.main, d.blue).props.prop("transform/scale").unwrap().value, Value::Vec3([50.0, 50.0, 100.0]));
    assert_eq!(layer(p, d.main, d.blue).out_point, common::seconds(2.0));
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn a_keyframe_added_and_a_keyframe_moved_on_another_property_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);

    let mut after_a = a.model().project.clone();
    set_key(&mut after_a, d.main, d.red, "transform/opacity", 0.5, Value::Scalar(25.0));
    record(&mut a, &person("a"), &after_a);

    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.green).props.prop_mut("transform/position").unwrap().keys[1].time = common::seconds(3.0);
    record(&mut b, &person("b"), &after_b);

    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let p = &b.model().project;
    let opacity: Vec<_> = keys(p, d.main, d.red, "transform/opacity").iter().map(|k| k.time).collect();
    assert_eq!(opacity, vec![common::seconds(0.0), common::seconds(0.5), common::seconds(1.0)], "a's key arrived, in time order");
    let position: Vec<_> = keys(p, d.main, d.green, "transform/position").iter().map(|k| k.time).collect();
    assert_eq!(position, vec![common::seconds(0.0), common::seconds(3.0)], "b's move stayed");
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn edits_of_two_keys_of_one_property_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    let mut after_a = a.model().project.clone();
    layer_mut(&mut after_a, d.main, d.red).props.prop_mut("transform/opacity").unwrap().keys[0].value = Value::Scalar(5.0);
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.red).props.prop_mut("transform/opacity").unwrap().keys[1].value = Value::Scalar(95.0);
    record(&mut b, &person("b"), &after_b);
    sync(&mut a, &mut b);
    let values: Vec<_> = keys(&a.model().project, d.main, d.red, "transform/opacity").iter().map(|k| k.value.clone()).collect();
    assert_eq!(values, vec![Value::Scalar(5.0), Value::Scalar(95.0)]);
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn deleting_a_precomp_a_peer_uses_removes_the_layer_using_it() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);

    // b places the (unused) precomposition in the main composition and parents Red to it.
    let mut after_b = b.model().project.clone();
    let nested = add_layer(&mut after_b, d.main, "Inner", LayerSource::Comp { item: d.inner });
    layer_mut(&mut after_b, d.main, d.red).parent = Some(nested);
    record(&mut b, &person("b"), &after_b);

    // a, unaware, deletes the precomposition.
    let mut after_a = a.model().project.clone();
    after_a.items.remove(&d.inner);
    record(&mut a, &person("a"), &after_a);

    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    for p in [&a.model().project, &b.model().project] {
        assert!(p.item(d.inner).is_none(), "the precomposition stays deleted");
        let main = p.comp(d.main).unwrap();
        assert!(main.layer(nested).is_none(), "the layer that used it is gone");
        assert_eq!(main.layers.len(), 3);
        assert_eq!(main.layer(d.red).unwrap().parent, None, "nothing is parented to the removed layer");
    }
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn deleting_a_layer_a_peer_edits_merges_cleanly() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    let mut after_a = a.model().project.clone();
    after_a.comp_mut(d.main).unwrap().layers.retain(|l| l.id != d.green);
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    set_key(&mut after_b, d.main, d.green, "transform/position", 1.0, Value::Vec3([10.0, 10.0, 0.0]));
    layer_mut(&mut after_b, d.main, d.blue).track_matte =
        Some(kition_motion_project::TrackMatte { layer: d.green, kind: kition_motion_project::MatteKind::Alpha });
    record(&mut b, &person("b"), &after_b);
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let main = a.model().project.comp(d.main).unwrap().clone();
    assert!(main.layer(d.green).is_none(), "the deleted layer stays deleted");
    assert_eq!(main.layer(d.blue).unwrap().track_matte, None, "no matte names it");
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn concurrent_parenting_into_a_cycle_is_broken() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    let mut after_a = a.model().project.clone();
    layer_mut(&mut after_a, d.main, d.red).parent = Some(d.green);
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    layer_mut(&mut after_b, d.main, d.green).parent = Some(d.red);
    record(&mut b, &person("b"), &after_b);
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let p = &a.model().project;
    let (red, green) = (layer(p, d.main, d.red).parent, layer(p, d.main, d.green).parent);
    assert!(red.is_none() || green.is_none(), "no cycle survives: {red:?} {green:?}");
    assert!(red.is_some() || green.is_some(), "only one of the two parentings is undone");
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn concurrent_keys_at_one_time_keep_one() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    for (session, value, name) in [(&mut a, 30.0, "a"), (&mut b, 70.0, "b")] {
        let mut after = session.model().project.clone();
        set_key(&mut after, d.main, d.red, "transform/opacity", 2.0, Value::Scalar(value));
        record(session, &person(name), &after);
    }
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let times: Vec<_> = keys(&a.model().project, d.main, d.red, "transform/opacity").iter().map(|k| k.time).collect();
    assert_eq!(times, vec![common::seconds(0.0), common::seconds(1.0), common::seconds(2.0)], "one key at 2 s");
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn concurrent_nesting_of_two_comps_in_each_other_is_broken() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    let mut after_a = a.model().project.clone();
    add_layer(&mut after_a, d.main, "Inner", LayerSource::Comp { item: d.inner });
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    add_layer(&mut after_b, d.inner, "Main", LayerSource::Comp { item: d.main });
    record(&mut b, &person("b"), &after_b);
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let p = &a.model().project;
    assert!(!(p.comp_contains(d.main, d.inner) && p.comp_contains(d.inner, d.main)), "no precomposition cycle survives");
    assert_same_content(&a.model().project, &b.model().project, "converged");
}

#[test]
fn concurrent_render_queue_adds_keep_both_with_distinct_ids() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    for (session, name) in [(&mut a, "a"), (&mut b, "b")] {
        let mut after = session.model().project.clone();
        let id = after.render_queue.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        after.render_queue.push(kition_motion_project::render_queue::RenderQueueItem::new(id, d.main));
        record(session, &person(name), &after);
    }
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let queue = &a.model().project.render_queue;
    assert_eq!(queue.len(), 2);
    assert_ne!(queue[0].id, queue[1].id);
    assert_same_content(&a.model().project, &b.model().project, "converged");
}
