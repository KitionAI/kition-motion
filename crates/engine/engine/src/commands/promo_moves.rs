//! Promo moves that carry a beat: a deterministic decaying shake on an impact (`layer.shake`), a
//! slow handheld float that holds still where asked (`layer.handheld`), a ballistic hop with a
//! squash-and-stretch landing (`layer.hop`) and a fanned spiral gather into a ring
//! (`layer.gather`).
//!
//! Shake and float are written as expressions on top of what the property already does: its
//! keys (`value`), or its existing expression, which runs first. Everything is seeded and
//! closed-form, so a frame always renders the same. Hop and gather are keyed every frame.

use kition_motion_keyframe::Value as KV;
use kition_motion_project::{Comp, Expression, ItemId, LayerId, LayerSource, Property};
use kition_motion_time::Tick;
use serde_json::{Value, json};

use super::promo_carry::{ctx_at, frame_steps, local_rect, num, point_p, pos, splice, tr3};
use super::promo_ease::{self as pe, Curve};
use super::promo_light::{js, js_str, start_p};
use super::{CommandSpec, bad, comp_id, has_comp, layer_mut, layer_p, resolve_layer};
use crate::{EngineError, Result, Session, cmd};

/// The layer a move applies to: `layer`, or `camera` (a camera layer, or `true` for the comp's
/// top camera). Returns the layer and whether it is a camera.
fn target(s: &Session, p: &Value, cmd: &str) -> Result<(ItemId, LayerId, bool)> {
    let cid = comp_id(s, p)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let is_cam = |c: &Comp, id: LayerId| c.layer(id).is_some_and(|l| matches!(l.source, LayerSource::Camera));
    let lid = match (p.get("layer"), p.get("camera")) {
        (Some(v), _) if !v.is_null() => resolve_layer(comp, v).ok_or_else(|| bad(cmd, format!("no layer {v}")))?,
        (_, Some(Value::Bool(true))) => comp
            .layers
            .iter()
            .find(|l| matches!(l.source, LayerSource::Camera) && l.switches.video)
            .map(|l| l.id)
            .ok_or_else(|| bad(cmd, "the comp has no camera"))?,
        (_, Some(v)) if !v.is_null() && !matches!(v, Value::Bool(false)) => {
            let id = resolve_layer(comp, v).ok_or_else(|| bad(cmd, format!("no layer {v}")))?;
            if !is_cam(comp, id) {
                return Err(bad(cmd, format!("layer {v} is not a camera")));
            }
            id
        }
        _ => return Err(bad(cmd, "`layer` or `camera` required")),
    };
    Ok((cid, lid, is_cam(comp, lid)))
}

/// Seeded phases in 0..2π (splitmix64).
fn phases(seed: u64, n: usize) -> Vec<f64> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x2545_F491_4F6C_DD1D);
    (0..n)
        .map(|_| {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            (z >> 11) as f64 / (1u64 << 53) as f64 * std::f64::consts::TAU
        })
        .collect()
}

/// Put an offset on top of what `pr` does: `body` (JavaScript) defines the offsets, `offs` are
/// the expressions added to dimensions 0, 1 (vectors) or to the value (numbers).
fn add_offset(pr: &mut Property, comment: &str, body: &str, offs: &[&str]) {
    let base = match &pr.expr {
        Some(e) if e.enabled && !e.text.trim().is_empty() => format!("eval({})", js_str(&e.text)),
        _ => "value".to_string(),
    };
    let scalar = offs.first().copied().unwrap_or("0");
    let vector: String = offs.iter().enumerate().map(|(i, o)| format!("  if (r.length > {i}) r[{i}] = r[{i}] + ({o});\n")).collect();
    let text = format!(
        "// {comment}\n(function () {{\n  var b = {base};\n{body}\n  if (typeof b === \"number\") return b + ({scalar});\n  var r = b.slice();\n{vector}  return r;\n}})();"
    );
    pr.expr = Some(Expression { text, enabled: true });
}

