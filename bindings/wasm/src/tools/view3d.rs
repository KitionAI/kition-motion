//! What the viewer draws over a 3D view other than the active camera's (Front, Left, Top, a
//! Custom View), so the person sees where they are: a ground grid under the composition, the
//! outline of every 3D layer (a layer seen edge-on still shows as a line), and each camera and
//! light as an icon with the direction it points in (a camera's frustum, a light's line to its
//! point of interest).

use kition_kit::tools::ToolOverlay;
use kition_motion_engine::Session;
use kition_motion_engine::geom::{Vec3, vec3};
use kition_motion_engine::project::{Layer, LightKind};
use kition_motion_engine::render::EvalCtx;
use kition_motion_engine::render::three_d::CameraState;

use super::geometry::{self, Pt};
/// The active composition at the current time, without changing the view the boxes are seen through.
fn with_ctx_plain<R>(session: &Session, f: impl FnOnce(&EvalCtx) -> R) -> Option<R> {
    let cid = session.active_comp_id()?;
    let comp = session.project.comp(cid)?;
    let ctx = EvalCtx { footage: Some(session.footage.as_ref()), expr: session.expr.as_deref(), ..EvalCtx::new(&session.project, cid, comp, session.time()) };
    Some(f(&ctx))
}

/// The ground grid's and the 3D layers' outlines' colour (grey, half seen).
pub(super) const GRID: [u8; 4] = [150, 150, 150, 140];
/// The outline of a 3D layer that is not selected.
pub(super) const OUTLINE: [u8; 4] = [190, 190, 190, 200];
/// Cameras and lights.
pub(super) const CAMERA: [u8; 4] = [255, 196, 64, 255];
pub(super) const LIGHT: [u8; 4] = [255, 240, 120, 255];
/// Grid lines each way.
const GRID_LINES: i32 = 10;

/// A world point on the canvas (comp pixels) through `view`, or None behind it.
fn on_canvas(view: &CameraState, w: f64, h: f64, p: Vec3) -> Option<Pt> {
    view.project(w, h, p).map(|v| [v.x, v.y]).filter(|p| p[0].is_finite() && p[1].is_finite())
}

fn line(view: &CameraState, w: f64, h: f64, a: Vec3, b: Vec3, color: [u8; 4], dashed: bool, out: &mut Vec<ToolOverlay>) {
    if let (Some(a), Some(b)) = (on_canvas(view, w, h, a), on_canvas(view, w, h, b)) {
        out.push(ToolOverlay::Line { ax: a[0], ay: a[1], bx: b[0], by: b[1], color, dashed });
    }
}

/// A layer's position in the world and its point of interest (cameras and lights).
fn place(ctx: &EvalCtx, layer: &Layer) -> (Vec3, Option<Vec3>) {
    let eye = ctx.world_matrix(layer).apply(vec3(0.0, 0.0, 0.0));
    let poi = layer.transform().and_then(|tr| tr.get("poi").map(|_| Vec3::from(ctx.v3(layer, tr, "poi", [0.0; 3]))));
    (eye, poi)
}

/// The overlays of the 3D view the viewer shows; nothing for the active camera's.
pub(super) fn overlays(session: &Session, out: &mut Vec<ToolOverlay>) {
    let Some(cid) = session.active_comp_id() else { return };
    let Some(view) = session.view_camera(cid) else { return };
    overlays_through(session, view, out);
}

/// The ground, the 3D layers' outlines, cameras and lights as `view` sees them (the second view
/// of the two-view layout draws them too).
pub(crate) fn overlays_through(session: &Session, view: CameraState, out: &mut Vec<ToolOverlay>) {
    super::geometry::through_view(Some(view), || overlays_seen(session, view, out));
}

