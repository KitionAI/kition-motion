//! The canvas tools' geometry: where each layer's content lies in the composition (its
//! transformed content box), which layer a point is on, the selection box's handles, and the
//! maths of moving, scaling and rotating a layer through its parent chain. Comp pixels throughout
//! unless a name says otherwise; nothing here changes the project.

use kition_motion_engine::geom::{Mat3, vec2};
use kition_motion_engine::project::{Comp, Layer, LayerId};
use kition_motion_engine::render::EvalCtx;

/// A point in comp (or layer) pixels.
pub(crate) type Pt = [f64; 2];

/// One layer's content as the canvas shows it: its content box in layer space and the matrix
/// from layer space to the composition.
#[derive(Clone, Debug)]
pub(crate) struct LayerBox {
    pub id: LayerId,
    /// Layer space `[x0, y0, x1, y1]`.
    pub bounds: [f64; 4],
    /// Layer space → comp pixels.
    pub m: Mat3,
    /// The anchor point in layer space.
    pub anchor: Pt,
    /// A 2D layer: the handles scale and rotate it (a 3D layer's box is a projection).
    pub flat: bool,
}

impl LayerBox {
    /// A layer-space point in the composition.
    pub fn to_comp(&self, p: Pt) -> Pt {
        let q = self.m.apply(vec2(p[0], p[1]));
        [q.x, q.y]
    }

    /// A comp point in layer space; `None` when the layer is scaled to nothing.
    pub fn to_layer(&self, p: Pt) -> Option<Pt> {
        let q = self.m.inverse()?.apply(vec2(p[0], p[1]));
        (q.x.is_finite() && q.y.is_finite()).then_some([q.x, q.y])
    }

    /// The box's corners in the composition: top left, top right, bottom right, bottom left.
    pub fn corners(&self) -> [Pt; 4] {
        let [x0, y0, x1, y1] = self.bounds;
        [self.to_comp([x0, y0]), self.to_comp([x1, y0]), self.to_comp([x1, y1]), self.to_comp([x0, y1])]
    }

    /// Whether a comp point is on the box (its edge counts).
    pub fn contains(&self, p: Pt) -> bool {
        let Some([x, y]) = self.to_layer(p) else { return false };
        let [x0, y0, x1, y1] = self.bounds;
        let slack = 1e-9;
        x >= x0 - slack && x <= x1 + slack && y >= y0 - slack && y <= y1 + slack
    }

    /// The anchor point in the composition.
    pub fn anchor_comp(&self) -> Pt {
        self.to_comp(self.anchor)
    }

    /// The box's axis-aligned extent in the composition `[x0, y0, x1, y1]`.
    #[cfg(test)]
    pub fn extent(&self) -> [f64; 4] {
        let c = self.corners();
        let xs = c.iter().map(|p| p[0]);
        let ys = c.iter().map(|p| p[1]);
        [
            xs.clone().fold(f64::INFINITY, f64::min),
            ys.clone().fold(f64::INFINITY, f64::min),
            xs.fold(f64::NEG_INFINITY, f64::max),
            ys.fold(f64::NEG_INFINITY, f64::max),
        ]
    }
}

/// A layer's box at the context time, when it has content to show.
pub(crate) fn layer_box(ctx: &EvalCtx, layer: &Layer) -> Option<LayerBox> {
    let view = VIEW.with(std::cell::Cell::get);
    // A 3D view other than the active camera's shows 3D layers only (as the viewer draws it).
    if view.is_some() && !layer.is_3d() {
        return None;
    }
    let bounds = kition_motion_engine::render::content_bounds(ctx, layer)?;
    if !bounds.iter().all(|v| v.is_finite()) || bounds[2] < bounds[0] || bounds[3] < bounds[1] {
        return None;
    }
    let m = match view {
        Some(camera) => (camera.projection(f64::from(ctx.comp.width), f64::from(ctx.comp.height)) * ctx.world_matrix(layer)).plane_to_mat3(),
        None => ctx.layer_to_comp(layer).0,
    };
    let anchor = layer.transform().map(|tr| ctx.v3(layer, tr, "anchor", [0.0; 3])).unwrap_or([0.0; 3]);
    Some(LayerBox { id: layer.id, bounds, m, anchor: [anchor[0], anchor[1]], flat: !layer.is_3d() })
}

