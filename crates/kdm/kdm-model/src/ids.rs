//! Engine ids to the objects that hold them.
//!
//! Engine ids are the objects' own data (see the crate docs), so these maps are not a
//! translation the merge depends on: they only let [`crate::write_changes`] find the object it
//! must change without searching the replica. A project read from a file can hold one id twice
//! (two layers of one id in two compositions, a property uid repeated in one layer's tree); every
//! key carries the occurrence (`nth`, 0 for the first, in a fixed traversal order), so even then
//! each layer and each node has an object of its own.

use std::collections::HashMap;

use kition_kit::kdm::ObjectId;
use kition_motion_project::props::{Node, PropGroup, Property, Uid};
use kition_motion_project::template_props::{EgControl, EgKind};
use kition_motion_project::{ItemId, LayerId, Project};

use crate::{KdmResult, MAX_DEPTH, kdm_err};

/// A layer: its id and which occurrence of that id it is in [`layer_keys`]' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerKey {
    pub id: LayerId,
    pub nth: u32,
}

/// A node of one layer's property tree: its uid and which occurrence of that uid it is in the
/// tree's pre-order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeKey {
    pub uid: Uid,
    pub nth: u32,
}

/// One layer's object and the objects of its property tree and markers.
#[derive(Clone, Debug)]
pub struct LayerIds {
    pub object: ObjectId,
    pub nodes: HashMap<NodeKey, ObjectId>,
    /// Each property's keyframe objects, in the property's key order.
    pub keys: HashMap<NodeKey, Vec<ObjectId>>,
    /// The layer's `Marker` objects, in the layer's marker order.
    pub markers: Vec<ObjectId>,
}

/// A Graphics Templates control of one composition: its id and which occurrence of that id it
/// is in the controls' pre-order (a group, then its children).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ControlKey {
    pub id: u64,
    pub nth: u32,
}

/// The objects of a composition's markers, guides and Graphics Templates controls.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompParts {
    /// `Marker` objects, in the composition's marker order.
    pub markers: Vec<ObjectId>,
    /// `Guide` objects, in the composition's guide order.
    pub guides: Vec<ObjectId>,
    pub controls: HashMap<ControlKey, ObjectId>,
}

/// Every object a loaded view knows, by the engine id it holds.
#[derive(Clone, Debug, Default)]
pub struct Ids {
    pub items: HashMap<ItemId, ObjectId>,
    /// The `Comp` object of each composition item.
    pub comps: HashMap<ItemId, ObjectId>,
    /// The markers, guides and template controls of each composition item.
    pub parts: HashMap<ItemId, CompParts>,
    pub layers: HashMap<LayerKey, LayerIds>,
    /// The Render Queue's objects, in queue order.
    pub render: Vec<ObjectId>,
}

impl Ids {
    /// The object of layer `key`.
    pub fn layer(&self, key: LayerKey) -> Option<&ObjectId> {
        self.layers.get(&key).map(|l| &l.object)
    }

    /// The first layer of id `id` (the usual, unique case), with its key.
    pub fn layer_by_id(&self, id: LayerId) -> Option<&ObjectId> {
        self.layer(LayerKey { id, nth: 0 })
    }

    /// The object of the node of uid `uid` in layer `layer` (its first occurrence).
    pub fn node(&self, layer: LayerId, uid: Uid) -> Option<&ObjectId> {
        self.layers.get(&LayerKey { id: layer, nth: 0 })?.nodes.get(&NodeKey { uid, nth: 0 })
    }

