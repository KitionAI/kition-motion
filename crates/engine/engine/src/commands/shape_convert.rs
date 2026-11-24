//! Layer ▸ Mask and Shape Path ▸ Convert to Bezier Path (`path.convertToBezier`) for a shape layer's parametric shapes: a
//! Rectangle, Ellipse or Polystar Path item becomes a Path item with the same outline at the
//! current time, so its vertices can be edited one by one (the Pen and Selection tools, the
//! `mask.*Vertex` commands, which take a shape Path item's uid).

use kition_motion_keyframe::Value as KV;
use kition_motion_project::build::{self, Ids};
use kition_motion_project::{Node, PropGroup, Uid};
use kition_motion_render::EvalCtx;
use serde_json::{Value, json};

use super::{bad, layer_mut, layer_p};
use crate::{EngineError, Result, Session};

const C: &str = "path.convertToBezier";

/// How deep shape groups are searched (groups nest; a malformed project may nest without end).
const MAX_DEPTH: usize = 64;

/// The parametric shapes this converts, by their match name.
const PARAMETRIC: [&str; 3] = ["rect", "ellipse", "star"];

/// Whether `g` is a Rectangle, Ellipse or Polystar Path item.
pub(crate) fn is_parametric(g: &PropGroup) -> bool {
    PARAMETRIC.contains(&g.match_id.as_str())
}

/// The parametric shape items under `g` (a layer's Contents), by uid, depth first.
fn parametric_items(g: &PropGroup, depth: usize, out: &mut Vec<Uid>) {
    if depth > MAX_DEPTH {
        return;
    }
    for node in &g.children {
        let Node::Group(child) = node else { continue };
        if is_parametric(child) {
            out.push(child.uid);
        } else {
            parametric_items(child, depth + 1, out);
        }
    }
}

/// Replaces the child group `uid` somewhere under `g` with what `make` builds from it; whether it
/// was found.
fn replace(g: &mut PropGroup, uid: Uid, depth: usize, make: &mut dyn FnMut(&PropGroup) -> Result<PropGroup>) -> Result<bool> {
    if depth > MAX_DEPTH {
        return Ok(false);
    }
    for node in g.children.iter_mut() {
        let Node::Group(child) = node else { continue };
        if child.uid == uid {
            let made = make(child)?;
            *node = made.into();
            return Ok(true);
        }
        if replace(child, uid, depth + 1, make)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether the layer `p` names (or the selected one) is a shape layer with a Rectangle, Ellipse or
/// Polystar Path.
pub(crate) fn has_parametric(s: &Session, p: &Value) -> bool {
    let Ok((cid, lid)) = layer_p(s, p, C) else { return false };
    let Some(contents) = s.project.comp(cid).and_then(|c| c.layer(lid)).and_then(|l| l.props.sub("contents")) else { return false };
    let mut all = Vec::new();
    parametric_items(contents, 0, &mut all);
    !all.is_empty()
}

/// `path.convertToBezier` on a shape layer, `{layer?, shape?: uid}`: the shape named (a Rectangle,
/// Ellipse or Polystar Path), or without one every such item of the layer, becomes a Path item
/// drawing the same outline at the current time (its keyframes and expressions are dropped, as
/// the outline is fixed). One undo step. Answers the new Path items' uids.
pub(crate) fn convert(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, lid) = layer_p(s, p, C)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let layer = comp.layer(lid).ok_or_else(|| bad(C, "no such layer"))?;
    let contents =
        layer.props.sub("contents").ok_or_else(|| bad(C, "not a shape layer: only a shape layer's Rectangle, Ellipse and Polystar Paths convert"))?;
    let items: Vec<Uid> = match p.get("shape").filter(|v| !v.is_null()) {
        Some(v) => {
            let uid = v.as_u64().ok_or_else(|| bad(C, "`shape` is a shape item's uid"))?;
            let found = contents.find_group(uid).ok_or_else(|| bad(C, format!("the layer has no shape item {uid}")))?;
            if !is_parametric(found) {
                return Err(bad(C, format!("`{}` is not a Rectangle, Ellipse or Polystar Path", found.name)));
            }
            vec![uid]
        }
        None => {
            let mut all = Vec::new();
            parametric_items(contents, 0, &mut all);
            all
        }
    };
    if items.is_empty() {
        return Err(bad(C, "the layer has no Rectangle, Ellipse or Polystar Path to convert"));
    }
    // Each outline at the current time, with its expressions (the renderer's own evaluation).
    let t = s.time();
    let ctx = EvalCtx { project: &s.project, comp_id: cid, comp, time: t, expr: s.expr.as_deref(), footage: None };
    let mut outlines = Vec::new();
    for uid in &items {
        let Some(g) = contents.find_group(*uid) else { continue };
        let shape = match g.match_id.as_str() {
            "rect" => kition_motion_path::rect(ctx.v2(layer, g, "size", [100.0; 2]), ctx.v2(layer, g, "position", [0.0; 2]), ctx.f(layer, g, "roundness", 0.0)),
            "ellipse" => kition_motion_path::ellipse(ctx.v2(layer, g, "size", [100.0; 2]), ctx.v2(layer, g, "position", [0.0; 2])),
            _ => kition_motion_path::polystar(
                ctx.e(layer, g, "type") == 0,
                ctx.f(layer, g, "points", 5.0),
                ctx.v2(layer, g, "position", [0.0; 2]),
                ctx.f(layer, g, "rotation", 0.0),
                ctx.f(layer, g, "innerRadius", 50.0),
                ctx.f(layer, g, "outerRadius", 100.0),
                ctx.f(layer, g, "innerRoundness", 0.0),
                ctx.f(layer, g, "outerRoundness", 0.0),
            ),
        };
        let path = kition_motion_path::from_kurbo(&shape).into_iter().next().ok_or_else(|| bad(C, format!("`{}` has no outline to convert", g.name)))?;
        let direction = g.get("direction").map(|d| d.value.clone()).unwrap_or(KV::Enum(0));
        outlines.push((*uid, path, direction));
    }
    let made = s.edit("Convert To Bezier Path", None, |proj, _| {
        let mut next = proj.next_id;
        let mut ids = Ids(&mut next);
        let l = layer_mut(proj, cid, lid)?;
        let contents = l.props.sub_mut("contents").ok_or_else(|| bad(C, "not a shape layer"))?;
        let mut made = Vec::new();
        for (uid, path, direction) in &outlines {
            let mut build_path = |old: &PropGroup| -> Result<PropGroup> {
                let mut item = build::shape_path(&mut ids, path.clone());
                item.name = old.name.replace("Rectangle Path", "Path").replace("Ellipse Path", "Path").replace("Polystar Path", "Path");
                item.enabled = old.enabled;
                if let Some(d) = item.get_mut("direction") {
                    d.value = direction.clone();
                }
                made.push(item.uid);
                Ok(item)
            };
            if !replace(contents, *uid, 0, &mut build_path)? {
                return Err(bad(C, format!("the layer has no shape item {uid}")));
            }
        }
        proj.next_id = next;
        Ok(made)
    })?;
    Ok(json!({ "paths": made }))
}