thread_local! {
    /// The 3D view the boxes are seen through while [`through_view`] runs (None: the active camera).
    static VIEW: std::cell::Cell<Option<kition_motion_engine::render::three_d::CameraState>> = const { std::cell::Cell::new(None) };
}

/// Runs `f` with the layers' boxes seen through `view` (the viewer's 3D view; None for the
/// active camera's), then puts back the view there was.
pub(crate) fn through_view<R>(view: Option<kition_motion_engine::render::three_d::CameraState>, f: impl FnOnce() -> R) -> R {
    let before = VIEW.with(|cell| cell.replace(view));
    let out = f();
    VIEW.with(|cell| cell.set(before));
    out
}

/// Whether the canvas shows a layer now and lets it be picked: active at the time, its video
/// on (and soloed while any layer is), not locked, not a hidden shy layer, and something that
/// draws (cameras and lights have no box).
pub(crate) fn pickable(comp: &Comp, layer: &Layer, time: kition_motion_engine::time::Tick) -> bool {
    let any_solo = comp.layers.iter().any(|l| l.switches.solo && l.source.is_av() && l.is_active_at(time));
    layer.is_active_at(time)
        && layer.switches.video
        && (!any_solo || layer.switches.solo)
        && !layer.switches.locked
        && !(comp.hide_shy && layer.switches.shy)
        && !layer.is_camera()
        && !layer.is_light()
}

/// Every layer ([`pick`]'s `only` without a narrowing).
pub(crate) fn any_layer(_: &Layer) -> bool {
    true
}

/// The topmost pickable layer whose content box holds `p` (comp pixels); `only` narrows the
/// layers looked at (text layers for the Type tool).
pub(crate) fn pick(ctx: &EvalCtx, p: Pt, only: &dyn Fn(&Layer) -> bool) -> Option<LayerId> {
    ctx.comp.layers.iter().filter(|l| pickable(ctx.comp, l, ctx.time) && only(l)).find(|l| layer_box(ctx, l).is_some_and(|b| b.contains(p))).map(|l| l.id)
}

/// The pickable layers whose boxes meet the rectangle `r` (comp pixels, any corner order).
pub(crate) fn in_rect(ctx: &EvalCtx, r: [f64; 4]) -> Vec<LayerId> {
    let (x0, x1) = (r[0].min(r[2]), r[0].max(r[2]));
    let (y0, y1) = (r[1].min(r[3]), r[1].max(r[3]));
    ctx.comp
        .layers
        .iter()
        .filter(|l| pickable(ctx.comp, l, ctx.time))
        .filter_map(|l| layer_box(ctx, l))
        .filter(|b| quad_meets_rect(&b.corners(), [x0, y0, x1, y1]))
        .map(|b| b.id)
        .collect()
}

/// Whether a convex quad and an axis-aligned rectangle overlap (separating axes).
pub(crate) fn quad_meets_rect(quad: &[Pt; 4], r: [f64; 4]) -> bool {
    let rect = [[r[0], r[1]], [r[2], r[1]], [r[2], r[3]], [r[0], r[3]]];
    let mut axes: Vec<Pt> = vec![[1.0, 0.0], [0.0, 1.0]];
    for i in 0..4 {
        let (a, b) = (quad[i], quad[(i + 1) % 4]);
        axes.push([b[1] - a[1], a[0] - b[0]]);
    }
    axes.iter().filter(|a| a[0].abs() + a[1].abs() > 1e-12).all(|axis| {
        let project = |points: &[Pt; 4]| {
            let dots = points.iter().map(|p| p[0] * axis[0] + p[1] * axis[1]);
            (dots.clone().fold(f64::INFINITY, f64::min), dots.fold(f64::NEG_INFINITY, f64::max))
        };
        let (a0, a1) = project(quad);
        let (b0, b1) = project(&rect);
        a0 <= b1 && b0 <= a1
    })
}

