//! The 3D handles of the Selection tool, on the first selected 3D layer, camera or light, in
//! every 3D view (the active camera's too): an arrow along each world axis (X red, Y green, Z
//! blue) moves along it, a small square between two arrows moves in their plane, and a ring about
//! each of the layer's own rotation axes turns it (X Rotation, Y Rotation, Z Rotation). A drag
//! moves or turns every selected 3D layer, camera and light by the same amount, as one undo step
//! (`prop.set` with the gesture's `merge` key); Shift keeps a plane move to its main axis, an
//! arrow to steps of 10 pixels and a turn to steps of 15°.

use kition_kit::tools::{ToolMods, ToolOverlay};
use kition_motion_engine::Session;
use kition_motion_engine::geom::{Mat4, Vec3, vec3};
use kition_motion_engine::project::{Layer, LayerId};
use kition_motion_engine::render::EvalCtx;
use kition_motion_engine::render::three_d::camera::{layer_rotation, parent_world, rig_rotation};
use kition_motion_engine::render::three_d::{CameraState, active_camera};
use serde_json::json;

use super::geometry::Pt;
use super::{Gesture, ToolState, with_ctx};
use crate::MotionDocument;

/// The axes' colours: X, Y, Z.
pub(super) const AXIS: [[u8; 4]; 3] = [[235, 84, 84, 255], [88, 196, 96, 255], [84, 140, 255, 255]];
/// A plane handle's fill (its two axes' colours are on its sides).
const PLANE_FILL: [[u8; 4]; 3] = [[84, 140, 255, 70], [235, 84, 84, 70], [88, 196, 96, 70]];
/// An arrow's length and a ring's radius on screen (pixels, whatever the zoom).
const ARROW_PX: f64 = 90.0;
const RING_PX: f64 = 66.0;
/// A plane handle spans this part of the arrows, from the origin.
const PLANE_FROM: f64 = 0.22;
const PLANE_TO: f64 = 0.42;
/// How near (screen pixels) a press must be to an arrow or a ring.
const REACH_PX: f64 = 7.0;
/// Points on a ring.
const RING_STEPS: usize = 48;
/// An arrow shorter than this on screen (pixels) points at the viewer: it is not drawn or taken.
const SHORTEST_PX: f64 = 6.0;

/// A part of the handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// Moving along world axis 0 (X), 1 (Y) or 2 (Z).
    Axis(usize),
    /// Moving in the plane of two world axes.
    Plane(usize, usize),
    /// Turning about the layer's X, Y or Z rotation axis.
    Ring(usize),
}

/// The handles as the view shows them now.
struct Handles {
    origin: Vec3,
    origin_s: Pt,
    /// An arrow's world length (its screen length is `ARROW_PX`).
    length: f64,
    /// The rotation axes (unit, world): about X, Y and Z Rotation.
    rings: [Vec3; 3],
    cam: CameraState,
    w: f64,
    h: f64,
}

const WORLD: [Vec3; 3] = [Vec3 { x: 1.0, y: 0.0, z: 0.0 }, Vec3 { x: 0.0, y: 1.0, z: 0.0 }, Vec3 { x: 0.0, y: 0.0, z: 1.0 }];

impl Handles {
    fn project(&self, p: Vec3) -> Option<Pt> {
        self.cam.project(self.w, self.h, p).map(|v| [v.x, v.y]).filter(|p| p[0].is_finite() && p[1].is_finite())
    }

    /// An arrow's tip on screen, if it shows (not pointing at the viewer).
    fn tip(&self, axis: usize, zoom: f64) -> Option<Pt> {
        let dir = WORLD.get(axis)?;
        let tip = self.project(self.origin + *dir * self.length)?;
        (dist(tip, self.origin_s) * zoom >= SHORTEST_PX).then_some(tip)
    }

    fn plane(&self, a: usize, b: usize) -> Option<[Pt; 4]> {
        let (u, v) = (*WORLD.get(a)? * self.length, *WORLD.get(b)? * self.length);
        let at = |s: f64, t: f64| self.project(self.origin + u * s + v * t);
        Some([at(PLANE_FROM, PLANE_FROM)?, at(PLANE_TO, PLANE_FROM)?, at(PLANE_TO, PLANE_TO)?, at(PLANE_FROM, PLANE_TO)?])
    }

