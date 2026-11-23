//! Replica -> `Project`: rebuilds the engine's project from a committed replica.
//!
//! Reading is lenient where a merge can leave the replica between repairs (a list naming a
//! deleted object, a reference to a deleted layer, one object listed twice): such an entry is
//! skipped or read as "none", never a panic. Malformed data (a register that is not what the
//! schema's writer puts there) is an error the caller reports.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kition_kit::kdm::{ObjectId, Replica};
use kition_motion_keyframe::Keyframe;
use kition_motion_project::props::{Node, PropGroup, Uid};
use kition_motion_project::render_queue::RenderQueueItem;
use kition_motion_project::template_props::{EgControl, EgKind, GraphicsTemplate};
use kition_motion_project::{Guide, Item, ItemId, ItemKind, LayerId, LayerSource, Marker, MatteKind, Project, TrackMatte};
use serde_json::{Map, Value, json};

use crate::fields::{self, Fields};
use crate::ids::{self, CompParts, ControlKey, Ids, LayerIds};
use crate::session::SLOT_SIZE;
use crate::validate::{REPAIR_BASE, repair_id, stored_id};
use crate::{KdmResult, MAX_DEPTH, kdm_err, project_id, tick};

pub(crate) fn as_ref(value: &Value) -> Option<ObjectId> {
    value.as_str().map(ObjectId::from_string)
}

fn as_u64(value: &Value, what: &str) -> KdmResult<u64> {
    value.as_u64().ok_or_else(|| kdm_err(format!("`{what}` is not a whole number")))
}

fn map_fields(replica: &Replica, object: &ObjectId, field: &str) -> KdmResult<Fields> {
    Ok(replica.map_entries(object, field)?.into_iter().collect::<Map<String, Value>>())
}

/// The registers `names` of `object` that hold a value.
fn registers(replica: &Replica, object: &ObjectId, names: &[&str]) -> KdmResult<Fields> {
    let mut fields = Fields::new();
    for name in names {
        let value = replica.get(object, name)?;
        if !value.is_null() {
            fields.insert((*name).to_owned(), value);
        }
    }
    Ok(fields)
}

const MARKER_REGISTERS: [&str; 8] = ["time", "duration", "label", "chapter", "url", "protected", "frameTarget", "cuePoint"];
const GUIDE_REGISTERS: [&str; 2] = ["vertical", "position"];
const CONTROL_REGISTERS: [&str; 4] = ["id", "name", "kind", "asType"];

/// The live objects `owner.field` lists, each once, in list order.
fn listed(replica: &Replica, owner: &ObjectId, field: &str) -> KdmResult<Vec<ObjectId>> {
    let mut seen = HashSet::new();
    Ok(replica.list(owner, field)?.into_iter().filter(|o| replica.exists(o) && seen.insert(o.clone())).collect())
}

/// The markers `owner` lists, in time order as the engine keeps them (markers at one time in
/// list order), with their objects.
fn read_markers(replica: &Replica, owner: &ObjectId) -> KdmResult<(Vec<Marker>, Vec<ObjectId>)> {
    let mut read = Vec::new();
    for object in listed(replica, owner, "markers")? {
        let marker = fields::marker_from(&registers(replica, &object, &MARKER_REGISTERS)?, replica.text(&object, "comment")?)?;
        read.push((marker, object));
    }
    read.sort_by_key(|(m, _)| m.time);
    Ok(read.into_iter().unzip())
}

/// The guides a composition lists, with their objects.
fn read_guides(replica: &Replica, comp: &ObjectId) -> KdmResult<(Vec<Guide>, Vec<ObjectId>)> {
    let mut guides = Vec::new();
    let mut objects = Vec::new();
    for object in listed(replica, comp, "guides")? {
        guides.push(fields::guide_from(&registers(replica, &object, &GUIDE_REGISTERS)?)?);
        objects.push(object);
    }
    Ok((guides, objects))
}

