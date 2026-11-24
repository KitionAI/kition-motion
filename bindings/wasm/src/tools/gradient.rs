//! A shape layer's gradient fills and strokes on the canvas: with the Selection tool, each
//! gradient of the selected shape layers shows its start and end points joined by a dashed line,
//! and either point drags (`prop.set` on Start Point or End Point, keyed at the current time when
//! animated; one undo step a drag).

use kition_kit::tools::ToolOverlay;
use kition_motion_engine::Session;
use kition_motion_engine::commands::prop_path;
use kition_motion_engine::geom::{Mat3, vec2};
use kition_motion_engine::project::{LayerId, LayerSource, PropGroup, Value as KV};
use serde_json::json;

use super::geometry::Pt;
use super::{Gesture, SELECTION, ToolState, selected_boxes, with_ctx};
use crate::MotionDocument;

/// How near (screen pixels) a press must be to a gradient point to take it.
const REACH: f64 = 7.0;
/// How deep shape groups are searched.
const MAX_DEPTH: usize = 32;

/// One gradient point: its layer, the property path (`contents/…/gfill/start`), and where it is now.
pub(crate) struct GradientPoint {
    pub layer: LayerId,
    pub path: String,
    /// The gradient's space → comp pixels.
    pub m: Mat3,
    pub at: Pt,
}

fn gradients<'a>(g: &'a PropGroup, depth: usize, out: &mut Vec<&'a PropGroup>) {
    if depth > MAX_DEPTH {
        return;
    }
    for sub in g.groups() {
        if matches!(sub.match_id.as_str(), "gfill" | "gstroke") {
            out.push(sub);
        } else if sub.match_id == "group"
            && let Some(inner) = sub.sub("contents")
        {
            gradients(inner, depth + 1, out);
        }
    }
}

/// The selected shape layers' gradient points (start and end of each gradient, in that order).
pub(crate) fn points(session: &Session) -> Vec<(GradientPoint, GradientPoint)> {
    let shapes =
        session.active_comp().is_some_and(|c| c.layers.iter().any(|l| session.state.selected_layers.contains(&l.id) && matches!(l.source, LayerSource::Shape)));
    if !shapes {
        return Vec::new();
    }
    let boxes = selected_boxes(session);
    with_ctx(session, |ctx| {
        let mut out = Vec::new();
        for b in boxes.iter().filter(|b| b.flat) {
            let Some(layer) = ctx.comp.layer(b.id).filter(|l| matches!(l.source, LayerSource::Shape)) else { continue };
            let Some(contents) = layer.props.sub("contents") else { continue };
            let t = layer.layer_time(ctx.time);
            let mut found = Vec::new();
            gradients(contents, 0, &mut found);
            for item in found {
                let m = b.m * kition_motion_engine::render::shapes::item_matrix(ctx, layer, item.uid).unwrap_or(Mat3::IDENTITY);
                let point = |name: &str| {
                    let prop = item.get(name)?;
                    let KV::Vec2(v) = prop.value_at(t) else { return None };
                    let q = m.apply(vec2(v[0], v[1]));
                    Some(GradientPoint { layer: b.id, path: prop_path(&layer.props, prop.uid)?, m, at: [q.x, q.y] })
                };
                if let (Some(start), Some(end)) = (point("start"), point("end")) {
                    out.push((start, end));
                }
            }
        }
        out
    })
    .unwrap_or_default()
}

pub(super) fn overlays(session: &Session, out: &mut Vec<ToolOverlay>) {
    for (start, end) in points(session) {
        out.push(ToolOverlay::Line { ax: start.at[0], ay: start.at[1], bx: end.at[0], by: end.at[1], color: SELECTION, dashed: true });
        out.push(ToolOverlay::Anchor { x: start.at[0], y: start.at[1], color: SELECTION, filled: true, size: 8.0 });
        out.push(ToolOverlay::Anchor { x: end.at[0], y: end.at[1], color: SELECTION, filled: false, size: 8.0 });
    }
}

impl MotionDocument {
    /// A press on a gradient point starts dragging it; answers whether one was there.
    pub(super) fn gradient_down(&mut self, st: &mut ToolState, p: Pt) -> bool {
        let reach = REACH / st.zoom;
        let hit = points(&self.session)
            .into_iter()
            .flat_map(|(a, b)| [a, b])
            .find(|point| ((point.at[0] - p[0]).powi(2) + (point.at[1] - p[1]).powi(2)).sqrt() <= reach);
        let Some(point) = hit else { return false };
        let merge = st.merge_key("gradient");
        st.gesture = Gesture::GradientPoint { layer: point.layer, path: point.path, merge };
        true
    }

    /// Drags the gradient point to `p` (in the gradient's own space).
    pub(super) fn gradient_drag(&mut self, st: &mut ToolState, p: Pt) {
        let Gesture::GradientPoint { layer, path, merge } = &st.gesture else { return };
        let (layer, path, merge) = (*layer, path.clone(), merge.clone());
        let Some(point) = points(&self.session).into_iter().flat_map(|(a, b)| [a, b]).find(|point| point.layer == layer && point.path == path) else { return };
        let Some(inverse) = point.m.inverse() else { return };
        let q = inverse.apply(vec2(p[0], p[1]));
        if !(q.x.is_finite() && q.y.is_finite()) {
            return;
        }
        let time = self.now();
        if let Err(error) =
            self.tool_run("prop.set", json!({ "layer": layer.0, "path": path, "value": [q.x, q.y], "time": time, "timeBase": "comp", "merge": merge }))
        {
            st.status = Some(error);
        }
    }
}
