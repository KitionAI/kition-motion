//! Promo light: a spotlight that darkens the frame around a layer (`light.spotlight`), one
//! specular sheen across a layer (`light.sheen`) and an elevation-driven lift shadow
//! (`layer.liftShadow`). Built from standard effects (Circle, Gaussian Blur, CC Light Sweep,
//! Drop Shadow, Slider Control) wired with expressions, so everything stays editable.

use kition_motion_color::Label;
use kition_motion_keyframe::Value as KV;
use kition_motion_project::build::{self, Ids};
use kition_motion_project::{Comp, Expression, ItemKind, Layer, LayerSource, PropGroup, Solid};
use kition_motion_time::Tick;
use serde_json::{Value, json};

use super::layer::color_p;
use super::promo_ease::{self as pe, Curve};
use super::{CommandSpec, bad, comp_id, f_p, has_comp, layer_mut, layer_p, resolve_layer, str_p};
use crate::{EngineError, Result, Session, cmd};

/// A non-negative finite number parameter.
fn num(p: &Value, k: &str, d: f64, cmd: &str) -> Result<f64> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(d),
        Some(v) => v.as_f64().filter(|x| x.is_finite() && *x >= 0.0).ok_or_else(|| bad(cmd, format!("`{k}` must be a number ≥ 0"))),
    }
}

pub(crate) fn js(x: f64) -> String {
    if x.is_finite() { format!("{x:.6}") } else { "0".into() }
}

pub(crate) fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// `start`: comp seconds, default the comp's current time.
pub(crate) fn start_p(s: &Session, p: &Value, cid: kition_motion_project::ItemId, cmd: &str) -> Result<Tick> {
    match p.get("start") {
        None | Some(Value::Null) => Ok(s.time_of(cid)),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).map(Tick::from_seconds_f64).ok_or_else(|| bad(cmd, "`start` must be comp seconds")),
    }
}

/// A new effect instance named uniquely in `fx`.
pub(crate) fn effect(ids: &mut Ids, fx: &PropGroup, id: &str, name: &str, size: [f64; 2]) -> Result<PropGroup> {
    let spec = kition_motion_effects::lookup(id).ok_or_else(|| EngineError::Other(format!("effect {id} is not available")))?;
    let name = super::effect::unique_name(fx, name);
    Ok(kition_motion_effects::instantiate(spec, ids, &name, size))
}

fn set(g: &mut PropGroup, path: &str, v: KV) {
    if let Some(pr) = g.prop_mut(path) {
        pr.value = v;
    }
}

fn expr(g: &mut PropGroup, path: &str, text: String) {
    if let Some(pr) = g.prop_mut(path) {
        pr.expr = Some(Expression { text, enabled: true });
    }
}

/// A Slider Control named `name`, keyed `(layer time, value)` with `curve`, or static `value`.
pub(crate) fn slider(ids: &mut Ids, fx: &PropGroup, name: &str, value: f64, keys: &[(Tick, f64)], curve: Curve, size: [f64; 2]) -> Result<PropGroup> {
    let mut g = effect(ids, fx, "ec.control.slider", name, size)?;
    if let Some(pr) = g.prop_mut("slider") {
        pr.value = KV::Scalar(value);
        if keys.len() > 1 {
            let ks: Vec<(Tick, KV)> = keys.iter().map(|(t, v)| (*t, KV::Scalar(*v))).collect();
            pe::set_keys(pr, &ks, &[curve]);
        }
    }
    Ok(g)
}

/// `thisComp.layer(…)` for `l`: by name when it is unique in the comp, else by index.
fn layer_ref(comp: &Comp, l: &Layer, index_after_insert: usize) -> String {
    if comp.layers.iter().filter(|x| x.name == l.name).count() == 1 {
        format!("thisComp.layer({})", js_str(&l.name))
    } else {
        format!("thisComp.layer({index_after_insert})")
    }
}

// ---------------------------------------------------------------- spotlight

