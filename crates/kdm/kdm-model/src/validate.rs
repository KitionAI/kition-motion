//! Deterministic repair after a merge, an undo/redo, or an import ([`KitModel::validate`]): every
//! replica runs the same steps over the same merged state, so they all write the same repair.
//!
//! What a merge can leave that the engine must never see, and what becomes of it (the same as
//! the engine's own edits do it, where they do):
//! - a layer whose source item was deleted (a precomposition deleted while someone used it) is
//!   deleted, as `project.delete` deletes the layers using an item; so is a Render Queue item
//!   whose composition is gone;
//! - a parent, track matte or folder that is gone (or not in the layer's composition) is cleared;
//!   parenting, folder and precomposition cycles made by two concurrent edits are broken;
//! - list entries naming a deleted object are dropped, and an object two concurrent moves listed
//!   twice keeps its first listing;
//! - `Project.items` lists every live item once, repairing missing, stale or duplicate listings;
//! - a property's keys are put back in time order, two keys at one time (added concurrently)
//!   keep one;
//! - an id two actors minted alike (only possible if they share an id slot, see
//!   [`crate::slot_base`]) is given a repair id of its own; so is a Render Queue id two people
//!   added at once.
//!
//! [`KitModel::validate`]: kition_kit::session::KitModel::validate

use std::collections::{BTreeSet, HashMap, HashSet};

use kition_kit::kdm::{ObjectId, Tx};
use serde_json::{Value, json};

use crate::read::as_ref;
use crate::{KdmResult, MAX_DEPTH, kdm_err, list, project_id, tick};

/// The first repair id: above every actor's slot (`crate::slot_base` stays below 2^52), below
/// the ±2^53 a KDM integer holds.
pub(crate) const REPAIR_BASE: u64 = 1 << 52;

/// The id a repair gives an object whose engine id another object already holds: derived from
/// its object id (the same on every replica), in a range no actor mints from.
pub fn repair_id(object: &ObjectId) -> u64 {
    // FNV-1a.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in object.as_str().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    REPAIR_BASE + hash % REPAIR_BASE
}

pub(crate) fn validate(tx: &mut Tx<'_>, changed: &BTreeSet<ObjectId>) -> KdmResult<bool> {
    let mut wrote = false;
    // Deleted objects have no type left to tell what they were: then everything is checked.
    let unknown = changed.iter().any(|o| !tx.exists(o));
    let touched = |tx: &Tx<'_>, ty: &str| unknown || changed.iter().any(|o| tx.object_type(o).ok().as_deref() == Some(ty));
    let trees = touched(tx, "Node") || touched(tx, "Key") || touched(tx, "Layer");

    wrote |= items(tx)?;
    wrote |= item_list(tx)?;
    wrote |= layers(tx)?;
    wrote |= precomp_cycles(tx)?;
    if trees {
        wrote |= nodes(tx)?;
    }
    wrote |= render_queue(tx)?;
    Ok(wrote)
}

fn get_ref(tx: &Tx<'_>, object: &ObjectId, field: &str) -> KdmResult<Option<ObjectId>> {
    Ok(as_ref(&tx.get(object, field)?))
}

/// Removes `owner.field`'s entries naming a deleted object, and second listings; answers the
/// entries left.
fn clean_list(tx: &mut Tx<'_>, owner: &ObjectId, field: &str, listed: &mut HashSet<ObjectId>) -> KdmResult<(bool, Vec<ObjectId>)> {
    let items = tx.list_items(owner, field)?;
    let mut keep = Vec::with_capacity(items.len());
    let mut drop = Vec::new();
    for (index, object) in items.iter().enumerate() {
        if tx.exists(object) && listed.insert(object.clone()) {
            keep.push(object.clone());
        } else {
            drop.push(index);
        }
    }
    for index in drop.iter().rev() {
        tx.list_remove(owner, field, *index)?;
    }
    Ok((!drop.is_empty(), keep))
}

