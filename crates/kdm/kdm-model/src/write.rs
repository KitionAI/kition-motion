//! `Project` -> replica: a full write (a new replica, a file import) and an incremental diff (the
//! engine command bridge) that writes only what changed.
//!
//! The order of a write: the project's own fields; items (every item, and every composition's
//! `Comp` object, exists before anything names it; a new item is appended to `Project.items`, a
//! deleted one leaves it in the same transaction); item folders; layers (deleted, created —
//! their property trees and keys first — or diffed; then each changed composition's layer list,
//! then the references of every layer written: source item, parent, track matte); the Render
//! Queue last. A composition's markers, guides and Graphics Templates controls, and a layer's
//! markers, are written with their composition or layer.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kition_kit::kdm::{ObjectId, Tx};
use kition_motion_keyframe::Keyframe;
use kition_motion_project::props::PropGroup;
use kition_motion_project::render_queue::RenderQueueItem;
use kition_motion_project::template_props::EgControl;
use kition_motion_project::{Comp, Guide, Item, ItemId, ItemKind, Layer, LayerId, Marker, Project};
use serde_json::{Map, Value, json};

use crate::fields::{self, Fields};
use crate::ids::{self, CompParts, ControlKey, Flat, FlatControl, Ids, LayerIds, LayerKey, NodeKey, NodeRef};
use crate::{KdmResult, kdm_err, list, project_id, tick};

fn obj(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn ref_value(object: Option<&ObjectId>) -> Value {
    object.map_or(Value::Null, |o| Value::String(o.as_str().to_owned()))
}

/// Writes only the keys of a map field that differ between `before` (`None`: an empty map) and
/// `after`: changed keys are set, keys `after` no longer has are deleted.
pub(crate) fn diff_map(tx: &mut Tx<'_>, object: &ObjectId, field: &str, before: Option<&Fields>, after: &Fields) -> KdmResult<bool> {
    let mut wrote = false;
    if let Some(before) = before {
        for key in before.keys() {
            if !after.contains_key(key) {
                tx.map_delete(object, field, key)?;
                wrote = true;
            }
        }
    }
    for (key, value) in after {
        if before.and_then(|b| b.get(key)) != Some(value) {
            tx.map_set(object, field, key, value.clone())?;
            wrote = true;
        }
    }
    Ok(wrote)
}

/// One object of a list whose objects the engine gives no id (a marker, a guide): its registers
/// and, for a marker, its comment (a `text` field).
#[derive(Clone, Debug, PartialEq)]
struct Record {
    fields: Fields,
    text: Option<String>,
}

fn marker_records(markers: &[Marker]) -> KdmResult<Vec<Record>> {
    markers
        .iter()
        .map(|m| {
            let (fields, comment) = fields::marker_parts(m)?;
            Ok(Record { fields, text: Some(comment) })
        })
        .collect()
}

fn guide_records(guides: &[Guide]) -> Vec<Record> {
    guides.iter().map(|g| Record { fields: fields::guide_parts(g), text: None }).collect()
}

/// What of a composition is written as lists of objects of its own.
#[derive(Clone, Copy)]
struct Parts<'a> {
    markers: &'a [Marker],
    guides: &'a [Guide],
    controls: &'a [EgControl],
}

impl<'a> Parts<'a> {
    const NONE: Parts<'static> = Parts { markers: &[], guides: &[], controls: &[] };

    fn of(comp: &'a Comp) -> Self {
        Parts { markers: &comp.markers, guides: &comp.guides, controls: comp.graphics_template.as_ref().map_or(&[], |t| t.controls.as_slice()) }
    }
}

/// Makes text field `field` of `object` read `after` where it read `before`, with one deletion
/// and one insertion (the common prefix and suffix kept), so a concurrent edit elsewhere in the
/// text keeps its place. When the field does not hold `before` (never expected) it is rewritten
/// whole.
fn splice_text(tx: &mut Tx<'_>, object: &ObjectId, field: &str, before: &str, after: &str) -> KdmResult<bool> {
    if before == after {
        return Ok(false);
    }
    let b: Vec<char> = before.chars().collect();
    let a: Vec<char> = after.chars().collect();
    if tx.text_len(object, field)? != b.len() {
        let len = tx.text_len(object, field)?;
        if len > 0 {
            tx.text_delete(object, field, 0, len)?;
        }
        if !after.is_empty() {
            tx.text_insert(object, field, 0, after)?;
        }
        return Ok(true);
    }
    let prefix = b.iter().zip(&a).take_while(|(x, y)| x == y).count();
    let room = b.len().min(a.len()).saturating_sub(prefix);
    let suffix = b.iter().rev().zip(a.iter().rev()).take(room).take_while(|(x, y)| x == y).count();
    let removed = b.len().saturating_sub(prefix).saturating_sub(suffix);
    if removed > 0 {
        tx.text_delete(object, field, prefix, removed)?;
    }
    let inserted: String = a.get(prefix..a.len().saturating_sub(suffix)).unwrap_or_default().iter().collect();
    if !inserted.is_empty() {
        tx.text_insert(object, field, prefix, &inserted)?;
    }
    Ok(true)
}

/// Writes a whole project into a fresh replica's creating transaction (import, or any other bulk
/// build). Returns the `Project` object's id.
pub fn write_document(tx: &mut Tx<'_>, project: &Project, ids: &mut Ids) -> KdmResult<ObjectId> {
    let proj = project_id();
    tx.create_with_id("Project", &proj, Map::new())?;
    *ids = Ids::default();
    Writer { tx, ids }.write(None, project)?;
    Ok(proj)
}

/// Writes only what differs between `before` (the project as the replica holds it, with `ids`
/// its objects) and `after`, and leaves `ids` naming `after`'s objects. Answers whether it wrote
/// anything.
pub fn write_changes(tx: &mut Tx<'_>, before: &Project, after: &Project, ids: &mut Ids) -> KdmResult<bool> {
    Writer { tx, ids }.write(Some(before), after)
}

