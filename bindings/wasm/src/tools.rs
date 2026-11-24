//! The canvas tools behind [`KitTools`] (the SDK's engine-tools capability): Selection, Hand,
//! Zoom, Rectangle, Ellipse and Text. Pointer events arrive in comp pixels (the document is the
//! active composition's frame); the tools select layers, move, scale and rotate them, draw shape
//! layers and type text, every change through the engine's own commands (`layer.select`,
//! `layer.nudge`, `prop.set`, `shape.newShape`, `layer.newText`, `text.*`), so a gesture is one
//! undo step (its `merge` key) and an animated property gets a keyframe at the current time as
//! `prop.set` gives one. The answer carries the selection box, its handles and anchor point, the
//! marquee, a shape being drawn, snapping guides and the text caret as overlays, and the cursor.
//!
//! Hand and Zoom are the editor's own (the view is not the document's): they are listed so the
//! rail shows them, and do nothing here.
//!
//! The tools' state (the chosen tool, a gesture under way) belongs to the person's view, not to
//! the project, and lives beside the document in this thread ([`STATE`]): an editor runs one
//! document per worker.

mod camera;
mod geometry;
mod gizmo;
mod gradient;
mod pen;
mod select;
mod text;
mod view3d;

use std::cell::RefCell;

use kition_kit::KitDocument;
use kition_kit::tools::{KitTools, PointerAction, ToolInfo, ToolMods, ToolOverlay, ToolPointer, ToolRequest, ToolResponse};
use kition_motion_engine::Session;
use kition_motion_engine::project::LayerId;
use kition_motion_engine::render::EvalCtx;
use kition_motion_engine::viewer::SnapTarget;
use serde_json::{Value, json};

use crate::MotionDocument;
use geometry::{LayerBox, Pt};

/// The tools on the rail, in its order: (id, label, shortcut, icon).
const TOOLS: [(Tool, &str, &str, &str, &str); 10] = [
    (Tool::Selection, "selection", "Selection Tool", "V", "tool-selection"),
    (Tool::Hand, "hand", "Hand Tool", "H", "tool-hand"),
    (Tool::Zoom, "zoom", "Zoom Tool", "Z", "tool-zoom"),
    (Tool::Orbit, "orbit", "Orbit Camera Tool", "C", "tool-rotate-camera"),
    (Tool::Track, "track", "Pan Camera Tool", "C", "tool-move"),
    (Tool::Dolly, "dolly", "Dolly Camera Tool", "C", "tool-scale-camera"),
    (Tool::Rectangle, "rectangle", "Rectangle Tool", "Q", "tool-rect"),
    (Tool::Ellipse, "ellipse", "Ellipse Tool", "Q", "tool-ellipse"),
    (Tool::Text, "text", "Text Tool", "T", "tool-type"),
    (Tool::Pen, "pen", "Pen Tool", "G", "tool-pen"),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tool {
    #[default]
    Selection,
    Hand,
    Zoom,
    Rectangle,
    Ellipse,
    Text,
    Pen,
    /// The camera tools (see [`camera`]).
    Orbit,
    Track,
    Dolly,
}

impl Tool {
    fn id(self) -> &'static str {
        TOOLS.iter().find(|t| t.0 == self).map_or("selection", |t| t.1)
    }

    fn from_id(id: &str) -> Option<Tool> {
        TOOLS.iter().find(|t| t.1 == id).map(|t| t.0)
    }
}

