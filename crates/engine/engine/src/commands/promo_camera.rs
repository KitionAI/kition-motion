//! Promo camera moves (`camera.move`): a two-node camera parented to a 3D null at the target
//! (an orbit rig), keyed with the promo curves — pushHover, craneRise, orbit (continuous-velocity
//! rotation) — and a 2.5D parallax pan of layers at different speeds with depth blur.
//!
//! The camera's view is described by a scale `s` (the target plane appears `s` times its size:
//! the camera sits `zoom / s` in front of it), a framing `f` (1 = the comp's own framing, 0 = the
//! target centred) and the rig's X / Y rotation (the camera orbits the target).

use kition_motion_geom::vec2;
use kition_motion_keyframe::Value as KV;
use kition_motion_project::build::{self, Ids};
use kition_motion_project::{Comp, ItemId, Layer, LayerId, LayerSource, PropGroup};
use kition_motion_render::EvalCtx;
use kition_motion_time::Tick;
use serde_json::{Value, json};

use super::promo_ease::{self as pe, Curve};
use super::{CommandSpec, bad, comp_id, has_comp, layer_mut, resolve_layer, str_p};
use crate::{EngineError, Result, Session, cmd};

const C: &str = "camera.move";

fn num(p: &Value, k: &str, d: f64) -> Result<f64> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(d),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad(C, format!("`{k}` must be a number"))),
    }
}

fn pos_num(p: &Value, k: &str, d: f64) -> Result<f64> {
    let v = num(p, k, d)?;
    if v < 0.0 { Err(bad(C, format!("`{k}` must be ≥ 0"))) } else { Ok(v) }
}

fn layers_list(comp: &Comp, p: &Value) -> Result<Vec<LayerId>> {
    match p.get("layers") {
        None | Some(Value::Null) => Ok(vec![]),
        Some(Value::Array(a)) => a.iter().map(|v| resolve_layer(comp, v).ok_or_else(|| bad(C, format!("no layer {v}")))).collect(),
        Some(_) => Err(bad(C, "`layers`: [layer id, name or #n]")),
    }
}

/// The middle of a layer's content in comp pixels at `t`.
fn layer_center(s: &Session, cid: ItemId, comp: &Comp, l: &Layer, t: Tick) -> [f64; 2] {
    let ctx = EvalCtx { project: &s.project, comp_id: cid, comp, time: t, expr: s.expr.as_deref(), footage: Some(s.footage.as_ref()) };
    let local = match &l.source {
        LayerSource::Text => match l.props.prop("text/sourceText").map(|pr| pr.value_at(l.layer_time(t))) {
            Some(KV::Text(d)) => {
                let b = kition_motion_text::layout_doc(&d).bounds;
                [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0]
            }
            _ => [0.0, 0.0],
        },
        _ => {
            let (size, o) = kition_motion_render::effect_bounds(&s.project, comp, l);
            [o[0] + size[0] / 2.0, o[1] + size[1] / 2.0]
        }
    };
    let q = ctx.layer_to_comp(l).0.apply(vec2(local[0], local[1]));
    [q.x, q.y]
}

/// One view of the rig at a key: scale, framing, X / Y rotation.
#[derive(Clone, Copy)]
struct View {
    s: f64,
    f: f64,
    rx: f64,
    ry: f64,
}

fn set_static(l: &mut Layer, path: &str, v: KV) {
    if let Some(pr) = l.props.prop_mut(path) {
        pr.value = v;
        pr.keys.clear();
        pr.expr = None;
    }
}

fn prop<'a>(l: &'a mut Layer, path: &str) -> Result<&'a mut kition_motion_project::Property> {
    l.props.prop_mut(path).ok_or_else(|| bad(C, format!("the rig has no {path}")))
}

/// Gaussian Blur and Hue/Saturation for depth on a parallax layer.
fn depth_fx(ids: &mut Ids, fx: &mut PropGroup, size: [f64; 2], blur: f64, desaturate: f64) -> Result<()> {
    let add = |ids: &mut Ids, fx: &mut PropGroup, id: &str, name: &str, set: &[(&str, KV)]| -> Result<()> {
        let spec = kition_motion_effects::lookup(id).ok_or_else(|| EngineError::Other(format!("effect {id} is not available")))?;
        let name = super::effect::unique_name(fx, name);
        let mut g = kition_motion_effects::instantiate(spec, ids, &name, size);
        for (k, v) in set {
            if let Some(pr) = g.prop_mut(k) {
                pr.value = v.clone();
            }
        }
        fx.children.push(g.into());
        Ok(())
    };
    if blur > 0.0 {
        add(ids, fx, "ec.blur.gaussian", "Depth Blur", &[("blurriness", KV::Scalar(blur * 2.0))])?;
    }
    if desaturate > 0.0 {
        add(ids, fx, "ec.color.huesaturation", "Depth Desaturate", &[("saturation", KV::Scalar(-desaturate * 100.0))])?;
    }
    Ok(())
}