/// Reads Graphics Templates controls, recording each one's object by key in pre-order (the order
/// [`ids::flatten_controls`] gives).
struct Controls<'r> {
    replica: &'r Replica,
    objects: HashMap<ControlKey, ObjectId>,
    counts: HashMap<u64, u32>,
    /// Controls already read: one listed twice, or under its own descendant, is read once.
    seen: HashSet<ObjectId>,
}

impl Controls<'_> {
    fn list(&mut self, owner: &ObjectId, field: &str, depth: usize) -> KdmResult<Vec<EgControl>> {
        if depth > MAX_DEPTH {
            return Err(kdm_err("Graphics Templates controls are nested too deep to read"));
        }
        let mut out = Vec::new();
        for object in self.replica.list(owner, field)? {
            if !self.replica.exists(&object) || !self.seen.insert(object.clone()) {
                continue;
            }
            let mut control = fields::control_from(&registers(self.replica, &object, &CONTROL_REGISTERS)?)?;
            let nth = self.counts.entry(control.id).or_insert(0);
            self.objects.insert(ControlKey { id: control.id, nth: *nth }, object.clone());
            *nth = nth.saturating_add(1);
            if let EgKind::Group { children } = &mut control.kind {
                *children = self.list(&object, "children", depth + 1)?;
            }
            out.push(control);
        }
        Ok(out)
    }
}

/// One composition's layers as read, before their keys are known: (layer, its ids).
type ReadLayers = Vec<(kition_motion_project::Layer, LayerIds)>;