/// A gesture under way.
#[derive(Clone, Debug, Default)]
pub(crate) enum Gesture {
    #[default]
    Idle,
    /// A press on a selected layer, not yet a drag: a click, or the start of a move.
    Press { start: Pt, layer: LayerId, shift: bool },
    /// Moving the selected layers: the offset applied so far, their snap features at the start
    /// and the targets they snap to.
    Move { start: Pt, applied: Pt, layers: Vec<LayerId>, features: Vec<Pt>, targets: Vec<SnapTarget>, merge: String },
    /// A rubber band from `start`; `base` stays selected (Shift).
    Marquee { start: Pt, end: Pt, base: Vec<LayerId> },
    /// Dragging a handle of `start.id`'s box.
    Scale(Box<select::Scaling>),
    /// Turning the selected layers about the dragged one's anchor point.
    Rotate(Box<select::Turning>),
    /// Drawing a shape from `start`.
    Draw { start: Pt, end: Pt, square: bool, centred: bool },
    /// Selecting text in the edited layer from character `anchor`.
    TextDrag { anchor: usize },
    /// The Pen's press that placed a point at `start`: a drag pulls its curve handles.
    PenPoint { start: Pt },
    /// Dragging a selected path vertex's curve handle.
    Vertex { vertex: kition_motion_engine::VertexRef, part: pen::VertexPart, merge: String },
    /// Moving the selected path vertices from `start`, applied up to `last`; `alone`: a click
    /// (no drag) on one of several selected vertices selects it alone.
    Vertices { start: Pt, last: Pt, moved: bool, alone: Option<kition_motion_engine::VertexRef>, merge: String },
    /// Dragging a shape gradient's Start Point or End Point (`path`).
    GradientPoint { layer: LayerId, path: String, merge: String },
    /// A box over path vertices from `start`; `base` stays selected (Shift).
    VertexBox { start: Pt, end: Pt, base: Vec<kition_motion_engine::VertexRef> },
    /// A press on a box corner that is also a path vertex, not yet a drag: a drag scales the
    /// layer from the corner, a click selects the vertex (and edits the path).
    Corner { start: Pt, layer_box: Box<geometry::LayerBox>, handle: geometry::Handle, vertex: kition_motion_engine::VertexRef },
    /// Orbiting, panning or dollying the view of the 3D layers from `start`; `last` is where it
    /// was applied, `axis` the direction a Shift-drag keeps to (true: across).
    Camera { start: Pt, last: Pt, axis: Option<bool>, merge: String },
    /// Dragging the 3D handles: an arrow, a plane square or a ring (see [`gizmo`]).
    Gizmo(Box<gizmo::Drag>),
}

/// The tools' state for the open document.
#[derive(Debug)]
pub(crate) struct ToolState {
    tool: Tool,
    zoom: f64,
    gesture: Gesture,
    cursor: String,
    /// What snapped in the last move (drawn as guides while it lasts).
    snaps: Vec<SnapTarget>,
    /// Gestures started, for their undo `merge` keys.
    serial: u64,
    /// The typing session's `merge` key: text typed into one layer is one undo step.
    typing: Option<String>,
    status: Option<String>,
    /// The path the Pen is drawing.
    pen: Option<pen::PenDraft>,
    /// The layers whose paths are being edited (a double-click on a path, or a vertex picked):
    /// their transform boxes hide and a press on the canvas is the paths'. Escape, or a click on
    /// empty space, ends it.
    path_edit: Vec<LayerId>,
    /// Whether the status bar says paths are being edited ([`PATH_EDIT_HINT`]).
    path_hint: bool,
    /// The Pen has just finished drawing a path (nothing else done with it since): the Selection
    /// tool picked next shows the layers' transform boxes, not the new path's vertices.
    pen_drew: bool,
}

/// The status bar while paths are being edited with the Selection tool (the editor also lets
/// Escape reach the tool while it shows).
pub(crate) const PATH_EDIT_HINT: &str = "Editing the path: drag its vertices or edges; Esc or a click on empty space ends it";

impl Default for ToolState {
    fn default() -> Self {
        ToolState {
            tool: Tool::Selection,
            zoom: 1.0,
            gesture: Gesture::Idle,
            cursor: "default".into(),
            snaps: Vec::new(),
            serial: 0,
            typing: None,
            status: None,
            pen: None,
            path_edit: Vec::new(),
            path_hint: false,
            pen_drew: false,
        }
    }
}

impl ToolState {
    /// A new undo key for a gesture.
    fn merge_key(&mut self, what: &str) -> String {
        self.serial = self.serial.wrapping_add(1);
        format!("canvas-{what}-{}", self.serial)
    }
}

thread_local! {
    static STATE: RefCell<ToolState> = RefCell::new(ToolState::default());
}

/// Runs `f` on the tools' state (taken out for the call, so engine commands run meanwhile).
fn with_state<R>(f: impl FnOnce(&mut ToolState) -> R) -> R {
    let mut state = STATE.with(|cell| std::mem::take(&mut *cell.borrow_mut()));
    let out = f(&mut state);
    STATE.with(|cell| *cell.borrow_mut() = state);
    out
}

