//! Promo carries: transitions where something on screen survives the boundary and becomes the
//! next beat, instead of a cut, a fade or a dissolve.
//!
//! - `transition.iris`: a circular mask on the incoming layer opens from a point (fixed, or the
//!   middle of a layer it follows) until it covers the frame, optionally with a ring at its edge.
//! - `transition.morphRect`: container transform — one layer's rectangle morphs into another's;
//!   the outgoing content scales and crops into the incoming one, which fades in inside it.
//! - `transition.zoomThrough`: a log-space push into a point of the outgoing layer that hands
//!   over to the incoming layer settling from a slightly-too-large scale.
//!
//! Each command is one undo step. Moves whose shape is not a single eased curve (a cover scale
//! that follows a morphing rectangle, a log-space zoom) are written as one key per frame, so they
//! stay exact and editable.

use kition_motion_color::Label;
use kition_motion_keyframe::{ShapePath, Value as KV};
use kition_motion_project::build::{self, Ids};
use kition_motion_project::{Comp, Expression, ItemId, Layer, LayerId, LayerSource, MaskMode, Property};
use kition_motion_render::EvalCtx;
use kition_motion_time::Tick;
use serde_json::{Value, json};

use super::layer::color_p;
use super::promo_ease::{self as pe, Curve};
use super::promo_light::{js, js_str, slider, start_p};
use super::{CommandSpec, bad, has_comp, layer_mut, layer_p, resolve_layer};
use crate::{EngineError, Result, Session, cmd};

// ---------------------------------------------------------------- shared helpers

/// A finite number parameter.
pub(crate) fn num(p: &Value, k: &str, d: f64, cmd: &str) -> Result<f64> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(d),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad(cmd, format!("`{k}` must be a number"))),
    }
}

/// A finite number parameter ≥ 0.
pub(crate) fn pos(p: &Value, k: &str, d: f64, cmd: &str) -> Result<f64> {
    let v = num(p, k, d, cmd)?;
    if v < 0.0 { Err(bad(cmd, format!("`{k}` must be ≥ 0"))) } else { Ok(v) }
}

/// `[x, y]` (finite numbers), if given.
pub(crate) fn point_p(p: &Value, k: &str, cmd: &str) -> Result<Option<[f64; 2]>> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => match a.iter().map(Value::as_f64).collect::<Option<Vec<f64>>>().as_deref() {
            Some([x, y]) if x.is_finite() && y.is_finite() => Ok(Some([*x, *y])),
            _ => Err(bad(cmd, format!("`{k}`: [x, y] in comp pixels"))),
        },
        Some(_) => Err(bad(cmd, format!("`{k}`: [x, y] in comp pixels"))),
    }
}

/// A layer parameter that must name a layer of `comp`.
pub(crate) fn layer_arg(comp: &Comp, p: &Value, k: &str, cmd: &str) -> Result<LayerId> {
    let v = p.get(k).ok_or_else(|| bad(cmd, format!("`{k}` (a layer id, name or #n) required")))?;
    resolve_layer(comp, v).ok_or_else(|| bad(cmd, format!("no layer {v}")))
}

/// An evaluation context at comp time `t` with the session's expression engine.
pub(crate) fn ctx_at<'a>(s: &'a Session, cid: ItemId, comp: &'a Comp, t: Tick) -> EvalCtx<'a> {
    let mut c = EvalCtx::new(&s.project, cid, comp, t);
    c.expr = s.expr.as_deref();
    c.footage = Some(s.footage.as_ref());
    c
}

/// A layer's content rectangle in layer space, `[x0, y0, x1, y1]`: text and shape layers by what
/// they draw, other layers by their source.
pub(crate) fn local_rect(ctx: &EvalCtx, l: &Layer) -> [f64; 4] {
    let drawn = match &l.source {
        LayerSource::Text => {
            let paths: Vec<_> = kition_motion_render::text::glyph_paths(ctx, l).into_iter().map(|(p, _)| p).collect();
            kition_motion_path::bounds(&paths)
        }
        LayerSource::Shape => l.props.sub("contents").and_then(|c| kition_motion_render::shapes::content_bounds(ctx, l, c)),
        _ => None,
    };
    match drawn {
        Some(b) if b.width() > 0.0 && b.height() > 0.0 => [b.x0, b.y0, b.x1, b.y1],
        _ => {
            let (size, o) = ctx.effect_bounds(l);
            [o[0], o[1], o[0] + size[0], o[1] + size[1]]
        }
    }
}