fn parallax(s: &mut Session, p: &Value, cid: ItemId, comp: Comp, start: Tick) -> Result<Value> {
    let layers = layers_list(&comp, p)?;
    if layers.len() < 2 {
        return Err(bad(C, "parallax needs `layers`: at least two, far to near"));
    }
    let n = layers.len();
    let speeds: Vec<f64> = match p.get("speeds") {
        Some(Value::Array(a)) if a.len() == n => {
            a.iter().map(|v| v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad(C, "`speeds`: numbers"))).collect::<Result<_>>()?
        }
        Some(_) => return Err(bad(C, "`speeds`: one number per layer")),
        None if n == 3 => vec![0.35, 0.7, 1.4],
        None => (0..n).map(|i| 0.35 + (1.4 - 0.35) * i as f64 / (n - 1) as f64).collect(),
    };
    let distance = num(p, "distance", 600.0)?;
    let dir = match p.get("direction") {
        None | Some(Value::Null) => [-1.0, 0.0],
        Some(Value::Array(a)) => match a.iter().filter_map(Value::as_f64).collect::<Vec<_>>().as_slice() {
            [x, y] if (x * x + y * y) > 1e-12 => {
                let l = (x * x + y * y).sqrt();
                [x / l, y / l]
            }
            _ => return Err(bad(C, "`direction`: [x, y], not zero")),
        },
        Some(_) => return Err(bad(C, "`direction`: [x, y]")),
    };
    let frames = pos_num(p, "frames", 90.0)?;
    let curve = pe::curve_p(p, "ease", Curve::new(0.65, 0.0, 0.35, 1.0), C)?;
    let far_blur = pos_num(p, "farBlur", 5.0)?;
    let far_desat = pos_num(p, "farDesaturate", 0.7)?.min(1.0);
    let near_blur = pos_num(p, "nearBlur", 7.0)?;
    let end = start + pe::frames(&comp, frames);
    let mb = pe::motion_blur_p(p);
    let sizes: Vec<[f64; 2]> = layers.iter().filter_map(|id| comp.layer(*id)).map(|l| kition_motion_render::effect_bounds(&s.project, &comp, l).0).collect();
    s.edit("Camera Move Parallax", None, |proj, _| {
        for (i, lid) in layers.iter().enumerate() {
            let mut next = proj.next_id;
            let l = layer_mut(proj, cid, *lid)?;
            let (t0, t1) = (l.layer_time(start), l.layer_time(end));
            let travel = distance * speeds.get(i).copied().unwrap_or(1.0);
            let pr = prop(l, "transform/position")?;
            let p0 = if pr.keys.is_empty() { pr.value.clone() } else { kition_motion_keyframe::evaluate(&pr.keys, t0, pr.spatial).unwrap_or(pr.value.clone()) };
            let mut c = p0.components();
            if c.len() >= 2 {
                c[0] += dir[0] * travel;
                c[1] += dir[1] * travel;
            }
            let p1 = p0.with_components(&c);
            pe::set_keys(pr, &[(t0, p0), (t1, p1)], &[curve]);
            let size = sizes.get(i).copied().unwrap_or([100.0, 100.0]);
            let (blur, desat) = if i == 0 {
                (far_blur, far_desat)
            } else if i + 1 == n {
                (near_blur, 0.0)
            } else {
                (0.0, 0.0)
            };
            let mut ids = Ids(&mut next);
            if let Some(fx) = l.props.sub_mut("effects") {
                depth_fx(&mut ids, fx, size, blur, desat)?;
            }
            proj.next_id = next;
        }
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &layers, pe::SHUTTER);
        }
        Ok(())
    })?;
    Ok(json!({"preset": "parallax", "layers": layers.iter().map(|l| l.0).collect::<Vec<_>>(), "speeds": speeds, "end": end.seconds()}))
}

