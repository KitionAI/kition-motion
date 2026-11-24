//! The Selection tool's gestures (click, Shift-click, marquee, move, handle scaling, rotation)
//! and the shape tools' drag, on the person's own layer selection (`layer.select`, the one the
//! timeline and the Properties panel show).

use kition_kit::tools::ToolMods;
use kition_motion_engine::geom::Mat3;
use kition_motion_engine::project::{LayerId, LayerSource};
use kition_motion_engine::viewer::{self, SnapOptions, SnapSource, SnapTarget};
use serde_json::json;

use super::geometry::{self, Handle, LayerBox, Pt};
use super::{Gesture, SNAP_REACH, Tool, ToolState, selected_boxes, text, with_ctx};
use crate::MotionDocument;

/// How far (screen pixels) a press travels before it is a drag, not a click.
const DRAG_START: f64 = 3.0;
/// A drawn shape smaller than this (comp pixels) is a slip, not a shape.
const SMALLEST_SHAPE: f64 = 2.0;

/// A handle drag: the layer's box, matrices and values when it began, and the other selected
/// layers' scales (they scale by the same factors about their own anchor points).
#[derive(Clone, Debug)]
pub(crate) struct Scaling {
    pub start: LayerBox,
    pub handle: Handle,
    pub parent: Mat3,
    pub scale: [f64; 3],
    pub position: [f64; 3],
    pub others: Vec<(LayerId, [f64; 3])>,
    pub merge: String,
}

/// A rotation: the pivot (the dragged layer's anchor point in the comp), the pointer's angle
/// about it so far (unwrapped), where it began, and each selected layer's rotation then with the
/// sign its parents give a turn on screen.
#[derive(Clone, Debug)]
pub(crate) struct Turning {
    pub pivot: Pt,
    pub first: f64,
    pub last: f64,
    pub layers: Vec<(LayerId, f64, f64)>,
    pub merge: String,
}

/// A layer's transform value at the current time (`[0, 0, 0]` when it has none).
fn transform_value(doc: &MotionDocument, id: LayerId, name: &str) -> [f64; 3] {
    with_ctx(&doc.session, |ctx| {
        let layer = ctx.comp.layer(id)?;
        let tr = layer.transform()?;
        Some(if name == "position" { ctx.position(layer, tr) } else { ctx.v3(layer, tr, name, [0.0; 3]) })
    })
    .flatten()
    .unwrap_or([0.0; 3])
}

fn rotation_value(doc: &MotionDocument, id: LayerId) -> f64 {
    with_ctx(&doc.session, |ctx| {
        let layer = ctx.comp.layer(id)?;
        Some(ctx.f(layer, layer.transform()?, "rotation", 0.0))
    })
    .flatten()
    .unwrap_or(0.0)
}

fn distance(a: Pt, b: Pt) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

impl MotionDocument {
    fn selected(&self) -> Vec<LayerId> {
        self.session.state.selected_layers.clone()
    }

    fn set_prop(&mut self, layer: LayerId, path: &str, value: serde_json::Value, merge: &str) {
        let time = self.now();
        let _ = self.tool_run("prop.set", json!({ "layer": layer.0, "path": path, "value": value, "time": time, "timeBase": "comp", "merge": merge }));
    }

    pub(super) fn pointer_down(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) {
        st.gesture = Gesture::Idle;
        match st.tool {
            Tool::Selection => self.select_down(st, p, mods),
            Tool::Rectangle | Tool::Ellipse => {
                self.end_text(st);
                st.gesture = Gesture::Draw { start: p, end: p, square: mods.shift, centred: mods.alt };
            }
            Tool::Text => text::text_down(self, st, p),
            Tool::Pen => {
                // A mask's vertex or handle drags when no mask is being drawn; any other press draws.
                if st.pen.is_none() && self.vertex_down(st, p) {
                    return;
                }
                self.pen_down(st, p);
            }
            Tool::Hand | Tool::Zoom => {}
        }
    }

