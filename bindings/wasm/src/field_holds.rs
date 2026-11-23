//! What a step that names a layer reads and assigns of that layer, field by field (task card
//! C8-motion-pin, run 4): the layer is held by those fields instead of whole, so another writer's
//! change of something else of it (its name, a property the step does not touch) applies, while a
//! change of a field the step reads or assigns still refuses `changed`.
//!
//! Each command's fields come from its handler (file:line in `crates/engine/engine/src/commands`).
//! `None` keeps the layer held whole: a parameter spelled another way (an effect or mask by name or
//! by position), a branch that reads more of the layer, or a group the layer lacks.
use kition_kit::Premise;
use kition_motion_engine::Session;
use kition_motion_project::{Layer, Uid};
use serde_json::Value;

/// The field path of the property-tree node `uid`. A path holds at most one selector (kition-sdk
/// `tools/selector.rs`), so a top-level group is named by its uid when it is the node itself
/// (`props.children[uid=a]`) and by its place otherwise (a layer's top-level groups are fixed);
/// below it the instance (an effect, a mask, an animator) by its uid, and what is inside the
/// instance by place: `props.children.1.children[uid=b].children.2`.
pub(crate) fn node_path(layer: &Layer, uid: Uid) -> Option<String> {
    let chain = layer.props.node_chain(uid)?;
    let (top, rest) = chain.split_first()?;
    if rest.is_empty() {
        return Some(format!("props.children[uid={}]", top.uid()));
    }
    let mut path = format!("props.children.{}", layer.props.children.iter().position(|node| node.uid() == top.uid())?);
    let mut parent = top.as_group()?;
    for (depth, node) in rest.iter().enumerate() {
        if depth == 0 {
            path.push_str(&format!(".children[uid={}]", node.uid()));
        } else {
            path.push_str(&format!(".children.{}", parent.children.iter().position(|child| child.uid() == node.uid())?));
        }
        if let Some(group) = node.as_group() {
            parent = group;
        }
    }
    Some(path)
}

/// The layer's top-level group with match id `name` (`masks`, `effects`): its path as a whole
/// (`props.children[uid=a]`) and the place its members' paths start from (`props.children.1`).
fn top_group(layer: &Layer, name: &str) -> Option<(String, String)> {
    let group = layer.props.sub(name)?;
    let index = layer.props.children.iter().position(|node| node.uid() == group.uid)?;
    Some((format!("props.children[uid={}]", group.uid), format!("props.children.{index}")))
}

fn held<S: Into<String>>(layer: &Layer, fields: impl IntoIterator<Item = S>, timed: bool) -> Premise {
    let mut fields: Vec<String> = fields.into_iter().map(Into::into).collect();
    if timed {
        // The handler samples or writes at the layer time of the current time
        // (`Layer::layer_time`: the layer's start and stretch).
        fields.push("attrs.stretch".to_owned());
    }
    Premise::on(serde_json::json!(layer.id.0)).fields(fields).positions(if timed { vec!["startTime"] } else { Vec::new() })
}

/// An effect of the layer named by its uid (`effect.toggle`'s `find_fx`, effect.rs:224-238, looks
/// the number up as a uid first, then as a position).
fn effect_by_uid(layer: &Layer, params: &Value) -> Option<String> {
    let uid = params.get("effect")?.as_u64()?;
    let (_, group) = top_group(layer, "effects")?;
    layer.effects()?.groups().any(|effect| effect.uid == uid).then(|| format!("{group}.children[uid={uid}]"))
}

/// A mask of the layer named by its uid (`target_masks`, layer_menu.rs:294-313: a uid first, then
/// a position, then a name).
fn mask_by_uid(layer: &Layer, params: &Value) -> Option<String> {
    let uid = params.get("mask")?.as_u64()?;
    let (_, group) = top_group(layer, "masks")?;
    layer.masks()?.children.iter().any(|mask| mask.uid() == uid).then(|| format!("{group}.children[uid={uid}]"))
}

/// The node `prop.select` / `prop.deselect` read for `uid` (prop.rs:681-745 `path_points`): a mask
/// or shape path is read through its group (its kind, its Path at the layer time), so a Path
/// property is held by the group that owns it.
fn selected_node(layer: &Layer, uid: Uid) -> Option<String> {
    if layer.props.find_group(uid).is_none()
        && let Some(owner) = layer.props.parent_of(uid).filter(|group| group.get("path").is_some_and(|path| path.uid == uid))
    {
        return node_path(layer, owner.uid);
    }
    node_path(layer, uid)
}