/// Rebuilds the whole project from a committed replica, with the object ids of everything in it.
/// The project's id counter is past every id it holds and never below this replica's own actor's
/// slot ([`crate::slot_base`]).
///
/// The project is what the root reaches: the items `Project.items` lists (a deleted item, or a
/// second listing, skipped), as an agent following fields from the root finds them. A replica of
/// an earlier schema is refused, never read: there is no migration, so a room made before the
/// root listed its items is opened again from its `.kmotion` file.
pub fn read_document(replica: &Replica) -> KdmResult<(Project, Ids)> {
    let made = replica.min_schema_version();
    let reads = replica.schema().schema_version;
    if made < reads {
        return Err(kdm_err(format!(
            "this replica was made on schema {made} by an earlier version of the Motion kit, which reads schema {reads} only: open the project's .kmotion file instead"
        )));
    }
    let proj = project_id();
    let attrs = map_fields(replica, &proj, "attrs")?;
    let mut project: Project = fields::rebuild(attrs, vec![("items", json!({})), ("next_id", json!(0))])?;
    project.render_queue.clear();
    let mut ids = Ids::default();

    // Every item's id first (folders and source items name each other), each id once, in
    // object-id order (the order the repair gives repeated ids new ones in).
    let mut objects: Vec<ObjectId> = replica.list(&proj, "items")?.into_iter().filter(|o| replica.exists(o)).collect();
    objects.sort();
    objects.dedup();
    let mut item_of: HashMap<ObjectId, ItemId> = HashMap::new();
    let mut taken: HashSet<ItemId> = HashSet::new();
    for object in &objects {
        // An item without an id (a batch made it and the repair has not numbered it yet) reads with
        // the repair's id, derived from its object id.
        let stored = stored_id(&replica.get(object, "id")?, "an item's")?;
        let mut id = ItemId(stored.unwrap_or_else(|| repair_id(object)));
        if !taken.insert(id) {
            id = ItemId(repair_id(object));
            taken.insert(id);
        }
        item_of.insert(object.clone(), id);
    }
    // Every layer's id, by object: parents and mattes name layers.
    let mut layer_of: HashMap<ObjectId, LayerId> = HashMap::new();
    for object in &objects {
        let Some(comp) = as_ref(&replica.get(object, "comp")?).filter(|c| replica.exists(c)) else { continue };
        for layer in replica.list(&comp, "layers")? {
            if replica.exists(&layer) && !layer_of.contains_key(&layer) {
                layer_of.insert(layer.clone(), LayerId(stored_id(&replica.get(&layer, "id")?, "a layer's")?.unwrap_or_else(|| repair_id(&layer))));
            }
        }
    }

    let mut comp_layers: HashMap<ItemId, Vec<LayerIds>> = HashMap::new();
    let mut listed: HashSet<ObjectId> = HashSet::new();
    for object in &objects {
        let Some(id) = item_of.get(object).copied() else { continue };
        let attrs = map_fields(replica, object, "attrs")?;
        let parent = as_ref(&replica.get(object, "parent")?).and_then(|p| item_of.get(&p).copied());
        let kind_name = replica.get(object, "kind")?.as_str().map(str::to_owned).ok_or_else(|| kdm_err("an item's kind is not a string"))?;
        let kind = if kind_name == "comp" {
            let comp_object =
                as_ref(&replica.get(object, "comp")?).filter(|c| replica.exists(c)).ok_or_else(|| kdm_err("a composition item has no composition"))?;
            let mut comp = fields::comp_from(map_fields(replica, &comp_object, "attrs")?)?;
            let mut parts = CompParts::default();
            (comp.markers, parts.markers) = read_markers(replica, &comp_object)?;
            (comp.guides, parts.guides) = read_guides(replica, &comp_object)?;
            let mut controls = Controls { replica, objects: HashMap::new(), counts: HashMap::new(), seen: HashSet::new() };
            let read_controls = controls.list(&comp_object, "controls", 0)?;
            parts.controls = controls.objects;
            match &mut comp.graphics_template {
                Some(template) => template.controls = read_controls,
                // Controls a collaborator added while another's template was not there yet.
                None if !read_controls.is_empty() => comp.graphics_template = Some(GraphicsTemplate { name: String::new(), controls: read_controls }),
                None => {}
            }
            ids.parts.insert(id, parts);
            let mut read: ReadLayers = Vec::new();
            let in_comp: HashSet<ObjectId> = replica.list(&comp_object, "layers")?.into_iter().filter(|l| replica.exists(l)).collect();
            for layer_object in replica.list(&comp_object, "layers")? {
                // A layer listed twice (two concurrent moves) is read once, where it is first listed.
                if !replica.exists(&layer_object) || !listed.insert(layer_object.clone()) {
                    continue;
                }
                read.push(read_layer(replica, &layer_object, &item_of, &layer_of, &in_comp)?);
            }
            let mut layer_ids = Vec::with_capacity(read.len());
            for (layer, lids) in read {
                comp.layers.push(layer);
                layer_ids.push(lids);
            }
            comp_layers.insert(id, layer_ids);
            ids.comps.insert(id, comp_object);
            ItemKind::Comp(Arc::new(comp))
        } else {
            fields::kind_from(&kind_name, map_fields(replica, object, "data")?)?
        };
        let name = replica.get(object, "name")?;
        let mut item: Item =
            fields::rebuild(attrs, vec![("id", json!(id.0)), ("name", name), ("parent", json!(parent.map(|p| p.0))), ("kind", json!({ "type": "Folder" }))])?;
        item.kind = kind;
        ids.items.insert(id, object.clone());
        project.items.insert(id, item);
    }

    // Layer keys follow the canonical order `layer_keys` gives, which is the order read above.
    let mut slots: HashMap<ItemId, std::vec::IntoIter<LayerIds>> = comp_layers.into_iter().map(|(k, v)| (k, v.into_iter())).collect();
    for (cid, _, key) in ids::layer_keys(&project) {
        if let Some(lids) = slots.get_mut(&cid).and_then(Iterator::next) {
            ids.layers.insert(key, lids);
        }
    }

    for object in replica.list(&proj, "renderQueue")? {
        if !replica.exists(&object) {
            continue;
        }
        let mut item: RenderQueueItem = fields::render_item_from(replica.get(&object, "item")?)?;
        if let Some(comp) = as_ref(&replica.get(&object, "comp")?).and_then(|c| item_of.get(&c).copied()) {
            item.comp = comp;
        }
        project.render_queue.push(item);
        ids.render.push(object);
    }

    project.next_id = next_id_hint(&project, crate::slot_base(replica.actor()));
    Ok((project, ids))
}

