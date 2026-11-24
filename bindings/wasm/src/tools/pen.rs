//! The Pen tool (G) and mask editing on the canvas. The Pen draws a mask on the selected layer
//! (or the layer clicked first): a click places a corner, a drag pulls the point's curve handles,
//! a click on the first point closes the mask, Enter keeps it open and Escape drops it. The mask
//! is made in one step (`mask.new`, in the layer's space), so one undo takes it away. With the
//! Selection tool (or the Pen) the selected layers' masks show their vertices: a vertex drags to a
//! new place and, once picked, its curve handles drag too (`mask.setVertex`, one undo step a drag).

use kition_kit::tools::{ToolMods, ToolOverlay};
use kition_motion_engine::Session;
use kition_motion_engine::project::{GroupKind, LayerId, Uid, Value as KV};
use serde_json::json;

use super::geometry::{self, LayerBox, Pt};
use super::{Gesture, SELECTION, Tool, ToolState, selected_boxes, with_ctx};
use crate::MotionDocument;

/// How near (screen pixels) a press must be to a vertex or a handle to take it.
const REACH: f64 = 7.0;

/// A press that moves less than this (screen pixels) places a corner, not a curve.
const CURVE_START: f64 = 3.0;

/// A mask being drawn: its layer, and per point its place and outgoing handle (comp pixels; the
/// incoming handle mirrors it).
#[derive(Clone, Debug, Default)]
pub(crate) struct PenDraft {
    pub layer: Option<LayerId>,
    pub points: Vec<(Pt, Pt)>,
}

/// A mask vertex: its layer, its mask group and its index in the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VertexRef {
    pub layer: LayerId,
    pub mask: Uid,
    pub index: usize,
}

/// What dragging a vertex moves: the point itself or one of its curve handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VertexPart {
    Point,
    In,
    Out,
}

/// A mask of a layer, in the composition: its group, its path's vertices and tangents (comp
/// pixels; tangents as the points they reach), and whether it is closed.
struct MaskShape {
    layer: LayerId,
    uid: Uid,
    vertices: Vec<Pt>,
    ins: Vec<Pt>,
    outs: Vec<Pt>,
    closed: bool,
}

fn distance(a: Pt, b: Pt) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The selected layers' masks at the current time, in the composition.
fn masks(session: &Session) -> Vec<MaskShape> {
    // Most selections have no mask: skip measuring their boxes.
    let any = session
        .active_comp()
        .is_some_and(|comp| comp.layers.iter().any(|l| session.state.selected_layers.contains(&l.id) && l.masks().is_some_and(|m| !m.children.is_empty())));
    if !any {
        return Vec::new();
    }
    let boxes = selected_boxes(session);
    with_ctx(session, |ctx| {
        let mut out = Vec::new();
        for b in boxes.iter().filter(|b| b.flat) {
            let Some(layer) = ctx.comp.layer(b.id) else { continue };
            let Some(group) = layer.masks() else { continue };
            let t = layer.layer_time(ctx.time);
            for mask in group.groups().filter(|g| matches!(g.kind, GroupKind::Mask { .. })) {
                let Some(KV::Path(path)) = mask.get("path").map(|p| p.value_at(t)) else { continue };
                let at = |i: usize, tangent: Option<&Vec<Pt>>| {
                    let v = path.vertices.get(i).copied().unwrap_or([0.0; 2]);
                    let d = tangent.and_then(|list| list.get(i)).copied().unwrap_or([0.0; 2]);
                    b.to_comp([v[0] + d[0], v[1] + d[1]])
                };
                let n = path.vertices.len();
                out.push(MaskShape {
                    layer: b.id,
                    uid: mask.uid,
                    vertices: (0..n).map(|i| at(i, None)).collect(),
                    ins: (0..n).map(|i| at(i, Some(&path.in_tangents))).collect(),
                    outs: (0..n).map(|i| at(i, Some(&path.out_tangents))).collect(),
                    closed: path.closed,
                });
            }
        }
        out
    })
    .unwrap_or_default()
}

/// A path through points with outgoing and incoming control points, as SVG path data.
fn bezier_path(points: &[Pt], ins: &[Pt], outs: &[Pt], closed: bool) -> String {
    let Some(first) = points.first() else { return String::new() };
    let mut d = format!("M{} {}", first[0], first[1]);
    let n = points.len();
    let segments = if closed { n } else { n.saturating_sub(1) };
    for i in 0..segments {
        let j = (i + 1) % n;
        let (Some(a), Some(b), Some(p)) = (outs.get(i), ins.get(j), points.get(j)) else { break };
        d.push_str(&format!(" C{} {} {} {} {} {}", a[0], a[1], b[0], b[1], p[0], p[1]));
    }
    if closed {
        d.push_str(" Z");
    }
    d
}

