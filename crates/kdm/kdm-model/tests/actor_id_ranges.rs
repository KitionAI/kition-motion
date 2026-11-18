//! Per-actor id slots: engine ids are the objects' own data, so two sessions that mint ids
//! without exchanging first must never mint the same one. Each actor mints from its own slot
//! (`kdm_model::slot_base`), above the ids a file starts with.

mod common;

use common::{add_layer, demo, first, layer, peer, person, record, solid, sync};
use kition_motion_project::LayerSource;

#[test]
fn slots_are_distinct_disjoint_and_below_the_repair_ids() {
    // `u64::MAX` is reserved by the KDM (no replica has it), so `u64::MAX - 1` is the largest actor.
    let bases: Vec<u64> = [1u64, 2, 3, (1 << 20) - 1, 1 << 20, u64::MAX - 1].iter().map(|a| kdm_model::slot_base(common::actor(*a))).collect();
    for base in &bases {
        assert!(*base >= 1 << 32, "no actor mints among a file's own ids");
        assert!(base + (1 << 32) <= 1 << 52, "every slot ends below the repair ids");
    }
    assert_ne!(bases[0], bases[1]);
    assert_ne!(bases[1], bases[2]);
    let object = kition_kit::kdm::ObjectId::from_string("some-object");
    let repair = kdm_model::repair_id(&object);
    assert!((1u64 << 52..1u64 << 53).contains(&repair));
    assert_eq!(repair, kdm_model::repair_id(&object), "the same on every replica");
}

#[test]
fn layers_added_concurrently_get_different_ids_and_both_survive() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    let mut ids = Vec::new();
    for (session, name) in [(&mut a, "a"), (&mut b, "b")] {
        let mut after = session.model().project.clone();
        let item = solid(&mut after, &format!("{name} solid"), [1.0, 1.0, 1.0]);
        let id = add_layer(&mut after, d.main, name, LayerSource::Solid { item });
        record(session, &person(name), &after);
        ids.push((item, id));
    }
    assert_ne!(ids[0], ids[1], "two actors never mint the same ids");
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    let p = &a.model().project;
    for (item, id) in ids {
        assert!(p.item(item).is_some(), "item {item:?} survives");
        assert_eq!(layer(p, d.main, id).source.item(), Some(item), "layer {id:?} survives with its source");
    }
    assert_eq!(p.comp(d.main).unwrap().layers.len(), 5);
}

fn in_own_slot(session: &common::Session) -> bool {
    let base = kdm_model::slot_base(session.replica().actor());
    (base..base + (1 << 32)).contains(&session.model().project.next_id)
}

#[test]
fn an_actor_keeps_minting_in_its_own_slot_after_reading_a_peers_ids() {
    let d = demo();
    let mut a = first(&d.project);
    let mut c = peer(&a, 3);
    // c's ids sit in a higher slot than a's; reading them must not move a's counter there.
    let mut after = c.model().project.clone();
    let item = solid(&mut after, "c solid", [1.0, 1.0, 1.0]);
    add_layer(&mut after, d.main, "c", LayerSource::Solid { item });
    record(&mut c, &person("c"), &after);
    sync(&mut a, &mut c);
    assert!(in_own_slot(&a), "a's counter stays in a's slot");
    assert!(in_own_slot(&c), "c's counter stays in c's slot");
    let mut ids = Vec::new();
    for (session, name) in [(&mut a, "a"), (&mut c, "c")] {
        let mut after = session.model().project.clone();
        let item = solid(&mut after, &format!("{name} solid 2"), [1.0, 1.0, 1.0]);
        ids.push((item, add_layer(&mut after, d.main, name, LayerSource::Solid { item })));
        record(session, &person(name), &after);
    }
    assert_ne!(ids[0], ids[1], "two actors never mint the same ids");
    sync(&mut a, &mut c);
    assert_eq!(a.model().project.comp(d.main).unwrap().layers.len(), 6);
}

#[test]
fn the_counter_never_goes_down_when_a_peer_deletes_what_it_minted() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    let mut after = a.model().project.clone();
    let item = solid(&mut after, "a solid", [1.0, 1.0, 1.0]);
    let id = add_layer(&mut after, d.main, "a", LayerSource::Solid { item });
    record(&mut a, &person("a"), &after);
    sync(&mut a, &mut b);
    // b deletes what a minted; an undo of b's (or a peer that has not seen the delete yet) can
    // still hold it, so a must never mint those ids again.
    let mut after = b.model().project.clone();
    after.items.remove(&item);
    after.comp_mut(d.main).unwrap().layers.retain(|l| l.id != id);
    record(&mut b, &person("b"), &after);
    sync(&mut a, &mut b);
    assert!(a.model().project.next_id > id.0.max(item.0), "a's counter is still past the ids it handed out");
}