    /// Two unit vectors spanning the plane of ring `axis`, so that a turn from the first towards
    /// the second is a positive turn about the axis.
    fn ring_basis(&self, axis: usize) -> Option<(Vec3, Vec3)> {
        let n = *self.rings.get(axis)?;
        let helper = if n.x.abs() < 0.9 { vec3(1.0, 0.0, 0.0) } else { vec3(0.0, 1.0, 0.0) };
        let a = n.cross(helper);
        if a.length() < 1e-9 {
            return None;
        }
        let a = a.normalize();
        Some((a, n.cross(a)))
    }

    /// The ring's world point at angle `phi` (radians).
    fn ring_point(&self, axis: usize, phi: f64) -> Option<Vec3> {
        let (a, b) = self.ring_basis(axis)?;
        let r = self.length * RING_PX / ARROW_PX;
        Some(self.origin + a * (r * phi.cos()) + b * (r * phi.sin()))
    }

    /// The ring on screen: (angle, point) every step.
    fn ring(&self, axis: usize) -> Vec<(f64, Pt)> {
        (0..=RING_STEPS)
            .filter_map(|k| {
                let phi = std::f64::consts::TAU * k as f64 / RING_STEPS as f64;
                self.ring_point(axis, phi).and_then(|p| self.project(p)).map(|p| (phi, p))
            })
            .collect()
    }
}

fn dist(a: Pt, b: Pt) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The distance from `p` to the segment `a`–`b`.
fn to_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len2 = d[0] * d[0] + d[1] * d[1];
    let t = if len2 > 1e-12 { (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / len2).clamp(0.0, 1.0) } else { 0.0 };
    dist(p, [a[0] + d[0] * t, a[1] + d[1] * t])
}

/// Whether `p` is inside the convex quad `q`.
fn in_quad(p: Pt, q: &[Pt; 4]) -> bool {
    let side = |a: Pt, b: Pt| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let s: Vec<f64> = (0..4).map(|k| side(q[k], q[(k + 1) % 4])).collect();
    s.iter().all(|v| *v >= 0.0) || s.iter().all(|v| *v <= 0.0)
}

/// Whether the canvas tools may move this layer in 3D.
fn movable(ctx: &EvalCtx, layer: &Layer) -> bool {
    layer.is_3d() && layer.is_active_at(ctx.time) && !layer.switches.locked && layer.transform().is_some()
}

/// The selected layers the handles move: 3D layers, cameras and lights, topmost first.
fn targets(session: &Session) -> Vec<LayerId> {
    let selected = &session.state.selected_layers;
    with_ctx(session, |ctx| ctx.comp.layers.iter().filter(|l| selected.contains(&l.id) && movable(ctx, l)).map(|l| l.id).collect()).unwrap_or_default()
}

/// A layer's rotation axes in the world: X Rotation's, Y Rotation's and Z Rotation's (each turn
/// is applied inside the ones after it: Orientation, then Z, Y and X).
fn rotation_axes(ctx: &EvalCtx, layer: &Layer) -> [Vec3; 3] {
    let Some(tr) = layer.transform() else { return WORLD };
    let o = Vec3::from(ctx.v3(layer, tr, "orientation", [0.0; 3]));
    let rz = ctx.f(layer, tr, "rotation", 0.0);
    let ry = ctx.f(layer, tr, "rotationY", 0.0);
    // A camera's or light's point of interest turns it first.
    let aim = if layer.is_camera() || layer.is_light() {
        layer_rotation(ctx, layer).inverse().map(|inverse| rig_rotation(ctx, layer) * inverse).unwrap_or(Mat4::IDENTITY)
    } else {
        Mat4::IDENTITY
    };
    let outer = parent_world(ctx, layer) * aim * Mat4::orientation(o);
    let unit = |v: Vec3| if v.length() > 1e-9 { v.normalize() } else { v };
    [
        unit((outer * Mat4::rotate_z(rz) * Mat4::rotate_y(ry)).apply_vec(WORLD[0])),
        unit((outer * Mat4::rotate_z(rz)).apply_vec(WORLD[1])),
        unit(outer.apply_vec(WORLD[2])),
    ]
}