struct Writer<'t, 'a> {
    tx: &'t mut Tx<'a>,
    ids: &'t mut Ids,
}

impl Writer<'_, '_> {
    fn write(&mut self, before: Option<&Project>, after: &Project) -> KdmResult<bool> {
        let empty = Project::default();
        let b = before.unwrap_or(&empty);
        let proj = project_id();
        let mut wrote = false;
        let before_attrs = before.map(fields::project_attrs).transpose()?;
        let after_attrs = fields::project_attrs(after)?;
        wrote |= diff_map(self.tx, &proj, "attrs", before_attrs.as_ref(), &after_attrs)?;
        let created = self.items(b, after, &mut wrote)?;
        wrote |= self.item_parents(b, after, &created)?;
        wrote |= self.layers(b, after)?;
        wrote |= self.render_queue(b, after)?;
        Ok(wrote)
    }

    // --- items ----------------------------------------------------------------------------

    /// Deletes, creates and diffs items (and their `Comp` objects); answers the items it created.
    fn items(&mut self, b: &Project, a: &Project, wrote: &mut bool) -> KdmResult<HashSet<ItemId>> {
        for id in b.items.keys() {
            if a.items.contains_key(id) {
                continue;
            }
            if let Some(object) = self.ids.items.remove(id) {
                unlist(self.tx, &project_id(), "items", &object)?;
                self.delete(&object)?;
                *wrote = true;
            }
            if let Some(comp) = self.ids.comps.remove(id) {
                self.delete(&comp)?;
            }
            if let Some(parts) = self.ids.parts.remove(id) {
                self.delete_parts(&parts)?;
            }
        }
        let mut created = HashSet::new();
        for (id, item) in &a.items {
            let known = self.ids.items.get(id).cloned().filter(|o| self.tx.exists(o));
            match (b.items.get(id), known) {
                (Some(old), Some(object)) => *wrote |= self.diff_item(&object, *id, old, item)?,
                _ => {
                    self.create_item(*id, item)?;
                    created.insert(*id);
                    *wrote = true;
                }
            }
        }
        Ok(created)
    }

    fn create_item(&mut self, id: ItemId, item: &Item) -> KdmResult<()> {
        let (kind, data) = fields::item_kind(item)?;
        let object = self.tx.create("Item", obj(json!({ "id": id.0, "kind": kind, "name": item.name })))?;
        let proj = project_id();
        let end = self.tx.list_items(&proj, "items")?.len();
        self.tx.list_insert(&proj, "items", end, &object)?;
        diff_map(self.tx, &object, "attrs", None, &fields::item_attrs(item)?)?;
        diff_map(self.tx, &object, "data", None, &data)?;
        if let ItemKind::Comp(comp) = &item.kind {
            let comp_object = self.create_comp(id, comp)?;
            self.tx.set(&object, "comp", ref_value(Some(&comp_object)))?;
        }
        self.ids.items.insert(id, object);
        Ok(())
    }

    /// Creates composition `id`'s `Comp` object with its markers, guides and template controls
    /// (deleting any it had), and records them in the ids.
    fn create_comp(&mut self, id: ItemId, comp: &Comp) -> KdmResult<ObjectId> {
        if let Some(stale) = self.ids.parts.remove(&id) {
            self.delete_parts(&stale)?;
        }
        let object = self.tx.create("Comp", Map::new())?;
        diff_map(self.tx, &object, "attrs", None, &fields::comp_attrs(comp)?)?;
        let (_, parts) = self.diff_parts(&object, Parts::NONE, &CompParts::default(), Parts::of(comp))?;
        self.ids.parts.insert(id, parts);
        self.ids.comps.insert(id, object.clone());
        Ok(object)
    }

    /// Writes a composition's markers, guides and template controls over `before`'s (`objects`
    /// their objects). Answers whether it wrote anything, and the objects of `after`'s.
    fn diff_parts(&mut self, comp: &ObjectId, before: Parts<'_>, objects: &CompParts, after: Parts<'_>) -> KdmResult<(bool, CompParts)> {
        let mut wrote = false;
        let mut parts = CompParts::default();
        let (w, markers) = if before.markers == after.markers && objects.markers.len() == before.markers.len() {
            (false, objects.markers.clone())
        } else {
            self.diff_records(
                comp,
                "markers",
                "Marker",
                Some("comment"),
                &["time"],
                &marker_records(before.markers)?,
                &objects.markers,
                &marker_records(after.markers)?,
            )?
        };
        wrote |= w;
        parts.markers = markers;
        let (w, guides) = if before.guides == after.guides && objects.guides.len() == before.guides.len() {
            (false, objects.guides.clone())
        } else {
            self.diff_records(comp, "guides", "Guide", None, &["position"], &guide_records(before.guides), &objects.guides, &guide_records(after.guides))?
        };
        wrote |= w;
        parts.guides = guides;
        let (w, controls) = if before.controls == after.controls && ids::flatten_controls(before.controls)?.0.len() == objects.controls.len() {
            (false, objects.controls.clone())
        } else {
            self.diff_controls(comp, before.controls, &objects.controls, after.controls)?
        };
        wrote |= w;
        parts.controls = controls;
        Ok((wrote, parts))
    }

    fn delete_parts(&mut self, parts: &CompParts) -> KdmResult<()> {
        for object in parts.markers.iter().chain(&parts.guides).chain(parts.controls.values()) {
            self.delete(object)?;
        }
        Ok(())
    }