/// The active composition at the current time, for hit tests and boxes, seen through the 3D view
/// the viewer shows (Front, Top, a custom view: [`geometry::layer_box`] projects through it).
pub(crate) fn with_ctx<R>(session: &Session, f: impl FnOnce(&EvalCtx) -> R) -> Option<R> {
    let cid = session.active_comp_id()?;
    let comp = session.project.comp(cid)?;
    let ctx = EvalCtx { footage: Some(session.footage.as_ref()), expr: session.expr.as_deref(), ..EvalCtx::new(&session.project, cid, comp, session.time()) };
    Some(geometry::through_view(session.view_camera(cid), || f(&ctx)))
}

/// The selected layers' boxes, topmost first.
pub(crate) fn selected_boxes(session: &Session) -> Vec<LayerBox> {
    let selected = &session.state.selected_layers;
    with_ctx(session, |ctx| {
        ctx.comp
            .layers
            .iter()
            .filter(|l| selected.contains(&l.id) && l.is_active_at(ctx.time) && !l.is_camera() && !l.is_light())
            .filter_map(|l| geometry::layer_box(ctx, l))
            .collect()
    })
    .unwrap_or_default()
}

/// The selection box's colour.
const SELECTION: [u8; 4] = [62, 136, 255, 255];
/// Snapping guides' colour.
const GUIDE: [u8; 4] = [255, 64, 160, 255];
/// How near (screen pixels) a moved feature snaps.
const SNAP_REACH: f64 = 8.0;

fn quad_path(q: &[Pt; 4]) -> String {
    format!("M{} {} L{} {} L{} {} L{} {} Z", q[0][0], q[0][1], q[1][0], q[1][1], q[2][0], q[2][1], q[3][0], q[3][1])
}

fn ellipse_path(b: [f64; 4]) -> String {
    let (cx, cy, rx, ry) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0, (b[2] - b[0]) / 2.0, (b[3] - b[1]) / 2.0);
    format!("M{} {cy} A{rx} {ry} 0 1 0 {} {cy} A{rx} {ry} 0 1 0 {} {cy} Z", cx - rx, cx + rx, cx - rx)
}

/// A small cross at `p`, `arm` comp pixels each way.
fn cross(p: Pt, arm: f64, color: [u8; 4], out: &mut Vec<ToolOverlay>) {
    out.push(ToolOverlay::Line { ax: p[0] - arm, ay: p[1], bx: p[0] + arm, by: p[1], color, dashed: false });
    out.push(ToolOverlay::Line { ax: p[0], ay: p[1] - arm, bx: p[0], by: p[1] + arm, color, dashed: false });
}

/// The selection's overlays: each selected layer's box, its handles, its rotation knob and its
/// anchor point (handles for 2D layers only).
fn selection_overlays(session: &Session, path_edit: &[LayerId], zoom: f64, out: &mut Vec<ToolOverlay>) {
    // A layer whose paths are being edited shows its vertices, not its transform box.
    for b in selected_boxes(session).into_iter().filter(|b| !session.state.selected_vertices.iter().any(|v| v.layer == b.id) && !path_edit.contains(&b.id)) {
        out.push(ToolOverlay::Path { d: quad_path(&b.corners()), color: SELECTION, width: 1.0, dashed: false });
        if b.flat {
            let (base, tip) = geometry::knob(&b, zoom);
            out.push(ToolOverlay::Line { ax: base[0], ay: base[1], bx: tip[0], by: tip[1], color: SELECTION, dashed: false });
            out.push(ToolOverlay::Handle { x: tip[0], y: tip[1], color: SELECTION });
            for h in geometry::HANDLES {
                let p = b.to_comp(h.at(b.bounds));
                out.push(ToolOverlay::Anchor { x: p[0], y: p[1], color: SELECTION, filled: false, size: 7.0 });
            }
        }
        cross(b.anchor_comp(), 7.0 / zoom, SELECTION, out);
    }
}