/// Apply the offsets to a layer's position and rotation, or a camera's position, point of
/// interest and Z rotation. Returns the paths changed.
fn apply(s: &mut Session, cid: ItemId, lid: LayerId, cam: bool, label: &str, comment: &str, body: &str, xy: [&str; 2], rot: &str) -> Result<Vec<String>> {
    s.edit(label, None, |proj, _| {
        let l = layer_mut(proj, cid, lid)?;
        let mut done = vec![];
        let mut paths = vec!["transform/position"];
        if cam {
            paths.push("transform/poi");
        }
        for path in paths {
            if let Some(pr) = l.props.prop_mut(path) {
                add_offset(pr, comment, body, &xy);
                done.push(path.to_string());
            }
        }
        if let Some(pr) = l.props.prop_mut("transform/rotation") {
            add_offset(pr, comment, body, &[rot]);
            done.push("transform/rotation".to_string());
        }
        if done.is_empty() {
            return Err(bad(label, "the layer has no position to move"));
        }
        Ok(done)
    })
}

fn seed_p(p: &Value, cmd: &str) -> Result<u64> {
    let v = num(p, "seed", 1.0, cmd)?;
    if !(0.0..=4_294_967_295.0).contains(&v) { Err(bad(cmd, "`seed` must be 0..4294967295")) } else { Ok(v.round() as u64) }
}

// ---------------------------------------------------------------- shake

fn shake(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "layer.shake";
    let (cid, lid, cam) = target(s, p, C)?;
    let hit = match p.get("frame") {
        None | Some(Value::Null) => s.time_of(cid).seconds(),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad(C, "`frame` must be comp seconds"))?,
    };
    let amp = pos(p, "amplitude", 14.0, C)?;
    let rot = pos(p, "rotation", 1.5, C)?;
    let decay = pos(p, "decay", 6.0, C)?;
    let freq = pos(p, "frequency", 12.0, C)?;
    let seed = seed_p(p, C)?;
    let ph = phases(seed, 9);
    let ph = |i: usize| js(ph.get(i).copied().unwrap_or(0.0));
    let body = format!(
        "  // Decaying shake from {hit} s: three sines per axis (seed {seed}), zero before the hit.\n\
  var t = time - {}, k = t < 0 ? 0 : Math.exp(-{} * t), w = 2 * Math.PI * {} * Math.max(t, 0);\n\
  var nx = 0.6 * Math.sin(w + {}) + 0.3 * Math.sin(1.71 * w + {}) + 0.1 * Math.sin(2.93 * w + {});\n\
  var ny = 0.6 * Math.sin(1.07 * w + {}) + 0.3 * Math.sin(1.83 * w + {}) + 0.1 * Math.sin(3.11 * w + {});\n\
  var dx = {} * k * nx, dy = {} * k * ny, dr = {} * k * (0.7 * Math.sin(0.93 * w + {}) + 0.3 * Math.sin(2.37 * w + {}));",
        js(hit),
        js(decay),
        js(freq),
        ph(0),
        ph(1),
        ph(2),
        ph(3),
        ph(4),
        ph(5),
        js(amp),
        js(amp),
        js(rot),
        ph(6),
        ph(7)
    );
    let done = apply(s, cid, lid, cam, "Shake", &format!("Shake (layer.shake): {amp} px, {rot}°, decay {decay}/s, {freq} Hz"), &body, ["dx", "dy"], "dr")?;
    Ok(json!({"layer": lid.0, "camera": cam, "frame": hit, "props": done}))
}

// ---------------------------------------------------------------- handheld