fn read_layer(
    replica: &Replica,
    object: &ObjectId,
    item_of: &HashMap<ObjectId, ItemId>,
    layer_of: &HashMap<ObjectId, LayerId>,
    in_comp: &HashSet<ObjectId>,
) -> KdmResult<(kition_motion_project::Layer, LayerIds)> {
    let id = LayerId(stored_id(&replica.get(object, "id")?, "a layer's")?.unwrap_or_else(|| repair_id(object)));
    let attrs = map_fields(replica, object, "attrs")?;
    let start = tick::from_value(&replica.get(object, "startTime")?)?;
    let in_point = tick::from_value(&replica.get(object, "inPoint")?)?;
    let out_point = tick::from_value(&replica.get(object, "outPoint")?)?;
    let same_comp = |field: &str| -> KdmResult<Option<LayerId>> {
        Ok(as_ref(&replica.get(object, field)?).filter(|l| l != object && in_comp.contains(l)).and_then(|l| layer_of.get(&l).copied()))
    };
    let parent = same_comp("parent")?;
    let matte = match (same_comp("matteLayer")?, replica.get(object, "matte")?) {
        (Some(layer), kind) if !kind.is_null() => Some(TrackMatte { layer, kind: serde_json::from_value::<MatteKind>(kind).map_err(kdm_err)? }),
        _ => None,
    };
    let mut layer: kition_motion_project::Layer = fields::rebuild(
        attrs,
        vec![
            ("id", json!(id.0)),
            ("name", replica.get(object, "name")?),
            ("start_time", json!(start.0)),
            ("in_point", json!(in_point.0)),
            ("out_point", json!(out_point.0)),
            ("parent", json!(parent.map(|p| p.0))),
            ("track_matte", serde_json::to_value(matte).map_err(kdm_err)?),
            ("props", fields::empty_group_json()),
            ("switches", fields::switches_from(map_fields(replica, object, "switches")?)?),
        ],
    )?;
    let (markers, marker_objects) = read_markers(replica, object)?;
    layer.markers = markers;
    if let Some(item) = as_ref(&replica.get(object, "sourceItem")?).and_then(|i| item_of.get(&i).copied()) {
        match &mut layer.source {
            LayerSource::Footage { item: slot } | LayerSource::Comp { item: slot } | LayerSource::Solid { item: slot } | LayerSource::Model { item: slot } => {
                *slot = item
            }
            _ => {}
        }
    }
    let root = as_ref(&replica.get(object, "props")?).filter(|r| replica.exists(r)).ok_or_else(|| kdm_err("a layer has no property tree"))?;
    let mut tree = Tree { replica, preorder: Vec::new(), keys: Vec::new(), seen: HashSet::new() };
    let Some(Node::Group(props)) = tree.node(&root, 0)? else { return Err(kdm_err("a layer's property tree is not a group")) };
    layer.props = props;
    // Node keys follow `flatten`'s pre-order, which is the order read.
    let flat = ids::flatten(&layer.props)?;
    let mut lids = LayerIds { object: object.clone(), nodes: HashMap::with_capacity(flat.len()), keys: HashMap::new(), markers: marker_objects };
    for ((f, node), keys) in flat.iter().zip(tree.preorder).zip(tree.keys) {
        lids.nodes.insert(f.key, node);
        if !keys.is_empty() {
            lids.keys.insert(f.key, keys);
        }
    }
    Ok((layer, lids))
}

/// Reads a property tree, recording its node objects (and each property's key objects) in
/// pre-order.
struct Tree<'r> {
    replica: &'r Replica,
    preorder: Vec<ObjectId>,
    keys: Vec<Vec<ObjectId>>,
    /// Nodes already read: one listed twice, or under its own descendant, is read once.
    seen: HashSet<ObjectId>,
}

