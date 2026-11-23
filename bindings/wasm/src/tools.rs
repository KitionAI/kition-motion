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

mod geometry;
mod select;
mod text;

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
const TOOLS: [(Tool, &str, &str, &str, &str); 6] = [
    (Tool::Selection, "selection", "Selection Tool", "V", "tool-selection"),
    (Tool::Hand, "hand", "Hand Tool", "H", "tool-hand"),
    (Tool::Zoom, "zoom", "Zoom Tool", "Z", "tool-zoom"),
    (Tool::Rectangle, "rectangle", "Rectangle Tool", "Q", "tool-rect"),
    (Tool::Ellipse, "ellipse", "Ellipse Tool", "Q", "tool-ellipse"),
    (Tool::Text, "text", "Text Tool", "T", "tool-type"),
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
}

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

/// The active composition at the current time, for hit tests and boxes.
pub(crate) fn with_ctx<R>(session: &Session, f: impl FnOnce(&EvalCtx) -> R) -> Option<R> {
    let cid = session.active_comp_id()?;
    let comp = session.project.comp(cid)?;
    let ctx = EvalCtx { footage: Some(session.footage.as_ref()), expr: session.expr.as_deref(), ..EvalCtx::new(&session.project, cid, comp, session.time()) };
    Some(f(&ctx))
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
fn selection_overlays(session: &Session, zoom: f64, out: &mut Vec<ToolOverlay>) {
    for b in selected_boxes(session) {
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
        } else if matches!(st.tool, Tool::Selection | Tool::Rectangle | Tool::Ellipse | Tool::Text) {
            selection_overlays(&self.session, zoom, &mut overlays);
        }
        match &st.gesture {
            Gesture::Marquee { start, end, .. } => overlays.push(ToolOverlay::Marquee { x0: start[0], y0: start[1], x1: end[0], y1: end[1] }),
            Gesture::Draw { start, end, square, centred } => {
                let b = geometry::drawn_box(*start, *end, *square, *centred);
                let d = if st.tool == Tool::Ellipse { ellipse_path(b) } else { quad_path(&[[b[0], b[1]], [b[2], b[1]], [b[2], b[3]], [b[0], b[3]]]) };
                overlays.push(ToolOverlay::Path { d, color: SELECTION, width: 1.0, dashed: false });
                overlays.push(ToolOverlay::Measure { x: b[2], y: b[3], text: format!("{} × {}", (b[2] - b[0]).round(), (b[3] - b[1]).round()) });
            }
            Gesture::Move { .. } => snap_overlays(&self.session, &st.snaps, zoom, &mut overlays),
            _ => {}
        }
        let mut requests = Vec::new();
        if let Some(tool) = switch_to {
            requests.push(ToolRequest::SwitchTool { id: tool.id().to_owned() });
        }
        if let Some(text) = st.status.take() {
            requests.push(ToolRequest::Status { text });
        }
        let cursor = match st.tool {
            Tool::Rectangle | Tool::Ellipse => "crosshair".to_owned(),
            Tool::Text => "text".to_owned(),
            _ if editing && matches!(st.gesture, Gesture::TextDrag { .. }) => "text".to_owned(),
            _ => st.cursor.clone(),
        };
        ToolResponse { requests, overlays, cursor, busy: !matches!(st.gesture, Gesture::Idle) || editing, wants_text: editing }
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
            st.tool = tool;
            st.cursor = "default".into();
            self.respond(st, None)
        }))
    }

    fn pointer(&mut self, event: &ToolPointer) -> Result<ToolResponse, String> {
        if !(event.x.is_finite() && event.y.is_finite()) {
            return Err("the pointer is not at a finite position".into());
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
                    if st.tool == Tool::Selection {
                        text::double_click(self, st, p);
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
            } else if matches!(key, "Escape" | "Esc") {
                self.cancel(st);
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