/// The comp-space bounding box of a layer's content at the context time.
pub(crate) fn comp_rect(ctx: &EvalCtx, l: &Layer) -> [f64; 4] {
    let r = local_rect(ctx, l);
    let m = ctx.layer_to_comp(l).0;
    let mut out = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for (x, y) in [(r[0], r[1]), (r[2], r[1]), (r[2], r[3]), (r[0], r[3])] {
        let q = m.apply(kition_motion_geom::vec2(x, y));
        out = [out[0].min(q.x), out[1].min(q.y), out[2].max(q.x), out[3].max(q.y)];
    }
    if out.iter().all(|v| v.is_finite()) { out } else { [0.0; 4] }
}

/// A transform property's keyed value (before its expression: presets write keys, and an
/// expression stays on top of them) at the context time, as up to three numbers.
pub(crate) fn tr3(ctx: &EvalCtx, l: &Layer, name: &str, d: [f64; 3]) -> [f64; 3] {
    let Some(pr) = l.transform().and_then(|t| t.get(name)) else { return d };
    let c = pr.value_at(l.layer_time(ctx.time)).components();
    [c.first().copied().unwrap_or(d[0]), c.get(1).copied().unwrap_or(d[1]), c.get(2).copied().unwrap_or(d[2])]
}

/// A closed rounded rectangle centred at `c`, size `w` × `h`, corner radii `rx`, `ry`; always
/// eight vertices so keyed paths interpolate vertex by vertex at any radius.
pub(crate) fn rounded_rect(c: [f64; 2], w: f64, h: f64, rx: f64, ry: f64) -> ShapePath {
    const K: f64 = 0.552_284_75;
    let (hw, hh) = (w.abs() / 2.0, h.abs() / 2.0);
    let (rx, ry) = (rx.max(0.0).min(hw), ry.max(0.0).min(hh));
    let (l, r, t, b) = (c[0] - hw, c[0] + hw, c[1] - hh, c[1] + hh);
    // Clockwise (y down) from the top edge's left end.
    let vertices = vec![[l + rx, t], [r - rx, t], [r, t + ry], [r, b - ry], [r - rx, b], [l + rx, b], [l, b - ry], [l, t + ry]];
    let (kx, ky) = (rx * K, ry * K);
    let in_tangents = vec![[-kx, 0.0], [0.0, 0.0], [0.0, -ky], [0.0, 0.0], [kx, 0.0], [0.0, 0.0], [0.0, ky], [0.0, 0.0]];
    let out_tangents = vec![[0.0, 0.0], [kx, 0.0], [0.0, 0.0], [0.0, ky], [0.0, 0.0], [-kx, 0.0], [0.0, 0.0], [0.0, -ky]];
    ShapePath { vertices, in_tangents, out_tangents, closed: true, feather: Vec::new() }
}

/// Comp times of every frame from `start` over `frames` frames (both ends), with their progress
/// 0..1.
pub(crate) fn frame_steps(comp: &Comp, start: Tick, frames: f64) -> Vec<(f64, Tick)> {
    let n = frames.ceil().clamp(1.0, 10_000.0) as usize;
    (0..=n)
        .map(|i| {
            let f = (i as f64).min(frames);
            ((f / frames.max(1e-9)).clamp(0.0, 1.0), start + pe::frames(comp, f))
        })
        .collect()
}

/// Write `keys` (layer time, sorted) into `pr`, replacing the keys it had between the first and
/// the last of them and keeping the rest. A property without keys gets exactly `keys`.
pub(crate) fn splice(pr: &mut Property, keys: Vec<(Tick, KV)>) {
    let (Some(t0), Some(t1)) = (keys.first().map(|k| k.0), keys.last().map(|k| k.0)) else { return };
    let mut ks: Vec<kition_motion_keyframe::Keyframe> = pr.keys.iter().filter(|k| k.time < t0 || k.time > t1).cloned().collect();
    for (t, v) in keys {
        let mut k = kition_motion_keyframe::Keyframe::new(t, v);
        if pr.hold_only {
            k = k.hold();
        }
        ks.push(k);
    }
    ks.sort_by_key(|k| k.time);
    ks.dedup_by_key(|k| k.time);
    if pr.keys.is_empty()
        && let Some(k) = ks.first()
    {
        pr.value = k.value.clone();
    }
    pr.keys = ks;
}

/// Hold keys: `(layer time, value)` that jump rather than interpolate.
pub(crate) fn splice_hold(pr: &mut Property, keys: Vec<(Tick, KV)>) {
    splice(pr, keys.clone());
    for k in pr.keys.iter_mut().filter(|k| keys.iter().any(|(t, _)| *t == k.time)) {
        k.in_interp = kition_motion_keyframe::Interp::Hold;
        k.out_interp = kition_motion_keyframe::Interp::Hold;
    }
}

