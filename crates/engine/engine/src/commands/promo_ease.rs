//! Promo motion: named easing curves and springs (`keys.applyEase`), and the key helpers the
//! promo presets (text, light, camera) build their keyframes with.
//!
//! A curve is written as a CSS `cubic-bezier(x1, y1, x2, y2)` from 0 to 1 and converted exactly
//! into the usual temporal ease: the out side of the first key gets influence `x1` and the
//! speed of the handle's slope (`y1 / x1` × Δvalue / duration), the in side of the second key
//! influence `1 − x2` and speed `(1 − y2) / (1 − x2)` × Δvalue / duration. The keyframe engine
//! draws a Bezier segment in (time, value) space from exactly those handles, so the motion is the
//! CSS curve. Curves whose `y` leaves 0..1 (overshoot landings) are exact the same way on
//! ordinary properties; on a motion path (Position, effect points) the speed along the path
//! cannot turn back, so the curve is split at each turning point and a key is inserted there.
//!
//! A spring is an expression on the property that replaces the motion into its last key with a
//! damped spring (damping, stiffness, mass) starting at the key before it; the value overshoots
//! the last key and settles on it.

use kition_motion_keyframe::{Ease, Interp, Keyframe, Value as KV, spatial_segment_length};
use kition_motion_project::{Comp, Expression, LayerId, Property};
use kition_motion_time::Tick;
use serde_json::{Value, json};

use super::prop::prop_ref;
use super::{CommandSpec, bad, f_p, has_comp, layer_mut, merge_p, str_p};
use crate::{Result, Session, cmd, query};

/// A CSS-style cubic Bezier easing curve from (0, 0) to (1, 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Curve {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Curve {
    pub(crate) const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Curve {
        Curve { x1, y1, x2, y2 }
    }
    pub(crate) const LINEAR: Curve = Curve::new(0.0, 0.0, 1.0, 1.0);

    fn x(&self, u: f64) -> f64 {
        bez(self.x1, self.x2, u)
    }
    fn y(&self, u: f64) -> f64 {
        bez(self.y1, self.y2, u)
    }
    fn dx(&self, u: f64) -> f64 {
        bez_d(self.x1, self.x2, u)
    }
    fn dy(&self, u: f64) -> f64 {
        bez_d(self.y1, self.y2, u)
    }

    /// The curve's value at progress `p` (0..1 of the time): solves x(u) = p, returns y(u).
    pub(crate) fn at(&self, p: f64) -> f64 {
        if !p.is_finite() || p <= 0.0 {
            return 0.0;
        }
        if p >= 1.0 {
            return 1.0;
        }
        let mut u = p;
        for _ in 0..8 {
            let e = self.x(u) - p;
            let d = self.dx(u);
            if e.abs() < 1e-9 || d.abs() < 1e-9 {
                break;
            }
            u -= e / d;
        }
        if !(0.0..=1.0).contains(&u) || (self.x(u) - p).abs() > 1e-6 {
            let (mut lo, mut hi) = (0.0, 1.0);
            for _ in 0..50 {
                u = (lo + hi) / 2.0;
                if self.x(u) < p { lo = u } else { hi = u }
            }
        }
        self.y(u)
    }

    /// Where the value turns back outside 0..1 (overshoot / anticipation), as curve parameters.
    fn turning_points(&self) -> Vec<f64> {
        // y'(u) = 3a u² + 2b u + c with y(u) = a u³ + b u² + c u.
        let a = 1.0 - 3.0 * self.y2 + 3.0 * self.y1;
        let b = 3.0 * self.y2 - 6.0 * self.y1;
        let c = 3.0 * self.y1;
        let (qa, qb, qc) = (3.0 * a, 2.0 * b, c);
        let mut roots = vec![];
        if qa.abs() < 1e-12 {
            if qb.abs() > 1e-12 {
                roots.push(-qc / qb);
            }
        } else {
            let disc = qb * qb - 4.0 * qa * qc;
            if disc >= 0.0 {
                let r = disc.sqrt();
                roots.push((-qb - r) / (2.0 * qa));
                roots.push((-qb + r) / (2.0 * qa));
            }
        }
        let mut out: Vec<f64> =
            roots.into_iter().filter(|u| u.is_finite() && *u > 1e-4 && *u < 1.0 - 1e-4).filter(|u| !(-1e-6..=1.0 + 1e-6).contains(&self.y(*u))).collect();
        out.sort_by(f64::total_cmp);
        out.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        out
    }

    /// CSS `cubic-bezier()` as a JavaScript function body for expressions: `function NAME(p)`.
    pub(crate) fn js(&self, name: &str) -> String {
        let Curve { x1, y1, x2, y2 } = *self;
        format!(
            "function {name}(p){{if(p<=0)return 0;if(p>=1)return 1;var ax=1-3*{x2}+3*{x1},bx=3*{x2}-6*{x1},cx=3*{x1},ay=1-3*{y2}+3*{y1},by=3*{y2}-6*{y1},cy=3*{y1};\
var u=p;for(var i=0;i<8;i++){{var e=((ax*u+bx)*u+cx)*u-p;var d=(3*ax*u+2*bx)*u+cx;if(Math.abs(e)<1e-7||Math.abs(d)<1e-7)break;u=u-e/d;}}\
if(!(u>=0&&u<=1)||Math.abs(((ax*u+bx)*u+cx)*u-p)>1e-5){{var lo=0,hi=1;for(var j=0;j<40;j++){{u=(lo+hi)/2;if(((ax*u+bx)*u+cx)*u<p)lo=u;else hi=u;}}}}\
return ((ay*u+by)*u+cy)*u;}}"
        )
    }
}

