//! Random multi-session convergence: three sessions make random edits (keys added, moved,
//! edited and deleted, layers added, deleted, renamed, reordered, reparented, moved between
//! compositions, precompositions nested, items deleted, the Render Queue grown) and exchange
//! changes at random moments; whenever two have exchanged everything they hold the same replica
//! and the same project, and every project they hold is one the engine can use.

mod common;

use common::{Session, add_layer, assert_same_content, demo, first, peer, person, record, solid, sync};
use kition_motion_project::{ItemId, ItemKind, Keyframe, LayerId, LayerSource, Project, Value};

/// A tiny deterministic generator (no extra dependency).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn pick<T: Clone>(&mut self, items: &[T]) -> Option<T> {
        if items.is_empty() { None } else { items.get(self.below(items.len())).cloned() }
    }
}

const PATHS: [&str; 4] = ["transform/opacity", "transform/rotation", "transform/position", "transform/scale"];

fn value_for(path: &str, rng: &mut Rng) -> Value {
    let x = rng.below(400) as f64;
    match path {
        "transform/position" | "transform/scale" => Value::Vec3([x, x / 2.0, 0.0]),
        _ => Value::Scalar(x / 4.0),
    }
}

fn layers_of(p: &Project) -> Vec<(ItemId, LayerId)> {
    p.comps().flat_map(|(cid, c)| c.layers.iter().map(move |l| (*cid, l.id))).collect()
}

/// Deletes layers as the engine does: their children are unparented, their mattes cleared.
fn delete_layers(p: &mut Project, cid: ItemId, gone: &[LayerId]) {
    if let Some(comp) = p.comp_mut(cid) {
        comp.layers.retain(|l| !gone.contains(&l.id));
        for l in &mut comp.layers {
            if l.parent.is_some_and(|x| gone.contains(&x)) {
                l.parent = None;
            }
            if l.track_matte.is_some_and(|m| gone.contains(&m.layer)) {
                l.track_matte = None;
            }
        }
    }
}