/// `thisComp.layer(…)` for `l`: by name when it is unique in `comp`, else by its 1-based index
/// `index` (the index it will have once the command's new layers are in).
pub(crate) fn layer_ref(comp: &Comp, l: &Layer, index: usize) -> String {
    if comp.layers.iter().filter(|x| x.name == l.name).count() == 1 {
        format!("thisComp.layer({})", js_str(&l.name))
    } else {
        format!("thisComp.layer({index})")
    }
}

/// The ring and the mask share one radius: `c` (comp px) and `r` from the Iris slider `p`.
fn iris_js(progress: &str, center: &str, feather: f64) -> String {
    format!(
        "var p = {progress};\n{center}\n\
var W = thisComp.width, H = thisComp.height;\n\
var R = Math.max(Math.hypot(c[0], c[1]), Math.hypot(W - c[0], c[1]), Math.hypot(c[0], H - c[1]), Math.hypot(W - c[0], H - c[1])) + {};\n\
var r = Math.max(R * Math.max(p, 0), 0.01);",
        js(feather)
    )
}

// ---------------------------------------------------------------- iris

fn iris(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "transition.iris";
    let (cid, lid) = layer_p(s, p, C)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let layer = comp.layer(lid).cloned().ok_or(EngineError::NoComp)?;
    if layer.props.sub("masks").is_none() || layer.props.sub("effects").is_none() {
        return Err(bad(C, "the incoming layer can't have masks (use an AV, solid, text, shape or precomp layer)"));
    }
    let start = start_p(s, p, cid, C)?;
    let frames = pos(p, "frames", 14.0, C)?.max(1.0);
    let feather = pos(p, "feather", 0.0, C)?;
    let curve = pe::curve_p(p, "ease", Curve::new(0.4, 0.0, 0.2, 1.0), C)?;
    let end = start + pe::frames(&comp, frames);
    // Centre: a fixed comp point, a layer to follow, or the comp centre.
    enum Centre {
        Point([f64; 2]),
        Layer(LayerId),
    }
    let centre = match p.get("center") {
        None | Some(Value::Null) => Centre::Point([comp.width as f64 / 2.0, comp.height as f64 / 2.0]),
        Some(Value::Array(_)) => Centre::Point(point_p(p, "center", C)?.unwrap_or([0.0, 0.0])),
        Some(v) => Centre::Layer(resolve_layer(&comp, v).ok_or_else(|| bad(C, format!("`center`: no layer {v}")))?),
    };
    let ring = match p.get("ring") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => None,
        Some(Value::Bool(true)) => Some(([1.0f32, 1.0, 1.0], 6.0)),
        Some(r @ Value::Object(_)) => Some((color_p(r, "color").unwrap_or([1.0, 1.0, 1.0]), pos(r, "width", 6.0, C)?)),
        Some(_) => return Err(bad(C, "`ring`: {color?, width?} or true")),
    };
    let at = comp.layers.iter().position(|l| l.id == lid).unwrap_or(0);
    // 1-based index of a layer once the ring (if any) is inserted above the incoming layer.
    let index_after = |id: LayerId| comp.layers.iter().position(|l| l.id == id).map(|i| if ring.is_some() && i >= at { i + 2 } else { i + 1 }).unwrap_or(1);
    let centre_js = match centre {
        Centre::Point(c) => format!("var c = [{}, {}];", js(c[0]), js(c[1])),
        Centre::Layer(id) => {
            let l = comp.layer(id).ok_or(EngineError::NoComp)?;
            format!(
                "var L = {}, q = L.sourceRectAtTime(time, false), c = L.toComp([q.left + q.width / 2, q.top + q.height / 2]);",
                layer_ref(&comp, l, index_after(id))
            )
        }
    };
    let size = [comp.width as f64, comp.height as f64];
    let ring_name = comp.unique_layer_name("Iris Ring");
    let in_ref = layer_ref(&comp, &layer, index_after(lid));
    let (mask_uid, ring_id) = s.edit("Iris", None, |proj, st| {
        let mut next = proj.next_id;
        let l = layer_mut(proj, cid, lid)?;
        let mut ids = Ids(&mut next);
        let keys = [(l.layer_time(start), 0.0), (l.layer_time(end), 1.0)];
        let fx = l.props.sub_mut("effects").ok_or_else(|| bad(C, "the layer has no effects"))?;
        let sl = slider(&mut ids, fx, "Iris", 0.0, &keys, curve, size)?;
        let sl_name = sl.name.clone();
        fx.children.push(sl.into());
        let shared = iris_js(&format!("effect({})(\"Slider\")", js_str(&sl_name)), &centre_js, feather);
        let path_expr = format!(
            "// Iris (transition.iris): a circle opening from c to the farthest corner, in comp space.\n{shared}\n\
var k = 0.5523 * r;\n\
var P = [[0, -r], [r, 0], [0, r], [-r, 0]], I = [[-k, 0], [0, -k], [k, 0], [0, k]], O = [[k, 0], [0, k], [-k, 0], [0, -k]];\n\
var vs = [], ins = [], outs = [];\n\
for (var i = 0; i < 4; i++) {{\n\
  var a = [c[0] + P[i][0], c[1] + P[i][1]], m = fromComp(a);\n\
  var mi = fromComp([a[0] + I[i][0], a[1] + I[i][1]]), mo = fromComp([a[0] + O[i][0], a[1] + O[i][1]]);\n\
  vs.push([m[0], m[1]]); ins.push([mi[0] - m[0], mi[1] - m[1]]); outs.push([mo[0] - m[0], mo[1] - m[1]]);\n\
}}\n\
createPath(vs, ins, outs, true);"
        );
        let masks = l.props.sub_mut("masks").ok_or_else(|| bad(C, "the layer has no masks"))?;
        let k = masks.children.len();
        let mut g = build::mask(&mut ids, &format!("Iris {}", k + 1), rounded_rect([0.0, 0.0], 0.0, 0.0, 0.0, 0.0), MaskMode::Add, build::MASK_COLORS[k % 8]);
        if let Some(pr) = g.prop_mut("path") {
            pr.expr = Some(Expression { text: path_expr, enabled: true });
        }
        if let Some(pr) = g.prop_mut("feather") {
            pr.value = KV::Vec2([feather, feather]);
        }
        let mask_uid = g.uid;
        masks.children.push(g.into());
        proj.next_id = next;
        let ring_id = match ring {
            None => None,
            Some((color, width)) => {
                let comp_now = proj.comp(cid).cloned().ok_or(EngineError::NoComp)?;
                let mut rl = build::layer(proj, &comp_now, &ring_name, LayerSource::Shape, (comp_now.width, comp_now.height), None);
                rl.label = Label::Yellow;
                rl.in_point = start;
                rl.out_point = end.max(start);
                if let Some(pr) = rl.props.prop_mut("transform/position") {
                    pr.value = KV::Vec3([0.0, 0.0, 0.0]);
                }
                let mut next = proj.next_id;
                let mut ids = Ids(&mut next);
                let shared = iris_js(&format!("{in_ref}.effect({})(\"Slider\")", js_str(&sl_name)), &centre_js, feather);
                let mut ell = build::shape_ellipse(&mut ids, [0.0, 0.0], [0.0, 0.0]);
                if let Some(pr) = ell.prop_mut("size") {
                    pr.expr = Some(Expression { text: format!("// Iris ring: the iris edge.\n{shared}\n[2 * r, 2 * r];"), enabled: true });
                }
                if let Some(pr) = ell.prop_mut("position") {
                    pr.expr = Some(Expression { text: format!("// Iris ring: the iris centre.\n{shared}\n[c[0], c[1]];"), enabled: true });
                }
                let stroke = build::shape_stroke(&mut ids, [f64::from(color[0]), f64::from(color[1]), f64::from(color[2]), 1.0], width);
                let grp = build::shape_group(&mut ids, "Ring", vec![ell, stroke]);
                if let Some(c) = rl.props.sub_mut("contents") {
                    c.children.push(grp.into());
                }
                proj.next_id = next;
                Some(super::layer::insert_layer_at(proj, st, cid, rl, Some(at))?)
            }
        };
        st.selected_layers = vec![lid];
        Ok((mask_uid, ring_id))
    })?;
    Ok(json!({"layer": lid.0, "mask": mask_uid, "ring": ring_id.map(|r| r.0), "end": end.seconds()}))
}