fn camera_move(s: &mut Session, p: &Value) -> Result<Value> {
    let cid = comp_id(s, p)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let preset = str_p(p, "preset").ok_or_else(|| bad(C, "`preset`: pushHover|craneRise|orbit|parallax"))?.to_string();
    let start = match p.get("start") {
        None | Some(Value::Null) => s.time_of(cid),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).map(Tick::from_seconds_f64).ok_or_else(|| bad(C, "`start` must be comp seconds"))?,
    };
    if preset == "parallax" {
        return parallax(s, p, cid, comp, start);
    }
    // The point the camera works around: `point`, else the middle of `target`.
    let target = match p.get("target") {
        None | Some(Value::Null) => None,
        Some(v) => Some(resolve_layer(&comp, v).ok_or_else(|| bad(C, format!("no layer {v}")))?),
    };
    let point = match (p.get("point").and_then(Value::as_array), target) {
        (Some(a), _) => match a.iter().filter_map(Value::as_f64).collect::<Vec<_>>().as_slice() {
            [x, y] => [*x, *y],
            _ => return Err(bad(C, "`point`: [x, y] in comp pixels")),
        },
        (None, Some(t)) => {
            let l = comp.layer(t).ok_or(EngineError::NoComp)?;
            layer_center(s, cid, &comp, l, start)
        }
        (None, None) => [comp.width as f64 / 2.0, comp.height as f64 / 2.0],
    };
    let mut three: Vec<LayerId> = layers_list(&comp, p)?;
    if three.is_empty() {
        three.extend(target);
    }
    let curve_in = pe::curve_p(p, "ease", Curve::new(0.16, 1.0, 0.3, 1.0), C)?;
    let smooth = Curve::new(0.65, 0.0, 0.35, 1.0);
    // (frame offset from start, view, curve into this key)
    let plan: Vec<(f64, View, Curve)> = match preset.as_str() {
        "pushHover" => {
            let from = pos_num(p, "from", 0.8)?.max(0.01);
            let to = pos_num(p, "scale", 2.6)?.max(0.01);
            let (ry, rx) = (num(p, "rotY", 34.0)?, num(p, "rotX", 8.0)?);
            let push = pos_num(p, "pushFrames", 18.0)?;
            let total = pos_num(p, "frames", 66.0)?;
            let hover = (total - 2.0 * push).max(0.0);
            vec![
                (0.0, View { s: from, f: 1.0, rx: 0.0, ry: 0.0 }, Curve::LINEAR),
                (push, View { s: to, f: 0.0, rx, ry }, curve_in),
                (push + hover, View { s: to * 1.03, f: 0.0, rx, ry: ry * 0.88 }, Curve::LINEAR),
                (push * 2.0 + hover, View { s: 1.0, f: 1.0, rx: 0.0, ry: 0.0 }, smooth),
            ]
        }
        "craneRise" => {
            let from = pos_num(p, "from", 3.0)?.max(0.01);
            let to = pos_num(p, "scale", 1.0)?.max(0.01);
            let frames = pos_num(p, "frames", 102.0)?;
            let tilt = num(p, "rotX", -8.0)?;
            let curve = pe::curve_p(p, "ease", Curve::new(0.2, 0.0, 0.25, 1.0), C)?;
            let anticip = 6.0;
            vec![
                (0.0, View { s: from, f: 0.0, rx: tilt, ry: 0.0 }, Curve::LINEAR),
                (anticip, View { s: from * 1.015, f: 0.0, rx: tilt, ry: 0.0 }, smooth),
                (anticip + frames, View { s: to, f: 1.0, rx: 0.0, ry: 0.0 }, curve),
            ]
        }
        "orbit" => {
            let deg = num(p, "degrees", 30.0)?;
            let frames = pos_num(p, "frames", 90.0)?;
            let sc = pos_num(p, "scale", 1.0)?.max(0.01);
            let rx = num(p, "rotX", 0.0)?;
            [0.0, 0.15, 0.5, 0.85, 1.0]
                .iter()
                .enumerate()
                .map(|(i, k)| (frames * i as f64 / 4.0, View { s: sc, f: 1.0, rx, ry: deg * k }, Curve::LINEAR))
                .collect()
        }
        other => return Err(bad(C, format!("no camera preset {other} (pushHover, craneRise, orbit, parallax)"))),
    };
    let end_frames = plan.last().map(|k| k.0).unwrap_or(0.0);
    let end = start + pe::frames(&comp, end_frames);
    let (cw, ch) = (comp.width as f64, comp.height as f64);
    let cam_name = comp.unique_layer_name(str_p(p, "name").unwrap_or("Promo Camera"));
    let rig_name = comp.unique_layer_name(&format!("{cam_name} Rig"));
    let continuous = preset == "orbit";
    let mb = pe::motion_blur_p(p);
    let (cam_id, rig_id) = s.edit(&format!("Camera Move {preset}"), None, |proj, st| {
        let mut cam = build::layer(proj, &comp, &cam_name, LayerSource::Camera, (comp.width, comp.height), None);
        let mut rig = build::layer(proj, &comp, &rig_name, LayerSource::Null, (comp.width, comp.height), None);
        rig.switches.three_d = true;
        let anchor = rig.props.prop("transform/anchor").map(|a| a.value.as_vec3()).unwrap_or([0.0; 3]);
        set_static(&mut rig, "transform/position", KV::Vec3([point[0], point[1], 0.0]));
        let zoom = cam.props.prop("cameraOptions/zoom").map(|z| z.value.as_f64()).unwrap_or_else(|| kition_motion_geom::default_camera_zoom(cw));
        cam.parent = Some(rig.id);
        // Keys live from `start`; the camera takes over the view there.
        cam.in_point = start.max(cam.in_point);
        rig.in_point = cam.in_point;
        let to_rig = |w: [f64; 3]| KV::Vec3([w[0] - point[0] + anchor[0], w[1] - point[1] + anchor[1], w[2] + anchor[2]]);
        let eye = |v: &View| {
            let c = [point[0] + (cw / 2.0 - point[0]) * v.f, point[1] + (ch / 2.0 - point[1]) * v.f];
            (to_rig([c[0], c[1], -zoom / v.s.max(0.01)]), to_rig([c[0], c[1], 0.0]))
        };
        let times: Vec<Tick> = plan.iter().map(|(f, ..)| cam.layer_time(start + pe::frames(&comp, *f))).collect();
        let curves: Vec<Curve> = plan.iter().skip(1).map(|k| k.2).collect();
        let pos: Vec<(Tick, KV)> = plan.iter().zip(&times).map(|((_, v, _), t)| (*t, eye(v).0)).collect();
        let poi: Vec<(Tick, KV)> = plan.iter().zip(&times).map(|((_, v, _), t)| (*t, eye(v).1)).collect();
        let still = |list: &[(Tick, KV)]| list.windows(2).all(|w| w[0].1 == w[1].1);
        for (path, list) in [("transform/position", pos), ("transform/poi", poi)] {
            let pr = prop(&mut cam, path)?;
            if still(&list) {
                pr.value = list.first().map(|k| k.1.clone()).unwrap_or(pr.value.clone());
            } else {
                pe::set_keys(pr, &list, &curves);
            }
        }
        for (path, get) in [("transform/rotationX", 0usize), ("transform/rotationY", 1)] {
            let list: Vec<(Tick, KV)> = plan.iter().zip(&times).map(|((_, v, _), t)| (*t, KV::Scalar(if get == 0 { v.rx } else { v.ry }))).collect();
            let pr = prop(&mut rig, path)?;
            if still(&list) {
                pr.value = list.first().map(|k| k.1.clone()).unwrap_or(pr.value.clone());
            } else if continuous {
                pe::set_keys(pr, &list, &[Curve::LINEAR]);
                pe::monotone(&mut pr.keys);
            } else {
                pe::set_keys(pr, &list, &curves);
            }
        }
        let (cam_id, rig_id) = (cam.id, rig.id);
        let c = proj.comp_mut(cid).ok_or(EngineError::NoComp)?;
        for l in c.layers.iter_mut().filter(|l| three.contains(&l.id)) {
            l.switches.three_d = true;
        }
        if mb {
            pe::motion_blur(c, &three, pe::SHUTTER);
        }
        c.layers.insert(0, rig);
        c.layers.insert(0, cam);
        st.selected_layers = vec![cam_id];
        Ok((cam_id, rig_id))
    })?;
    Ok(
        json!({"preset": preset, "camera": cam_id.0, "rig": rig_id.0, "point": point, "layers3d": three.iter().map(|l| l.0).collect::<Vec<_>>(), "end": end.seconds()}),
    )
}

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "camera.move",
        "Camera Move",
        [],
        None,
        "{comp?, preset: pushHover|craneRise|orbit|parallax, target?: json (layer id, name or #n the camera works around), point?: [x, y] (comp px, instead of target), start?: seconds (comp time; default the current time), frames?: number (pushHover total 66, craneRise 102, orbit 90, parallax 90), layers?: [integer] (made 3D, default the target; parallax: the layers far to near), from?: number (start scale: pushHover 0.8, craneRise 3), scale?: number (pushHover 2.6, craneRise 1, orbit 1), rotY?: degrees (pushHover 34), rotX?: degrees (pushHover 8, craneRise -8, orbit 0), pushFrames?: number (pushHover in and out, 18), degrees?: degrees (orbit sweep, 30), ease?: json (curve name or [x1, y1, x2, y2] for the main move), speeds?: [number] (parallax, default 0.35, 0.7, 1.4), distance?: px (parallax travel of a speed-1 layer, 600), direction?: [x, y] (parallax, default [-1, 0]), farBlur?: px (5), farDesaturate?: number (0.7), nearBlur?: px (7), name?: string, motionBlur?: boolean (moving layers get motion blur, shutter 180, 10 samples; default true)}",
        has_comp,
        camera_move
    )]
}