fn bez(p1: f64, p2: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    3.0 * v * v * u * p1 + 3.0 * v * u * u * p2 + u * u * u
}
fn bez_d(p1: f64, p2: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    3.0 * v * v * p1 + 6.0 * v * u * (p2 - p1) + 3.0 * u * u * (1.0 - p2)
}

/// Named curves: (id, curve, when to use it).
pub(crate) const CURVES: &[(&str, Curve, &str)] = &[
    ("snappy", Curve::new(0.16, 1.0, 0.3, 1.0), "fast start, long soft landing: entrances, pushes, UI cards arriving"),
    ("out", Curve::new(0.22, 1.0, 0.36, 1.0), "ease-out for text and elements fading/sliding in (blurSlide)"),
    ("smooth", Curve::new(0.65, 0.0, 0.35, 1.0), "symmetric in-out: moves between two resting states, camera returns"),
    ("swift", Curve::new(0.4, 0.0, 0.2, 1.0), "material-style standard curve: sweeps, wipes, sheen passes"),
    ("settle", Curve::new(0.3, 0.0, 0.25, 1.15), "lands 15% past the target and settles back: drops, scale-ins"),
    ("pop", Curve::new(0.2, 1.25, 0.3, 1.0), "bursts past the target early then settles: pop-ins, badges, counters"),
    ("reseat", Curve::new(0.4, 0.0, 0.3, 1.05), "a small 5% overshoot: re-seating an element that was already in place"),
    ("fly", Curve::new(0.34, 1.4, 0.44, 1.0), "big early overshoot: elements flying in from off screen"),
    ("linear", Curve::LINEAR, "constant speed: spotlight travel, scrolling, orbits"),
];

/// Springs: (id, damping, stiffness, mass, when to use it).
pub(crate) const SPRINGS: &[(&str, f64, f64, f64, &str)] = &[
    ("crisp", 18.0, 170.0, 1.0, "one quick small bounce: buttons, chips, card landings"),
    ("lively", 12.0, 170.0, 1.0, "two or three visible bounces: playful icons, stickers"),
    ("noBounce", 26.0, 170.0, 1.0, "critically damped: fast arrival with no overshoot"),
    ("cardFlip", 15.0, 190.0, 1.0, "rotations of cards and panels flipping into place"),
];

pub(crate) fn curve(name: &str) -> Option<Curve> {
    CURVES.iter().find(|(id, ..)| id.eq_ignore_ascii_case(name)).map(|(_, c, _)| *c)
}

/// `bezier: [x1, y1, x2, y2]` with x1, x2 in 0..1.
pub(crate) fn bezier_p(p: &Value, key: &str, cmd: &str) -> Result<Option<Curve>> {
    let Some(v) = p.get(key) else { return Ok(None) };
    let a: Vec<f64> = v.as_array().map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
    match a.as_slice() {
        [x1, y1, x2, y2] if (0.0..=1.0).contains(x1) && (0.0..=1.0).contains(x2) && y1.is_finite() && y2.is_finite() => {
            Ok(Some(Curve::new(*x1, *y1, *x2, *y2)))
        }
        _ => Err(bad(cmd, format!("`{key}` must be [x1, y1, x2, y2] with x1 and x2 in 0..1"))),
    }
}