// ---------------------------------------------------------------- morphRect

/// The corner radius a shape layer draws its first rectangle with (layer px), else 0.
fn shape_roundness(ctx: &EvalCtx, l: &Layer) -> f64 {
    fn find(g: &kition_motion_project::PropGroup) -> Option<&Property> {
        for c in &g.children {
            if let kition_motion_project::Node::Group(sub) = c {
                if sub.match_id == "rect" {
                    return sub.get("roundness");
                }
                if let Some(p) = find(sub) {
                    return Some(p);
                }
            }
        }
        None
    }
    if !matches!(l.source, LayerSource::Shape) {
        return 0.0;
    }
    l.props.sub("contents").and_then(find).map(|pr| ctx.value(l, pr).as_f64()).filter(|v| v.is_finite()).unwrap_or(0.0).max(0.0)
}

/// A layer's 2D placement at one time: anchor, position, scale (fractions), and its content
/// rectangle in layer space.
#[derive(Clone, Copy)]
struct Placement {
    anchor: [f64; 3],
    position: [f64; 3],
    scale: [f64; 3],
    local: [f64; 4],
    opacity: f64,
}

fn placement(ctx: &EvalCtx, l: &Layer, cmd: &str) -> Result<Placement> {
    if l.parent.is_some() || l.is_3d() {
        return Err(bad(cmd, format!("{} must be an unparented 2D layer", l.name)));
    }
    if tr3(ctx, l, "rotation", [0.0; 3])[0].abs() > 0.01 {
        return Err(bad(cmd, format!("{} must not be rotated", l.name)));
    }
    let sc = tr3(ctx, l, "scale", [100.0; 3]);
    Ok(Placement {
        anchor: tr3(ctx, l, "anchor", [0.0; 3]),
        position: tr3(ctx, l, "position", [0.0; 3]),
        scale: [sc[0] / 100.0, sc[1] / 100.0, sc[2] / 100.0],
        local: local_rect(ctx, l),
        opacity: tr3(ctx, l, "opacity", [100.0; 3])[0],
    })
}

