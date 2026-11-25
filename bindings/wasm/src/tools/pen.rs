//! The Pen tool (G) and path editing on the canvas. The Pen draws a mask on the selected layer, a
//! new path into the selected shape layer, or with no layer selected a new shape layer: a click
//! places a corner, a drag pulls the point's curve handles, a click on the first point closes the
//! path, Enter keeps it open and Escape drops it. The path is made in one step (`mask.new`,
//! `shape.newPath`), so one undo takes it away.
//!
//! With the Selection tool (or the Pen) the selected layers' masks and shape paths show their
//! vertices, editable through the engine's vertex selection (`mask.selectVertices`): a click
//! selects a vertex (Shift adds or removes it), a drag moves the selected vertices, a drag over
//! empty space while vertices are selected boxes vertices in, a press on a segment selects its
//! whole path (the Pen adds a vertex there instead, `mask.insertVertex`), Alt-click turns a vertex
//! from corner to smooth and back (`mask.convertVertex`), and a selected vertex's curve handles
//! drag (`mask.setVertex`). Delete removes the selected vertices (the editor's `edit.delete`).
//! Each gesture is one undo step.

use kition_kit::tools::{ToolMods, ToolOverlay};
use kition_motion_engine::Session;
use kition_motion_engine::geom::{Mat3, vec2};
use kition_motion_engine::project::{GroupKind, LayerId, LayerSource, PropGroup, Uid, Value as KV};
use serde_json::{Value, json};

use super::geometry::{self, LayerBox, Pt};
use super::{Gesture, SELECTION, Tool, ToolState, selected_boxes, with_ctx};
use crate::MotionDocument;

/// How deep shape groups are searched for Path items.
const MAX_DEPTH: usize = 32;
/// Curve handles show for at most this many selected vertices.
const HANDLE_LIMIT: usize = 16;
/// A path vertex's colour (amber: not the selection blue of the layer's handles).
const VERTEX: [u8; 4] = [255, 170, 0, 255];
/// Points sampled per segment to find a press on it.
const SEGMENT_SAMPLES: usize = 32;
/// How far (screen pixels) a press travels before it is a drag.
const DRAG_START: f64 = 3.0;

/// How near (screen pixels) a press must be to a vertex or a handle to take it.
const REACH: f64 = 7.0;

/// The status line when a mask cannot go on the layer (the editor translates it).
pub(super) const NO_SIZE: &str = "The layer has no size here to put a mask on";
pub(super) const SCALED_TO_NOTHING: &str = "The layer is scaled to nothing: no mask can go on it";

/// A press that moves less than this (screen pixels) places a corner, not a curve.
const CURVE_START: f64 = 3.0;

/// A mask being drawn: its layer, and per point its place and outgoing handle (comp pixels; the
/// incoming handle mirrors it).
#[derive(Clone, Debug, Default)]
pub(crate) struct PenDraft {
    pub layer: Option<LayerId>,
    pub points: Vec<(Pt, Pt)>,
}

/// Which of a vertex's curve handles a drag pulls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VertexPart {
    In,
    Out,
}

/// An editable path of a selected layer, in the composition: a mask, or a shape layer's Path item.
/// `m` maps the path's own space (the layer's for a mask, the enclosing shape groups' for a shape
/// path) to the composition; vertices and tangents (as the points they reach) are in comp pixels.
pub(crate) struct EditPath {
    pub layer: LayerId,
    pub uid: Uid,
    pub m: Mat3,
    pub vertices: Vec<Pt>,
    pub ins: Vec<Pt>,
    pub outs: Vec<Pt>,
    pub closed: bool,
}

impl EditPath {
    fn of(layer: LayerId, uid: Uid, m: Mat3, path: &kition_motion_engine::project::keyframe::ShapePath) -> EditPath {
        let at = |i: usize, tangent: Option<&Vec<Pt>>| {
            let v = path.vertices.get(i).copied().unwrap_or([0.0; 2]);
            let d = tangent.and_then(|list| list.get(i)).copied().unwrap_or([0.0; 2]);
            let q = m.apply(vec2(v[0] + d[0], v[1] + d[1]));
            [q.x, q.y]
        };
        let n = path.vertices.len();
        EditPath {
            layer,
            uid,
            m,
            vertices: (0..n).map(|i| at(i, None)).collect(),
            ins: (0..n).map(|i| at(i, Some(&path.in_tangents))).collect(),
            outs: (0..n).map(|i| at(i, Some(&path.out_tangents))).collect(),
            closed: path.closed,
        }
    }