/// A curve parameter: `ease` (a name or `[x1, y1, x2, y2]`), else `default`.
pub(crate) fn curve_p(p: &Value, key: &str, default: Curve, cmd: &str) -> Result<Curve> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::String(n)) => curve(n).ok_or_else(|| bad(cmd, format!("`{key}`: no curve {n} (one of {})", names()))),
        Some(_) => bezier_p(p, key, cmd).map(|c| c.unwrap_or(default)),
    }
}

fn names() -> String {
    CURVES.iter().map(|(n, ..)| *n).collect::<Vec<_>>().join(", ")
}

fn secs(t: Tick) -> f64 {
    t.seconds()
}

fn ease_list(slope: f64, influence: f64, deltas: &[f64], dur: f64) -> Vec<Ease> {
    let inf = influence.clamp(0.001, 1.0);
    deltas.iter().map(|dv| Ease { speed: slope * dv / dur, influence: inf }).collect()
}

/// Handles of a piece of the curve between parameters `ua` and `ub`, normalised to the piece:
/// (out influence, out slope, in influence, in slope) where slopes are Δprogress / Δtime of the
/// whole curve.
fn piece(c: &Curve, ua: f64, ub: f64) -> (f64, f64, f64, f64) {
    let (xa, ya, xb, yb) = (c.x(ua), c.y(ua), c.x(ub), c.y(ub));
    let h = (ub - ua) / 3.0;
    let (c1x, c1y) = (xa + h * c.dx(ua), ya + h * c.dy(ua));
    let (c2x, c2y) = (xb - h * c.dx(ub), yb - h * c.dy(ub));
    let span = (xb - xa).max(1e-12);
    let slope = |dy: f64, dx: f64| if dx.abs() < 1e-9 { 0.0 } else { dy / dx };
    ((c1x - xa) / span, slope(c1y - ya, c1x - xa), (xb - c2x) / span, slope(yb - c2y, xb - c2x))
}

fn set_out(k: &mut Keyframe, e: Vec<Ease>) {
    k.out_interp = Interp::Bezier;
    k.out_ease = e;
    k.auto_bezier = false;
    k.continuous = false;
}
fn set_in(k: &mut Keyframe, e: Vec<Ease>) {
    k.in_interp = Interp::Bezier;
    k.in_ease = e;
    k.auto_bezier = false;
    k.continuous = false;
}

/// Give segment `i` → `i + 1` the curve `c`. Returns the number of keys inserted (overshoot
/// turning points on a motion path), 0 when the segment can't be eased (hold, text, …).
pub(crate) fn ease_segment(keys: &mut Vec<Keyframe>, i: usize, c: Curve, spatial: bool) -> usize {
    let (Some(a), Some(b)) = (keys.get(i), keys.get(i + 1)) else { return 0 };
    if !a.value.interpolates() || a.out_interp == Interp::Hold {
        return 0;
    }
    let (t0, t1) = (a.time, b.time);
    let dur = (secs(t1) - secs(t0)).max(1e-9);
    let is_spatial = spatial && matches!(a.value, KV::Vec2(_) | KV::Vec3(_));
    let deltas: Vec<f64> = if is_spatial {
        vec![spatial_segment_length(keys, i)]
    } else {
        let (ca, cb) = (a.value.components(), b.value.components());
        if ca.is_empty() || ca.len() != cb.len() { vec![1.0] } else { ca.iter().zip(&cb).map(|(x, y)| y - x).collect() }
    };
    let turns = if is_spatial { c.turning_points() } else { vec![] };
    if turns.is_empty() {
        let (oi, os, ii, is) = piece(&c, 0.0, 1.0);
        let out = ease_list(os, oi, &deltas, dur);
        let inn = ease_list(is, ii, &deltas, dur);
        set_out(&mut keys[i], out);
        set_in(&mut keys[i + 1], inn);
        return 0;
    }
    // Motion path: split at each turning point; a key there, moving along the chord.
    let (v0, v1) = (keys[i].value.clone(), keys[i + 1].value.clone());
    let (p0, p1) = (v0.components(), v1.components());
    let len = deltas.first().copied().unwrap_or(0.0).abs();
    let mut bounds = vec![0.0];
    bounds.extend(&turns);
    bounds.push(1.0);
    let mut new_keys = vec![];
    for u in &turns {
        let t = Tick::from_seconds_f64(secs(t0) + c.x(*u) * dur);
        if t <= t0 || t >= t1 || new_keys.iter().any(|k: &Keyframe| k.time >= t) {
            // Too short to split: ease without the turn.
            let (oi, os, ii, is) = piece(&c, 0.0, 1.0);
            let (out, inn) = (ease_list(os, oi, &deltas, dur), ease_list(is, ii, &deltas, dur));
            set_out(&mut keys[i], out);
            set_in(&mut keys[i + 1], inn);
            return 0;
        }
        let y = c.y(*u);
        let comps: Vec<f64> = p0.iter().zip(&p1).map(|(a, b)| a + (b - a) * y).collect();
        let mut k = Keyframe::new(t, v0.with_components(&comps));
        k.spatial_auto = false;
        new_keys.push(k);
    }
    let n = new_keys.len();
    for (j, k) in new_keys.into_iter().enumerate() {
        keys.insert(i + 1 + j, k);
    }
    for j in 0..=n {
        let (oi, os, ii, is) = piece(&c, bounds[j], bounds[j + 1]);
        // Influence is relative to the piece; speed is along the path (no sign).
        let out = vec![Ease { speed: (os * len / dur).abs(), influence: oi.clamp(0.001, 1.0) }];
        let inn = vec![Ease { speed: (is * len / dur).abs(), influence: ii.clamp(0.001, 1.0) }];
        set_out(&mut keys[i + j], out);
        set_in(&mut keys[i + j + 1], inn);
    }
    n
}