/// Where a layer goes so its content covers container `r` (`[x0, y0, x1, y1]`, comp px): its
/// scale (fractions), position, and the container in layer space (centre, size, corner radii).
fn cover(pl: &Placement, home: [f64; 4], r: [f64; 4], radius: f64) -> ([f64; 3], [f64; 3], [f64; 2], [f64; 2], [f64; 2]) {
    let (hw, hh) = ((home[2] - home[0]).max(1e-6), (home[3] - home[1]).max(1e-6));
    let (rw, rh) = ((r[2] - r[0]).max(0.0), (r[3] - r[1]).max(0.0));
    let k = (rw / hw).max(rh / hh);
    let sx = if (pl.scale[0] * k).abs() < 1e-9 { 1e-9 } else { pl.scale[0] * k };
    let sy = if (pl.scale[1] * k).abs() < 1e-9 { 1e-9 } else { pl.scale[1] * k };
    let lc = [(pl.local[0] + pl.local[2]) / 2.0, (pl.local[1] + pl.local[3]) / 2.0];
    let rc = [(r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0];
    let position = [rc[0] - sx * (lc[0] - pl.anchor[0]), rc[1] - sy * (lc[1] - pl.anchor[1]), pl.position[2]];
    let scale = [sx * 100.0, sy * 100.0, pl.scale[2] * 100.0];
    (scale, position, lc, [rw / sx.abs(), rh / sy.abs()], [radius / sx.abs(), radius / sy.abs()])
}

fn morph_rect(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "transition.morphRect";
    let cid = super::comp_id(s, p)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let from = layer_arg(&comp, p, "from", C)?;
    let to = layer_arg(&comp, p, "to", C)?;
    if from == to {
        return Err(bad(C, "`from` and `to` must be two different layers"));
    }
    let (fl, tl) = (comp.layer(from).cloned().ok_or(EngineError::NoComp)?, comp.layer(to).cloned().ok_or(EngineError::NoComp)?);
    for l in [&fl, &tl] {
        if l.props.sub("masks").is_none() {
            return Err(bad(C, format!("{} can't have masks", l.name)));
        }
    }
    let start = start_p(s, p, cid, C)?;
    let frames = pos(p, "frames", 18.0, C)?.max(1.0);
    let curve = pe::curve_p(p, "ease", Curve::new(0.65, 0.0, 0.35, 1.0), C)?;
    let end = start + pe::frames(&comp, frames);
    let (c0, c1) = (ctx_at(s, cid, &comp, start), ctx_at(s, cid, &comp, end));
    let (pf, pt) = (placement(&c0, &fl, C)?, placement(&c1, &tl, C)?);
    let (a, b) = (comp_rect(&c0, &fl), comp_rect(&c1, &tl));
    if a[2] - a[0] < 1e-3 || a[3] - a[1] < 1e-3 || b[2] - b[0] < 1e-3 || b[3] - b[1] < 1e-3 {
        return Err(bad(C, "both layers need visible content (an empty rectangle can't morph)"));
    }
    let detect = |ctx: &EvalCtx, l: &Layer, pl: &Placement| shape_roundness(ctx, l) * pl.scale[0].abs().min(pl.scale[1].abs());
    let radius: [f64; 2] = match p.get("radius") {
        None | Some(Value::Null) => [detect(&c0, &fl, &pf), detect(&c1, &tl, &pt)],
        Some(Value::Number(n)) => {
            let r = n.as_f64().filter(|r| r.is_finite() && *r >= 0.0).ok_or_else(|| bad(C, "`radius` must be ≥ 0"))?;
            [r, r]
        }
        Some(Value::Array(v)) => match v.iter().map(Value::as_f64).collect::<Option<Vec<f64>>>().as_deref() {
            Some([r0, r1]) if r0.is_finite() && r1.is_finite() && *r0 >= 0.0 && *r1 >= 0.0 => [*r0, *r1],
            _ => return Err(bad(C, "`radius`: px, or [from, to] ≥ 0")),
        },
        Some(_) => return Err(bad(C, "`radius`: px, or [from, to]")),
    };
    // The incoming layer fades in over the last third; when it sits below the outgoing one, the
    // outgoing one fades out instead of cutting at the end.
    let idx = |id: LayerId| comp.layers.iter().position(|l| l.id == id).unwrap_or(0);
    let to_above = idx(to) < idx(from);
    let fade_at = start + pe::frames(&comp, frames * 2.0 / 3.0);
    let steps = frame_steps(&comp, start, frames);
    let lerp = |x: f64, y: f64, e: f64| x + (y - x) * e;
    struct Track {
        pos: Vec<(Tick, KV)>,
        scale: Vec<(Tick, KV)>,
        path: Vec<(Tick, KV)>,
    }
    let mut tracks = [Track { pos: vec![], scale: vec![], path: vec![] }, Track { pos: vec![], scale: vec![], path: vec![] }];
    for (u, t) in &steps {
        let e = curve.at(*u);
        let r = [lerp(a[0], b[0], e), lerp(a[1], b[1], e), lerp(a[2], b[2], e), lerp(a[3], b[3], e)];
        let rad = lerp(radius[0], radius[1], e);
        for (i, (l, pl, home)) in [(&fl, &pf, a), (&tl, &pt, b)].into_iter().enumerate() {
            let (sc, po, lc, size, rr) = cover(pl, home, r, rad);
            let lt = l.layer_time(*t);
            let tr = &mut tracks[i];
            tr.pos.push((lt, KV::Vec3(po)));
            tr.scale.push((lt, KV::Vec3(sc)));
            tr.path.push((lt, KV::Path(rounded_rect(lc, size[0], size[1], rr[0], rr[1]))));
        }
    }
    let mb = pe::motion_blur_p(p);
    let [tf, tt] = tracks;
    let masks = s.edit("Morph Rect", None, |proj, st| {
        let mut out = vec![];
        for (lid, l0, tr, pl) in [(from, &fl, tf, &pf), (to, &tl, tt, &pt)] {
            let mut next = proj.next_id;
            let l = layer_mut(proj, cid, lid)?;
            if let Some(pr) = l.props.prop_mut("transform/position") {
                splice(pr, tr.pos);
            }
            if let Some(pr) = l.props.prop_mut("transform/scale") {
                splice(pr, tr.scale);
            }
            let (t0, t23, t1) = (l0.layer_time(start), l0.layer_time(fade_at), l0.layer_time(end));
            if let Some(pr) = l.props.prop_mut("transform/opacity") {
                if lid == to {
                    splice(pr, vec![(t0, KV::Scalar(0.0)), (t23, KV::Scalar(0.0)), (t1, KV::Scalar(pl.opacity))]);
                } else if to_above {
                    splice_hold(pr, vec![(t0, KV::Scalar(pl.opacity)), (t1, KV::Scalar(0.0))]);
                } else {
                    splice(pr, vec![(t0, KV::Scalar(pl.opacity)), (t23, KV::Scalar(pl.opacity)), (t1, KV::Scalar(0.0))]);
                }
            }
            let ms = l.props.sub_mut("masks").ok_or_else(|| bad(C, "the layer has no masks"))?;
            let k = ms.children.len();
            let first = tr.path.first().map(|k| k.1.clone()).unwrap_or(KV::Path(rounded_rect([0.0; 2], 0.0, 0.0, 0.0, 0.0)));
            let path = match first {
                KV::Path(sp) => sp,
                _ => rounded_rect([0.0; 2], 0.0, 0.0, 0.0, 0.0),
            };
            let mut g = build::mask(&mut Ids(&mut next), &format!("Morph {}", k + 1), path, MaskMode::Add, build::MASK_COLORS[k % 8]);
            if let Some(pr) = g.prop_mut("path") {
                splice(pr, tr.path);
            }
            out.push(g.uid);
            ms.children.push(g.into());
            proj.next_id = next;
        }
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &[from, to], pe::SHUTTER);
        }
        st.selected_layers = vec![to];
        Ok(out)
    })?;
    Ok(json!({"from": from.0, "to": to.0, "masks": masks, "fromRect": a, "toRect": b, "radius": radius, "end": end.seconds()}))
}