fn spotlight(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "light.spotlight";
    let cid = comp_id(s, p)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let tv = p.get("target").ok_or_else(|| bad(C, "`target` (the layer the light follows) required"))?;
    let target = resolve_layer(&comp, tv).ok_or_else(|| bad(C, format!("no layer {tv}")))?;
    let tl = comp.layer(target).cloned().ok_or(EngineError::NoComp)?;
    // Above the topmost of `layers` (they and everything under them darken), else on top.
    let at = match p.get("layers") {
        Some(Value::Array(a)) if !a.is_empty() => {
            let mut idx = vec![];
            for v in a {
                let id = resolve_layer(&comp, v).ok_or_else(|| bad(C, format!("no layer {v}")))?;
                idx.push(comp.layers.iter().position(|l| l.id == id).unwrap_or(0));
            }
            idx.into_iter().min().unwrap_or(0)
        }
        _ => 0,
    };
    let radius: [f64; 2] = match p.get("radius") {
        None | Some(Value::Null) => [400.0, 300.0],
        Some(Value::Number(n)) => {
            let r = n.as_f64().unwrap_or(300.0);
            [r, r]
        }
        Some(Value::Array(a)) => match a.iter().filter_map(Value::as_f64).collect::<Vec<_>>().as_slice() {
            [a, b] if *a >= 0.0 && *b >= 0.0 => [*a, *b],
            _ => return Err(bad(C, "`radius`: a number or [from, to] in pixels")),
        },
        Some(_) => return Err(bad(C, "`radius`: a number or [from, to] in pixels")),
    };
    let dim = num(p, "dim", 0.92, C)?.min(1.0);
    let pulse = num(p, "pulse", 0.06, C)?;
    let period = num(p, "period", 2.0, C)?.max(0.05);
    let soft = num(p, "softness", 22.0, C)?;
    let spill = num(p, "spill", 0.15, C)?.min(1.0);
    let spill_soft = num(p, "spillSoftness", 60.0, C)?;
    let frames = num(p, "frames", 24.0, C)?;
    let curve = pe::curve_p(p, "ease", Curve::new(0.22, 1.0, 0.36, 1.0), C)?;
    let color = color_p(p, "color").unwrap_or([0.0, 0.0, 0.0]);
    let start = start_p(s, p, cid, C)?;
    let settle = start + pe::frames(&comp, frames);
    let name = comp.unique_layer_name(str_p(p, "name").unwrap_or("Spotlight"));
    // The target's index once the spotlight is inserted above `at`.
    let t_index = comp.layers.iter().position(|l| l.id == target).map(|i| if i >= at { i + 2 } else { i + 1 }).unwrap_or(1);
    let tref = layer_ref(&comp, &tl, t_index);
    let center = format!(
        "// Spotlight centre: the middle of {}.\nvar L = {tref}, r = L.sourceRectAtTime(time, false);\nvar q = fromComp(L.toComp([r.left + r.width / 2, r.top + r.height / 2]));\n[q[0], q[1]];",
        tl.name
    );
    let (w, h) = (comp.width, comp.height);
    let size = [w as f64, h as f64];
    let lid = s.edit("Spotlight", None, |proj, st| {
        let folder = proj.folder_named("Solids").unwrap_or_else(|| proj.add_item("Solids", Label::Yellow, None, ItemKind::Folder));
        let sid = proj.add_item(&name, Label::Purple, Some(folder), ItemKind::Solid(Solid { color, width: w, height: h, pixel_aspect: comp.pixel_aspect }));
        let mut l = build::layer(proj, &comp, &name, LayerSource::Solid { item: sid }, (w, h), None);
        l.label = Label::Purple;
        // The light comes on at `start`: the layer begins there and dims in with the radius.
        l.in_point = start.max(l.in_point);
        let ks = [(l.layer_time(start), KV::Scalar(0.0)), (l.layer_time(settle), KV::Scalar(dim * 100.0))];
        if let Some(pr) = l.props.prop_mut("transform/opacity") {
            pe::set_keys(pr, &ks, &[curve]);
        }
        let lid = l.id;
        let mut next = proj.next_id;
        let mut ids = Ids(&mut next);
        let keys = [(l.layer_time(start), radius[0]), (l.layer_time(settle), radius[1])];
        let mut fx = l.props.sub_mut("effects").cloned().ok_or_else(|| bad(C, "solids have effects"))?;
        let rad = slider(&mut ids, &fx, "Spot Radius", radius[0], &keys, curve, size)?;
        let rad_name = rad.name.clone();
        fx.children.push(rad.into());
        let radius_expr = format!(
            "// Radius: the Spot Radius slider, breathing up to +{pulse} once it has settled.\nvar r = effect({})(\"Slider\"), t = time - {};\nr * (1 + {} * (t > 0 ? 0.5 - 0.5 * Math.cos(2 * Math.PI * t / {}) : 0));",
            js_str(&rad_name),
            js(settle.seconds()),
            js(pulse),
            js(period)
        );
        // Spill: a faint, very soft hole; then the core hole with its own softness.
        for (hole, op, sig) in [("Spill Hole", spill, spill_soft), ("Spot Hole", 1.0, soft)] {
            if op <= 0.0 {
                continue;
            }
            let mut c = effect(&mut ids, &fx, "ec.generate.circle", hole, size)?;
            set(&mut c, "blendingMode", KV::Enum(19));
            set(&mut c, "opacity", KV::Scalar(op * 100.0));
            expr(&mut c, "center", center.clone());
            expr(&mut c, "radius", radius_expr.clone());
            fx.children.push(c.into());
            if sig > 0.0 {
                let mut b = effect(&mut ids, &fx, "ec.blur.gaussian", &format!("{hole} Softness"), size)?;
                set(&mut b, "blurriness", KV::Scalar(sig * 2.0));
                set(&mut b, "repeatEdge", KV::Bool(true));
                fx.children.push(b.into());
            }
        }
        if let Some(e) = l.props.sub_mut("effects") {
            *e = fx;
        }
        proj.next_id = next;
        super::layer::insert_layer_at(proj, st, cid, l, Some(at))?;
        Ok(lid)
    })?;
    Ok(json!({"layer": lid.0, "target": target.0, "settle": settle.seconds()}))
}