/// Continuous-velocity keys through `keys` (a 1D or per-dimension value): monotone cubic
/// (Fritsch–Carlson) speeds at every key, both sides equal, influence 1/3, so the value passes
/// through each key without stopping and never overshoots between them.
pub(crate) fn monotone(keys: &mut [Keyframe]) {
    let n = keys.len();
    if n < 2 {
        return;
    }
    let dims = keys[0].value.components().len().max(1);
    let t: Vec<f64> = keys.iter().map(|k| secs(k.time)).collect();
    let mut speeds = vec![vec![0.0; dims]; n];
    for d in 0..dims {
        let v: Vec<f64> = keys.iter().map(|k| k.value.components().get(d).copied().unwrap_or(0.0)).collect();
        let sec: Vec<f64> = (0..n - 1).map(|i| (v[i + 1] - v[i]) / (t[i + 1] - t[i]).max(1e-9)).collect();
        let mut m = vec![0.0; n];
        m[0] = sec[0];
        m[n - 1] = sec[n - 2];
        for i in 1..n - 1 {
            m[i] = if sec[i - 1] * sec[i] <= 0.0 { 0.0 } else { (sec[i - 1] + sec[i]) / 2.0 };
        }
        for i in 0..n - 1 {
            if sec[i].abs() < 1e-12 {
                m[i] = 0.0;
                m[i + 1] = 0.0;
                continue;
            }
            let (a, b) = (m[i] / sec[i], m[i + 1] / sec[i]);
            let h = a * a + b * b;
            if h > 9.0 {
                let tau = 3.0 / h.sqrt();
                m[i] = tau * a * sec[i];
                m[i + 1] = tau * b * sec[i];
            }
        }
        for i in 0..n {
            speeds[i][d] = m[i];
        }
    }
    for (i, k) in keys.iter_mut().enumerate() {
        let e: Vec<Ease> = speeds[i].iter().map(|s| Ease { speed: *s, influence: 1.0 / 3.0 }).collect();
        k.in_interp = Interp::Bezier;
        k.out_interp = Interp::Bezier;
        k.in_ease = e.clone();
        k.out_ease = e;
        k.auto_bezier = false;
        k.continuous = true;
    }
}

