//! Ordered lists of objects (a composition's layers, a group's children, a property's keys, the
//! Render Queue): a write makes the live list exactly the target order with the fewest moves,
//! so a draft's edit elsewhere in the list keeps its place when it merges (data model 6.6, D6).

use std::collections::HashSet;

use kition_kit::kdm::{ObjectId, Tx};

use crate::KdmResult;

/// The longest common subsequence of `a` and `b` under `eq`, as index pairs in order (both sides
/// increasing). Capped: the table is O(n·m), and above a million cells (lists far longer than any
/// a person edits) nothing is matched, which callers treat as "every element changed" — still
/// correct, only less minimal.
pub(crate) fn lcs<A, B>(a: &[A], b: &[B], eq: impl Fn(&A, &B) -> bool) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 || n.saturating_mul(m) > 1_000_000 {
        return Vec::new();
    }
    let width = m + 1;
    let mut dp = vec![0u32; (n + 1) * width];
    let at = |i: usize, j: usize| i * width + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            let value = match (a.get(i), b.get(j)) {
                (Some(x), Some(y)) if eq(x, y) => dp.get(at(i + 1, j + 1)).copied().unwrap_or(0).saturating_add(1),
                _ => dp.get(at(i + 1, j)).copied().unwrap_or(0).max(dp.get(at(i, j + 1)).copied().unwrap_or(0)),
            };
            if let Some(cell) = dp.get_mut(at(i, j)) {
                *cell = value;
            }
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        let same = matches!((a.get(i), b.get(j)), (Some(x), Some(y)) if eq(x, y));
        if same {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if dp.get(at(i + 1, j)).copied().unwrap_or(0) >= dp.get(at(i, j + 1)).copied().unwrap_or(0) {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Makes `owner.field` exactly `target` (every object of which already exists): objects the list
/// holds that `target` does not are removed from it (not deleted; the caller decides that), as
/// are second listings of one object; the objects of a longest common subsequence of the live
/// list and `target` stay where they are, and every other one is moved (or inserted) right after
/// its predecessor in `target`. Answers how many list operations that took.
pub(crate) fn align(tx: &mut Tx<'_>, owner: &ObjectId, field: &str, target: &[ObjectId]) -> KdmResult<usize> {
    let mut current = tx.list_items(owner, field)?;
    if current == target {
        return Ok(0);
    }
    let mut ops = 0usize;
    let wanted: HashSet<&ObjectId> = target.iter().collect();
    let mut seen: HashSet<ObjectId> = HashSet::new();
    let mut keep = Vec::with_capacity(current.len());
    for object in &current {
        keep.push(wanted.contains(object) && seen.insert(object.clone()));
    }
    for index in (0..current.len()).rev() {
        if !keep.get(index).copied().unwrap_or(false) {
            tx.list_remove(owner, field, index)?;
            current.remove(index);
            ops += 1;
        }
    }
    let stay: HashSet<ObjectId> = lcs(&current, target, |a, b| a == b).into_iter().filter_map(|(i, _)| current.get(i).cloned()).collect();
    for (k, object) in target.iter().enumerate() {
        if stay.contains(object) {
            continue;
        }
        let dest = match k.checked_sub(1).and_then(|p| target.get(p)) {
            None => 0,
            Some(previous) => current.iter().position(|o| o == previous).map_or(0, |p| p + 1),
        };
        match current.iter().position(|o| o == object) {
            Some(from) => {
                let to = if from < dest { dest - 1 } else { dest };
                if from != to {
                    tx.list_move(owner, field, from, to)?;
                    let moved = current.remove(from);
                    current.insert(to, moved);
                    ops += 1;
                }
            }
            None => {
                let dest = dest.min(current.len());
                tx.list_insert(owner, field, dest, object)?;
                current.insert(dest, object.clone());
                ops += 1;
            }
        }
    }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use kition_kit::kdm::{ActorId, DocId, ObjectId, Origin, Replica};
    use kition_kit::session::KitModel;
    use serde_json::json;

    use super::align;
    use crate::{MotionModel, project_id};

    /// A replica whose Render Queue lists `n` objects, and those objects.
    fn queue(n: usize) -> (Replica, Vec<ObjectId>) {
        let actor = ActorId::from_u64(1).unwrap();
        Replica::create_with(MotionModel::schema(), DocId::new(), actor, &Origin::system("create"), |tx| {
            tx.create_with_id("Project", &project_id(), serde_json::Map::new())?;
            let mut objects = Vec::new();
            for i in 0..n {
                let object = tx.create("RenderItem", json!({ "item": { "id": i } }).as_object().cloned().unwrap())?;
                tx.list_insert(&project_id(), "renderQueue", i, &object)?;
                objects.push(object);
            }
            Ok(objects)
        })
        .unwrap()
    }

    fn aligned(replica: &mut Replica, target: &[ObjectId]) -> usize {
        let ops = replica.transact(&Origin::person("user:test"), |tx| align(tx, &project_id(), "renderQueue", target)).unwrap();
        assert_eq!(replica.list(&project_id(), "renderQueue").unwrap(), target);
        ops
    }

    #[test]
    fn one_object_moved_is_one_move() {
        let (mut replica, o) = queue(6);
        assert_eq!(aligned(&mut replica, &[o[1].clone(), o[2].clone(), o[3].clone(), o[0].clone(), o[4].clone(), o[5].clone()]), 1);
        assert_eq!(aligned(&mut replica, &[o[5].clone(), o[1].clone(), o[2].clone(), o[3].clone(), o[0].clone(), o[4].clone()]), 1);
    }

    #[test]
    fn reversals_inserts_and_removals_reach_the_target() {
        let (mut replica, o) = queue(5);
        let mut reversed = o.clone();
        reversed.reverse();
        assert_eq!(aligned(&mut replica, &reversed), 4, "a reversal of five keeps one in place");
        assert_eq!(aligned(&mut replica, &[o[0].clone(), o[2].clone()]), 4, "three removed, one moved back in order");
        assert_eq!(aligned(&mut replica, &[o[0].clone(), o[2].clone()]), 0, "already there");
    }
}