// ---------------------------------------------------------------- sheen

fn sheen(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "light.sheen";
    let (cid, lid) = layer_p(s, p, C)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let layer = comp.layer(lid).cloned().ok_or(EngineError::NoComp)?;
    let (size, offset) = kition_motion_render::effect_bounds(&s.project, &comp, &layer);
    let frames = num(p, "frames", 32.0, C)?;
    let angle = p.get("angle").and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(115.0);
    let width = num(p, "width", 2.0, C)?;
    let soft = num(p, "soft", 0.14, C)?;
    let core = num(p, "core", 0.5, C)?;
    let color = color_p(p, "color").unwrap_or([1.0, 0.965, 0.92]);
    let curve = pe::curve_p(p, "ease", Curve::new(0.4, 0.0, 0.2, 1.0), C)?;
    let start = start_p(s, p, cid, C)?;
    let end = start + pe::frames(&comp, frames);
    // CC Light Sweep's Direction is the band's normal; a CSS gradient angle is measured from "up".
    let dir = angle - 90.0;
    // The band (2x the layer's width) holds a wide soft light and a narrow core.
    let (soft_w, core_w) = (width * 0.4, width * 0.09);
    let lid2 = lid;
    let uids = s.edit("Sheen", None, |proj, _| {
        let mut next = proj.next_id;
        let l = layer_mut(proj, cid, lid2)?;
        let keys = [(l.layer_time(start), 0.0), (l.layer_time(end), 1.0)];
        let mut fx = l.props.sub_mut("effects").cloned().ok_or_else(|| bad(C, "this layer type has no effects"))?;
        let mut ids = Ids(&mut next);
        let prog = slider(&mut ids, &fx, "Sheen Progress", 0.0, &keys, curve, size)?;
        let prog_name = prog.name.clone();
        let mut uids = vec![prog.uid];
        fx.children.push(prog.into());
        let center = format!(
            "// Sheen centre: sweeps once across the layer as Sheen Progress goes 0 to 1.\n\
var r = sourceRectAtTime(time, false), p = effect({})(\"Slider\");\n\
var a = {} * Math.PI / 180, c = Math.max(Math.abs(Math.cos(a)), 0.05), tn = Math.abs(Math.tan(a));\n\
var m = (r.width * {} / 2) / c + tn * r.height / 2 + 2;\n\
[r.left - m + p * (r.width + 2 * m) - {}, r.top + r.height / 2 - {}];",
            js_str(&prog_name),
            js(dir),
            js(soft_w),
            js(offset[0]),
            js(offset[1])
        );
        for (name, intensity, w) in [("Sheen Soft", soft, soft_w), ("Sheen Core", core, core_w)] {
            let mut g = effect(&mut ids, &fx, "ec.generate.cclightsweep", name, size)?;
            set(&mut g, "direction", KV::Scalar(dir));
            set(&mut g, "shape", KV::Enum(1));
            set(&mut g, "sweepIntensity", KV::Scalar(intensity * 100.0));
            set(&mut g, "edgeIntensity", KV::Scalar(0.0));
            set(&mut g, "lightColor", KV::Color([f64::from(color[0]), f64::from(color[1]), f64::from(color[2]), 1.0]));
            set(&mut g, "lightReceptionMode", KV::Enum(0));
            expr(&mut g, "center", center.clone());
            expr(&mut g, "width", format!("sourceRectAtTime(time, false).width * {}", js(w)));
            uids.push(g.uid);
            fx.children.push(g.into());
        }
        if let Some(e) = l.props.sub_mut("effects") {
            *e = fx;
        }
        proj.next_id = next;
        Ok(uids)
    })?;
    Ok(json!({"layer": lid.0, "effects": uids, "end": end.seconds()}))
}