/// Replace a property's keys with `(time, value)` keys and ease every segment with `curves[i]`
/// (the last curve repeats). Times are layer time.
pub(crate) fn set_keys(pr: &mut Property, keys: &[(Tick, KV)], curves: &[Curve]) {
    let mut ks: Vec<Keyframe> = keys.iter().map(|(t, v)| Keyframe::new(*t, v.clone())).collect();
    ks.sort_by_key(|k| k.time);
    ks.dedup_by_key(|k| k.time);
    if pr.hold_only {
        ks = ks.into_iter().map(Keyframe::hold).collect();
    }
    if let Some(k) = ks.first() {
        pr.value = k.value.clone();
    }
    pr.keys = ks;
    if pr.hold_only {
        return;
    }
    let mut i = 0;
    let mut seg = 0;
    while i + 1 < pr.keys.len() {
        let c = curves.get(seg).or(curves.last()).copied().unwrap_or(Curve::LINEAR);
        let added = if c == Curve::LINEAR { 0 } else { ease_segment(&mut pr.keys, i, c, pr.spatial) };
        i += 1 + added;
        seg += 1;
    }
}

/// Promo presets that move fast turn on motion blur: the layers' switch, and the comp's shutter
/// (`shutter`, at most 270°; presets use 180°) at 10 samples per frame.
pub(crate) fn motion_blur(comp: &mut Comp, layers: &[LayerId], shutter: f64) {
    for l in comp.layers.iter_mut().filter(|l| layers.contains(&l.id)) {
        l.switches.motion_blur = true;
    }
    comp.enable_motion_blur = true;
    comp.shutter_angle = shutter.clamp(0.0, 270.0);
    comp.motion_blur_samples = 10;
}

/// `motionBlur` (default on).
pub(crate) fn motion_blur_p(p: &Value) -> bool {
    p.get("motionBlur").and_then(Value::as_bool).unwrap_or(true)
}

/// The shutter promo presets use.
pub(crate) const SHUTTER: f64 = 180.0;

/// Frames → ticks at the comp's rate.
pub(crate) fn frames(comp: &Comp, f: f64) -> Tick {
    Tick::from_seconds_f64(f * comp.frame_rate.den.max(1) as f64 / comp.frame_rate.num.max(1) as f64)
}

/// Frames per second of a comp.
pub(crate) fn fps(comp: &Comp) -> f64 {
    comp.frame_rate.num.max(1) as f64 / comp.frame_rate.den.max(1) as f64
}

/// The spring expression: the motion into the last key follows a damped spring that starts at the
/// key before it (value and time), overshoots the last key's value and settles on it.
pub(crate) fn spring_expr(damping: f64, stiffness: f64, mass: f64) -> String {
    format!(
        "// Promo spring (damping {damping}, stiffness {stiffness}, mass {mass}): the motion into the last key.\n\
var n = numKeys;\n\
if (n < 2) {{ value; }} else {{\n\
  var a = key(n - 1), b = key(n), t = time - a.time;\n\
  if (t <= 0) {{ value; }} else {{\n\
    var m = {mass}, k = {stiffness}, c = {damping};\n\
    var w0 = Math.sqrt(k / m), z = c / (2 * Math.sqrt(k * m)), p;\n\
    if (z < 1) {{ var wd = w0 * Math.sqrt(1 - z * z); p = 1 - Math.exp(-z * w0 * t) * (Math.cos(wd * t) + (z * w0 / wd) * Math.sin(wd * t)); }}\n\
    else if (z == 1) {{ p = 1 - Math.exp(-w0 * t) * (1 + w0 * t); }}\n\
    else {{ var s = Math.sqrt(z * z - 1), r1 = -w0 * (z - s), r2 = -w0 * (z + s); p = 1 + (r2 * Math.exp(r1 * t) - r1 * Math.exp(r2 * t)) / (r1 - r2); }}\n\
    a.value + (b.value - a.value) * p;\n\
  }}\n\
}}"
    )
}

/// What `preset` names: a curve or a spring.
enum Shape {
    Curve(String, Curve),
    Spring(String, f64, f64, f64),
}