/// The handles of the first selected 3D layer, camera or light, through the view shown.
fn handles(session: &Session, zoom: f64) -> Option<Handles> {
    let first = *targets(session).first()?;
    let cid = session.active_comp_id()?;
    let view = session.view_camera(cid);
    with_ctx(session, |ctx| {
        let layer = ctx.comp.layer(first)?;
        let cam = view.unwrap_or_else(|| active_camera(ctx));
        let anchor = if layer.is_camera() || layer.is_light() {
            [0.0; 3]
        } else {
            layer.transform().map(|tr| ctx.v3(layer, tr, "anchor", [0.0; 3])).unwrap_or([0.0; 3])
        };
        let origin = ctx.world_matrix(layer).apply(Vec3::from(anchor));
        let (w, h) = (f64::from(ctx.comp.width), f64::from(ctx.comp.height));
        // Comp pixels per world unit at the origin's depth.
        let per_unit = if cam.ortho {
            cam.zoom
        } else {
            let depth = cam.depth(origin);
            if depth <= 1e-6 {
                return None;
            }
            cam.zoom / depth
        };
        if !(per_unit.is_finite() && per_unit > 1e-9) {
            return None;
        }
        let length = ARROW_PX / zoom / per_unit;
        let origin_s = cam.project(w, h, origin).map(|v| [v.x, v.y]).filter(|p| p[0].is_finite() && p[1].is_finite())?;
        Some(Handles { origin, origin_s, length, rings: rotation_axes(ctx, layer), cam, w, h })
    })
    .flatten()
}

/// The part of the handles under `p` (comp pixels), if any: the arrows first, then the plane
/// squares, then the rings.
fn part_at(g: &Handles, p: Pt, zoom: f64) -> Option<Part> {
    let reach = REACH_PX / zoom;
    let mut best: Option<(f64, Part)> = None;
    for axis in 0..3 {
        let Some(tip) = g.tip(axis, zoom) else { continue };
        // Not the arrow's first part: a press on the origin is the layer's (a plain move).
        let from = [g.origin_s[0] + (tip[0] - g.origin_s[0]) * 0.2, g.origin_s[1] + (tip[1] - g.origin_s[1]) * 0.2];
        let d = to_segment(p, from, tip).min(dist(p, tip) * 0.8);
        if d <= reach && best.is_none_or(|(b, _)| d < b) {
            best = Some((d, Part::Axis(axis)));
        }
    }
    if let Some((_, part)) = best {
        return Some(part);
    }
    for (a, b) in [(0, 1), (1, 2), (0, 2)] {
        if let Some(q) = g.plane(a, b)
            && in_quad(p, &q)
        {
            return Some(Part::Plane(a, b));
        }
    }
    for axis in 0..3 {
        let ring = g.ring(axis);
        let d = ring.windows(2).map(|s| to_segment(p, s[0].1, s[1].1)).fold(f64::INFINITY, f64::min);
        if d <= reach && best.is_none_or(|(b, _)| d < b) {
            best = Some((d, Part::Ring(axis)));
        }
    }
    best.map(|(_, part)| part)
}

/// A drag of the handles: what it began with.
#[derive(Clone, Debug)]
pub(crate) struct Drag {
    part: Part,
    start: Pt,
    /// For a move: each arrow's screen vector for one `length` (comp pixels).
    screen: [Pt; 3],
    length: f64,
    /// For a turn: the ring's screen tangent at the press (comp pixels per radian).
    tangent: Pt,
    /// Each layer's position (its parent's space) and the matrix from a world move to its parent's.
    layers: Vec<(LayerId, [f64; 3], Mat4, bool)>,
    /// Each layer's rotation about the ring's axis when the drag began.
    turns: Vec<(LayerId, f64)>,
    merge: String,
}

/// The property a ring turns.
fn ring_path(axis: usize) -> &'static str {
    match axis {
        0 => "transform/rotationX",
        1 => "transform/rotationY",
        _ => "transform/rotation",
    }
}