impl Tree<'_> {
    fn node(&mut self, object: &ObjectId, depth: usize) -> KdmResult<Option<Node>> {
        if depth > MAX_DEPTH {
            return Err(kdm_err("a property tree is nested too deep to read"));
        }
        if !self.replica.exists(object) || !self.seen.insert(object.clone()) {
            return Ok(None);
        }
        let r = self.replica;
        let uid: Uid = as_u64(&r.get(object, "uid")?, "uid")?;
        let meta = r.get(object, "meta")?;
        match r.get(object, "node")?.as_str() {
            Some("prop") => {
                let mut prop = fields::prop_from(uid, meta, r.get(object, "value")?, r.get(object, "expr")?)?;
                let mut keyed: Vec<(Keyframe, ObjectId)> = Vec::new();
                for key in r.list(object, "keys")? {
                    if !r.exists(&key) {
                        continue;
                    }
                    let time = tick::from_value(&r.get(&key, "time")?)?;
                    keyed.push((fields::key_from(time, r.get(&key, "value")?, r.get(&key, "data")?)?, key));
                }
                // The engine keeps keys in time order; a merge can leave them otherwise until
                // the repair sorts them.
                keyed.sort_by_key(|(k, _)| k.time);
                let (keys, objects): (Vec<Keyframe>, Vec<ObjectId>) = keyed.into_iter().unzip();
                prop.keys = keys;
                self.preorder.push(object.clone());
                self.keys.push(objects);
                Ok(Some(Node::Prop(prop)))
            }
            Some("group") => {
                let enabled = r.get(object, "enabled")?.as_bool().unwrap_or(true);
                let mut group: PropGroup = fields::group_from(uid, meta, enabled)?;
                self.preorder.push(object.clone());
                self.keys.push(Vec::new());
                for child in r.list(object, "children")? {
                    if let Some(node) = self.node(&child, depth + 1)? {
                        group.children.push(node);
                    }
                }
                Ok(Some(Node::Group(group)))
            }
            _ => Err(kdm_err("a node is neither a property nor a group")),
        }
    }
}

/// An id counter past every id `project` holds (items, layers, property uids, Graphics Templates
/// controls) in the slot that starts at `floor` ([`crate::slot_base`]), and never below `floor`.
/// Ids outside that slot are not counted: a file's own ids sit below every slot, repair ids
/// ([`crate::repair_id`]) above them all, and another actor's ids in that actor's slot (counting
/// those would move this actor into the other's slot, where both then mint the same ids). A
/// session that reloads its view must still never *lower* its own counter to this (an id it
/// handed out may live only in an undo step, or on a peer that has not yet seen its deletion).
pub(crate) fn next_id_hint(project: &Project, floor: u64) -> u64 {
    let end = floor.saturating_add(SLOT_SIZE).min(REPAIR_BASE);
    let mut max = 0u64;
    let mut bump = |v: u64| {
        if (floor..end).contains(&v) {
            max = max.max(v);
        }
    };
    fn controls(list: &[EgControl], bump: &mut dyn FnMut(u64), depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        for c in list {
            bump(c.id);
            if let EgKind::Group { children } = &c.kind {
                controls(children, bump, depth + 1);
            }
        }
    }
    fn uids(group: &PropGroup, bump: &mut dyn FnMut(u64), depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        bump(group.uid);
        for child in &group.children {
            match child {
                Node::Prop(p) => bump(p.uid),
                Node::Group(g) => uids(g, bump, depth + 1),
            }
        }
    }
    for (id, item) in &project.items {
        bump(id.0);
        if let ItemKind::Comp(comp) = &item.kind {
            if let Some(eg) = &comp.graphics_template {
                controls(&eg.controls, &mut bump, 0);
            }
            for layer in &comp.layers {
                bump(layer.id.0);
                uids(&layer.props, &mut bump, 0);
            }
        }
    }
    max.saturating_add(1).max(floor)
}