    fn select_down(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) {
        if self.session.state.text_edit.is_some() {
            if text::inside_edited(&self.session, p) {
                text::place_caret(self, st, p);
                return;
            }
            self.end_text(st);
        }
        let zoom = st.zoom;
        // A selected layer's mask vertices (and the picked vertex's handles) first.
        if self.vertex_down(st, p) {
            return;
        }
        st.vertex = None;
        // A selected layer's handles first (Shift held from the start keeps a corner's
        // proportions), then, without Shift, its rotation zones.
        let boxes = selected_boxes(&self.session);
        for b in boxes.iter().filter(|b| b.flat) {
            if let Some(handle) = geometry::handle_at(b, p, zoom) {
                self.begin_scale(st, b.clone(), handle);
                return;
            }
        }
        if !mods.shift
            && let Some(b) = boxes.iter().find(|b| geometry::rotates_at(b, p, zoom))
        {
            self.begin_rotate(st, b.anchor_comp(), p);
            return;
        }
        let hit = with_ctx(&self.session, |ctx| geometry::pick(ctx, p, &geometry::any_layer)).flatten();
        let selected = self.selected();
        match hit {
            Some(id) if mods.shift => {
                let _ = self.tool_run("layer.select", json!({ "layers": [id.0], "toggle": true }));
                if self.session.state.selected_layers.contains(&id) {
                    st.gesture = Gesture::Press { start: p, layer: id, shift: true };
                }
            }
            Some(id) => {
                if !selected.contains(&id) {
                    let _ = self.tool_run("layer.select", json!({ "layers": [id.0] }));
                }
                st.gesture = Gesture::Press { start: p, layer: id, shift: false };
            }
            None => {
                let base = if mods.shift { selected } else { Vec::new() };
                if !mods.shift && !self.session.state.selected_layers.is_empty() {
                    let _ = self.tool_run("edit.deselectAll", json!({}));
                }
                st.gesture = Gesture::Marquee { start: p, end: p, base };
            }
        }
    }

    fn begin_scale(&mut self, st: &mut ToolState, start: LayerBox, handle: Handle) {
        let parent = with_ctx(&self.session, |ctx| ctx.comp.layer(start.id).map(|l| geometry::parent_matrix(ctx, l))).flatten().unwrap_or(Mat3::IDENTITY);
        let scale = transform_value(self, start.id, "scale");
        let position = transform_value(self, start.id, "position");
        let others = self.selected().into_iter().filter(|id| *id != start.id).map(|id| (id, transform_value(self, id, "scale"))).collect();
        let merge = st.merge_key("scale");
        st.gesture = Gesture::Scale(Box::new(Scaling { start, handle, parent, scale, position, others, merge }));
    }

    fn begin_rotate(&mut self, st: &mut ToolState, pivot: Pt, p: Pt) {
        let layers = self
            .selected()
            .into_iter()
            .filter_map(|id| {
                let sign = with_ctx(&self.session, |ctx| {
                    let layer = ctx.comp.layer(id)?;
                    (!layer.is_3d() && !layer.switches.locked).then(|| if geometry::parent_matrix(ctx, layer).determinant() < 0.0 { -1.0 } else { 1.0 })
                })
                .flatten()?;
                Some((id, rotation_value(self, id), sign))
            })
            .collect();
        let first = geometry::angle_about(pivot, p);
        let merge = st.merge_key("rotate");
        st.gesture = Gesture::Rotate(Box::new(Turning { pivot, first, last: first, layers, merge }));
    }

    /// The selected layers start moving: their snap features and what they may snap to.
    fn begin_move(&mut self, st: &mut ToolState, start: Pt) {
        let layers: Vec<LayerId> = self.selected();
        let (features, targets) = with_ctx(&self.session, |ctx| {
            let features: Vec<Pt> = layers.iter().filter_map(|id| ctx.comp.layer(*id)).flat_map(|l| viewer::layer_features(ctx, l)).take(4096).collect();
            // Guides and the grid are targets while View ▸ Snap to Guides / Snap to Grid are on.
            let opts = SnapOptions {
                layers: true,
                features: self.session.state.snap_features,
                guides: self.snap_guides,
                grid: self.snap_grid,
                grid_spacing: self.session.prefs.grids.grid_spacing,
            };
            (features, viewer::targets(ctx, &layers, opts))
        })
        .unwrap_or_default();
        let merge = st.merge_key("move");
        st.gesture = Gesture::Move { start, applied: [0.0, 0.0], layers, features, targets, merge };
    }