// ---------------------------------------------------------------- zoomThrough

fn zoom_through(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "transition.zoomThrough";
    let (cid, from) = layer_p(s, p, C)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let to = layer_arg(&comp, p, "toLayer", C)?;
    if from == to {
        return Err(bad(C, "`layer` and `toLayer` must be two different layers"));
    }
    let (fl, tl) = (comp.layer(from).cloned().ok_or(EngineError::NoComp)?, comp.layer(to).cloned().ok_or(EngineError::NoComp)?);
    let start = start_p(s, p, cid, C)?;
    let frames = pos(p, "frames", 12.0, C)?.max(1.0);
    let settle = pos(p, "settleFrames", 16.0, C)?.max(1.0);
    let blend = pos(p, "blend", 2.0, C)?;
    let overshoot = pos(p, "overshoot", 1.25, C)?.max(0.01);
    let size = pos(p, "size", 48.0, C)?.max(0.5);
    let push = pe::curve_p(p, "ease", Curve::new(0.5, 0.0, 0.75, 0.5), C)?;
    let land = pe::curve_p(p, "settleEase", Curve::new(0.16, 1.0, 0.3, 1.0), C)?;
    let (cw, ch) = (comp.width as f64, comp.height as f64);
    let centre = [cw / 2.0, ch / 2.0];
    let fill = cw.max(ch) / size;
    if fill <= 1.0 {
        return Err(bad(C, "`size` must be smaller than the frame"));
    }
    let boundary = start + pe::frames(&comp, frames);
    // The layers cross once the frame is filled: the upper one fades over `blend` frames, the
    // lower one cuts underneath, so the frame never dips through to what is behind them.
    let (b0, b1) = (boundary, boundary + pe::frames(&comp, blend));
    let to_above = comp.layers.iter().position(|l| l.id == to) < comp.layers.iter().position(|l| l.id == from);
    let landed = boundary + pe::frames(&comp, settle);
    let c0 = ctx_at(s, cid, &comp, start);
    let pf = placement(&c0, &fl, C)?;
    let target = match point_p(p, "target", C)? {
        Some(t) => t,
        None => {
            let r = comp_rect(&c0, &fl);
            [(r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0]
        }
    };
    let c1 = ctx_at(s, cid, &comp, landed);
    let pt = placement(&c1, &tl, C)?;
    // The target in the outgoing layer's space.
    let sx0 = if pf.scale[0].abs() < 1e-9 { 1e-9 } else { pf.scale[0] };
    let sy0 = if pf.scale[1].abs() < 1e-9 { 1e-9 } else { pf.scale[1] };
    let q = [pf.anchor[0] + (target[0] - pf.position[0]) / sx0, pf.anchor[1] + (target[1] - pf.position[1]) / sy0];
    let ln_fill = fill.ln();
    let mut out_pos = vec![];
    let mut out_scale = vec![];
    for (u, t) in frame_steps(&comp, start, frames) {
        // Scale grows in log space; the target slides to the frame centre as the zoom grows.
        let g = (ln_fill * push.at(u)).exp();
        let w = (g - 1.0) / (fill - 1.0);
        let at = [target[0] + (centre[0] - target[0]) * w, target[1] + (centre[1] - target[1]) * w];
        let (sx, sy) = (sx0 * g, sy0 * g);
        let lt = fl.layer_time(t);
        out_pos.push((lt, KV::Vec3([at[0] - sx * (q[0] - pf.anchor[0]), at[1] - sy * (q[1] - pf.anchor[1]), pf.position[2]])));
        out_scale.push((lt, KV::Vec3([sx * 100.0, sy * 100.0, pf.scale[2] * 100.0])));
    }
    // The incoming layer: from `overshoot` × its scale (about the frame centre) settling to 1.
    let ln_over = overshoot.ln();
    let place_in = |k: f64| {
        (
            KV::Vec3([centre[0] + k * (pt.position[0] - centre[0]), centre[1] + k * (pt.position[1] - centre[1]), pt.position[2]]),
            KV::Vec3([pt.scale[0] * k * 100.0, pt.scale[1] * k * 100.0, pt.scale[2] * 100.0]),
        )
    };
    let (mut in_pos, mut in_scale) = (vec![], vec![]);
    for (u, t) in frame_steps(&comp, boundary, settle) {
        let (po, sc) = place_in((ln_over * (1.0 - land.at(u))).exp());
        in_pos.push((tl.layer_time(t), po));
        in_scale.push((tl.layer_time(t), sc));
    }
    let mb = pe::motion_blur_p(p);
    s.edit("Zoom Through", None, |proj, st| {
        let l = layer_mut(proj, cid, from)?;
        if let Some(pr) = l.props.prop_mut("transform/position") {
            splice(pr, out_pos);
        }
        if let Some(pr) = l.props.prop_mut("transform/scale") {
            splice(pr, out_scale);
        }
        if let Some(pr) = l.props.prop_mut("transform/opacity") {
            let (ts, t0, t1) = (fl.layer_time(start), fl.layer_time(b0), fl.layer_time(b1));
            if !to_above && t1 > t0 {
                splice(pr, vec![(t0, KV::Scalar(pf.opacity)), (t1, KV::Scalar(0.0))]);
            } else {
                splice_hold(pr, vec![(ts, KV::Scalar(pf.opacity)), (t1, KV::Scalar(0.0))]);
            }
        }
        let l = layer_mut(proj, cid, to)?;
        if let Some(pr) = l.props.prop_mut("transform/position") {
            splice(pr, in_pos);
        }
        if let Some(pr) = l.props.prop_mut("transform/scale") {
            splice(pr, in_scale);
        }
        if let Some(pr) = l.props.prop_mut("transform/opacity") {
            let (ts, t0, t1) = (tl.layer_time(start), tl.layer_time(b0), tl.layer_time(b1));
            if to_above && t1 > t0 {
                splice(pr, vec![(ts, KV::Scalar(0.0)), (t0, KV::Scalar(0.0)), (t1, KV::Scalar(pt.opacity))]);
            } else {
                splice_hold(pr, vec![(ts, KV::Scalar(0.0)), (t0, KV::Scalar(pt.opacity))]);
            }
        }
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &[from, to], pe::SHUTTER);
        }
        st.selected_layers = vec![to];
        Ok(())
    })?;
    Ok(json!({"layer": from.0, "toLayer": to.0, "target": target, "fill": fill, "boundary": boundary.seconds(), "end": landed.seconds()}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "transition.iris",
            "Iris Transition",
            [],
            None,
            "{comp?, layer: json (the incoming layer or precomp), center?: json ([x, y] comp px, or a layer id, name or #n whose middle the iris follows; default the comp centre), start?: seconds (comp time; default the current time), frames?: number (default 14), feather?: px (mask feather, default 0), ease?: json (curve name or [x1, y1, x2, y2], default swift), ring?: json ({color?: color, width?: px (default 6)} or true: a stroke on the iris edge)}",
            has_comp,
            iris
        ),
        cmd!(
            "transition.morphRect",
            "Morph Rect Transition",
            [],
            None,
            "{comp?, from: json (the outgoing layer: its on-screen rectangle at start), to: json (the incoming layer: its rectangle at the end), start?: seconds (comp time; default the current time), frames?: number (default 18), ease?: json (curve name or [x1, y1, x2, y2], default smooth), radius?: json (corner radius px: a number or [from, to]; default each layer's own rectangle roundness, else 0), motionBlur?: boolean (default true)}",
            has_comp,
            morph_rect
        ),
        cmd!(
            "transition.zoomThrough",
            "Zoom Through Transition",
            [],
            None,
            "{comp?, layer: json (the outgoing layer), target?: [x, y] (comp px at start: the point pushed into; default the layer's middle), toLayer: json (the incoming layer), start?: seconds (comp time; default the current time), frames?: number (the push, default 12), size?: px (the area around the target that fills the frame at the end of the push, default 48), ease?: json (push curve in log scale, default ease-in [0.5, 0, 0.75, 0.5]), overshoot?: number (incoming start scale, default 1.25), settleFrames?: number (default 16), settleEase?: json (default snappy), blend?: number (frames the upper layer fades over once the frame is filled, default 2), motionBlur?: boolean (default true)}",
            has_comp,
            zoom_through
        ),
    ]
}