fn snap_overlays(session: &Session, snaps: &[SnapTarget], zoom: f64, out: &mut Vec<ToolOverlay>) {
    let Some(comp) = session.active_comp() else { return };
    let (w, h) = (f64::from(comp.width), f64::from(comp.height));
    for t in snaps {
        match t.kind {
            kition_motion_engine::viewer::SnapKind::VLine => {
                out.push(ToolOverlay::Line { ax: t.pos[0], ay: 0.0, bx: t.pos[0], by: h, color: GUIDE, dashed: true })
            }
            kition_motion_engine::viewer::SnapKind::HLine => {
                out.push(ToolOverlay::Line { ax: 0.0, ay: t.pos[1], bx: w, by: t.pos[1], color: GUIDE, dashed: true })
            }
            kition_motion_engine::viewer::SnapKind::Point => cross(t.pos, 9.0 / zoom, GUIDE, out),
        }
    }
}

/// Polylines (around the hot spot) as an SVG cursor with a system fallback.
fn drawn_cursor(lines: &[Vec<(f64, f64)>], fallback: &str) -> String {
    const HOT: f64 = 16.0;
    let points = |line: &Vec<(f64, f64)>| line.iter().map(|(x, y)| format!("{:.2},{:.2}", x + HOT, y + HOT)).collect::<Vec<_>>().join(" ");
    let mut body = String::new();
    for (width, colour) in [(3.5, "%23FFFFFF"), (1.5, "%231E1E1E")] {
        for line in lines {
            body.push_str(&format!(
                "%3Cpolyline points='{}' fill='none' stroke='{colour}' stroke-width='{width}' stroke-linecap='round' stroke-linejoin='round'/%3E",
                points(line)
            ));
        }
    }
    format!(
        "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='32' height='32' viewBox='0 0 32 32'%3E{body}%3C/svg%3E\") 16 16, {fallback}"
    )
}

/// The rotation cursor: a curved double arrow bulging away from the box, `degrees` being the
/// direction from the box's centre to the pointer.
fn rotate_cursor(degrees: f64) -> String {
    let degrees = if degrees.is_finite() { degrees.rem_euclid(360.0).round() } else { 0.0 };
    let (dir, r, half) = (degrees.to_radians(), 11.0_f64, 0.8_f64);
    let c = (-dir.cos() * r, -dir.sin() * r);
    let arc: Vec<(f64, f64)> = (0..=12).map(|i| dir - half + 2.0 * half * f64::from(i) / 12.0).map(|a| (c.0 + a.cos() * r, c.1 + a.sin() * r)).collect();
    let mut lines = vec![arc];
    for (end, sign) in [(dir - half, -1.0), (dir + half, 1.0)] {
        let tip = (c.0 + end.cos() * r, c.1 + end.sin() * r);
        let t = (-end.sin() * sign, end.cos() * sign);
        let n = (end.cos(), end.sin());
        lines.push(vec![
            (tip.0 - t.0 * 4.5 + n.0 * 3.5, tip.1 - t.1 * 4.5 + n.1 * 3.5),
            (tip.0 + t.0, tip.1 + t.1),
            (tip.0 - t.0 * 4.5 - n.0 * 3.5, tip.1 - t.1 * 4.5 - n.1 * 3.5),
        ]);
    }
    drawn_cursor(&lines, "grab")
}

/// What the Selection tool would do with a press at `p`: the cursor that says so.
fn hover_cursor(session: &Session, p: Pt, zoom: f64) -> String {
    if let Some(cursor) = gizmo::hover(session, p, zoom) {
        return cursor.to_owned();
    }
    for b in selected_boxes(session).iter().filter(|b| b.flat) {
        let centre = b.to_comp([(b.bounds[0] + b.bounds[2]) / 2.0, (b.bounds[1] + b.bounds[3]) / 2.0]);
        if let Some(h) = geometry::handle_at(b, p, zoom) {
            let at = b.to_comp(h.at(b.bounds));
            return geometry::resize_cursor(geometry::angle_about(centre, at)).to_owned();
        }
        if geometry::rotates_at(b, p, zoom) {
            return rotate_cursor(geometry::angle_about(centre, p));
        }
    }
    let over = with_ctx(session, |ctx| geometry::pick(ctx, p, &geometry::any_layer).is_some()).unwrap_or(false);
    if over { "move" } else { "default" }.to_owned()
}

impl MotionDocument {
    /// Whether paths are being edited: a vertex is picked, or a selected layer is in path editing.
    pub(crate) fn editing_paths(&self, st: &ToolState) -> bool {
        !self.session.state.selected_vertices.is_empty() || st.path_edit.iter().any(|layer| self.session.state.selected_layers.contains(layer))
    }