    fn holds(&self, v: &EngineVertex) -> bool {
        v.layer == self.layer && v.mask == self.uid && v.index < self.vertices.len()
    }

    /// A comp-pixel offset in the path's own space; `None` when the path is scaled to nothing.
    fn offset_in(&self, d: Pt) -> Option<Pt> {
        let q = self.m.inverse()?.apply_vec(vec2(d[0], d[1]));
        (q.x.is_finite() && q.y.is_finite()).then_some([q.x, q.y])
    }

    /// A comp point in the path's own space.
    fn point_in(&self, p: Pt) -> Option<Pt> {
        let q = self.m.inverse()?.apply(vec2(p[0], p[1]));
        (q.x.is_finite() && q.y.is_finite()).then_some([q.x, q.y])
    }

    /// The segments (from vertex `i` to the next), each as its four comp-space control points.
    fn segments(&self) -> impl Iterator<Item = (usize, [Pt; 4])> + '_ {
        let n = self.vertices.len();
        let count = if self.closed { n } else { n.saturating_sub(1) };
        (0..count).filter_map(move |i| {
            let j = (i + 1) % n.max(1);
            Some((i, [*self.vertices.get(i)?, *self.outs.get(i)?, *self.ins.get(j)?, *self.vertices.get(j)?]))
        })
    }
}

type EngineVertex = kition_motion_engine::VertexRef;