fn random_edit(session: &mut Session, rng: &mut Rng, name: &str) -> String {
    let mut p = session.model().project.clone();
    let layers = layers_of(&p);
    let comps: Vec<ItemId> = p.comps().map(|(id, _)| *id).collect();
    let what = match rng.below(13) {
        0 | 1 => {
            let Some((cid, id)) = rng.pick(&layers) else { return "no layer".into() };
            let path = PATHS[rng.below(PATHS.len())];
            let time = common::seconds(rng.below(40) as f64 / 10.0);
            let value = value_for(path, rng);
            if let Some(prop) = p.comp_mut(cid).and_then(|c| c.layer_mut(id)).and_then(|l| l.props.prop_mut(path)) {
                kition_motion_project::keyframe::set_key(&mut prop.keys, Keyframe::new(time, value));
            }
            format!("key {path} of {id:?}")
        }
        2 => {
            let Some((cid, id)) = rng.pick(&layers) else { return "no layer".into() };
            let path = PATHS[rng.below(PATHS.len())];
            let shift = rng.below(20) as f64 / 10.0;
            let value = value_for(path, rng);
            let Some(prop) = p.comp_mut(cid).and_then(|c| c.layer_mut(id)).and_then(|l| l.props.prop_mut(path)) else { return "no prop".into() };
            if prop.keys.is_empty() {
                return "no key".into();
            }
            let k = rng.below(prop.keys.len());
            let mut key = prop.keys.remove(k);
            if rng.below(2) == 0 {
                key.time = common::seconds(shift);
            } else {
                key.value = value;
            }
            kition_motion_project::keyframe::set_key(&mut prop.keys, key);
            format!("edit a key of {path} of {id:?}")
        }
        3 => {
            let Some((cid, id)) = rng.pick(&layers) else { return "no layer".into() };
            let path = PATHS[rng.below(PATHS.len())];
            let Some(prop) = p.comp_mut(cid).and_then(|c| c.layer_mut(id)).and_then(|l| l.props.prop_mut(path)) else { return "no prop".into() };
            if prop.keys.is_empty() {
                return "no key".into();
            }
            let k = rng.below(prop.keys.len());
            prop.keys.remove(k);
            format!("delete a key of {path} of {id:?}")
        }
        4 => {
            let Some(cid) = rng.pick(&comps) else { return "no comp".into() };
            let Some(comp) = p.comp_mut(cid) else { return "no comp".into() };
            if comp.layers.len() < 2 {
                return "nothing to reorder".into();
            }
            let from = rng.below(comp.layers.len());
            let layer = comp.layers.remove(from);
            let to = rng.below(comp.layers.len() + 1);
            comp.layers.insert(to, layer);
            format!("move layer {from} to {to} in {cid:?}")
        }
        5 => {
            let Some(cid) = rng.pick(&comps) else { return "no comp".into() };
            let item = solid(&mut p, &format!("{name} solid"), [0.5, 0.5, 0.5]);
            let id = add_layer(&mut p, cid, &format!("{name} layer"), LayerSource::Solid { item });
            format!("new layer {id:?} in {cid:?}")
        }
        6 => {
            let Some((cid, id)) = rng.pick(&layers) else { return "no layer".into() };
            delete_layers(&mut p, cid, &[id]);
            format!("delete layer {id:?}")
        }
        7 => {
            let Some((cid, id)) = rng.pick(&layers) else { return "no layer".into() };
            let n = rng.below(1000);
            if let Some(l) = p.comp_mut(cid).and_then(|c| c.layer_mut(id)) {
                l.name = format!("{name}{n}");
            }
            format!("rename {id:?}")
        }
        8 => {
            let Some((cid, id)) = rng.pick(&layers) else { return "no layer".into() };
            let siblings: Vec<LayerId> = p.comp(cid).map(|c| c.layers.iter().map(|l| l.id).filter(|x| *x != id).collect()).unwrap_or_default();
            let parent = rng.pick(&siblings);
            // Locally never a cycle (the engine refuses one); concurrently one can form.
            let cycle = parent.is_some_and(|mut at| {
                for _ in 0..64 {
                    if at == id {
                        return true;
                    }
                    match p.comp(cid).and_then(|c| c.layer(at)).and_then(|l| l.parent) {
                        Some(next) => at = next,
                        None => return false,
                    }
                }
                true
            });
            if cycle {
                return "would cycle".into();
            }
            if let Some(l) = p.comp_mut(cid).and_then(|c| c.layer_mut(id)) {
                l.parent = parent;
            }
            format!("parent {id:?} to {parent:?}")
        }
        9 => {
            let Some(cid) = rng.pick(&comps) else { return "no comp".into() };
            let seconds = 2.0 + rng.below(6) as f64;
            if let Some(c) = p.comp_mut(cid) {
                if rng.below(2) == 0 {
                    c.duration = common::seconds(seconds);
                } else {
                    c.background = [rng.below(10) as f32 / 10.0, 0.0, 0.0];
                }
            }
            format!("comp settings of {cid:?}")
        }
        10 => {
            let (Some(outer), Some(inner)) = (rng.pick(&comps), rng.pick(&comps)) else { return "no comp".into() };
            if p.comp_contains(inner, outer) {
                return "would nest in itself".into();
            }
            let id = add_layer(&mut p, outer, "nested", LayerSource::Comp { item: inner });
            format!("nest {inner:?} in {outer:?} as {id:?}")
        }
        11 => {
            let solids: Vec<ItemId> = p.items.values().filter(|i| matches!(i.kind, ItemKind::Solid(_))).map(|i| i.id).collect();
            let Some(item) = rng.pick(&solids) else { return "no solid".into() };
            p.items.remove(&item);
            for cid in comps {
                let gone: Vec<LayerId> =
                    p.comp(cid).map(|c| c.layers.iter().filter(|l| l.source.item() == Some(item)).map(|l| l.id).collect()).unwrap_or_default();
                delete_layers(&mut p, cid, &gone);
            }
            format!("delete item {item:?}")
        }
        _ => {
            let Some(cid) = rng.pick(&comps) else { return "no comp".into() };
            let id = p.render_queue.iter().map(|i| i.id).max().unwrap_or(0) + 1;
            p.render_queue.push(kition_motion_project::render_queue::RenderQueueItem::new(id, cid));
            format!("queue {cid:?}")
        }
    };
    record(session, &person(name), &p);
    what
}