/// What `command` with `params` reads and assigns of `layer` (see the module), or `None`.
pub(crate) fn premise(_session: &Session, layer: &Layer, command: &str, params: &Value) -> Option<Premise> {
    match command {
        // edit.rs:578-602: with layers named and no `keys`, the label is assigned and nothing read.
        "edit.label" if params.get("keys").is_none() => Some(held(layer, ["attrs.label"], false)),
        // layer_menu.rs:605-640: `none` assigns no matte; the stack order and the current matte are
        // read only by the other ops.
        "layer.trackMatte" if params.get("op").and_then(Value::as_str) == Some("none") => Some(held(layer, ["matte", "matteLayer"], false)),
        // effect.rs:329-339: the effect must be there; its fx switch is read (a toggle) and assigned.
        "effect.toggle" => {
            let effect = effect_by_uid(layer, params)?;
            let (_, group) = top_group(layer, "effects")?;
            Some(held(layer, [format!("{group}.meta.match"), format!("{effect}.node"), format!("{effect}.enabled")], false))
        }
        // prop_groups.rs:17-51: the group must be an instance (its kind); its name or switch is
        // assigned (the switch read for a toggle).
        "prop.renameGroup" | "prop.setGroupEnabled" => {
            let uid = params.get("prop")?.as_u64()?;
            layer.props.find_group(uid)?;
            let group = node_path(layer, uid)?;
            let assigned = if command == "prop.renameGroup" { "meta.name" } else { "enabled" };
            Some(held(layer, [format!("{group}.node"), format!("{group}.meta.kind"), format!("{group}.{assigned}")], false))
        }
        // effect.rs:316-327, layer_menu.rs:465-471, mask.rs:617-636: every effect or mask of the
        // layer is removed, so the whole group (what it holds now).
        "effect.removeAll" => Some(held(layer, [top_group(layer, "effects")?.0], false)),
        "layer.mask.removeAll" | "mask.removeAll" => Some(held(layer, [top_group(layer, "masks")?.0], false)),
        // layer_menu.rs:334-371, 473-501: the mask must be a mask (its kind); one member of the
        // kind is assigned (read for a toggle). The source size `edit_masks` takes is not used by
        // these three.
        "layer.mask.mode" | "layer.mask.invert" | "layer.mask.lock" => {
            let mask = mask_by_uid(layer, params)?;
            let (_, group) = top_group(layer, "masks")?;
            let member = match command {
                "layer.mask.mode" => "mode",
                "layer.mask.invert" => "inverted",
                _ => "locked",
            };
            Some(held(
                layer,
                [format!("{group}.meta.match"), format!("{mask}.uid"), format!("{mask}.meta.kind.kind"), format!("{mask}.meta.kind.{member}")],
                false,
            ))
        }
        // layer.rs:1333-1380: a locked layer is left alone; a flip reads and assigns Scale at the
        // layer time. The comp and source sizes are read only by reset, center and fit.
        "layer.transform" if matches!(params.get("op").and_then(Value::as_str), Some("flipH" | "flipV")) => {
            Some(held(layer, ["switches.locked", "transform/scale"], true))
        }
        // prop.rs:681-745: editor state only; the node (a mask's kind and Path at the layer time,
        // a property's keys) is read.
        "prop.select" | "prop.deselect" => {
            let uid = params.get("prop")?.as_u64()?;
            Some(held(layer, [selected_node(layer, uid)?], true))
        }
        // animation.rs:270-282: every text animator is removed.
        "text.removeAllAnimators" => {
            let animators = layer.props.group("text/animators")?;
            Some(held(layer, [node_path(layer, animators.uid)?], false))
        }
        // three_d.rs:503-531: the mode is assigned; towards the camera reads whether the layer is a
        // camera or light (its source), towards the point of interest also whether it has one.
        "layer.autoOrient" => {
            let mode = params.get("mode")?.as_str()?.to_ascii_lowercase().replace([' ', '-', '_'], "");
            let fields: &[&str] = match mode.as_str() {
                "off" | "alongpath" | "orientalongpath" => &["attrs.auto_orient"],
                "towardscamera" | "orienttowardscamera" => &["attrs.auto_orient", "attrs.source"],
                "towardspointofinterest" | "towardspoi" | "orienttowardspointofinterest" => &["attrs.auto_orient", "attrs.source", "transform/poi"],
                _ => return None,
            };
            Some(held(layer, fields.iter().copied(), false))
        }
        // layer_menu.rs:176-234: the member is read and assigned at the layer time; Position reads
        // whether it is separated (X, Y and Z then hold the picture and are written too).
        "layer.setTransform" => {
            let member = match params.get("prop")?.as_str()? {
                "anchor" | "anchorPoint" => "anchor",
                "position" => "position",
                "scale" => "scale",
                "orientation" => "orientation",
                "rotation" => "rotation",
                "opacity" => "opacity",
                _ => return None,
            };
            let mut fields = vec![format!("transform/{member}")];
            if member == "position" {
                fields.extend(["transform/positionX", "transform/positionY", "transform/positionZ"].map(str::to_owned));
            }
            Some(held(layer, fields, true))
        }
        // paint.rs:273-281: editor state only; the layer must be there.
        "paint.setCloneSource" => Some(held(layer, Vec::<String>::new(), false)),
        _ => None,
    }
}