/// The selected layers' masks: each path, its vertices (the picked one filled), and the picked
/// vertex's curve handles.
pub(super) fn mask_overlays(session: &Session, picked: Option<VertexRef>, out: &mut Vec<ToolOverlay>) {
    for mask in masks(session) {
        out.push(ToolOverlay::Path { d: bezier_path(&mask.vertices, &mask.ins, &mask.outs, mask.closed), color: SELECTION, width: 1.0, dashed: false });
        for (i, v) in mask.vertices.iter().enumerate() {
            let on = picked.is_some_and(|p| p.layer == mask.layer && p.mask == mask.uid && p.index == i);
            if on {
                for handle in [mask.ins[i], mask.outs[i]] {
                    if distance(handle, *v) > 1e-6 {
                        out.push(ToolOverlay::Line { ax: v[0], ay: v[1], bx: handle[0], by: handle[1], color: SELECTION, dashed: false });
                        out.push(ToolOverlay::Handle { x: handle[0], y: handle[1], color: SELECTION });
                    }
                }
            }
            out.push(ToolOverlay::Anchor { x: v[0], y: v[1], color: SELECTION, filled: on, size: 6.0 });
        }
    }
}

/// The draft's overlays: its path so far (with the segment to the pointer's last place), its
/// points (the first larger, to click to close), and the curve handles being pulled.
pub(super) fn draft_overlays(draft: &PenDraft, out: &mut Vec<ToolOverlay>) {
    let points: Vec<Pt> = draft.points.iter().map(|(p, _)| *p).collect();
    let outs: Vec<Pt> = draft.points.iter().map(|(p, t)| [p[0] + t[0], p[1] + t[1]]).collect();
    let ins: Vec<Pt> = draft.points.iter().map(|(p, t)| [p[0] - t[0], p[1] - t[1]]).collect();
    out.push(ToolOverlay::Path { d: bezier_path(&points, &ins, &outs, false), color: SELECTION, width: 1.0, dashed: false });
    for (i, (p, t)) in draft.points.iter().enumerate() {
        if t[0].abs() + t[1].abs() > 1e-6 {
            out.push(ToolOverlay::Line { ax: p[0] - t[0], ay: p[1] - t[1], bx: p[0] + t[0], by: p[1] + t[1], color: SELECTION, dashed: false });
            out.push(ToolOverlay::Handle { x: p[0] + t[0], y: p[1] + t[1], color: SELECTION });
        }
        out.push(ToolOverlay::Anchor { x: p[0], y: p[1], color: SELECTION, filled: i == 0, size: if i == 0 { 9.0 } else { 6.0 } });
    }
}

/// The vertex or handle of the selected layers' masks under `p` (handles only of the picked vertex).
pub(super) fn vertex_at(session: &Session, picked: Option<VertexRef>, p: Pt, zoom: f64) -> Option<(VertexRef, VertexPart)> {
    let reach = REACH / zoom;
    for mask in masks(session) {
        if let Some(pick) = picked.filter(|v| v.layer == mask.layer && v.mask == mask.uid) {
            let i = pick.index;
            if let (Some(v), Some(tin), Some(tout)) = (mask.vertices.get(i), mask.ins.get(i), mask.outs.get(i)) {
                for (handle, part) in [(*tout, VertexPart::Out), (*tin, VertexPart::In)] {
                    if distance(handle, *v) > 1e-6 && distance(handle, p) <= reach {
                        return Some((pick, part));
                    }
                }
            }
        }
        if let Some(index) = mask.vertices.iter().position(|v| distance(*v, p) <= reach) {
            return Some((VertexRef { layer: mask.layer, mask: mask.uid, index }, VertexPart::Point));
        }
    }
    None
}

/// The layer's box (its layer ↔ comp mapping) at the current time.
fn layer_box(session: &Session, layer: LayerId) -> Option<LayerBox> {
    with_ctx(session, |ctx| ctx.comp.layer(layer).and_then(|l| geometry::layer_box(ctx, l))).flatten()
}

impl MotionDocument {
    /// A press with the Pen: the first point picks the layer (the selected one, else the one under
    /// the pointer), a press on the first point of three or more closes the mask, any other adds a
    /// point (a drag from it pulls its handles).
    pub(super) fn pen_down(&mut self, st: &mut ToolState, p: Pt) {
        let zoom = st.zoom;
        let closing =
            st.pen.as_ref().is_some_and(|draft| draft.points.len() >= 3 && draft.points.first().is_some_and(|(first, _)| distance(*first, p) * zoom <= REACH));
        if closing {
            self.pen_finish(st, true);
            return;
        }
        let needs_layer = st.pen.as_ref().is_none_or(|draft| draft.layer.is_none());
        if needs_layer {
            let layer = self
                .session
                .state
                .selected_layers
                .first()
                .copied()
                .or_else(|| with_ctx(&self.session, |ctx| geometry::pick(ctx, p, &geometry::any_layer)).flatten());
            let Some(layer) = layer else {
                st.pen = None;
                st.status = Some("Select the layer to mask first, or click on it".into());
                return;
            };
            if !self.session.state.selected_layers.contains(&layer) {
                let _ = self.tool_run("layer.select", json!({ "layers": [layer.0] }));
            }
            st.pen = Some(PenDraft { layer: Some(layer), points: Vec::new() });
        }
        if let Some(draft) = st.pen.as_mut() {
            draft.points.push((p, [0.0, 0.0]));
        }
        st.gesture = Gesture::PenPoint { start: p };
    }