/// A handle of the selection box, by where it sits on the box in layer space: `fx`, `fy` are 0
/// (left / top), 0.5 (middle) or 1 (right / bottom). Corners have neither at 0.5.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Handle {
    pub fx: f64,
    pub fy: f64,
}

/// The eight handles: corners first, then the sides' middles.
pub(crate) const HANDLES: [Handle; 8] = [
    Handle { fx: 0.0, fy: 0.0 },
    Handle { fx: 1.0, fy: 0.0 },
    Handle { fx: 1.0, fy: 1.0 },
    Handle { fx: 0.0, fy: 1.0 },
    Handle { fx: 0.5, fy: 0.0 },
    Handle { fx: 1.0, fy: 0.5 },
    Handle { fx: 0.5, fy: 1.0 },
    Handle { fx: 0.0, fy: 0.5 },
];

impl Handle {
    pub fn is_corner(self) -> bool {
        self.fx != 0.5 && self.fy != 0.5
    }

    /// Where the handle is in layer space on `bounds`.
    pub fn at(self, bounds: [f64; 4]) -> Pt {
        [bounds[0] + (bounds[2] - bounds[0]) * self.fx, bounds[1] + (bounds[3] - bounds[1]) * self.fy]
    }

    /// The point that stays put while it is dragged: the opposite handle, or the box's centre.
    pub fn fixed(self, bounds: [f64; 4], about_centre: bool) -> Pt {
        let opposite = if about_centre { Handle { fx: 0.5, fy: 0.5 } } else { Handle { fx: 1.0 - self.fx, fy: 1.0 - self.fy } };
        opposite.at(bounds)
    }
}

/// How far (screen pixels) from a handle a press still takes it.
pub(crate) const HANDLE_REACH: f64 = 6.0;
/// How far (screen pixels) outside a corner a press rotates instead.
pub(crate) const ROTATE_REACH: f64 = 22.0;
/// How far (screen pixels) the rotation knob stands off the box's top edge.
pub(crate) const KNOB_OFFSET: f64 = 24.0;

fn distance(a: Pt, b: Pt) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The handle of `b` at comp point `p`, with `zoom` screen pixels per comp pixel.
pub(crate) fn handle_at(b: &LayerBox, p: Pt, zoom: f64) -> Option<Handle> {
    let reach = HANDLE_REACH / zoom;
    HANDLES.iter().map(|h| (*h, distance(b.to_comp(h.at(b.bounds)), p))).filter(|(_, d)| *d <= reach).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(h, _)| h)
}