fn overlays_seen(session: &Session, view: CameraState, out: &mut Vec<ToolOverlay>) {
    with_ctx_plain(session, |ctx| {
        let (w, h) = (f64::from(ctx.comp.width), f64::from(ctx.comp.height));
        // The ground: a grid on the plane through the composition's bottom edge, as wide as the
        // composition and as deep, centred under it.
        let size = w.max(h);
        let (cx, cz) = (w * 0.5, 0.0);
        for i in -GRID_LINES / 2..=GRID_LINES / 2 {
            let f = f64::from(i) / f64::from(GRID_LINES) * size;
            line(&view, w, h, vec3(cx + f, h, cz - size * 0.5), vec3(cx + f, h, cz + size * 0.5), GRID, false, out);
            line(&view, w, h, vec3(cx - size * 0.5, h, cz + f), vec3(cx + size * 0.5, h, cz + f), GRID, false, out);
        }
        for layer in ctx.comp.layers.iter().filter(|l| l.is_active_at(ctx.time) && l.switches.video) {
            if layer.is_camera() {
                let (eye, poi) = place(ctx, layer);
                camera_icon(&view, w, h, eye, poi.unwrap_or(eye + vec3(0.0, 0.0, 1000.0)), out);
            } else if layer.is_light() {
                let (at, poi) = place(ctx, layer);
                light_icon(
                    &view,
                    w,
                    h,
                    at,
                    poi.filter(|_| matches!(layer.source, kition_motion_engine::project::LayerSource::Light { kind: LightKind::Spot | LightKind::Parallel })),
                    out,
                );
            } else if layer.is_3d()
                && let Some(b) = geometry::layer_box(ctx, layer)
            {
                // Every 3D layer's outline: one seen edge-on (a layer facing the front, from the
                // top) is a line, never nothing.
                let c = b.corners();
                for k in 0..4 {
                    let (a, z) = (c[k], c[(k + 1) % 4]);
                    out.push(ToolOverlay::Line { ax: a[0], ay: a[1], bx: z[0], by: z[1], color: OUTLINE, dashed: true });
                }
            }
        }
    });
}

/// Every 3D layer (not a camera or light) as `view` sees it, for the second view to fill and name:
/// `{id, name, label: [r, g, b], selected, quad: [[x, y] × 4]}` in composition pixels, from the
/// bottom of the stack up. A layer seen edge-on has a flat quad (a line).
pub(crate) fn layers_through(session: &Session, view: CameraState) -> Vec<serde_json::Value> {
    super::geometry::through_view(Some(view), || {
        with_ctx_plain(session, |ctx| {
            ctx.comp
                .layers
                .iter()
                .rev()
                .filter(|l| l.is_active_at(ctx.time) && l.switches.video && l.is_3d() && !l.is_camera() && !l.is_light())
                .filter_map(|layer| {
                    let quad = geometry::layer_box(ctx, layer)?.corners();
                    quad.iter().all(|p| p[0].is_finite() && p[1].is_finite()).then(|| {
                        serde_json::json!({
                            "id": layer.id.0,
                            "name": layer.name,
                            "label": layer.label.rgb(),
                            "selected": session.state.selected_layers.contains(&layer.id),
                            "quad": quad,
                        })
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
    })
}

/// A camera: a small box at its eye and its frustum towards the point of interest.
fn camera_icon(view: &CameraState, w: f64, h: f64, eye: Vec3, poi: Vec3, out: &mut Vec<ToolOverlay>) {
    let ahead = poi - eye;
    let length = ahead.length();
    if length.is_nan() || length <= 1e-6 {
        return;
    }
    let forward = ahead / length;
    let mut right = forward.cross(vec3(0.0, -1.0, 0.0));
    if right.length() < 1e-6 {
        right = vec3(1.0, 0.0, 0.0);
    }
    let right = right.normalize();
    let up = right.cross(forward).normalize();
    // The frustum: to a rectangle the composition's shape, a third of the way to the point of interest.
    let d = (length / 3.0).clamp(60.0, 600.0);
    let (hw, hh) = (d * 0.5, d * 0.5 * h / w.max(1.0));
    let centre = eye + forward * d;
    let corners = [centre + right * hw + up * hh, centre - right * hw + up * hh, centre - right * hw - up * hh, centre + right * hw - up * hh];
    for k in 0..4 {
        line(view, w, h, eye, corners[k], CAMERA, false, out);
        line(view, w, h, corners[k], corners[(k + 1) % 4], CAMERA, false, out);
    }
    // Its body: a box around the eye.
    let s = 24.0;
    let body = [eye + right * s + up * s, eye - right * s + up * s, eye - right * s - up * s, eye + right * s - up * s];
    for k in 0..4 {
        line(view, w, h, body[k], body[(k + 1) % 4], CAMERA, false, out);
    }
    line(view, w, h, eye, poi, CAMERA, true, out);
}

/// A light: a star of short lines at its place and, for a spot or parallel light, a line to its
/// point of interest.
fn light_icon(view: &CameraState, w: f64, h: f64, at: Vec3, poi: Option<Vec3>, out: &mut Vec<ToolOverlay>) {
    let s = 20.0;
    for d in [vec3(s, 0.0, 0.0), vec3(0.0, s, 0.0), vec3(0.0, 0.0, s), vec3(s, s, 0.0) * 0.7, vec3(s, -s, 0.0) * 0.7] {
        line(view, w, h, at - d, at + d, LIGHT, false, out);
    }
    if let Some(poi) = poi {
        line(view, w, h, at, poi, LIGHT, true, out);
    }
}