    fn diff_item(&mut self, object: &ObjectId, id: ItemId, old: &Item, new: &Item) -> KdmResult<bool> {
        let mut wrote = false;
        if old.name != new.name {
            self.tx.set(object, "name", json!(new.name))?;
            wrote = true;
        }
        if fields::item_header(old) != fields::item_header(new) {
            wrote |= diff_map(self.tx, object, "attrs", Some(&fields::item_attrs(old)?), &fields::item_attrs(new)?)?;
        }
        let same_kind = match (&old.kind, &new.kind) {
            (ItemKind::Comp(_), ItemKind::Comp(_)) => true,
            (x, y) => x == y,
        };
        if !same_kind {
            let (old_kind, old_data) = fields::item_kind(old)?;
            let (new_kind, new_data) = fields::item_kind(new)?;
            if old_kind != new_kind {
                self.tx.set(object, "kind", json!(new_kind))?;
            }
            diff_map(self.tx, object, "data", Some(&old_data), &new_data)?;
            wrote = true;
        }
        match (&old.kind, &new.kind) {
            (ItemKind::Comp(x), ItemKind::Comp(y)) => {
                let known = self.ids.comps.get(&id).cloned().filter(|o| self.tx.exists(o));
                match known {
                    Some(comp) => {
                        if !Arc::ptr_eq(x, y) {
                            wrote |= diff_map(self.tx, &comp, "attrs", Some(&fields::comp_attrs(x)?), &fields::comp_attrs(y)?)?;
                            let objects = self.ids.parts.remove(&id).unwrap_or_default();
                            let (parts_wrote, parts) = self.diff_parts(&comp, Parts::of(x), &objects, Parts::of(y))?;
                            wrote |= parts_wrote;
                            self.ids.parts.insert(id, parts);
                        }
                    }
                    None => {
                        let comp = self.create_comp(id, y)?;
                        self.tx.set(object, "comp", ref_value(Some(&comp)))?;
                        wrote = true;
                    }
                }
            }
            (ItemKind::Comp(_), _) => {
                if let Some(comp) = self.ids.comps.remove(&id) {
                    self.delete(&comp)?;
                }
                if let Some(parts) = self.ids.parts.remove(&id) {
                    self.delete_parts(&parts)?;
                }
                self.tx.set(object, "comp", Value::Null)?;
                wrote = true;
            }
            (_, ItemKind::Comp(y)) => {
                let comp = self.create_comp(id, y)?;
                self.tx.set(object, "comp", ref_value(Some(&comp)))?;
                wrote = true;
            }
            _ => {}
        }
        Ok(wrote)
    }

    /// Every item's folder, once every item has an object.
    fn item_parents(&mut self, b: &Project, a: &Project, created: &HashSet<ItemId>) -> KdmResult<bool> {
        let mut wrote = false;
        for (id, item) in &a.items {
            let is_new = created.contains(id);
            if !is_new && b.items.get(id).map(|old| old.parent) == Some(item.parent) {
                continue;
            }
            let Some(object) = self.ids.items.get(id).cloned() else { continue };
            let target = item.parent.and_then(|p| self.ids.items.get(&p)).cloned();
            if is_new && target.is_none() {
                continue;
            }
            self.tx.set(&object, "parent", ref_value(target.as_ref()))?;
            wrote = true;
        }
        Ok(wrote)
    }

    // --- layers ---------------------------------------------------------------------------

    fn layers(&mut self, b: &Project, a: &Project) -> KdmResult<bool> {
        let mut wrote = false;
        let before_keys = ids::layer_keys(b);
        let after_keys = ids::layer_keys(a);
        let index =
            |keys: &[(ItemId, usize, LayerKey)]| -> HashMap<LayerKey, (ItemId, usize)> { keys.iter().map(|(cid, i, key)| (*key, (*cid, *i))).collect() };
        let before_index = index(&before_keys);
        let after_index = index(&after_keys);
        // Compositions that are the very same `Arc` before and after, with the same layer keys:
        // nothing in them changed. (When one layer id is held twice, deleting the holder in
        // another composition shifts this one's key to the other holder's object.)
        let comp_keys =
            |keys: &[(ItemId, usize, LayerKey)], cid: ItemId| -> Vec<LayerKey> { keys.iter().filter(|(c, _, _)| *c == cid).map(|(_, _, k)| *k).collect() };
        let unchanged: HashSet<ItemId> = a
            .comps()
            .filter(|(id, _)| matches!((b.comp_arc(**id), a.comp_arc(**id)), (Some(x), Some(y)) if Arc::ptr_eq(&x, &y)))
            .filter(|(id, _)| comp_keys(&before_keys, **id) == comp_keys(&after_keys, **id))
            .map(|(id, _)| *id)
            .collect();

        let mut old = std::mem::take(&mut self.ids.layers);
        for key in before_index.keys() {
            if after_index.contains_key(key) {
                continue;
            }
            if let Some(lids) = old.remove(key) {
                self.delete_layer(&lids)?;
                wrote = true;
            }
        }
        let mut layers: HashMap<LayerKey, LayerIds> = HashMap::with_capacity(after_keys.len());
        let mut touched: Vec<(ItemId, LayerKey)> = Vec::new();
        for (cid, i, key) in &after_keys {
            let Some(layer) = a.comp(*cid).and_then(|c| c.layers.get(*i)) else { continue };
            let previous = before_index.get(key).and_then(|(bcid, bi)| b.comp(*bcid).and_then(|c| c.layers.get(*bi)).map(|l| (*bcid, l)));
            let known = old.remove(key).filter(|l| self.tx.exists(&l.object));
            match (previous, known) {
                (Some((bcid, before_layer)), Some(mut lids)) => {
                    let untouched = bcid == *cid && unchanged.contains(cid);
                    if !untouched && before_layer != layer {
                        wrote |= self.diff_layer(&mut lids, before_layer, layer)?;
                        touched.push((*cid, *key));
                    } else if bcid != *cid {
                        // Moved to another composition as it was: its references resolve there.
                        touched.push((*cid, *key));
                    }
                    layers.insert(*key, lids);
                }
                (_, stale) => {
                    if let Some(stale) = stale {
                        self.delete_layer(&stale)?;
                    }
                    let lids = self.create_layer(*key, layer)?;
                    layers.insert(*key, lids);
                    touched.push((*cid, *key));
                    wrote = true;
                }
            }
        }
        // Whatever the old map still holds names no layer of `before` either (a stale view):
        // nothing lists those objects any more.
        for lids in old.values() {
            self.delete_layer(lids)?;
        }
        self.ids.layers = layers;

        // Each changed composition's stacking order.
        let mut comp_layers: HashMap<ItemId, Vec<LayerKey>> = HashMap::new();
        for (cid, _, key) in &after_keys {
            comp_layers.entry(*cid).or_default().push(*key);
        }
        for (cid, _) in a.comps() {
            if unchanged.contains(cid) {
                continue;
            }
            let Some(comp_object) = self.ids.comps.get(cid).cloned() else { continue };
            let target: Vec<ObjectId> = comp_layers.get(cid).map(|keys| keys.iter().filter_map(|k| self.ids.layer(*k).cloned()).collect()).unwrap_or_default();
            wrote |= list::align(self.tx, &comp_object, "layers", &target)? > 0;
        }

        // The references of every layer written, now that every layer has an object — and of
        // every other layer of a changed composition: when one id is held twice, deleting or
        // moving one holder shifts the other's key to another object, and a reference to it
        // from a layer nothing else changed must follow (`layer_refs` writes only what differs).
        let mut refs: Vec<(ItemId, LayerKey)> = Vec::with_capacity(after_keys.len());
        let mut seen: HashSet<LayerKey> = HashSet::with_capacity(after_keys.len());
        for (cid, key) in touched.into_iter().chain(after_keys.iter().filter(|(cid, _, _)| !unchanged.contains(cid)).map(|(cid, _, key)| (*cid, *key))) {
            if seen.insert(key) {
                refs.push((cid, key));
            }
        }
        for (cid, key) in refs {
            let Some((_, i)) = after_index.get(&key).copied() else { continue };
            let Some(comp) = a.comp(cid) else { continue };
            let Some(layer) = comp.layers.get(i) else { continue };
            let in_comp = |id: LayerId| -> Option<LayerKey> { comp_layers.get(&cid).and_then(|keys| keys.iter().find(|k| k.id == id).copied()) };
            wrote |= self.layer_refs(key, layer, &in_comp)?;
        }
        Ok(wrote)
    }