// ---------------------------------------------------------------- lift shadow

fn lift_shadow(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "layer.liftShadow";
    let (cid, lid) = layer_p(s, p, C)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let layer = comp.layer(lid).cloned().ok_or(EngineError::NoComp)?;
    let (size, _) = kition_motion_render::effect_bounds(&s.project, &comp, &layer);
    let elevation = num(p, "elevation", 3.0, C)?;
    let curve = pe::curve_p(p, "ease", Curve::new(0.65, 0.0, 0.35, 1.0), C)?;
    let start = start_p(s, p, cid, C)?;
    let color = color_p(p, "color").unwrap_or([0.0, 0.0, 0.0]);
    let direction = p.get("direction").and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(180.0);
    // Keys: `keys: [[comp s, elevation], …]`, or `to` reached `frames` after `start`.
    let mut keys: Vec<(Tick, f64)> = vec![];
    if let Some(Value::Array(a)) = p.get("keys") {
        for k in a {
            match (k.get(0).and_then(Value::as_f64), k.get(1).and_then(Value::as_f64)) {
                (Some(t), Some(e)) if t.is_finite() && e.is_finite() && e >= 0.0 => keys.push((layer.layer_time(Tick::from_seconds_f64(t)), e)),
                _ => return Err(bad(C, "`keys`: [[comp seconds, elevation ≥ 0], …]")),
            }
        }
    } else if let Some(to) = f_p(p, "to") {
        if !(to.is_finite() && to >= 0.0) {
            return Err(bad(C, "`to` must be an elevation ≥ 0"));
        }
        let frames = num(p, "frames", 18.0, C)?;
        keys = vec![(layer.layer_time(start), elevation), (layer.layer_time(start + pe::frames(&comp, frames)), to)];
    }
    let first = keys.first().map(|k| k.1).unwrap_or(elevation);
    let uids = s.edit("Lift Shadow", None, |proj, _| {
        let mut next = proj.next_id;
        let l = layer_mut(proj, cid, lid)?;
        let mut fx = l.props.sub_mut("effects").cloned().ok_or_else(|| bad(C, "this layer type has no effects"))?;
        let mut ids = Ids(&mut next);
        let el = slider(&mut ids, &fx, "Elevation", first, &keys, curve, size)?;
        let e = format!("var e = Math.max(0, effect({})(\"Slider\"));", js_str(&el.name));
        let mut uids = vec![el.uid];
        fx.children.push(el.into());
        // Contact: tight and dark near the surface, fading as the layer rises. Ambient: wide and
        // soft, darker the higher it is. Softness is twice the blur sigma.
        let shadows = [
            ("Contact Shadow", format!("{e}\n100 * 0.12 / (1 + e / 24);"), format!("{e}\n0.5 + e * 0.06;"), format!("{e}\n2 * (1 + e * 0.12);")),
            ("Ambient Shadow", format!("{e}\n100 * (0.08 + 0.22 * Math.min(e, 64) / 64);"), format!("{e}\ne * 0.45;"), format!("{e}\n2 * (2 + e * 0.9);")),
        ];
        for (name, op, dist, softness) in shadows {
            let mut g = effect(&mut ids, &fx, "ec.perspective.dropshadow", name, size)?;
            set(&mut g, "color", KV::Color([f64::from(color[0]), f64::from(color[1]), f64::from(color[2]), 1.0]));
            set(&mut g, "direction", KV::Scalar(direction));
            expr(&mut g, "opacity", op);
            expr(&mut g, "distance", dist);
            expr(&mut g, "softness", softness);
            uids.push(g.uid);
            fx.children.push(g.into());
        }
        if let Some(x) = l.props.sub_mut("effects") {
            *x = fx;
        }
        proj.next_id = next;
        Ok(uids)
    })?;
    Ok(json!({"layer": lid.0, "effects": uids}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "light.spotlight",
            "Spotlight",
            [],
            None,
            "{comp?, target: json (layer id, name or #n the light follows), layers?: [integer] (darken these and what is under them; default the whole comp), start?: seconds (comp time; default the current time), radius?: json (px: a number or [from, to], default [400, 300]), frames?: number (frames from the first radius to the second, default 24), ease?: json (curve name or [x1, y1, x2, y2], default out), dim?: number (0..1 darkness outside the light, default 0.92), pulse?: number (fraction the light breathes once settled, default 0.06), period?: seconds (breath length, default 2), softness?: px (edge blur sigma, default 22), spill?: number (0..1 light that leaks past the edge, default 0.15), spillSoftness?: px (default 60), color?: color, name?: string}",
            has_comp,
            spotlight
        ),
        cmd!(
            "light.sheen",
            "Sheen",
            [],
            None,
            "{layer?, start?: seconds (comp time; default the current time), frames?: number (sweep length, default 32), angle?: degrees (CSS gradient angle, default 115), width?: number (band width in layer widths, default 2), soft?: number (wide light intensity, default 0.14), core?: number (narrow core intensity, default 0.5), color?: color, ease?: json (curve name or [x1, y1, x2, y2], default swift)}",
            has_comp,
            sheen
        ),
        cmd!(
            "layer.liftShadow",
            "Lift Shadow",
            [],
            None,
            "{layer?, elevation?: number (px, default 3), to?: number (elevation reached `frames` after `start`), start?: seconds (comp time), frames?: number (default 18), keys?: [[seconds, number]] (comp time, elevation), ease?: json (curve name or [x1, y1, x2, y2], default smooth), color?: color, direction?: degrees (default 180, straight down)}",
            has_comp,
            lift_shadow
        ),
    ]
}