    /// A drag from the point just placed pulls its curve handles (the incoming one mirrors it).
    pub(super) fn pen_drag(st: &mut ToolState, p: Pt) {
        let Gesture::PenPoint { start } = st.gesture else { return };
        if distance(start, p) * st.zoom < CURVE_START {
            return;
        }
        if let Some((_, tangent)) = st.pen.as_mut().and_then(|draft| draft.points.last_mut()) {
            *tangent = [p[0] - start[0], p[1] - start[1]];
        }
    }

    /// Makes the drawn mask on its layer (closed or open), in one step, and starts afresh.
    pub(super) fn pen_finish(&mut self, st: &mut ToolState, closed: bool) {
        st.gesture = Gesture::Idle;
        let Some(draft) = st.pen.take() else { return };
        let Some(layer) = draft.layer else { return };
        if draft.points.len() < 2 {
            return;
        }
        let Some(b) = layer_box(&self.session, layer) else {
            st.status = Some("The layer has no size here to put a mask on".into());
            return;
        };
        let mut vertices = Vec::new();
        let (mut ins, mut outs) = (Vec::new(), Vec::new());
        for (p, t) in &draft.points {
            let (Some(v), Some(o), Some(i)) = (b.to_layer(*p), b.to_layer([p[0] + t[0], p[1] + t[1]]), b.to_layer([p[0] - t[0], p[1] - t[1]])) else {
                st.status = Some("The layer is scaled to nothing: no mask can go on it".into());
                return;
            };
            vertices.push(v);
            outs.push([o[0] - v[0], o[1] - v[1]]);
            ins.push([i[0] - v[0], i[1] - v[1]]);
        }
        match self.tool_run("mask.new", json!({ "layer": layer.0, "vertices": vertices, "inTangents": ins, "outTangents": outs, "closed": closed })) {
            Ok(answer) => {
                if let Some(uid) = answer.get("mask").and_then(serde_json::Value::as_u64) {
                    st.vertex = Some(VertexRef { layer, mask: uid, index: vertices.len().saturating_sub(1) });
                }
            }
            Err(error) => st.status = Some(error),
        }
    }

    /// A press on a mask vertex or a picked vertex's handle starts dragging it; answers whether one was there.
    pub(super) fn vertex_down(&mut self, st: &mut ToolState, p: Pt) -> bool {
        let Some((vertex, part)) = vertex_at(&self.session, st.vertex, p, st.zoom) else { return false };
        st.vertex = Some(vertex);
        let merge = st.merge_key("mask-vertex");
        st.gesture = Gesture::Vertex { vertex, part, merge };
        true
    }

    /// Drags a vertex (or its handle) to `p`, in the mask's layer space; Alt pulls one handle alone.
    pub(super) fn vertex_drag(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) {
        let Gesture::Vertex { vertex, part, merge } = &st.gesture else { return };
        let (vertex, part, merge) = (*vertex, *part, merge.clone());
        let Some(b) = layer_box(&self.session, vertex.layer) else { return };
        let Some(at) = b.to_layer(p) else { return };
        let params = match part {
            VertexPart::Point => json!({ "layer": vertex.layer.0, "mask": vertex.mask, "index": vertex.index, "point": at, "merge": merge }),
            VertexPart::In | VertexPart::Out => {
                let Some(origin) = self.vertex_point(vertex) else { return };
                let d = [at[0] - origin[0], at[1] - origin[1]];
                let mirror = [-d[0], -d[1]];
                let (tin, tout) = if part == VertexPart::Out { (mirror, d) } else { (d, mirror) };
                if mods.alt {
                    let mut one = json!({ "layer": vertex.layer.0, "mask": vertex.mask, "index": vertex.index, "merge": merge });
                    one[if part == VertexPart::Out { "out" } else { "in" }] = json!(d);
                    one
                } else {
                    json!({ "layer": vertex.layer.0, "mask": vertex.mask, "index": vertex.index, "in": tin, "out": tout, "merge": merge })
                }
            }
        };
        if let Err(error) = self.tool_run("mask.setVertex", params) {
            st.status = Some(error);
        }
    }

    /// A vertex's place in its layer's space now.
    fn vertex_point(&self, vertex: VertexRef) -> Option<Pt> {
        let comp = self.session.active_comp()?;
        let layer = comp.layer(vertex.layer)?;
        let mask = layer.props.find_group(vertex.mask)?;
        let KV::Path(path) = mask.get("path")?.value_at(layer.layer_time(self.session.time())) else { return None };
        path.vertices.get(vertex.index).copied()
    }
}

/// Whether the Pen tool is drawing.
pub(super) fn drawing(st: &ToolState) -> bool {
    st.tool == Tool::Pen && st.pen.as_ref().is_some_and(|draft| !draft.points.is_empty())
}