fn handheld(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "layer.handheld";
    let (cid, lid, cam) = target(s, p, C)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let start = num(p, "start", 0.0, C)?;
    let end = num(p, "end", comp.duration.seconds(), C)?;
    if end <= start {
        return Err(bad(C, "`end` must be after `start`"));
    }
    let amount = pos(p, "amount", 6.0, C)?;
    let rot = pos(p, "rotation", 0.4, C)?;
    let ramp = pos(p, "ramp", 0.4, C)?.max(0.001);
    let seed = seed_p(p, C)?;
    let mut holds = vec![];
    match p.get("holds") {
        None | Some(Value::Null) => {}
        Some(Value::Array(a)) => {
            for h in a {
                match h.as_array().map(|v| v.iter().map(Value::as_f64).collect::<Option<Vec<f64>>>()) {
                    Some(Some(v)) if v.len() == 2 && v.iter().all(|x| x.is_finite()) && v[1] >= v[0] => holds.push([v[0], v[1]]),
                    _ => return Err(bad(C, "`holds`: [[from, to], …] comp seconds, from ≤ to")),
                }
            }
        }
        Some(_) => return Err(bad(C, "`holds`: [[from, to], …] comp seconds")),
    }
    let ph = phases(seed, 6);
    let ph = |i: usize| js(ph.get(i).copied().unwrap_or(0.0));
    let hs: Vec<String> = holds.iter().map(|h| format!("[{}, {}]", js(h[0]), js(h[1]))).collect();
    let body = format!(
        "  // Handheld float {start}–{end} s (seed {seed}): two slow sines per axis; exactly zero inside the holds.\n\
  function ss(x) {{ x = Math.min(Math.max(x, 0), 1); return x * x * (3 - 2 * x); }}\n\
  var S = {}, E = {}, RAMP = {}, H = [{}];\n\
  var g = (time < S || time > E) ? 0 : ss((time - S) / RAMP) * ss((E - time) / RAMP);\n\
  for (var i = 0; i < H.length; i++) {{\n\
    if (time >= H[i][0] && time <= H[i][1]) g = 0;\n\
    else g = g * ss((time < H[i][0] ? H[i][0] - time : time - H[i][1]) / RAMP);\n\
  }}\n\
  var u = 2 * Math.PI * time;\n\
  var dx = {} * g * (0.65 * Math.sin(0.31 * u + {}) + 0.35 * Math.sin(0.137 * u + {}));\n\
  var dy = {} * g * (0.6 * Math.sin(0.23 * u + {}) + 0.4 * Math.sin(0.093 * u + {}));\n\
  var dr = {} * g * (0.7 * Math.sin(0.17 * u + {}) + 0.3 * Math.sin(0.071 * u + {}));",
        js(start),
        js(end),
        js(ramp),
        hs.join(", "),
        js(amount),
        ph(0),
        ph(1),
        js(amount),
        ph(2),
        ph(3),
        js(rot),
        ph(4),
        ph(5)
    );
    let done = apply(s, cid, lid, cam, "Handheld", &format!("Handheld (layer.handheld): {amount} px, {rot}°"), &body, ["dx", "dy"], "dr")?;
    Ok(json!({"layer": lid.0, "camera": cam, "start": start, "end": end, "holds": holds, "props": done}))
}

// ---------------------------------------------------------------- hop