fn shape_p(p: &Value) -> Result<Shape> {
    const C: &str = "keys.applyEase";
    if let Some(c) = bezier_p(p, "bezier", C)? {
        return Ok(Shape::Curve(format!("cubic-bezier({}, {}, {}, {})", c.x1, c.y1, c.x2, c.y2), c));
    }
    let name = str_p(p, "preset").ok_or_else(|| bad(C, "`preset` (a curve or spring name) or `bezier` required"))?;
    if let Some(c) = curve(name) {
        return Ok(Shape::Curve(name.to_string(), c));
    }
    if name.eq_ignore_ascii_case("spring") || SPRINGS.iter().any(|(id, ..)| id.eq_ignore_ascii_case(name)) {
        let base = SPRINGS.iter().find(|(id, ..)| id.eq_ignore_ascii_case(name)).copied().unwrap_or(SPRINGS[0]);
        let pos = |k: &str, d: f64| f_p(p, k).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(d);
        return Ok(Shape::Spring(name.to_string(), pos("damping", base.1), pos("stiffness", base.2), pos("mass", base.3)));
    }
    let springs: Vec<&str> = SPRINGS.iter().map(|(n, ..)| *n).collect();
    Err(bad(C, format!("no ease preset {name}; curves: {}; springs: {}, spring", names(), springs.join(", "))))
}

fn apply_ease(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "keys.applyEase";
    let (cid, lid, uid) = prop_ref(s, p, C)?;
    let shape = shape_p(p)?;
    // `from` and `to` snap to the comp's frame grid, as prop.addKey snaps the keys' times.
    let (from, to) = {
        let comp = s.project.comp(cid).ok_or(crate::EngineError::NoComp)?;
        let layer = comp.layer(lid).ok_or(crate::EngineError::NoComp)?;
        let base = super::prop::KeyTimeBase::from_params(p, C)?;
        (super::prop::key_time(layer, comp.frame_rate, p, "from", base, C, true)?, super::prop::key_time(layer, comp.frame_rate, p, "to", base, C, true)?)
    };
    let label = match &shape {
        Shape::Curve(n, _) | Shape::Spring(n, ..) => format!("Apply Ease {n}"),
    };
    s.edit(&label, merge_p(p), |proj, st| {
        let l = layer_mut(proj, cid, lid)?;
        let pr = l.props.find_mut(uid).ok_or_else(|| bad(C, "no property"))?;
        if pr.hold_only || !pr.value.interpolates() {
            return Err(bad(C, format!("{} only holds its keys; it can't be eased", pr.name)));
        }
        if pr.keys.len() < 2 {
            return Err(bad(C, format!("{} needs at least two keyframes (prop.addKey)", pr.name)));
        }
        match &shape {
            Shape::Curve(name, c) => {
                let inside = |t: Tick| from.is_none_or(|f| t >= f) && to.is_none_or(|e| t <= e);
                // Segments whose both keys are in range, last first (inserted keys shift indices).
                let segs: Vec<usize> = (0..pr.keys.len() - 1).filter(|i| inside(pr.keys[*i].time) && inside(pr.keys[*i + 1].time)).collect();
                if segs.is_empty() {
                    return Err(bad(C, "no two consecutive keyframes between `from` and `to`"));
                }
                let mut inserted = 0;
                for i in segs.iter().rev() {
                    inserted += ease_segment(&mut pr.keys, *i, *c, pr.spatial);
                }
                st.selected_keys.clear();
                Ok(json!({"preset": name, "segments": segs.len(), "insertedKeys": inserted, "keys": pr.keys.len()}))
            }
            Shape::Spring(name, d, k, m) => {
                pr.expr = Some(Expression { text: spring_expr(*d, *k, *m), enabled: true });
                Ok(json!({"preset": name, "expression": true, "damping": d, "stiffness": k, "mass": m}))
            }
        }
    })
}

fn list(_: &mut Session, _: &Value) -> Result<Value> {
    let curves: Vec<Value> = CURVES.iter().map(|(id, c, use_)| json!({"id": id, "kind": "curve", "bezier": [c.x1, c.y1, c.x2, c.y2], "use": use_})).collect();
    let springs: Vec<Value> =
        SPRINGS.iter().map(|(id, d, k, m, use_)| json!({"id": id, "kind": "spring", "damping": d, "stiffness": k, "mass": m, "use": use_})).collect();
    Ok(json!({"curves": curves, "springs": springs}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "keys.applyEase",
            "Apply Ease Preset",
            [],
            None,
            "{layer?, path?: string, prop?: integer, preset?: snappy|out|smooth|swift|settle|pop|reseat|fly|linear|crisp|lively|noBounce|cardFlip|spring, bezier?: [x1, y1, x2, y2], from?: seconds (layer time), to?: seconds (layer time), damping?: number, stiffness?: number, mass?: number, merge?: string}",
            has_comp,
            apply_ease
        ),
        query!("keys.easePresets", "Ease Presets", "{}", list),
    ]
}