    /// Where the 3D handles' parts are on the canvas at `zoom` (screen pixels per comp pixel), as
    /// JSON (see [`gizmo::spots`]): the browser tests press there.
    pub fn gizmo_handles(&self, zoom: f64) -> Value {
        gizmo::spots(&self.session, zoom_of(zoom))
    }

    /// The second view of the two-view layout: the active composition at the current time seen
    /// through 3D view `view` (`top`, `front`, `activeCamera`…; its edited camera, or its default),
    /// at most `width` pixels wide, as RGBA over the background (`rgba`, `width`, `height`), with
    /// the ground, the 3D layers' outlines, cameras and lights for that view as lines in
    /// composition pixels (`lines`: `[ax, ay, bx, by, r, g, b, a, dashed]`), and each 3D layer's
    /// outline with its name, label colour and whether it is selected (`layers`, see
    /// [`view3d::layers_through`]).
    pub fn view_frame(&self, view: &str, width: u32) -> Result<(Vec<u8>, u32, u32, Value, Value), String> {
        use kition_motion_engine::render::three_d::View3D;
        let cid = self.session.active_comp_id().ok_or("no composition is open")?;
        let comp = self.session.project.comp(cid).ok_or("no composition is open")?;
        let which = View3D::from_id(view).ok_or_else(|| format!("no 3D view `{view}`"))?;
        let (w, h) = (f64::from(comp.width.max(1)), f64::from(comp.height.max(1)));
        let camera = (which != View3D::ActiveCamera).then(|| self.session.state.views3d.get(&cid).cloned().unwrap_or_default().cam(which, w, h).state());
        let scale = (f64::from(width.clamp(16, 4096)) / w).min(1.0);
        let opts = kition_motion_engine::render::RenderOpts { scale, view: camera, ..Default::default() };
        let mut renderer = kition_motion_engine::render::Renderer::new(&self.session.project, &*self.session.footage, opts);
        renderer.expr = self.session.expr.as_deref();
        let image = renderer.comp_frame(cid, comp.frame_rate.snap(self.session.time()));
        let rgba = image.to_rgba8_over(comp.background);
        let mut lines = Vec::new();
        let mut layers = Vec::new();
        if let Some(camera) = camera {
            layers = view3d::layers_through(&self.session, camera);
            let mut overlays = Vec::new();
            view3d::overlays_through(&self.session, camera, &mut overlays);
            for overlay in overlays {
                if let ToolOverlay::Line { ax, ay, bx, by, color, dashed } = overlay {
                    lines.push(json!([ax, ay, bx, by, color[0], color[1], color[2], color[3], dashed]));
                }
            }
        }
        Ok((rgba, image.width, image.height, Value::Array(lines), Value::Array(layers)))
    }

    /// Runs an engine command for a tool, as the editor would.
    pub(crate) fn tool_run(&mut self, command: &str, params: Value) -> Result<Value, String> {
        KitDocument::execute(self, command, params)
    }

    /// The comp time now, in seconds (what `prop.set` takes with `timeBase: comp`).
    pub(crate) fn now(&self) -> f64 {
        self.session.time().seconds()
    }