/// An item's or layer's stored id: `None` when it has none (a batch made it: the field is
/// read-only, the kit's to write); an error for a value that is not a whole number, which no
/// repair can tell the meaning of.
pub(crate) fn stored_id(value: &Value, whose: &str) -> KdmResult<Option<u64>> {
    match value {
        Value::Null => Ok(None),
        other => other.as_u64().map(Some).ok_or_else(|| kdm_err(format!("{whose} id is not a whole number"))),
    }
}

fn items(tx: &mut Tx<'_>) -> KdmResult<bool> {
    let mut wrote = false;
    let items = tx.objects_of_type("Item");
    let mut ids: HashSet<u64> = HashSet::new();
    for item in &items {
        // No id (an item a batch created: the field is read-only, the kit's to write) or one
        // another item holds: a repair id of its own.
        let id = stored_id(&tx.get(item, "id")?, "an item's")?;
        if !id.is_some_and(|id| ids.insert(id)) {
            let fresh = repair_id(item);
            tx.set(item, "id", json!(fresh))?;
            ids.insert(fresh);
            wrote = true;
        }
        if tx.get(item, "kind")?.as_str() == Some("comp") && !get_ref(tx, item, "comp")?.is_some_and(|c| tx.exists(&c)) {
            tx.delete(item)?;
            wrote = true;
            continue;
        }
        if let Some(parent) = get_ref(tx, item, "parent")?
            && !tx.exists(&parent)
        {
            tx.set(item, "parent", Value::Null)?;
            wrote = true;
        }
    }
    // Folder cycles (two people moving two folders into each other): the item of the cycle with
    // the smallest object id goes back to the project's root.
    let items = tx.objects_of_type("Item");
    for _ in 0..items.len() {
        let mut broke = false;
        for start in &items {
            let mut chain = vec![start.clone()];
            let mut at = start.clone();
            while let Some(parent) = get_ref(tx, &at, "parent")?.filter(|p| tx.exists(p)) {
                if let Some(pos) = chain.iter().position(|o| *o == parent) {
                    if let Some(smallest) = chain.get(pos..).and_then(|cycle| cycle.iter().min()).cloned() {
                        tx.set(&smallest, "parent", Value::Null)?;
                        wrote = true;
                        broke = true;
                    }
                    break;
                }
                if chain.len() > items.len() {
                    break;
                }
                chain.push(parent.clone());
                at = parent;
            }
            if broke {
                break;
            }
        }
        if !broke {
            break;
        }
    }
    Ok(wrote)
}

/// The root's item listings are a query view, not a manually reorderable project panel.
fn item_list(tx: &mut Tx<'_>) -> KdmResult<bool> {
    let proj = project_id();
    let mut listed = HashSet::new();
    let (mut wrote, kept) = clean_list(tx, &proj, "items", &mut listed)?;
    let mut end = kept.len();
    for item in tx.objects_of_type("Item") {
        if !listed.contains(&item) {
            tx.list_insert(&proj, "items", end, &item)?;
            end += 1;
            wrote = true;
        }
    }
    Ok(wrote)
}