    pub(super) fn pointer_drag(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) {
        let zoom = st.zoom;
        if let Gesture::Press { start, .. } = st.gesture {
            if distance(start, p) * zoom < DRAG_START {
                return;
            }
            self.begin_move(st, start);
        }
        match &mut st.gesture {
            Gesture::PenPoint { .. } => Self::pen_drag(st, p),
            Gesture::Vertex { .. } => self.vertex_drag(st, p, mods),
            Gesture::Move { start, applied, layers, features, targets, merge } => {
                let mut d = [p[0] - start[0], p[1] - start[1]];
                if mods.shift {
                    d = geometry::constrain(d);
                }
                // Snapping to layers and the composition is on unless Cmd or Ctrl turns it off for
                // the drag (and the other way round); guides and the grid snap while their View
                // menu rows are on.
                let mut hits = Vec::new();
                let layers_too = self.session.state.snapping != mods.command;
                let pool: Vec<SnapTarget> =
                    targets.iter().filter(|t| layers_too || matches!(t.source, SnapSource::Guide | SnapSource::Grid)).cloned().collect();
                if !pool.is_empty() {
                    let moved: Vec<Pt> = features.iter().map(|f| [f[0] + d[0], f[1] + d[1]]).collect();
                    if let Some(snap) = viewer::snap(&moved, &pool, SNAP_REACH / zoom) {
                        let keep = if mods.shift { [d[0] != 0.0, d[1] != 0.0] } else { [true, true] };
                        d = [d[0] + if keep[0] { snap.delta[0] } else { 0.0 }, d[1] + if keep[1] { snap.delta[1] } else { 0.0 }];
                        hits = snap.hits;
                    }
                }
                let step = [d[0] - applied[0], d[1] - applied[1]];
                if step[0].abs() > 1e-9 || step[1].abs() > 1e-9 {
                    let ids: Vec<u64> = layers.iter().map(|l| l.0).collect();
                    let (layers_json, merge) = (json!(ids), merge.clone());
                    *applied = d;
                    st.snaps = hits;
                    let _ = self.tool_run("layer.nudge", json!({ "layers": layers_json, "x": step[0], "y": step[1], "merge": merge }));
                } else {
                    st.snaps = hits;
                }
            }
            Gesture::Marquee { start, end, base } => {
                *end = p;
                let rect = [start[0], start[1], end[0], end[1]];
                let mut layers = base.clone();
                for id in with_ctx(&self.session, |ctx| geometry::in_rect(ctx, rect)).unwrap_or_default() {
                    if !layers.contains(&id) {
                        layers.push(id);
                    }
                }
                if layers != self.session.state.selected_layers {
                    let ids: Vec<u64> = layers.iter().map(|l| l.0).collect();
                    let _ = self.tool_run("layer.select", json!({ "layers": ids }));
                }
            }
            Gesture::Scale(scaling) => {
                let s = scaling.clone();
                self.scale_to(&s, p, mods);
            }
            Gesture::Rotate(turning) => {
                turning.last = geometry::unwrap_angle(turning.last, geometry::angle_about(turning.pivot, p));
                let t = (**turning).clone();
                let delta = t.last - t.first;
                for (id, start, sign) in &t.layers {
                    let mut r = start + delta * sign;
                    if mods.shift {
                        r = geometry::snap_angle(r, 15.0);
                    }
                    self.set_prop(*id, "transform/rotation", json!(r), &t.merge);
                }
            }
            Gesture::Draw { end, square, centred, .. } => {
                *end = p;
                *square = mods.shift;
                *centred = mods.alt;
            }
            Gesture::TextDrag { anchor } => {
                let anchor = *anchor;
                text::drag_select(self, anchor, p);
            }
            Gesture::Idle | Gesture::Press { .. } => {}
        }
    }