fn hop(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "layer.hop";
    let (cid, lid) = layer_p(s, p, C)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let layer = comp.layer(lid).cloned().ok_or(EngineError::NoComp)?;
    let to = point_p(p, "to", C)?.ok_or_else(|| bad(C, "`to`: [x, y] landing position required"))?;
    let start = start_p(s, p, cid, C)?;
    let frames = pos(p, "frames", 18.0, C)?.max(1.0);
    let height = num(p, "height", 160.0, C)?;
    let squash = pos(p, "squash", 0.25, C)?.min(0.9);
    let stretch = pos(p, "stretch", squash * 0.4, C)?.min(0.9);
    let ring = pos(p, "ringFrames", 16.0, C)?.max(1.0);
    let ctx = ctx_at(s, cid, &comp, start);
    let p0 = tr3(&ctx, &layer, "position", [0.0; 3]);
    let anchor = tr3(&ctx, &layer, "anchor", [0.0; 3]);
    let sc = tr3(&ctx, &layer, "scale", [100.0; 3]);
    // The bottom of the content stays on the ground while it squashes.
    let bottom = (local_rect(&ctx, &layer)[3] - anchor[1]) * sc[1] / 100.0;
    let land = start + pe::frames(&comp, frames);
    let vel = |u: f64| (to[1] - p0[1]) - 4.0 * height * (1.0 - 2.0 * u);
    let vmax = vel(0.0).abs().max(vel(1.0).abs()).max(1e-9);
    let (mut ps, mut ss) = (vec![], vec![]);
    let key = |t: Tick, x: f64, y: f64, sx: f64, sy: f64, ps: &mut Vec<(Tick, KV)>, ss: &mut Vec<(Tick, KV)>| {
        let lt = layer.layer_time(t);
        ps.push((lt, KV::Vec3([x, y, p0[2]])));
        ss.push((lt, KV::Vec3([sc[0] * sx, sc[1] * sy, sc[2]])));
    };
    // The landing frame itself is the first frame of the squash.
    for (u, t) in frame_steps(&comp, start, frames).into_iter().filter(|(u, _)| *u < 1.0) {
        // Ballistic: x linear in time, y a parabola `height` above the straight line.
        let x = p0[0] + (to[0] - p0[0]) * u;
        let y = p0[1] + (to[1] - p0[1]) * u - 4.0 * height * u * (1.0 - u);
        // Stretch along the flight with the vertical speed, eased in over the take-off.
        let st = stretch * (vel(u).abs() / vmax) * (u * 4.0).min(1.0);
        key(t, x, y, 1.0 - st * 0.5, 1.0 + st, &mut ps, &mut ss);
    }
    // Landing: squash, then a damped ring-out (1.25 cycles, down to 3 %) over `ringFrames`.
    let dur = ring / pe::fps(&comp);
    let (k, w) = ((1.0f64 / 0.03).ln() / dur, std::f64::consts::TAU * 1.25 / dur);
    for (u, t) in frame_steps(&comp, land, ring) {
        let tau = u * dur;
        let a = if u >= 1.0 { 0.0 } else { squash * (-k * tau).exp() * (w * tau).cos() };
        let (sx, sy) = (1.0 + a, 1.0 - a);
        key(t, to[0], to[1] + bottom * (1.0 - sy), sx, sy, &mut ps, &mut ss);
    }
    let mb = pe::motion_blur_p(p);
    s.edit("Hop", None, |proj, st| {
        let l = layer_mut(proj, cid, lid)?;
        if let Some(pr) = l.props.prop_mut("transform/position") {
            splice(pr, ps);
        }
        if let Some(pr) = l.props.prop_mut("transform/scale") {
            splice(pr, ss);
        }
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &[lid], pe::SHUTTER);
        }
        st.selected_layers = vec![lid];
        Ok(())
    })?;
    let settled = land + pe::frames(&comp, ring);
    Ok(json!({"layer": lid.0, "land": land.seconds(), "end": settled.seconds()}))
}

// ---------------------------------------------------------------- gather