/// The live composition objects, in object-id order.
fn comps(tx: &Tx<'_>) -> KdmResult<Vec<ObjectId>> {
    let mut out = Vec::new();
    for item in tx.objects_of_type("Item") {
        if let Some(comp) = get_ref(tx, &item, "comp")?.filter(|c| tx.exists(c)) {
            out.push(comp);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn delete_listed(tx: &mut Tx<'_>, owner: &ObjectId, field: &str, object: &ObjectId) -> KdmResult<()> {
    if let Some(index) = tx.list_items(owner, field)?.iter().position(|o| o == object) {
        tx.list_remove(owner, field, index)?;
    }
    if tx.exists(object) {
        tx.delete(object)?;
    }
    Ok(())
}

fn layers(tx: &mut Tx<'_>) -> KdmResult<bool> {
    let mut wrote = false;
    let mut listed = HashSet::new();
    // Items and layers share one id space (the engine's counter, and the tool ids agents name
    // them by): a layer's id is unique among every item and every layer of the project.
    let mut ids: HashSet<u64> = HashSet::new();
    for item in tx.objects_of_type("Item") {
        ids.extend(stored_id(&tx.get(&item, "id")?, "an item's")?);
    }
    for comp in comps(tx)? {
        let (cleaned, layers) = clean_list(tx, &comp, "layers", &mut listed)?;
        wrote |= cleaned;
        let mut alive = Vec::with_capacity(layers.len());
        for layer in layers {
            if let Some(source) = get_ref(tx, &layer, "sourceItem")?
                && !tx.exists(&source)
            {
                delete_listed(tx, &comp, "layers", &layer)?;
                wrote = true;
                continue;
            }
            // No id (a layer a batch created) or one another item or layer holds: a repair id.
            let id = stored_id(&tx.get(&layer, "id")?, "a layer's")?;
            if !id.is_some_and(|id| ids.insert(id)) {
                let fresh = repair_id(&layer);
                tx.set(&layer, "id", json!(fresh))?;
                ids.insert(fresh);
                wrote = true;
            }
            alive.push(layer);
        }
        let here: HashSet<&ObjectId> = alive.iter().collect();
        for layer in &alive {
            for field in ["parent", "matteLayer"] {
                if let Some(target) = get_ref(tx, layer, field)?
                    && (target == *layer || !here.contains(&target))
                {
                    tx.set(layer, field, Value::Null)?;
                    if field == "matteLayer" {
                        tx.set(layer, "matte", Value::Null)?;
                    }
                    wrote = true;
                }
            }
            if get_ref(tx, layer, "matteLayer")?.is_none() && !tx.get(layer, "matte")?.is_null() {
                tx.set(layer, "matte", Value::Null)?;
                wrote = true;
            }
        }
        // Parenting cycles: the layer of the cycle with the smallest object id is unparented.
        for _ in 0..alive.len() {
            let mut broke = false;
            for start in &alive {
                let mut chain = vec![start.clone()];
                let mut at = start.clone();
                while let Some(parent) = get_ref(tx, &at, "parent")? {
                    if let Some(pos) = chain.iter().position(|o| *o == parent) {
                        if let Some(smallest) = chain.get(pos..).and_then(|cycle| cycle.iter().min()).cloned() {
                            tx.set(&smallest, "parent", Value::Null)?;
                            wrote = true;
                            broke = true;
                        }
                        break;
                    }
                    if chain.len() > alive.len() {
                        break;
                    }
                    chain.push(parent.clone());
                    at = parent;
                }
                if broke {
                    break;
                }
            }
            if !broke {
                break;
            }
        }
    }
    Ok(wrote)
}

/// Precomposition cycles (two people nesting two compositions in each other): the layer that
/// closes a cycle, found walking compositions in object-id order, is deleted.
fn precomp_cycles(tx: &mut Tx<'_>) -> KdmResult<bool> {
    let mut wrote = false;
    for _ in 0..64 {
        // Composition item -> (its Comp object, [(layer, nested composition item)]).
        let mut graph: HashMap<ObjectId, (ObjectId, Vec<(ObjectId, ObjectId)>)> = HashMap::new();
        let mut order = Vec::new();
        for item in tx.objects_of_type("Item") {
            let Some(comp) = get_ref(tx, &item, "comp")?.filter(|c| tx.exists(c)) else { continue };
            let mut edges = Vec::new();
            for layer in tx.list_items(&comp, "layers")? {
                if !tx.exists(&layer) {
                    continue;
                }
                if let Some(source) = get_ref(tx, &layer, "sourceItem")?
                    && get_ref(tx, &source, "comp").ok().flatten().is_some()
                {
                    edges.push((layer, source));
                }
            }
            order.push(item.clone());
            graph.insert(item, (comp, edges));
        }
        let Some((comp, layer)) = find_cycle(&graph, &order) else { break };
        delete_listed(tx, &comp, "layers", &layer)?;
        wrote = true;
    }
    Ok(wrote)
}

/// The first edge (composition object, layer) that closes a cycle, depth-first from each item in
/// `order`.
fn find_cycle(graph: &HashMap<ObjectId, (ObjectId, Vec<(ObjectId, ObjectId)>)>, order: &[ObjectId]) -> Option<(ObjectId, ObjectId)> {
    fn walk(
        at: &ObjectId,
        graph: &HashMap<ObjectId, (ObjectId, Vec<(ObjectId, ObjectId)>)>,
        stack: &mut Vec<ObjectId>,
        done: &mut HashSet<ObjectId>,
        depth: usize,
    ) -> Option<(ObjectId, ObjectId)> {
        if depth > MAX_DEPTH || done.contains(at) {
            return None;
        }
        stack.push(at.clone());
        let (comp, edges) = graph.get(at)?;
        for (layer, nested) in edges {
            if stack.contains(nested) {
                return Some((comp.clone(), layer.clone()));
            }
            if let Some(found) = walk(nested, graph, stack, done, depth + 1) {
                return Some(found);
            }
        }
        stack.pop();
        done.insert(at.clone());
        None
    }
    let mut done = HashSet::new();
    for start in order {
        let mut stack = Vec::new();
        if let Some(found) = walk(start, graph, &mut stack, &mut done, 0) {
            return Some(found);
        }
    }
    None
}

fn nodes(tx: &mut Tx<'_>) -> KdmResult<bool> {
    let mut wrote = false;
    let mut listed = HashSet::new();
    let mut keys_listed = HashSet::new();
    for node in tx.objects_of_type("Node") {
        wrote |= clean_list(tx, &node, "children", &mut listed)?.0;
        let (cleaned, keys) = clean_list(tx, &node, "keys", &mut keys_listed)?;
        wrote |= cleaned;
        if keys.len() < 2 {
            continue;
        }
        let mut timed = Vec::with_capacity(keys.len());
        for key in &keys {
            timed.push((tick::from_value(&tx.get(key, "time")?)?, key.clone()));
        }
        timed.sort();
        let mut target = Vec::with_capacity(timed.len());
        let mut last = None;
        for (time, key) in timed {
            if last == Some(time) {
                if tx.exists(&key) {
                    tx.delete(&key)?;
                }
                wrote = true;
                continue;
            }
            last = Some(time);
            target.push(key);
        }
        wrote |= list::align(tx, &node, "keys", &target)? > 0;
    }
    Ok(wrote)
}

fn render_queue(tx: &mut Tx<'_>) -> KdmResult<bool> {
    let proj = project_id();
    let mut listed = HashSet::new();
    let (mut wrote, entries) = clean_list(tx, &proj, "renderQueue", &mut listed)?;
    let mut alive = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(comp) = get_ref(tx, &entry, "comp")?
            && !tx.exists(&comp)
        {
            delete_listed(tx, &proj, "renderQueue", &entry)?;
            wrote = true;
            continue;
        }
        alive.push(entry);
    }
    let mut items = Vec::with_capacity(alive.len());
    for entry in &alive {
        items.push(tx.get(entry, "item")?);
    }
    let mut max = items.iter().filter_map(|i| i.get("id").and_then(Value::as_u64)).max().unwrap_or(0);
    let mut seen = HashSet::new();
    for (entry, mut item) in alive.iter().zip(items) {
        let id = item.get("id").and_then(Value::as_u64).unwrap_or(0);
        let id = if seen.insert(id) {
            id
        } else {
            max = max.saturating_add(1);
            if let Some(slot) = item.get_mut("id") {
                *slot = json!(max);
            }
            seen.insert(max);
            tx.set(entry, "item", item)?;
            wrote = true;
            max
        };
        if tx.get(entry, "id")? != json!(id) {
            tx.set(entry, "id", json!(id))?;
            wrote = true;
        }
    }
    Ok(wrote)
}