    fn layer_refs(&mut self, key: LayerKey, layer: &Layer, in_comp: &dyn Fn(LayerId) -> Option<LayerKey>) -> KdmResult<bool> {
        let Some(object) = self.ids.layer(key).cloned() else { return Ok(false) };
        let mut wrote = false;
        let source = layer.source.item().and_then(|i| self.ids.items.get(&i)).cloned();
        let parent = layer.parent.filter(|p| *p != layer.id).and_then(in_comp).and_then(|k| self.ids.layer(k)).cloned();
        let matte_target = layer.track_matte.filter(|m| m.layer != layer.id).and_then(|m| in_comp(m.layer)).and_then(|k| self.ids.layer(k)).cloned();
        let matte_kind = match (&matte_target, &layer.track_matte) {
            (Some(_), Some(m)) => serde_json::to_value(m.kind).map_err(kdm_err)?,
            _ => Value::Null,
        };
        for (field, value) in [
            ("sourceItem", ref_value(source.as_ref())),
            ("parent", ref_value(parent.as_ref())),
            ("matteLayer", ref_value(matte_target.as_ref())),
            ("matte", matte_kind),
        ] {
            if self.tx.get(&object, field)? != value {
                self.tx.set(&object, field, value)?;
                wrote = true;
            }
        }
        Ok(wrote)
    }

    fn create_layer(&mut self, key: LayerKey, layer: &Layer) -> KdmResult<LayerIds> {
        let mut lids = LayerIds {
            object: ObjectId::from_string(format!("layer:{}:{}", key.id.0, key.nth)),
            nodes: HashMap::new(),
            keys: HashMap::new(),
            markers: Vec::new(),
        };
        let root = self.diff_tree(&mut lids, None, &layer.props)?.1;
        let object = self.tx.create(
            "Layer",
            obj(json!({
                "id": layer.id.0,
                "name": layer.name,
                "startTime": tick::to_value(layer.start_time),
                "inPoint": tick::to_value(layer.in_point),
                "outPoint": tick::to_value(layer.out_point),
                "props": root.as_str(),
            })),
        )?;
        diff_map(self.tx, &object, "attrs", None, &fields::layer_attrs(layer)?)?;
        diff_map(self.tx, &object, "switches", None, &fields::switch_fields(&layer.switches)?)?;
        lids.markers = self.diff_records(&object, "markers", "Marker", Some("comment"), &["time"], &[], &[], &marker_records(&layer.markers)?)?.1;
        lids.object = object;
        Ok(lids)
    }

    fn diff_layer(&mut self, lids: &mut LayerIds, before: &Layer, after: &Layer) -> KdmResult<bool> {
        let object = lids.object.clone();
        let mut wrote = false;
        if before.id != after.id {
            self.tx.set(&object, "id", json!(after.id.0))?;
            wrote = true;
        }
        if before.name != after.name {
            self.tx.set(&object, "name", json!(after.name))?;
            wrote = true;
        }
        for (field, b, a) in
            [("startTime", before.start_time, after.start_time), ("inPoint", before.in_point, after.in_point), ("outPoint", before.out_point, after.out_point)]
        {
            if b != a {
                self.tx.set(&object, field, tick::to_value(a))?;
                wrote = true;
            }
        }
        if fields::layer_header(before) != fields::layer_header(after) {
            wrote |= diff_map(self.tx, &object, "attrs", Some(&fields::layer_attrs(before)?), &fields::layer_attrs(after)?)?;
        }
        if before.switches != after.switches {
            wrote |= diff_map(self.tx, &object, "switches", Some(&fields::switch_fields(&before.switches)?), &fields::switch_fields(&after.switches)?)?;
        }
        if before.markers != after.markers || lids.markers.len() != before.markers.len() {
            let objects = std::mem::take(&mut lids.markers);
            let (markers_wrote, markers) = self.diff_records(
                &object,
                "markers",
                "Marker",
                Some("comment"),
                &["time"],
                &marker_records(&before.markers)?,
                &objects,
                &marker_records(&after.markers)?,
            )?;
            wrote |= markers_wrote;
            lids.markers = markers;
        }
        if before.props != after.props {
            let (tree_wrote, root) = self.diff_tree(lids, Some(&before.props), &after.props)?;
            wrote |= tree_wrote;
            if self.tx.get(&object, "props")? != json!(root.as_str()) {
                self.tx.set(&object, "props", json!(root.as_str()))?;
                wrote = true;
            }
        }
        Ok(wrote)
    }