    /// Scales for a handle at `p`: the dragged layer keeps the opposite handle (or with Alt its
    /// centre) where it was; a corner scales each axis freely, and keeps the proportions while
    /// Shift is held.
    fn scale_to(&mut self, s: &Scaling, p: Pt, mods: ToolMods) {
        let Some(at) = s.start.to_layer(p) else { return };
        let bounds = s.start.bounds;
        let fixed = s.handle.fixed(bounds, mods.alt);
        let f = geometry::scale_factors(s.handle, s.handle.at(bounds), fixed, at, mods.shift);
        let scale = [s.scale[0] * f[0], s.scale[1] * f[1], s.scale[2]];
        self.set_prop(s.start.id, "transform/scale", json!(scale), &s.merge);
        if let Some(shift) = geometry::scale_shift(&s.start.m, &s.parent, s.start.anchor, fixed, f) {
            let position = [s.position[0] + shift[0], s.position[1] + shift[1], s.position[2]];
            self.set_prop(s.start.id, "transform/position", json!(position), &s.merge);
        }
        for (id, start) in &s.others {
            self.set_prop(*id, "transform/scale", json!([start[0] * f[0], start[1] * f[1], start[2]]), &s.merge);
        }
    }

    /// The press ends: a click on one of several selected layers selects just it, a drawn shape
    /// becomes a shape layer (and the tool goes back to Selection).
    pub(super) fn pointer_up(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) -> Option<Tool> {
        self.pointer_drag(st, p, mods);
        let gesture = std::mem::take(&mut st.gesture);
        st.snaps.clear();
        match gesture {
            Gesture::Press { layer, shift: false, .. } if self.session.state.selected_layers.len() > 1 => {
                let _ = self.tool_run("layer.select", json!({ "layers": [layer.0] }));
                None
            }
            Gesture::Draw { start, end, square, centred } => {
                let b = geometry::drawn_box(start, end, square, centred);
                let (w, h) = (b[2] - b[0], b[3] - b[1]);
                if w < SMALLEST_SHAPE || h < SMALLEST_SHAPE {
                    return None;
                }
                let kind = if st.tool == Tool::Ellipse { "ellipse" } else { "rect" };
                // A new layer, never a shape added into the selected shape layer.
                if !self.session.state.selected_layers.is_empty() {
                    let _ = self.tool_run("edit.deselectAll", json!({}));
                }
                let centre = [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0];
                match self.tool_run("shape.newShape", json!({ "kind": kind, "size": [w, h], "position": centre })) {
                    Ok(_) => {
                        st.tool = Tool::Selection;
                        st.cursor = "default".into();
                        Some(Tool::Selection)
                    }
                    Err(error) => {
                        st.status = Some(error);
                        None
                    }
                }
            }
            Gesture::TextDrag { .. } => {
                st.gesture = Gesture::Idle;
                None
            }
            _ => None,
        }
    }

    /// Escape during a gesture: the gesture is undone (a marquee restores the selection it began
    /// with, a shape being drawn is dropped).
    pub(super) fn cancel(&mut self, st: &mut ToolState) {
        let gesture = std::mem::take(&mut st.gesture);
        st.snaps.clear();
        let merge = match &gesture {
            Gesture::Move { merge, .. } => Some(merge.as_str()),
            Gesture::Scale(s) => Some(s.merge.as_str()),
            Gesture::Rotate(t) => Some(t.merge.as_str()),
            _ => None,
        };
        // Only a gesture that changed something left its step on top of the history.
        if merge.is_some() && self.session.history.merge_key.as_deref() == merge {
            let _ = self.tool_run("edit.undo", json!({}));
            return;
        }
        if let Gesture::Marquee { base, .. } = gesture {
            let ids: Vec<u64> = base.iter().map(|l| l.0).collect();
            let _ = self.tool_run("layer.select", json!({ "layers": ids }));
        }
    }
}

/// A double-click with the Selection tool on a precomp layer (the topmost layer under the
/// pointer) opens its composition (`layer.openSource`), as a double-click on its bar does.
pub(super) fn open_precomp(doc: &mut MotionDocument, st: &mut ToolState, p: Pt) {
    let hit = with_ctx(&doc.session, |ctx| {
        geometry::pick(ctx, p, &|_| true).filter(|id| ctx.comp.layer(*id).is_some_and(|l| matches!(l.source, LayerSource::Comp { .. })))
    })
    .flatten();
    let Some(id) = hit else { return };
    st.gesture = Gesture::Idle;
    let _ = doc.tool_run("layer.openSource", json!({ "layer": id.0 }));
}