    /// The answer after an event.
    fn respond(&mut self, st: &mut ToolState, switch_to: Option<Tool>) -> ToolResponse {
        let zoom = st.zoom;
        let mut overlays = Vec::new();
        let editing = self.session.state.text_edit.is_some();
        if editing {
            text::overlays(&self.session, zoom, &mut overlays);
        } else if matches!(st.tool, Tool::Selection | Tool::Rectangle | Tool::Ellipse | Tool::Text | Tool::Pen) {
            selection_overlays(&self.session, &st.path_edit, zoom, &mut overlays);
        }
        // The selected layers' masks and shape paths, editable with the Selection tool and the Pen.
        if !editing && matches!(st.tool, Tool::Selection | Tool::Pen) {
            pen::path_overlays(&self.session, &mut overlays);
        }
        // A 3D view other than the camera's: the ground, the 3D layers' outlines, cameras and lights.
        view3d::overlays(&self.session, &mut overlays);
        // The 3D handles of the selected 3D layer, camera or light (not while paths or text are edited).
        if !editing && st.tool == Tool::Selection && !self.editing_paths(st) {
            gizmo::overlays(&self.session, zoom, &mut overlays);
        }
        // The selected shape layers' gradients, their points dragged with the Selection tool.
        if !editing && st.tool == Tool::Selection {
            gradient::overlays(&self.session, &mut overlays);
        }
        if let Some(draft) = st.pen.as_ref().filter(|_| st.tool == Tool::Pen) {
            pen::draft_overlays(draft, &mut overlays);
        }
        match &st.gesture {
            Gesture::Marquee { start, end, .. } | Gesture::VertexBox { start, end, .. } => {
                overlays.push(ToolOverlay::Marquee { x0: start[0], y0: start[1], x1: end[0], y1: end[1] })
            }
            Gesture::Draw { start, end, square, centred } => {
                let b = geometry::drawn_box(*start, *end, *square, *centred);
                let d = if st.tool == Tool::Ellipse { ellipse_path(b) } else { quad_path(&[[b[0], b[1]], [b[2], b[1]], [b[2], b[3]], [b[0], b[3]]]) };
                overlays.push(ToolOverlay::Path { d, color: SELECTION, width: 1.0, dashed: false });
                overlays.push(ToolOverlay::Measure { x: b[2], y: b[3], text: format!("{} × {}", (b[2] - b[0]).round(), (b[3] - b[1]).round()) });
            }
            Gesture::Move { .. } => snap_overlays(&self.session, &st.snaps, zoom, &mut overlays),
            _ => {}
        }
        // Path editing started or ended: the status bar says so (an empty status clears it).
        let editing_paths = st.tool == Tool::Selection && !editing && self.editing_paths(st);
        if editing_paths != st.path_hint {
            st.path_hint = editing_paths;
            if st.status.is_none() {
                st.status = Some(if editing_paths { PATH_EDIT_HINT } else { "" }.to_owned());
            }
        }
        let mut requests = Vec::new();
        if let Some(tool) = switch_to {
            requests.push(ToolRequest::SwitchTool { id: tool.id().to_owned() });
        }
        if let Some(text) = st.status.take() {
            requests.push(ToolRequest::Status { text });
        }
        let cursor = match st.tool {
            Tool::Rectangle | Tool::Ellipse | Tool::Pen => "crosshair".to_owned(),
            Tool::Orbit | Tool::Track | Tool::Dolly => if matches!(st.gesture, Gesture::Camera { .. }) { "grabbing" } else { "grab" }.to_owned(),
            Tool::Text => "text".to_owned(),
            _ if editing && matches!(st.gesture, Gesture::TextDrag { .. }) => "text".to_owned(),
            _ => st.cursor.clone(),
        };
        let busy = !matches!(st.gesture, Gesture::Idle) || editing || pen::drawing(st);
        ToolResponse { requests, overlays, cursor, busy, wants_text: editing }
    }

    /// Ends text editing, if any.
    fn end_text(&mut self, st: &mut ToolState) {
        if self.session.state.text_edit.is_some() {
            let _ = self.tool_run("text.endEdit", json!({}));
        }
        st.typing = None;
    }
}

fn zoom_of(zoom: f64) -> f64 {
    if zoom.is_finite() && zoom > 1e-6 { zoom } else { 1.0 }
}

impl KitTools for MotionDocument {
    fn tools(&self) -> Vec<ToolInfo> {
        TOOLS
            .iter()
            .enumerate()
            .map(|(group, (_, id, label, key, icon))| ToolInfo {
                id: (*id).into(),
                label: (*label).into(),
                shortcut: Some((*key).into()),
                icon: (*icon).into(),
                group,
            })
            .collect()
    }

    fn tool(&self) -> String {
        with_state(|st| st.tool.id().to_owned())
    }

    fn select_tool(&mut self, id: &str, zoom: f64) -> Result<ToolResponse, String> {
        let tool = Tool::from_id(id).ok_or_else(|| format!("no tool {id}"))?;
        Ok(with_state(|st| {
            st.zoom = zoom_of(zoom);
            st.gesture = Gesture::Idle;
            st.snaps.clear();
            if tool != st.tool && tool != Tool::Text {
                self.end_text(st);
            }
            // A mask left half drawn is dropped with its tool.
            if tool != Tool::Pen {
                st.pen = None;
            }
            // Selection picked right after the Pen drew a path (V), or picked again while paths
            // are edited (the editor does that for Escape), is the plain one: the layers'
            // transform boxes, not the paths' vertices (a double-click on a path edits it again).
            // After the Pen added a vertex to a path, its vertices stay picked.
            if tool == Tool::Selection && (st.tool == Tool::Selection || st.pen_drew) {
                st.path_edit.clear();
                self.clear_vertices();
            }
            st.pen_drew = false;
            st.tool = tool;
            st.cursor = "default".into();
            self.respond(st, None)
        }))
    }