fn distance(a: Pt, b: Pt) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn cubic(c: &[Pt; 4], t: f64) -> Pt {
    let u = 1.0 - t;
    let (a, b, cc, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    [a * c[0][0] + b * c[1][0] + cc * c[2][0] + d * c[3][0], a * c[0][1] + b * c[1][1] + cc * c[2][1] + d * c[3][1]]
}

/// The selected layers' masks and shape paths at the current time.
pub(crate) fn paths(session: &Session) -> Vec<EditPath> {
    // Most selections have neither: skip measuring their boxes.
    let any = session.active_comp().is_some_and(|comp| {
        comp.layers.iter().any(|l| {
            session.state.selected_layers.contains(&l.id) && (l.masks().is_some_and(|m| !m.children.is_empty()) || matches!(l.source, LayerSource::Shape))
        })
    });
    if !any {
        return Vec::new();
    }
    let boxes = selected_boxes(session);
    with_ctx(session, |ctx| {
        let mut out = Vec::new();
        for b in boxes.iter().filter(|b| b.flat) {
            let Some(layer) = ctx.comp.layer(b.id) else { continue };
            let t = layer.layer_time(ctx.time);
            if let Some(group) = layer.masks() {
                for mask in group.groups().filter(|g| matches!(g.kind, GroupKind::Mask { .. })) {
                    if let Some(KV::Path(path)) = mask.get("path").map(|p| p.value_at(t)) {
                        out.push(EditPath::of(b.id, mask.uid, b.m, &path));
                    }
                }
            }
            if matches!(layer.source, LayerSource::Shape)
                && let Some(contents) = layer.props.sub("contents")
            {
                let mut found = Vec::new();
                shape_paths(contents, 0, &mut found);
                for item in found {
                    let Some(KV::Path(path)) = item.get("path").map(|p| p.value_at(t)) else { continue };
                    let inner = kition_motion_engine::render::shapes::item_matrix(ctx, layer, item.uid).unwrap_or(Mat3::IDENTITY);
                    out.push(EditPath::of(b.id, item.uid, b.m * inner, &path));
                }
            }
        }
        out
    })
    .unwrap_or_default()
}

/// The Path items under a shape layer's contents (inside shape groups too).
fn shape_paths<'a>(g: &'a PropGroup, depth: usize, out: &mut Vec<&'a PropGroup>) {
    if depth > MAX_DEPTH {
        return;
    }
    for sub in g.groups() {
        if sub.match_id == "path" && matches!(sub.get("path").map(|p| &p.value), Some(KV::Path(_))) {
            out.push(sub);
        } else if sub.match_id == "group"
            && let Some(inner) = sub.sub("contents")
        {
            shape_paths(inner, depth + 1, out);
        }
    }
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

/// The selected vertices that are on paths shown now.
fn shown_selection(session: &Session, paths: &[EditPath]) -> Vec<EngineVertex> {
    session.state.selected_vertices.iter().filter(|v| paths.iter().any(|path| path.holds(v))).copied().collect()
}

/// The selected layers' masks and shape paths: each path, its vertices (the selected ones filled),
/// and the selected vertices' curve handles.
pub(super) fn path_overlays(session: &Session, out: &mut Vec<ToolOverlay>) {
    let paths = paths(session);
    let selected = shown_selection(session, &paths);
    let handles = selected.len() <= HANDLE_LIMIT;
    for path in &paths {
        out.push(ToolOverlay::Path { d: bezier_path(&path.vertices, &path.ins, &path.outs, path.closed), color: SELECTION, width: 1.0, dashed: false });
        for (i, v) in path.vertices.iter().enumerate() {
            let on = selected.iter().any(|s| s.layer == path.layer && s.mask == path.uid && s.index == i);
            if on && handles {
                for handle in [path.ins[i], path.outs[i]] {
                    if distance(handle, *v) > 1e-6 {
                        out.push(ToolOverlay::Line { ax: v[0], ay: v[1], bx: handle[0], by: handle[1], color: SELECTION, dashed: false });
                        out.push(ToolOverlay::Handle { x: handle[0], y: handle[1], color: SELECTION });
                    }
                }
            }
            // Smaller than the transform box's handles and in another colour, so they never read as them.
            out.push(ToolOverlay::Anchor { x: v[0], y: v[1], color: VERTEX, filled: on, size: 5.0 });
        }
    }
}

/// What a press at `p` lands on among the paths: a selected vertex's handle, a vertex, or a segment.
enum PathHit {
    Handle(EngineVertex, VertexPart),
    Vertex(EngineVertex),
    Segment { layer: LayerId, uid: Uid, segment: usize, t: f64 },
}

fn path_hit(paths: &[EditPath], selected: &[EngineVertex], p: Pt, zoom: f64) -> Option<PathHit> {
    let reach = REACH / zoom;
    if selected.len() <= HANDLE_LIMIT {
        for v in selected {
            let Some(path) = paths.iter().find(|path| path.holds(v)) else { continue };
            let (Some(at), Some(tin), Some(tout)) = (path.vertices.get(v.index), path.ins.get(v.index), path.outs.get(v.index)) else { continue };
            for (handle, part) in [(*tout, VertexPart::Out), (*tin, VertexPart::In)] {
                if distance(handle, *at) > 1e-6 && distance(handle, p) <= reach {
                    return Some(PathHit::Handle(*v, part));
                }
            }
        }
    }
    for path in paths {
        if let Some(index) = path.vertices.iter().position(|v| distance(*v, p) <= reach) {
            return Some(PathHit::Vertex(EngineVertex { layer: path.layer, mask: path.uid, index }));
        }
    }
    let mut best: Option<(f64, PathHit)> = None;
    for path in paths {
        for (segment, c) in path.segments() {
            for k in 0..=SEGMENT_SAMPLES {
                let t = k as f64 / SEGMENT_SAMPLES as f64;
                let d = distance(cubic(&c, t), p);
                if d <= reach && best.as_ref().is_none_or(|(b, _)| d < *b) {
                    best = Some((d, PathHit::Segment { layer: path.layer, uid: path.uid, segment, t }));
                }
            }
        }
    }
    best.map(|(_, hit)| hit)
}

fn vertex_json(v: &EngineVertex) -> Value {
    json!({ "layer": v.layer.0, "mask": v.mask, "index": v.index })
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

/// The layer's box (its layer ↔ comp mapping) at the current time.
fn layer_box(session: &Session, layer: LayerId) -> Option<LayerBox> {
    with_ctx(session, |ctx| ctx.comp.layer(layer).and_then(|l| geometry::layer_box(ctx, l))).flatten()
}

impl MotionDocument {
    /// A press with the Pen: the first point takes the selected layer (the mask goes on it), or,
    /// with no layer selected, draws a path for a new shape layer; a press on the first point of
    /// three or more closes the path, any other adds a point (a drag from it pulls its handles).
    pub(super) fn pen_down(&mut self, st: &mut ToolState, p: Pt) {
        let zoom = st.zoom;
        let closing =
            st.pen.as_ref().is_some_and(|draft| draft.points.len() >= 3 && draft.points.first().is_some_and(|(first, _)| distance(*first, p) * zoom <= REACH));
        if closing {
            self.pen_finish(st, true);
            return;
        }
        if st.pen.is_none() {
            let layer = self.session.state.selected_layers.first().copied();
            st.pen = Some(PenDraft { layer, points: Vec::new() });
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

    /// Makes the drawn path, in one step, and starts afresh: a mask on its layer (closed or open),
    /// a new path in it when it is a shape layer, or with no layer a new shape layer drawing it
    /// (`shape.newPath`). The engine selects the path's last vertex.
    pub(super) fn pen_finish(&mut self, st: &mut ToolState, closed: bool) {
        st.gesture = Gesture::Idle;
        let Some(draft) = st.pen.take() else { return };
        if draft.points.len() < 2 {
            return;
        }
        st.pen_drew = true;
        // On a shape layer the path is a new shape path, unless the Pen draws masks (the editor's
        // Pen option, `setPenMasks`).
        let shape_layer = draft
            .layer
            .filter(|_| !self.pen_masks)
            .filter(|id| self.session.active_comp().and_then(|c| c.layer(*id)).is_some_and(|l| matches!(l.source, LayerSource::Shape)));
        let Some(layer) = draft.layer.filter(|_| shape_layer.is_none()) else {
            let vertices: Vec<Pt> = draft.points.iter().map(|(p, _)| *p).collect();
            let outs: Vec<Pt> = draft.points.iter().map(|(_, t)| *t).collect();
            let ins: Vec<Pt> = draft.points.iter().map(|(_, t)| [-t[0], -t[1]]).collect();
            let mut params = json!({ "vertices": vertices, "inTangents": ins, "outTangents": outs, "closed": closed, "space": "comp" });
            if let Some(id) = shape_layer {
                params["layer"] = json!(id.0);
            } else if let Some(name) = self.new_shape_layer_name() {
                params["name"] = json!(name);
            }
            if let Err(error) = self.tool_run("shape.newPath", params) {
                st.status = Some(error);
            }
            return;
        };
        let Some(b) = layer_box(&self.session, layer) else {
            st.status = Some(NO_SIZE.into());
            return;
        };
        let mut vertices = Vec::new();
        let (mut ins, mut outs) = (Vec::new(), Vec::new());
        for (p, t) in &draft.points {
            let (Some(v), Some(o), Some(i)) = (b.to_layer(*p), b.to_layer([p[0] + t[0], p[1] + t[1]]), b.to_layer([p[0] - t[0], p[1] - t[1]])) else {
                st.status = Some(SCALED_TO_NOTHING.into());
                return;
            };
            vertices.push(v);
            outs.push([o[0] - v[0], o[1] - v[1]]);
            ins.push([i[0] - v[0], i[1] - v[1]]);
        }
        if let Err(error) =
            self.tool_run("mask.new", json!({ "layer": layer.0, "vertices": vertices, "inTangents": ins, "outTangents": outs, "closed": closed }))
        {
            st.status = Some(error);
        }
    }

    /// Sets the engine's vertex selection (none, one, or several).
    pub(super) fn select_vertices(&mut self, vertices: &[EngineVertex], add: bool, toggle: bool) {
        let list: Vec<Value> = vertices.iter().map(vertex_json).collect();
        let _ = self.tool_run("mask.selectVertices", json!({ "vertices": list, "add": add, "toggle": toggle }));
    }

    /// Drops the vertex selection (a press elsewhere), so Delete acts on layers again.
    pub(super) fn clear_vertices(&mut self) {
        if !self.session.state.selected_vertices.is_empty() {
            self.select_vertices(&[], false, false);
        }
    }

    /// A press on the selected layers' paths: a selected vertex's handle drags it; a vertex is
    /// selected (Shift adds or removes it; Alt turns it corner ↔ smooth) and drags with the others
    /// selected; a segment selects its whole path to drag (the Pen adds a vertex there instead);
    /// with vertices selected, a press elsewhere (Selection tool) starts a box over vertices.
    /// Answers whether the press was the paths'.
    pub(super) fn path_down(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) -> bool {
        let paths = paths(&self.session);
        if paths.is_empty() {
            return false;
        }
        let selected = shown_selection(&self.session, &paths);
        match path_hit(&paths, &selected, p, st.zoom) {
            Some(PathHit::Handle(vertex, part)) => {
                let merge = st.merge_key("path-handle");
                st.gesture = Gesture::Vertex { vertex, part, merge };
            }
            Some(PathHit::Vertex(vertex)) => {
                if mods.alt {
                    if let Err(error) = self.tool_run("mask.convertVertex", json!({ "layer": vertex.layer.0, "mask": vertex.mask, "index": vertex.index })) {
                        st.status = Some(error);
                    }
                    self.select_vertices(&[vertex], false, false);
                    st.gesture = Gesture::Idle;
                    return true;
                }
                let was = selected.contains(&vertex);
                if mods.shift {
                    self.select_vertices(&[vertex], true, true);
                } else if !was {
                    self.select_vertices(&[vertex], false, false);
                }
                let alone = (!mods.shift && was && selected.len() > 1).then_some(vertex);
                let merge = st.merge_key("path-move");
                st.gesture = Gesture::Vertices { start: p, last: p, moved: false, alone, merge };
            }
            Some(PathHit::Segment { layer, uid, segment, t }) => {
                if st.tool == Tool::Pen {
                    // The Pen adds a vertex on the segment (selected, and a drag moves it).
                    if let Err(error) = self.tool_run("mask.insertVertex", json!({ "layer": layer.0, "mask": uid, "segment": segment, "t": t })) {
                        st.status = Some(error);
                        return true;
                    }
                } else {
                    let all: Vec<EngineVertex> = paths
                        .iter()
                        .filter(|path| path.layer == layer && path.uid == uid)
                        .flat_map(|path| (0..path.vertices.len()).map(move |index| EngineVertex { layer, mask: uid, index }))
                        .collect();
                    self.select_vertices(&all, mods.shift, false);
                }
                let merge = st.merge_key("path-move");
                st.gesture = Gesture::Vertices { start: p, last: p, moved: false, alone: None, merge };
            }
            // With vertices picked or the paths being edited, a press elsewhere boxes vertices (a
            // click there ends path editing).
            None if st.tool == Tool::Selection
                && (!selected.is_empty() || st.path_edit.iter().any(|layer| self.session.state.selected_layers.contains(layer))) =>
            {
                let base = if mods.shift { selected } else { Vec::new() };
                st.gesture = Gesture::VertexBox { start: p, end: p, base };
            }
            None => return false,
        }
        true
    }

    /// A drag of a path gesture: a handle follows the pointer (its opposite mirrors it unless Alt
    /// is held), the selected vertices move by the pointer's travel, a box selects the vertices in it.
    pub(super) fn path_drag(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) {
        let zoom = st.zoom;
        match &mut st.gesture {
            Gesture::Vertex { vertex, part, merge } => {
                let (vertex, part, merge) = (*vertex, *part, merge.clone());
                let paths = paths(&self.session);
                let Some(path) = paths.iter().find(|path| path.holds(&vertex)) else { return };
                let (Some(at), Some(origin)) = (path.point_in(p), path.vertices.get(vertex.index).and_then(|v| path.point_in(*v))) else { return };
                let d = [at[0] - origin[0], at[1] - origin[1]];
                let mirror = [-d[0], -d[1]];
                let mut params = json!({ "layer": vertex.layer.0, "mask": vertex.mask, "index": vertex.index, "merge": merge });
                let (own, other) = if part == VertexPart::Out { ("out", "in") } else { ("in", "out") };
                params[own] = json!(d);
                if !mods.alt {
                    params[other] = json!(mirror);
                }
                if let Err(error) = self.tool_run("mask.setVertex", params) {
                    st.status = Some(error);
                }
            }
            Gesture::Vertices { start, last, moved, merge, .. } => {
                if !*moved && distance(*start, p) * zoom < DRAG_START {
                    return;
                }
                *moved = true;
                let mut d = [p[0] - start[0], p[1] - start[1]];
                if mods.shift {
                    d = geometry::constrain(d);
                }
                let step = [start[0] + d[0] - last[0], start[1] + d[1] - last[1]];
                *last = [start[0] + d[0], start[1] + d[1]];
                let merge = merge.clone();
                if step[0].abs() < 1e-9 && step[1].abs() < 1e-9 {
                    return;
                }
                let paths = paths(&self.session);
                let selected = shown_selection(&self.session, &paths);
                for path in &paths {
                    let on: Vec<Value> = selected.iter().filter(|v| path.holds(v)).map(vertex_json).collect();
                    if on.is_empty() {
                        continue;
                    }
                    let Some(delta) = path.offset_in(step) else { continue };
                    if let Err(error) = self.tool_run("mask.moveVertices", json!({ "vertices": on, "delta": delta, "merge": merge })) {
                        st.status = Some(error);
                    }
                }
            }
            Gesture::VertexBox { start, end, base } => {
                *end = p;
                let (x0, x1) = (start[0].min(end[0]), start[0].max(end[0]));
                let (y0, y1) = (start[1].min(end[1]), start[1].max(end[1]));
                let mut chosen = base.clone();
                for path in paths(&self.session) {
                    for (index, v) in path.vertices.iter().enumerate() {
                        let one = EngineVertex { layer: path.layer, mask: path.uid, index };
                        if v[0] >= x0 && v[0] <= x1 && v[1] >= y0 && v[1] <= y1 && !chosen.contains(&one) {
                            chosen.push(one);
                        }
                    }
                }
                if chosen != self.session.state.selected_vertices {
                    self.select_vertices(&chosen, false, false);
                }
            }
            _ => {}
        }
    }

    /// A path gesture ends: a click on one of several selected vertices selects it alone; a box
    /// that never moved was a click on empty space, which leaves path editing (and clicks there).
    pub(super) fn path_up(&mut self, st: &mut ToolState, gesture: Gesture, p: Pt) {
        match gesture {
            Gesture::Vertices { moved: false, alone: Some(vertex), .. } => self.select_vertices(&[vertex], false, false),
            Gesture::VertexBox { start, .. } if distance(start, p) * st.zoom < DRAG_START => {
                self.clear_vertices();
                st.path_edit.clear();
                let hit = with_ctx(&self.session, |ctx| geometry::pick(ctx, p, &geometry::any_layer)).flatten();
                match hit {
                    Some(id) => {
                        let _ = self.tool_run("layer.select", json!({ "layers": [id.0] }));
                    }
                    None => {
                        let _ = self.tool_run("edit.deselectAll", json!({}));
                    }
                }
            }
            _ => {}
        }
    }
}

/// The vertex of the selected layers' paths under `p` (not a handle or a segment), if any.
pub(super) fn vertex_under(session: &Session, p: Pt, zoom: f64) -> Option<EngineVertex> {
    match path_hit(&paths(session), &[], p, zoom) {
        Some(PathHit::Vertex(vertex)) => Some(vertex),
        _ => None,
    }
}

/// The selected layer whose path (a vertex or a segment) is under `p`, if any.
pub(super) fn path_layer_under(session: &Session, p: Pt, zoom: f64) -> Option<LayerId> {
    match path_hit(&paths(session), &[], p, zoom)? {
        PathHit::Vertex(vertex) | PathHit::Handle(vertex, _) => Some(vertex.layer),
        PathHit::Segment { layer, .. } => Some(layer),
    }
}

/// Whether the Pen tool is drawing.
pub(super) fn drawing(st: &ToolState) -> bool {
    st.tool == Tool::Pen && st.pen.as_ref().is_some_and(|draft| !draft.points.is_empty())
}