impl MotionDocument {
    /// A press with the Selection tool on the 3D handles: begins their drag.
    pub(super) fn gizmo_down(&mut self, st: &mut ToolState, p: Pt) -> bool {
        let zoom = st.zoom;
        let Some(g) = handles(&self.session, zoom) else { return false };
        let Some(part) = part_at(&g, p, zoom) else { return false };
        let screen =
            [0, 1, 2].map(|axis| g.project(g.origin + WORLD[axis] * g.length).map_or([0.0, 0.0], |tip| [tip[0] - g.origin_s[0], tip[1] - g.origin_s[1]]));
        // A turn: the ring's direction on screen where it was pressed.
        let tangent = match part {
            Part::Ring(axis) => {
                let ring = g.ring(axis);
                let nearest = ring.iter().min_by(|a, b| dist(a.1, p).total_cmp(&dist(b.1, p))).map_or(0.0, |(phi, _)| *phi);
                let step = 0.02;
                match (g.ring_point(axis, nearest - step).and_then(|q| g.project(q)), g.ring_point(axis, nearest + step).and_then(|q| g.project(q))) {
                    (Some(a), Some(b)) => [(b[0] - a[0]) / (2.0 * step), (b[1] - a[1]) / (2.0 * step)],
                    _ => [0.0, 0.0],
                }
            }
            _ => [0.0, 0.0],
        };
        let ids = targets(&self.session);
        let layers = with_ctx(&self.session, |ctx| {
            ids.iter()
                .filter_map(|id| {
                    let layer = ctx.comp.layer(*id)?;
                    let tr = layer.transform()?;
                    let to_parent = parent_world(ctx, layer).inverse().unwrap_or(Mat4::IDENTITY);
                    Some((*id, ctx.position(layer, tr), to_parent, tr.get("positionX").is_some()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        let turns = match part {
            Part::Ring(axis) => with_ctx(&self.session, |ctx| {
                ids.iter()
                    .filter_map(|id| {
                        let layer = ctx.comp.layer(*id)?;
                        let name = ring_path(axis).trim_start_matches("transform/");
                        Some((*id, ctx.f(layer, layer.transform()?, name, 0.0)))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
            _ => Vec::new(),
        };
        let merge = st.merge_key("gizmo");
        st.gesture = Gesture::Gizmo(Box::new(Drag { part, start: p, screen, length: g.length, tangent, layers, turns, merge }));
        st.cursor = if matches!(part, Part::Ring(_)) { "grabbing" } else { "move" }.into();
        true
    }

    /// The handles dragged to `p`: the layers move or turn from where they began.
    pub(super) fn gizmo_drag(&mut self, drag: &Drag, p: Pt, mods: ToolMods) {
        let d = [p[0] - drag.start[0], p[1] - drag.start[1]];
        let time = self.now();
        match drag.part {
            Part::Ring(axis) => {
                let t = drag.tangent;
                let t2 = t[0] * t[0] + t[1] * t[1];
                if t2 < 1e-9 {
                    return;
                }
                let mut degrees = ((d[0] * t[0] + d[1] * t[1]) / t2).to_degrees();
                if mods.shift {
                    degrees = (degrees / 15.0).round() * 15.0;
                }
                if !degrees.is_finite() {
                    return;
                }
                for (id, start) in &drag.turns {
                    let _ = self.tool_run(
                        "prop.set",
                        json!({ "layer": id.0, "path": ring_path(axis), "value": start + degrees, "time": time, "timeBase": "comp", "merge": drag.merge }),
                    );
                }
            }
            Part::Axis(_) | Part::Plane(..) => {
                let Some(world) = world_move(drag, d, mods.shift) else { return };
                for (id, start, to_parent, separate) in &drag.layers {
                    let delta = to_parent.apply_vec(world);
                    let value = [start[0] + delta.x, start[1] + delta.y, start[2] + delta.z];
                    if !value.iter().all(|v| v.is_finite()) {
                        continue;
                    }
                    if *separate {
                        for (k, name) in ["positionX", "positionY", "positionZ"].iter().enumerate() {
                            let _ = self.tool_run(
                                "prop.set",
                                json!({ "layer": id.0, "path": format!("transform/{name}"), "value": value[k], "time": time, "timeBase": "comp", "merge": drag.merge }),
                            );
                        }
                    } else {
                        let _ = self.tool_run(
                            "prop.set",
                            json!({ "layer": id.0, "path": "transform/position", "value": value, "time": time, "timeBase": "comp", "merge": drag.merge }),
                        );
                    }
                }
            }
        }
    }

    /// The undo key of a handles drag (Escape undoes it).
    pub(super) fn gizmo_merge(drag: &Drag) -> &str {
        drag.merge.as_str()
    }
}

/// The world move a screen move `d` makes along the dragged arrow or in the dragged plane.
fn world_move(drag: &Drag, d: Pt, shift: bool) -> Option<Vec3> {
    match drag.part {
        Part::Axis(axis) => {
            let s = drag.screen.get(axis)?;
            let s2 = s[0] * s[0] + s[1] * s[1];
            if s2 < 1e-9 {
                return None;
            }
            let mut t = (d[0] * s[0] + d[1] * s[1]) / s2 * drag.length;
            if shift {
                t = (t / 10.0).round() * 10.0;
            }
            Some(*WORLD.get(axis)? * t)
        }
        Part::Plane(a, b) => {
            let (u, v) = (drag.screen.get(a)?, drag.screen.get(b)?);
            let det = u[0] * v[1] - u[1] * v[0];
            if det.abs() < 1e-9 {
                return None;
            }
            let mut su = (d[0] * v[1] - d[1] * v[0]) / det * drag.length;
            let mut sv = (u[0] * d[1] - u[1] * d[0]) / det * drag.length;
            // Shift: along the plane's main direction of the drag only.
            if shift {
                if su.abs() >= sv.abs() {
                    sv = 0.0;
                } else {
                    su = 0.0;
                }
            }
            Some(*WORLD.get(a)? * su + *WORLD.get(b)? * sv)
        }
        Part::Ring(_) => None,
    }
}

/// The handles drawn over the first selected 3D layer, camera or light.
pub(super) fn overlays(session: &Session, zoom: f64, out: &mut Vec<ToolOverlay>) {
    let Some(g) = handles(session, zoom) else { return };
    for axis in 0..3 {
        let ring = g.ring(axis);
        if ring.len() > 2 {
            let mut d = String::new();
            for (k, (_, p)) in ring.iter().enumerate() {
                d.push_str(&format!("{}{} {} ", if k == 0 { "M" } else { "L" }, p[0], p[1]));
            }
            out.push(ToolOverlay::Path { d, color: AXIS[axis], width: 1.5, dashed: false });
        }
    }
    for (k, (a, b)) in [(0, 1), (1, 2), (0, 2)].into_iter().enumerate() {
        if let Some(q) = g.plane(a, b) {
            out.push(ToolOverlay::Highlight { quad: q, color: PLANE_FILL[k] });
        }
    }
    for axis in 0..3 {
        if let Some(tip) = g.tip(axis, zoom) {
            out.push(ToolOverlay::Line { ax: g.origin_s[0], ay: g.origin_s[1], bx: tip[0], by: tip[1], color: AXIS[axis], dashed: false });
            out.push(ToolOverlay::Handle { x: tip[0], y: tip[1], color: AXIS[axis] });
        }
    }
}

/// The cursor over the handles, if `p` is on them.
pub(super) fn hover(session: &Session, p: Pt, zoom: f64) -> Option<&'static str> {
    let g = handles(session, zoom)?;
    part_at(&g, p, zoom).map(|part| if matches!(part, Part::Ring(_)) { "grab" } else { "move" })
}

/// Where a part of the handles is on the canvas (comp pixels) for the selection now: an arrow's
/// tip, a square's middle, a ring's point farthest from the arrows (for the browser tests, which
/// press there, and the unit tests).
pub(crate) fn spot(session: &Session, part: Part, zoom: f64) -> Option<Pt> {
    let g = handles(session, zoom)?;
    match part {
        Part::Axis(axis) => g.tip(axis, zoom),
        Part::Plane(a, b) => g.plane(a, b).map(|q| [(q[0][0] + q[2][0]) / 2.0, (q[0][1] + q[2][1]) / 2.0]),
        // Its point farthest from the arrows (where a press is the ring's).
        Part::Ring(axis) => {
            let tips: Vec<Pt> = (0..3).filter_map(|k| g.tip(k, zoom)).collect();
            let clear = |p: Pt| tips.iter().map(|tip| to_segment(p, g.origin_s, *tip)).fold(f64::INFINITY, f64::min);
            g.ring(axis).into_iter().max_by(|a, b| clear(a.1).total_cmp(&clear(b.1))).map(|(_, p)| p)
        }
    }
}

/// Every part's [`spot`] as JSON (`x`, `y`, `z`, `xy`, `yz`, `xz`, `rx`, `ry`, `rz`; a part not
/// shown is left out), with the handles' origin as `origin`; `{}` without handles.
pub(crate) fn spots(session: &Session, zoom: f64) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    let Some(g) = handles(session, zoom) else { return serde_json::Value::Object(out) };
    out.insert("origin".into(), json!(g.origin_s));
    let parts = [
        ("x", Part::Axis(0)),
        ("y", Part::Axis(1)),
        ("z", Part::Axis(2)),
        ("xy", Part::Plane(0, 1)),
        ("yz", Part::Plane(1, 2)),
        ("xz", Part::Plane(0, 2)),
        ("rx", Part::Ring(0)),
        ("ry", Part::Ring(1)),
        ("rz", Part::Ring(2)),
    ];
    for (name, part) in parts {
        if let Some(p) = spot(session, part, zoom) {
            out.insert(name.into(), json!(p));
        }
    }
    serde_json::Value::Object(out)
}