    /// The keyframe objects of property `uid` in layer `layer`.
    pub fn keys(&self, layer: LayerId, uid: Uid) -> &[ObjectId] {
        self.layers.get(&LayerKey { id: layer, nth: 0 }).and_then(|l| l.keys.get(&NodeKey { uid, nth: 0 })).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Every layer of `project` with its key, in the canonical order: compositions by item id, each
/// one's layers top first. Answers (composition, layer index, key).
pub fn layer_keys(project: &Project) -> Vec<(ItemId, usize, LayerKey)> {
    let mut seen: HashMap<LayerId, u32> = HashMap::new();
    let mut out = Vec::new();
    for (cid, comp) in project.comps() {
        for (index, layer) in comp.layers.iter().enumerate() {
            let nth = seen.entry(layer.id).or_insert(0);
            out.push((*cid, index, LayerKey { id: layer.id, nth: *nth }));
            *nth = nth.saturating_add(1);
        }
    }
    out
}

/// One node of a flattened property tree.
#[derive(Clone, Copy, Debug)]
pub(crate) enum NodeRef<'a> {
    Group(&'a PropGroup),
    Prop(&'a Property),
}

/// A node of a property tree with its key and, for a group, its children's keys in order.
#[derive(Debug)]
pub(crate) struct Flat<'a> {
    pub key: NodeKey,
    pub node: NodeRef<'a>,
    pub children: Vec<NodeKey>,
}

/// `root`'s tree in pre-order (a group, then each child in order, a sub-group's own subtree
/// before the next child), the root first.
pub(crate) fn flatten(root: &PropGroup) -> KdmResult<Vec<Flat<'_>>> {
    let mut out = Vec::new();
    let mut seen = HashMap::new();
    visit(root, &mut out, &mut seen, 0)?;
    Ok(out)
}

fn key_for(uid: Uid, seen: &mut HashMap<Uid, u32>) -> NodeKey {
    let nth = seen.entry(uid).or_insert(0);
    let key = NodeKey { uid, nth: *nth };
    *nth = nth.saturating_add(1);
    key
}

fn visit<'a>(group: &'a PropGroup, out: &mut Vec<Flat<'a>>, seen: &mut HashMap<Uid, u32>, depth: usize) -> KdmResult<NodeKey> {
    if depth > MAX_DEPTH {
        return Err(kdm_err("a property tree is nested too deep to replicate"));
    }
    let key = key_for(group.uid, seen);
    let index = out.len();
    out.push(Flat { key, node: NodeRef::Group(group), children: Vec::new() });
    let mut children = Vec::with_capacity(group.children.len());
    for child in &group.children {
        match child {
            Node::Prop(p) => {
                let k = key_for(p.uid, seen);
                out.push(Flat { key: k, node: NodeRef::Prop(p), children: Vec::new() });
                children.push(k);
            }
            Node::Group(g) => children.push(visit(g, out, seen, depth + 1)?),
        }
    }
    if let Some(flat) = out.get_mut(index) {
        flat.children = children;
    }
    Ok(key)
}

/// One control of a flattened Graphics Templates control list.
#[derive(Debug)]
pub(crate) struct FlatControl<'a> {
    pub key: ControlKey,
    pub control: &'a EgControl,
    /// A group's children's keys, in order (empty for any other control).
    pub children: Vec<ControlKey>,
}

/// `controls` in pre-order (a control, then a group's own children before the next control),
/// and the keys of the top-level ones in order.
pub(crate) fn flatten_controls(controls: &[EgControl]) -> KdmResult<(Vec<FlatControl<'_>>, Vec<ControlKey>)> {
    let mut out = Vec::new();
    let mut seen = HashMap::new();
    let top = visit_controls(controls, &mut out, &mut seen, 0)?;
    Ok((out, top))
}

fn visit_controls<'a>(controls: &'a [EgControl], out: &mut Vec<FlatControl<'a>>, seen: &mut HashMap<u64, u32>, depth: usize) -> KdmResult<Vec<ControlKey>> {
    if depth > MAX_DEPTH {
        return Err(kdm_err("Graphics Templates controls are nested too deep to replicate"));
    }
    let mut keys = Vec::with_capacity(controls.len());
    for control in controls {
        let nth = seen.entry(control.id).or_insert(0);
        let key = ControlKey { id: control.id, nth: *nth };
        *nth = nth.saturating_add(1);
        let index = out.len();
        out.push(FlatControl { key, control, children: Vec::new() });
        if let EgKind::Group { children } = &control.kind {
            let child_keys = visit_controls(children, out, seen, depth + 1)?;
            if let Some(flat) = out.get_mut(index) {
                flat.children = child_keys;
            }
        }
        keys.push(key);
    }
    Ok(keys)
}