    fn delete_layer(&mut self, lids: &LayerIds) -> KdmResult<()> {
        self.delete(&lids.object)?;
        for marker in &lids.markers {
            self.delete(marker)?;
        }
        for node in lids.nodes.values() {
            self.delete(node)?;
        }
        for keys in lids.keys.values() {
            for key in keys {
                self.delete(key)?;
            }
        }
        Ok(())
    }

    fn delete(&mut self, object: &ObjectId) -> KdmResult<()> {
        if self.tx.exists(object) {
            self.tx.delete(object)?;
        }
        Ok(())
    }

    // --- markers and guides ---------------------------------------------------------------

    /// Aligns a list of id-less objects (markers, guides) so that an edited, moved or untouched
    /// one keeps its object and only a real insert or delete creates or deletes one, as
    /// [`Writer::diff_keys`] aligns keys: identical records first (a longest common
    /// subsequence), then records at the same `place` (an edited one), then records equal but for
    /// their place (a moved one), then, when exactly as many are left on both sides, the rest in
    /// order. Answers whether it wrote anything, and the objects in `after`'s order.
    #[allow(clippy::too_many_arguments)]
    fn diff_records(
        &mut self,
        owner: &ObjectId,
        field: &str,
        ty: &str,
        text: Option<&str>,
        place: &[&str],
        before: &[Record],
        objects: &[ObjectId],
        after: &[Record],
    ) -> KdmResult<(bool, Vec<ObjectId>)> {
        if before == after && objects.len() == before.len() {
            return Ok((false, objects.to_vec()));
        }
        if objects.len() != before.len() {
            // The view does not match its own objects (never expected): rewrite the list.
            for object in objects {
                self.delete(object)?;
            }
            let mut created = Vec::with_capacity(after.len());
            for record in after {
                created.push(self.create_record(ty, text, record)?);
            }
            list::align(self.tx, owner, field, &created)?;
            return Ok((true, created));
        }
        let mut match_b: Vec<Option<usize>> = vec![None; before.len()];
        let mut match_a: Vec<Option<usize>> = vec![None; after.len()];
        for (i, j) in list::lcs(before, after, |x, y| x == y) {
            match_b[i] = Some(j);
            match_a[j] = Some(i);
        }
        let rest = |r: &Record| -> (Vec<(String, Value)>, Option<String>) {
            (r.fields.iter().filter(|(k, _)| !place.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect(), r.text.clone())
        };
        let same_place = |x: &Record, y: &Record| place.iter().all(|k| x.fields.get(*k) == y.fields.get(*k));
        let pair_by = |match_b: &mut Vec<Option<usize>>, match_a: &mut Vec<Option<usize>>, same: &dyn Fn(&Record, &Record) -> bool| {
            for j in 0..after.len() {
                if match_a[j].is_some() {
                    continue;
                }
                if let Some(i) = (0..before.len()).find(|i| match_b[*i].is_none() && same(&before[*i], &after[j])) {
                    match_b[i] = Some(j);
                    match_a[j] = Some(i);
                }
            }
        };
        pair_by(&mut match_b, &mut match_a, &same_place);
        pair_by(&mut match_b, &mut match_a, &|x, y| rest(x) == rest(y));
        let left_b: Vec<usize> = (0..before.len()).filter(|i| match_b[*i].is_none()).collect();
        let left_a: Vec<usize> = (0..after.len()).filter(|j| match_a[*j].is_none()).collect();
        if left_b.len() == left_a.len() {
            for (i, j) in left_b.into_iter().zip(left_a) {
                match_b[i] = Some(j);
                match_a[j] = Some(i);
            }
        }
        let mut wrote = false;
        for (i, object) in objects.iter().enumerate() {
            if match_b[i].is_none() {
                self.delete(object)?;
                wrote = true;
            }
        }
        let mut target = Vec::with_capacity(after.len());
        for (j, record) in after.iter().enumerate() {
            match match_a[j].and_then(|i| objects.get(i).map(|o| (i, o.clone()))) {
                Some((i, object)) => {
                    wrote |= self.update_record(&object, text, &before[i], record)?;
                    target.push(object);
                }
                None => {
                    target.push(self.create_record(ty, text, record)?);
                    wrote = true;
                }
            }
        }
        wrote |= list::align(self.tx, owner, field, &target)? > 0;
        Ok((wrote, target))
    }

    fn create_record(&mut self, ty: &str, text: Option<&str>, record: &Record) -> KdmResult<ObjectId> {
        let object = self.tx.create(ty, record.fields.clone())?;
        if let (Some(field), Some(content)) = (text, &record.text)
            && !content.is_empty()
        {
            self.tx.text_insert(&object, field, 0, content)?;
        }
        Ok(object)
    }

    fn update_record(&mut self, object: &ObjectId, text: Option<&str>, before: &Record, after: &Record) -> KdmResult<bool> {
        let mut wrote = false;
        for key in before.fields.keys().chain(after.fields.keys().filter(|k| !before.fields.contains_key(*k))) {
            let new = after.fields.get(key);
            if before.fields.get(key) != new {
                self.tx.set(object, key, new.cloned().unwrap_or(Value::Null))?;
                wrote = true;
            }
        }
        if let Some(field) = text {
            wrote |= splice_text(self.tx, object, field, before.text.as_deref().unwrap_or_default(), after.text.as_deref().unwrap_or_default())?;
        }
        Ok(wrote)
    }

    // --- Graphics Templates controls -------------------------------------------------------

    /// Writes a composition's template controls over `before`'s (`objects` their objects by
    /// key), keeping every control whose key survives; a group's children are a list of its
    /// own. Answers whether it wrote anything, and the objects of `after`'s controls.
    fn diff_controls(
        &mut self,
        comp: &ObjectId,
        before: &[EgControl],
        objects: &HashMap<ControlKey, ObjectId>,
        after: &[EgControl],
    ) -> KdmResult<(bool, HashMap<ControlKey, ObjectId>)> {
        let (before_flat, _) = ids::flatten_controls(before)?;
        let (after_flat, top) = ids::flatten_controls(after)?;
        let before_map: HashMap<ControlKey, &FlatControl<'_>> = before_flat.iter().map(|f| (f.key, f)).collect();
        let after_keys: HashSet<ControlKey> = after_flat.iter().map(|f| f.key).collect();
        let mut old = objects.clone();
        let mut wrote = false;
        for f in &before_flat {
            if !after_keys.contains(&f.key)
                && let Some(object) = old.remove(&f.key)
            {
                self.delete(&object)?;
                wrote = true;
            }
        }
        let mut nodes: HashMap<ControlKey, ObjectId> = HashMap::with_capacity(after_flat.len());
        let mut created: HashSet<ControlKey> = HashSet::new();
        for f in &after_flat {
            let parts = fields::control_parts(f.control)?;
            let known = old.remove(&f.key).filter(|o| self.tx.exists(o));
            match (before_map.get(&f.key), known) {
                (Some(b), Some(object)) => {
                    let old_parts = fields::control_parts(b.control)?;
                    for (key, value) in &parts {
                        if old_parts.get(key) != Some(value) {
                            self.tx.set(&object, key, value.clone())?;
                            wrote = true;
                        }
                    }
                    nodes.insert(f.key, object);
                }
                (_, stale) => {
                    if let Some(stale) = stale {
                        self.delete(&stale)?;
                    }
                    let mut values = parts;
                    if values.get("asType").is_some_and(Value::is_null) {
                        values.remove("asType");
                    }
                    nodes.insert(f.key, self.tx.create("Control", values)?);
                    created.insert(f.key);
                    wrote = true;
                }
            }
        }
        // Leftovers name no control of `before` (a stale view): nothing lists them any more.
        for object in old.values() {
            self.delete(object)?;
        }
        for f in &after_flat {
            let same = before_map.get(&f.key).is_some_and(|b| b.children == f.children) && !created.contains(&f.key);
            if same && !f.children.iter().any(|k| created.contains(k)) {
                continue;
            }
            let Some(object) = nodes.get(&f.key).cloned() else { continue };
            let target: Vec<ObjectId> = f.children.iter().filter_map(|k| nodes.get(k).cloned()).collect();
            wrote |= list::align(self.tx, &object, "children", &target)? > 0;
        }
        let target: Vec<ObjectId> = top.iter().filter_map(|k| nodes.get(k).cloned()).collect();
        wrote |= list::align(self.tx, comp, "controls", &target)? > 0;
        Ok((wrote, nodes))
    }

    // --- property trees -------------------------------------------------------------------

    /// Writes `after`'s tree over `before`'s (`None`: a new layer, nothing written yet), keeping
    /// every node whose key survives, and leaves `lids` naming `after`'s objects. Answers whether
    /// it wrote anything, and the root group's object.
    fn diff_tree(&mut self, lids: &mut LayerIds, before: Option<&PropGroup>, after: &PropGroup) -> KdmResult<(bool, ObjectId)> {
        let mut wrote = false;
        let before_flat = match before {
            Some(root) => ids::flatten(root)?,
            None => Vec::new(),
        };
        let after_flat = ids::flatten(after)?;
        let before_map: HashMap<NodeKey, &Flat<'_>> = before_flat.iter().map(|f| (f.key, f)).collect();
        let after_keys: HashSet<NodeKey> = after_flat.iter().map(|f| f.key).collect();
        let mut old_nodes = std::mem::take(&mut lids.nodes);
        let mut old_keys = std::mem::take(&mut lids.keys);
        for f in &before_flat {
            if after_keys.contains(&f.key) {
                continue;
            }
            if let Some(object) = old_nodes.remove(&f.key) {
                self.delete(&object)?;
                wrote = true;
            }
            for key in old_keys.remove(&f.key).unwrap_or_default() {
                self.delete(&key)?;
            }
        }
        let mut nodes: HashMap<NodeKey, ObjectId> = HashMap::with_capacity(after_flat.len());
        let mut keys: HashMap<NodeKey, Vec<ObjectId>> = HashMap::new();
        let mut created: HashSet<NodeKey> = HashSet::new();
        for f in &after_flat {
            let previous = before_map.get(&f.key).copied();
            let known = old_nodes.remove(&f.key).filter(|o| self.tx.exists(o));
            let kept = match (previous, known) {
                (Some(b), Some(object)) if same_kind(b.node, f.node) => Some((b, object)),
                (_, stale) => {
                    if let Some(stale) = stale {
                        self.delete(&stale)?;
                        for key in old_keys.remove(&f.key).unwrap_or_default() {
                            self.delete(&key)?;
                        }
                    }
                    None
                }
            };
            match kept {
                Some((b, object)) => {
                    wrote |= self.diff_node(&object, b.node, f.node)?;
                    if let (NodeRef::Prop(bp), NodeRef::Prop(ap)) = (b.node, f.node) {
                        let before_objects = old_keys.remove(&f.key).unwrap_or_default();
                        let (keys_wrote, objects) = self.diff_keys(&object, &bp.keys, &before_objects, &ap.keys)?;
                        wrote |= keys_wrote;
                        if !objects.is_empty() {
                            keys.insert(f.key, objects);
                        }
                    }
                    nodes.insert(f.key, object);
                }
                None => {
                    let object = self.create_node(f.node)?;
                    if let NodeRef::Prop(p) = f.node {
                        let mut objects = Vec::with_capacity(p.keys.len());
                        for (index, k) in p.keys.iter().enumerate() {
                            let key_object = self.create_key(k)?;
                            self.tx.list_insert(&object, "keys", index, &key_object)?;
                            objects.push(key_object);
                        }
                        if !objects.is_empty() {
                            keys.insert(f.key, objects);
                        }
                    }
                    nodes.insert(f.key, object);
                    created.insert(f.key);
                    wrote = true;
                }
            }
        }
        // Leftovers name no node of `before` (a stale view): nothing lists them any more.
        for object in old_nodes.values() {
            self.delete(object)?;
        }
        for objects in old_keys.values() {
            for object in objects {
                self.delete(object)?;
            }
        }
        for f in &after_flat {
            if !matches!(f.node, NodeRef::Group(_)) {
                continue;
            }
            let same_children = before_map.get(&f.key).is_some_and(|b| b.children == f.children) && !created.contains(&f.key);
            if same_children && !f.children.iter().any(|k| created.contains(k)) {
                continue;
            }
            let Some(object) = nodes.get(&f.key).cloned() else { continue };
            let target: Vec<ObjectId> = f.children.iter().filter_map(|k| nodes.get(k).cloned()).collect();
            wrote |= list::align(self.tx, &object, "children", &target)? > 0;
        }
        let root = after_flat.first().and_then(|f| nodes.get(&f.key)).cloned().ok_or_else(|| kdm_err("a layer's property tree has no root"))?;
        lids.nodes = nodes;
        lids.keys = keys;
        Ok((wrote, root))
    }

    fn create_node(&mut self, node: NodeRef<'_>) -> KdmResult<ObjectId> {
        let mut fields = match node {
            NodeRef::Prop(p) => obj(json!({ "uid": p.uid, "node": "prop", "meta": fields::prop_meta(p)?, "value": fields::value_json(&p.value)? })),
            NodeRef::Group(g) => obj(json!({ "uid": g.uid, "node": "group", "meta": fields::group_meta(g)?, "enabled": g.enabled })),
        };
        if let NodeRef::Prop(p) = node
            && let Some(expr) = &p.expr
        {
            fields.insert("expr".into(), serde_json::to_value(expr).map_err(kdm_err)?);
        }
        self.tx.create("Node", fields)
    }

    fn diff_node(&mut self, object: &ObjectId, before: NodeRef<'_>, after: NodeRef<'_>) -> KdmResult<bool> {
        let mut wrote = false;
        match (before, after) {
            (NodeRef::Prop(b), NodeRef::Prop(a)) => {
                if b == a {
                    return Ok(false);
                }
                if b.uid != a.uid {
                    self.tx.set(object, "uid", json!(a.uid))?;
                    wrote = true;
                }
                if b.value != a.value {
                    self.tx.set(object, "value", fields::value_json(&a.value)?)?;
                    wrote = true;
                }
                if b.expr != a.expr {
                    self.tx.set(object, "expr", serde_json::to_value(&a.expr).map_err(kdm_err)?)?;
                    wrote = true;
                }
                if !fields::prop_meta_eq(b, a) {
                    self.tx.set(object, "meta", fields::prop_meta(a)?)?;
                    wrote = true;
                }
            }
            (NodeRef::Group(b), NodeRef::Group(a)) => {
                if b.uid != a.uid {
                    self.tx.set(object, "uid", json!(a.uid))?;
                    wrote = true;
                }
                if b.enabled != a.enabled {
                    self.tx.set(object, "enabled", json!(a.enabled))?;
                    wrote = true;
                }
                if !fields::group_meta_eq(b, a) {
                    self.tx.set(object, "meta", fields::group_meta(a)?)?;
                    wrote = true;
                }
            }
            _ => return Err(kdm_err("diff_node: a property and a group")),
        }
        Ok(wrote)
    }

    // --- keyframes ------------------------------------------------------------------------

    fn create_key(&mut self, k: &Keyframe) -> KdmResult<ObjectId> {
        let (time, value, data) = fields::key_parts(k)?;
        self.tx.create("Key", obj(json!({ "time": time, "value": value, "data": data })))
    }

    /// Aligns a property's keys (they have no ids in the engine) so that an edited, moved or
    /// untouched key keeps its object and only a real insert or delete creates or deletes one:
    /// a longest common subsequence of identical keys first; then, of what is left, keys at the
    /// same time (an edited key); then keys with the same content (a key moved in time); then,
    /// when exactly as many are left on both sides, the rest in order (keys moved and edited
    /// together). Answers whether it wrote anything, and the objects in `after`'s order.
    fn diff_keys(&mut self, owner: &ObjectId, before: &[Keyframe], objects: &[ObjectId], after: &[Keyframe]) -> KdmResult<(bool, Vec<ObjectId>)> {
        if before == after && objects.len() == before.len() {
            return Ok((false, objects.to_vec()));
        }
        if objects.len() != before.len() {
            // The view does not match its own objects (never expected): rewrite the list.
            for object in objects {
                self.delete(object)?;
            }
            let mut created = Vec::with_capacity(after.len());
            for k in after {
                created.push(self.create_key(k)?);
            }
            list::align(self.tx, owner, "keys", &created)?;
            return Ok((true, created));
        }
        let parts = |keys: &[Keyframe]| keys.iter().map(fields::key_parts).collect::<KdmResult<Vec<_>>>();
        let bp = parts(before)?;
        let ap = parts(after)?;
        let mut match_b: Vec<Option<usize>> = vec![None; bp.len()];
        let mut match_a: Vec<Option<usize>> = vec![None; ap.len()];
        for (i, j) in list::lcs(&bp, &ap, |x, y| x == y) {
            match_b[i] = Some(j);
            match_a[j] = Some(i);
        }
        let pair_by = |match_b: &mut Vec<Option<usize>>, match_a: &mut Vec<Option<usize>>, same: &dyn Fn(usize, usize) -> bool| {
            for j in 0..ap.len() {
                if match_a[j].is_some() {
                    continue;
                }
                if let Some(i) = (0..bp.len()).find(|i| match_b[*i].is_none() && same(*i, j)) {
                    match_b[i] = Some(j);
                    match_a[j] = Some(i);
                }
            }
        };
        pair_by(&mut match_b, &mut match_a, &|i, j| bp[i].0 == ap[j].0);
        pair_by(&mut match_b, &mut match_a, &|i, j| bp[i].1 == ap[j].1 && bp[i].2 == ap[j].2);
        let left_b: Vec<usize> = (0..bp.len()).filter(|i| match_b[*i].is_none()).collect();
        let left_a: Vec<usize> = (0..ap.len()).filter(|j| match_a[*j].is_none()).collect();
        if left_b.len() == left_a.len() {
            for (i, j) in left_b.into_iter().zip(left_a) {
                match_b[i] = Some(j);
                match_a[j] = Some(i);
            }
        }
        let mut wrote = false;
        for (i, object) in objects.iter().enumerate() {
            if match_b[i].is_none() {
                self.delete(object)?;
                wrote = true;
            }
        }
        let mut target = Vec::with_capacity(ap.len());
        for (j, (time, value, data)) in ap.iter().enumerate() {
            match match_a[j].and_then(|i| objects.get(i).map(|o| (i, o.clone()))) {
                Some((i, object)) => {
                    let (b_time, b_value, b_data) = &bp[i];
                    for (field, old, new) in [("time", b_time, time), ("value", b_value, value), ("data", b_data, data)] {
                        if old != new {
                            self.tx.set(&object, field, new.clone())?;
                            wrote = true;
                        }
                    }
                    target.push(object);
                }
                None => {
                    target.push(self.tx.create("Key", obj(json!({ "time": time, "value": value, "data": data })))?);
                    wrote = true;
                }
            }
        }
        wrote |= list::align(self.tx, owner, "keys", &target)? > 0;
        Ok((wrote, target))
    }

    // --- Render Queue ---------------------------------------------------------------------

    fn render_queue(&mut self, b: &Project, a: &Project) -> KdmResult<bool> {
        if b.render_queue == a.render_queue && self.ids.render.len() == b.render_queue.len() {
            return Ok(false);
        }
        let proj = project_id();
        let keyed = |queue: &[RenderQueueItem]| -> Vec<(u64, u32)> {
            let mut seen: HashMap<u64, u32> = HashMap::new();
            queue
                .iter()
                .map(|r| {
                    let n = seen.entry(r.id).or_insert(0);
                    let key = (r.id, *n);
                    *n = n.saturating_add(1);
                    key
                })
                .collect()
        };
        let mut old: HashMap<(u64, u32), (ObjectId, &RenderQueueItem)> = HashMap::new();
        if self.ids.render.len() == b.render_queue.len() {
            for ((key, item), object) in keyed(&b.render_queue).into_iter().zip(&b.render_queue).zip(self.ids.render.iter()) {
                old.insert(key, (object.clone(), item));
            }
        } else {
            for object in std::mem::take(&mut self.ids.render) {
                self.delete(&object)?;
            }
        }
        let mut wrote = false;
        let mut target = Vec::with_capacity(a.render_queue.len());
        for (key, item) in keyed(&a.render_queue).into_iter().zip(&a.render_queue) {
            let comp = ref_value(self.ids.items.get(&item.comp));
            let definition = fields::render_item(item)?;
            match old.remove(&key).filter(|(o, _)| self.tx.exists(o)) {
                Some((object, previous)) => {
                    // A change of what one machine's render left on the item (its status) is no
                    // change of the document.
                    if fields::render_item(previous)? != definition {
                        self.tx.set(&object, "item", definition)?;
                        if previous.comp != item.comp {
                            self.tx.set(&object, "comp", comp)?;
                        }
                        wrote = true;
                    }
                    target.push(object);
                }
                None => {
                    let mut fields = obj(json!({ "id": item.id, "item": definition }));
                    if !comp.is_null() {
                        fields.insert("comp".into(), comp);
                    }
                    target.push(self.tx.create("RenderItem", fields)?);
                    wrote = true;
                }
            }
        }
        for (object, _) in old.into_values() {
            self.delete(&object)?;
            wrote = true;
        }
        wrote |= list::align(self.tx, &proj, "renderQueue", &target)? > 0;
        self.ids.render = target;
        Ok(wrote)
    }
}

/// Removes every listing of `object` from `owner.field`.
pub(crate) fn unlist(tx: &mut Tx<'_>, owner: &ObjectId, field: &str, object: &ObjectId) -> KdmResult<()> {
    let listed = tx.list_items(owner, field)?;
    for index in (0..listed.len()).rev() {
        if listed.get(index) == Some(object) {
            tx.list_remove(owner, field, index)?;
        }
    }
    Ok(())
}

fn same_kind(a: NodeRef<'_>, b: NodeRef<'_>) -> bool {
    matches!((a, b), (NodeRef::Prop(_), NodeRef::Prop(_)) | (NodeRef::Group(_), NodeRef::Group(_)))
}