/// The rotation knob: a short stalk off the middle of the box's top edge, away from its centre.
pub(crate) fn knob(b: &LayerBox, zoom: f64) -> (Pt, Pt) {
    let [x0, y0, x1, y1] = b.bounds;
    let top = b.to_comp([(x0 + x1) / 2.0, y0]);
    let centre = b.to_comp([(x0 + x1) / 2.0, (y0 + y1) / 2.0]);
    let (dx, dy) = (top[0] - centre[0], top[1] - centre[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = if len > 1e-9 { (dx / len, dy / len) } else { (0.0, -1.0) };
    let off = KNOB_OFFSET / zoom;
    (top, [top[0] + ux * off, top[1] + uy * off])
}

/// Whether a press at `p` rotates `b`: on the knob, or just outside a corner of the box.
pub(crate) fn rotates_at(b: &LayerBox, p: Pt, zoom: f64) -> bool {
    if !b.flat {
        return false;
    }
    if distance(knob(b, zoom).1, p) <= HANDLE_REACH / zoom {
        return true;
    }
    if b.contains(p) {
        return false;
    }
    b.corners().iter().any(|c| distance(*c, p) <= ROTATE_REACH / zoom)
}

/// The scale factors a handle drag gives: the pointer `p` (layer space at the drag's start)
/// against the handle `h` and the fixed point `f`. A side handle scales one axis; a corner both,
/// by the same factor when `uniform` (the projection of the pointer on the handle's diagonal).
pub(crate) fn scale_factors(h: Handle, handle: Pt, fixed: Pt, p: Pt, uniform: bool) -> [f64; 2] {
    let axis = |i: usize| {
        let span = handle[i] - fixed[i];
        if span.abs() < 1e-9 { 1.0 } else { (p[i] - fixed[i]) / span }
    };
    let (mut sx, mut sy) = (if h.fx == 0.5 { 1.0 } else { axis(0) }, if h.fy == 0.5 { 1.0 } else { axis(1) });
    if h.is_corner() && uniform {
        let d = [handle[0] - fixed[0], handle[1] - fixed[1]];
        let len2 = d[0] * d[0] + d[1] * d[1];
        let s = if len2 < 1e-12 { 1.0 } else { ((p[0] - fixed[0]) * d[0] + (p[1] - fixed[1]) * d[1]) / len2 };
        (sx, sy) = (s, s);
    }
    [finite_or(sx, 1.0), finite_or(sy, 1.0)]
}

fn finite_or(v: f64, d: f64) -> f64 {
    if v.is_finite() { v } else { d }
}

/// How far (comp pixels) a layer's position moves so that `fixed` (layer space) stays where it
/// is when its scale is multiplied by `s` about its anchor `anchor`: `m` is the layer → comp
/// matrix before, `parent` the parent's layer → comp matrix (identity without a parent). The
/// answer is in the parent's space, as Position is.
pub(crate) fn scale_shift(m: &Mat3, parent: &Mat3, anchor: Pt, fixed: Pt, s: [f64; 2]) -> Option<Pt> {
    let moved = [anchor[0] + (fixed[0] - anchor[0]) * s[0], anchor[1] + (fixed[1] - anchor[1]) * s[1]];
    let before = m.apply(vec2(fixed[0], fixed[1]));
    let after = m.apply(vec2(moved[0], moved[1]));
    let d = parent.inverse()?.apply_vec(before - after);
    (d.x.is_finite() && d.y.is_finite()).then_some([d.x, d.y])
}

/// The parent's layer → comp matrix of `layer` (identity without one, or with a 3D one).
pub(crate) fn parent_matrix(ctx: &EvalCtx, layer: &Layer) -> Mat3 {
    match layer.parent.and_then(|id| ctx.comp.layer(id)) {
        Some(parent) => {
            let m = ctx.layer_to_comp(parent).0;
            if m.is_affine() { m } else { Mat3::IDENTITY }
        }
        None => Mat3::IDENTITY,
    }
}

/// The angle (degrees, clockwise on screen) of `p` about `centre`.
pub(crate) fn angle_about(centre: Pt, p: Pt) -> f64 {
    (p[1] - centre[1]).atan2(p[0] - centre[0]).to_degrees()
}

/// `next` unwrapped to within half a turn of `previous`, so a drag round past 180° keeps turning.
pub(crate) fn unwrap_angle(previous: f64, next: f64) -> f64 {
    let mut d = (next - previous) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    previous + d
}

/// `degrees` to the nearest multiple of `step`.
pub(crate) fn snap_angle(degrees: f64, step: f64) -> f64 {
    if step <= 0.0 || !degrees.is_finite() { degrees } else { (degrees / step).round() * step }
}

/// A drag offset held to its longer axis (Shift).
pub(crate) fn constrain(d: Pt) -> Pt {
    if d[0].abs() >= d[1].abs() { [d[0], 0.0] } else { [0.0, d[1]] }
}

/// A drawn shape's box from where the drag began and where it is: `[x0, y0, x1, y1]`, square
/// with `square` (the longer side, away from the start), from the centre with `centred`.
pub(crate) fn drawn_box(from: Pt, to: Pt, square: bool, centred: bool) -> [f64; 4] {
    let (mut dx, mut dy) = (to[0] - from[0], to[1] - from[1]);
    if square {
        let side = dx.abs().max(dy.abs());
        dx = side * if dx < 0.0 { -1.0 } else { 1.0 };
        dy = side * if dy < 0.0 { -1.0 } else { 1.0 };
    }
    if centred {
        [from[0] - dx.abs(), from[1] - dy.abs(), from[0] + dx.abs(), from[1] + dy.abs()]
    } else {
        [from[0].min(from[0] + dx), from[1].min(from[1] + dy), from[0].max(from[0] + dx), from[1].max(from[1] + dy)]
    }
}

/// The CSS resize cursor for a handle whose direction from the box's centre is `degrees`.
pub(crate) fn resize_cursor(degrees: f64) -> &'static str {
    if !degrees.is_finite() {
        return "default";
    }
    let a = degrees.rem_euclid(180.0);
    if !(22.5..157.5).contains(&a) {
        "ew-resize"
    } else if a < 67.5 {
        "nwse-resize"
    } else if a < 112.5 {
        "ns-resize"
    } else {
        "nesw-resize"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(m: Mat3) -> LayerBox {
        LayerBox { id: LayerId(1), bounds: [0.0, 0.0, 100.0, 50.0], m, anchor: [50.0, 25.0], flat: true }
    }

    #[test]
    fn a_rotated_scaled_box_holds_the_points_its_content_covers() {
        // Anchor at the box's centre, placed at (200, 100), scaled 200 %, turned 90°.
        let b = boxed(Mat3::layer_2d(vec2(50.0, 25.0), vec2(200.0, 100.0), vec2(200.0, 200.0), 90.0));
        assert!(b.contains([200.0, 100.0]), "the centre");
        // Turned a quarter, the 200-wide box stands 200 tall and 100 wide.
        assert!(b.contains([200.0, 195.0]) && b.contains([155.0, 100.0]));
        assert!(!b.contains([260.0, 100.0]) && !b.contains([200.0, 210.0]));
        let e = b.extent();
        assert!((e[0] - 150.0).abs() < 1e-9 && (e[1] - 0.0).abs() < 1e-9 && (e[2] - 250.0).abs() < 1e-9 && (e[3] - 200.0).abs() < 1e-9, "{e:?}");
    }

    #[test]
    fn a_box_scaled_to_nothing_holds_nothing() {
        let b = boxed(Mat3::layer_2d(vec2(0.0, 0.0), vec2(10.0, 10.0), vec2(0.0, 0.0), 0.0));
        assert!(!b.contains([10.0, 10.0]));
        assert!(b.to_layer([10.0, 10.0]).is_none());
    }

    #[test]
    fn quads_and_rectangles_meet_only_when_they_overlap() {
        let diamond = [[50.0, 0.0], [100.0, 50.0], [50.0, 100.0], [0.0, 50.0]];
        assert!(quad_meets_rect(&diamond, [40.0, 40.0, 60.0, 60.0]));
        // Inside the diamond's bounding box but outside the diamond itself.
        assert!(!quad_meets_rect(&diamond, [0.0, 0.0, 20.0, 20.0]));
        assert!(quad_meets_rect(&diamond, [90.0, 40.0, 200.0, 60.0]));
    }

    #[test]
    fn handles_are_found_within_reach_in_screen_pixels() {
        let b = boxed(Mat3::layer_2d(vec2(0.0, 0.0), vec2(0.0, 0.0), vec2(100.0, 100.0), 0.0));
        assert_eq!(handle_at(&b, [101.0, 51.0], 1.0), Some(Handle { fx: 1.0, fy: 1.0 }));
        assert_eq!(handle_at(&b, [50.0, -2.0], 1.0), Some(Handle { fx: 0.5, fy: 0.0 }));
        assert_eq!(handle_at(&b, [110.0, 50.0], 1.0), None, "10 screen pixels off at 100 %");
        assert_eq!(handle_at(&b, [110.0, 50.0], 0.5), Some(Handle { fx: 1.0, fy: 1.0 }), "but within reach at 50 %");
        // Just outside a corner rotates; inside the box does not.
        assert!(rotates_at(&b, [112.0, 62.0], 1.0));
        assert!(!rotates_at(&b, [90.0, 40.0], 1.0));
        let (_, tip) = knob(&b, 1.0);
        assert!((tip[0] - 50.0).abs() < 1e-9 && (tip[1] + KNOB_OFFSET).abs() < 1e-9, "{tip:?}");
        assert!(rotates_at(&b, tip, 1.0));
    }

    #[test]
    fn corners_scale_both_axes_sides_one() {
        let bounds = [0.0, 0.0, 100.0, 50.0];
        let corner = Handle { fx: 1.0, fy: 1.0 };
        let (h, f) = (corner.at(bounds), corner.fixed(bounds, false));
        assert_eq!(f, [0.0, 0.0]);
        assert_eq!(scale_factors(corner, h, f, [200.0, 50.0], false), [2.0, 1.0]);
        let s = scale_factors(corner, h, f, [200.0, 100.0], true);
        assert!((s[0] - 2.0).abs() < 1e-9 && (s[1] - 2.0).abs() < 1e-9);
        let side = Handle { fx: 0.5, fy: 0.0 };
        assert_eq!(scale_factors(side, side.at(bounds), side.fixed(bounds, false), [999.0, -50.0], true), [1.0, 2.0]);
        // About the centre the factor doubles for the same pointer travel.
        let c = corner.fixed(bounds, true);
        assert_eq!(c, [50.0, 25.0]);
        assert_eq!(scale_factors(corner, h, c, [150.0, 75.0], false), [2.0, 2.0]);
    }

    #[test]
    fn scaling_keeps_the_fixed_point_through_a_rotated_parent() {
        // A parent turned 90° and scaled 50 %; the child placed in it, anchor at its centre.
        let parent = Mat3::layer_2d(vec2(0.0, 0.0), vec2(300.0, 300.0), vec2(50.0, 50.0), 90.0);
        let (anchor, position, scale, rotation) = ([50.0, 25.0], [40.0, 20.0], [100.0, 100.0], 30.0);
        let local = |position: [f64; 2], scale: [f64; 2]| {
            Mat3::layer_2d(vec2(anchor[0], anchor[1]), vec2(position[0], position[1]), vec2(scale[0], scale[1]), rotation)
        };
        let m = parent * local(position, scale);
        let fixed = [0.0, 0.0];
        let s = [1.5, 0.5];
        let d = scale_shift(&m, &parent, anchor, fixed, s).unwrap();
        let after = parent * local([position[0] + d[0], position[1] + d[1]], [scale[0] * s[0], scale[1] * s[1]]);
        let (a, b) = (m.apply(vec2(0.0, 0.0)), after.apply(vec2(0.0, 0.0)));
        assert!((a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9, "{a:?} {b:?}");
    }

    #[test]
    fn angles_unwrap_and_snap() {
        assert!((angle_about([0.0, 0.0], [0.0, 10.0]) - 90.0).abs() < 1e-9, "clockwise on screen");
        assert!((unwrap_angle(170.0, -170.0) - 190.0).abs() < 1e-9);
        assert!((unwrap_angle(-170.0, 170.0) + 190.0).abs() < 1e-9);
        assert_eq!(snap_angle(37.0, 15.0), 30.0);
        assert_eq!(snap_angle(38.0, 15.0), 45.0);
        assert!(snap_angle(f64::NAN, 15.0).is_nan());
        assert_eq!(constrain([5.0, -7.0]), [0.0, -7.0]);
        assert_eq!(constrain([8.0, -7.0]), [8.0, 0.0]);
    }

    #[test]
    fn drawn_boxes_follow_the_drag_square_and_centred() {
        assert_eq!(drawn_box([10.0, 10.0], [50.0, 30.0], false, false), [10.0, 10.0, 50.0, 30.0]);
        assert_eq!(drawn_box([50.0, 30.0], [10.0, 10.0], false, false), [10.0, 10.0, 50.0, 30.0]);
        assert_eq!(drawn_box([10.0, 10.0], [50.0, 30.0], true, false), [10.0, 10.0, 50.0, 50.0]);
        assert_eq!(drawn_box([10.0, 10.0], [-30.0, 20.0], true, false), [-30.0, 10.0, 10.0, 50.0]);
        assert_eq!(drawn_box([100.0, 100.0], [120.0, 90.0], false, true), [80.0, 90.0, 120.0, 110.0]);
    }

    #[test]
    fn resize_cursors_follow_the_handle_direction() {
        assert_eq!(resize_cursor(0.0), "ew-resize");
        assert_eq!(resize_cursor(45.0), "nwse-resize");
        assert_eq!(resize_cursor(-90.0), "ns-resize");
        assert_eq!(resize_cursor(135.0), "nesw-resize");
        assert_eq!(resize_cursor(f64::NAN), "default");
    }
}