    fn pointer(&mut self, event: &ToolPointer) -> Result<ToolResponse, String> {
        if !(event.x.is_finite() && event.y.is_finite()) {
            return Err("the pointer is not at a finite position".into());
        }
        // A press after the Pen drew: the path is being worked on now.
        if matches!(event.action, PointerAction::Down) {
            with_state(|st| st.pen_drew = false);
        }
        Ok(with_state(|st| {
            st.zoom = zoom_of(event.zoom);
            let p = [event.x, event.y];
            let switch = match event.action {
                PointerAction::Down => {
                    st.snaps.clear();
                    self.pointer_down(st, p, event.mods);
                    None
                }
                PointerAction::Drag => {
                    self.pointer_drag(st, p, event.mods);
                    None
                }
                PointerAction::Up => self.pointer_up(st, p, event.mods),
                PointerAction::Move => {
                    if st.tool == Tool::Selection && matches!(st.gesture, Gesture::Idle) {
                        st.cursor = if self.session.state.text_edit.is_some() && text::inside_edited(&self.session, p) {
                            "text".into()
                        } else {
                            hover_cursor(&self.session, p, st.zoom)
                        };
                    }
                    None
                }
                PointerAction::DoubleClick => {
                    // On a selected layer's path: edit its vertices (its transform box hides).
                    if st.tool == Tool::Selection
                        && self.session.state.text_edit.is_none()
                        && let Some(layer) = pen::path_layer_under(&self.session, p, st.zoom)
                    {
                        st.path_edit = vec![layer];
                    } else if st.tool == Tool::Selection && !text::double_click(self, st, p) {
                        select::open_precomp(self, st, p);
                    }
                    None
                }
            };
            self.respond(st, switch)
        }))
    }

    fn key(&mut self, key: &str, mods: ToolMods, zoom: f64) -> Result<ToolResponse, String> {
        Ok(with_state(|st| {
            st.zoom = zoom_of(zoom);
            if self.session.state.text_edit.is_some() && matches!(st.gesture, Gesture::Idle | Gesture::TextDrag { .. }) {
                text::key(self, st, key, mods);
            } else if pen::drawing(st) {
                // Enter keeps the drawn path as an open mask, Escape drops it, Backspace takes the last point back.
                match key {
                    "Enter" => self.pen_finish(st, false),
                    "Escape" | "Esc" => {
                        st.pen = None;
                        st.gesture = Gesture::Idle;
                    }
                    "Backspace" | "Delete" => {
                        if let Some(draft) = st.pen.as_mut() {
                            draft.points.pop();
                        }
                    }
                    _ => {}
                }
            } else if matches!(key, "Escape" | "Esc") {
                // Escape with no gesture under way ends path editing; during one, undoes it.
                if matches!(st.gesture, Gesture::Idle) && self.editing_paths(st) {
                    st.path_edit.clear();
                    self.clear_vertices();
                } else {
                    self.cancel(st);
                }
            }
            self.respond(st, None)
        }))
    }

    fn text(&mut self, text: &str, zoom: f64) -> Result<ToolResponse, String> {
        with_state(|st| {
            st.zoom = zoom_of(zoom);
            if self.session.state.text_edit.is_some() {
                text::type_text(self, st, text)?;
            }
            Ok(self.respond(st, None))
        })
    }

    fn state(&mut self, x: f64, y: f64, _mods: ToolMods, zoom: f64) -> ToolResponse {
        with_state(|st| {
            st.zoom = zoom_of(zoom);
            if st.tool == Tool::Selection && matches!(st.gesture, Gesture::Idle) && x.is_finite() && y.is_finite() && self.session.state.text_edit.is_none() {
                st.cursor = hover_cursor(&self.session, [x, y], st.zoom);
            }
            self.respond(st, None)
        })
    }
}

#[cfg(test)]
mod tests;