fn gather(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "layer.gather";
    let cid = comp_id(s, p)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
    let layers: Vec<LayerId> = match p.get("layers") {
        Some(Value::Array(a)) if !a.is_empty() => {
            a.iter().map(|v| resolve_layer(&comp, v).ok_or_else(|| bad(C, format!("no layer {v}")))).collect::<Result<_>>()?
        }
        _ => return Err(bad(C, "`layers`: [layer id, name or #n, …] (at least one)")),
    };
    let start = start_p(s, p, cid, C)?;
    let frames = pos(p, "frames", 30.0, C)?.max(1.0);
    let centre = point_p(p, "center", C)?.unwrap_or([comp.width as f64 / 2.0, comp.height as f64 / 2.0]);
    let radius = pos(p, "radius", 120.0, C)?;
    let stagger = pos(p, "stagger", 2.0, C)?;
    let arc = num(p, "arc", 90.0, C)?;
    let curve = pe::curve_p(p, "ease", Curve::new(0.22, 1.0, 0.36, 1.0), C)?;
    let n = layers.len();
    let wrap = |d: f64| {
        let r = (d + 180.0).rem_euclid(360.0) - 180.0;
        if r <= -180.0 { r + 360.0 } else { r }
    };
    let mut plans = vec![];
    for (i, lid) in layers.iter().enumerate() {
        let l = comp.layer(*lid).ok_or(EngineError::NoComp)?;
        let t0 = start + pe::frames(&comp, stagger * i as f64);
        let ctx = ctx_at(s, cid, &comp, t0);
        let p0 = tr3(&ctx, l, "position", [0.0; 3]);
        let (dx, dy) = (p0[0] - centre[0], p0[1] - centre[1]);
        let r0 = dx.hypot(dy);
        let slot = -90.0 + 360.0 * i as f64 / n as f64;
        let a0 = if r0 < 1e-6 { slot - arc } else { dy.atan2(dx).to_degrees() };
        // Every layer sweeps about `arc` degrees the same way, landing exactly on its slot.
        let sweep = arc + wrap(slot - a0 - arc);
        let keys: Vec<(Tick, KV)> = frame_steps(&comp, t0, frames)
            .into_iter()
            .map(|(u, t)| {
                let e = curve.at(u);
                let (r, a) = (r0 + (radius - r0) * e, (a0 + sweep * e).to_radians());
                (l.layer_time(t), KV::Vec3([centre[0] + r * a.cos(), centre[1] + r * a.sin(), p0[2]]))
            })
            .collect();
        plans.push((*lid, keys));
    }
    let mb = pe::motion_blur_p(p);
    s.edit("Gather", None, |proj, st| {
        for (lid, keys) in plans {
            let l = layer_mut(proj, cid, lid)?;
            if let Some(pr) = l.props.prop_mut("transform/position") {
                splice(pr, keys);
            }
        }
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &layers, pe::SHUTTER);
        }
        st.selected_layers = layers.clone();
        Ok(())
    })?;
    let end = start + pe::frames(&comp, stagger * (n.saturating_sub(1)) as f64 + frames);
    Ok(json!({"layers": layers.iter().map(|l| l.0).collect::<Vec<_>>(), "center": centre, "radius": radius, "end": end.seconds()}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "layer.shake",
            "Shake",
            [],
            None,
            "{comp?, layer?: json (layer id, name or #n; a camera layer shakes its position and point of interest), camera?: json (a camera layer, or true for the comp's top camera), frame?: seconds (comp time of the hit; zero before it; default the current time), amplitude?: px (default 14), rotation?: degrees (default 1.5), decay?: number (per second: falls off as exp(-decay t), default 6), frequency?: Hz (default 12), seed?: integer (default 1)}",
            has_comp,
            shake
        ),
        cmd!(
            "layer.handheld",
            "Handheld",
            [],
            None,
            "{comp?, layer?: json (layer id, name or #n), camera?: json (a camera layer, or true for the comp's top camera), start?: seconds (comp time, default 0), end?: seconds (default the comp's end), amount?: px (default 6), rotation?: degrees (default 0.4), ramp?: seconds (ease in and out of the float and of each hold, default 0.4), holds?: [[seconds, seconds]] (comp time ranges where the float is exactly zero), seed?: integer (default 1)}",
            has_comp,
            handheld
        ),
        cmd!(
            "layer.hop",
            "Hop",
            [],
            None,
            "{comp?, layer?: json (layer id, name or #n), to: [x, y] (landing position, in the layer's position space: comp px when unparented), start?: seconds (comp time; default the current time), frames?: number (the flight, default 18), height?: px (apex above the straight line, default 160), squash?: number (landing squash, 0.25 = 125 % wide / 75 % tall), stretch?: number (stretch along the flight, default 0.4 × squash), ringFrames?: number (damped ring-out after landing, default 16), motionBlur?: boolean (default true)}",
            has_comp,
            hop
        ),
        cmd!(
            "layer.gather",
            "Gather",
            [],
            None,
            "{comp?, layers: [json] (layer ids, names or #n; slot order around the ring from the top, clockwise), start?: seconds (comp time; default the current time), frames?: number (each layer's flight, default 30), center?: [x, y] (comp px, default the comp centre), radius?: px (default 120), stagger?: number (frames between layers, default 2), arc?: degrees (how far each spirals around, default 90; negative turns the other way), ease?: json (curve name or [x1, y1, x2, y2], default out), motionBlur?: boolean (default true)}",
            has_comp,
            gather
        ),
    ]
}