/// What the engine relies on: keys in time order at distinct times, layers whose source item
/// exists, parents and mattes in the layer's own composition, render items whose composition
/// exists, distinct Render Queue ids.
fn check(p: &Project, context: &str) {
    for (cid, comp) in p.comps() {
        let ids: Vec<LayerId> = comp.layers.iter().map(|l| l.id).collect();
        for l in &comp.layers {
            if let Some(item) = l.source.item() {
                assert!(p.item(item).is_some(), "{context}: layer {:?} of {cid:?} uses a deleted item", l.id);
            }
            if let Some(parent) = l.parent {
                assert!(ids.contains(&parent), "{context}: layer {:?} has a parent outside its comp", l.id);
            }
            l.props.walk("", &mut |path, prop| {
                assert!(prop.keys.windows(2).all(|w| w[0].time < w[1].time), "{context}: keys of {path} of {:?} out of order", l.id);
            });
        }
    }
    let mut seen = std::collections::HashSet::new();
    for r in &p.render_queue {
        assert!(p.comp(r.comp).is_some(), "{context}: a render item's composition is gone");
        assert!(seen.insert(r.id), "{context}: render queue id {} twice", r.id);
    }
}

#[test]
fn random_edits_on_three_sessions_converge() {
    for seed in [1u64, 7, 42, 2026] {
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        let d = demo();
        let a = first(&d.project);
        let b = peer(&a, 2);
        let c = peer(&a, 3);
        let mut sessions = [a, b, c];
        let mut log: Vec<String> = Vec::new();
        for step in 0..120 {
            let who = rng.below(3);
            let name = ["a", "b", "c"][who];
            let what = random_edit(&mut sessions[who], &mut rng, name);
            log.push(format!("{name}: {what}"));
            let context = format!("seed {seed}, step {step}; last: {}", log[log.len().saturating_sub(5)..].join(" | "));
            common::assert_same_content(&sessions[who].model().project, &common::read(sessions[who].replica()), &format!("{context}: project vs its replica"));
            if rng.below(4) == 0 {
                let x = rng.below(3);
                let y = (x + 1 + rng.below(2)) % 3;
                let (lo, hi) = (x.min(y), x.max(y));
                let (left, right) = sessions.split_at_mut(hi);
                sync(&mut left[lo], &mut right[0]);
                assert_eq!(left[lo].state_hash().unwrap(), right[0].state_hash().unwrap(), "{context}: replicas differ after an exchange");
                assert_same_content(&left[lo].model().project, &right[0].model().project, &context);
                check(&left[lo].model().project, &context);
            }
        }
        let [a, b, c] = &mut sessions;
        sync(a, b);
        sync(b, c);
        sync(a, b);
        assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap(), "seed {seed}: a and b");
        assert_eq!(b.state_hash().unwrap(), c.state_hash().unwrap(), "seed {seed}: b and c");
        assert_same_content(&a.model().project, &c.model().project, &format!("seed {seed}: final"));
        check(&a.model().project, &format!("seed {seed}: final"));
        // A fresh peer of the converged replica reads the same project.
        let fresh = peer(a, 9);
        assert_same_content(&a.model().project, &fresh.model().project, &format!("seed {seed}: a fresh peer"));
    }
}
