//! The camera tools: Orbit, Pan and Dolly. A drag on the canvas moves the view of the 3D layers:
//! the active camera layer when the viewer shows the Active Camera (one undo step per drag, with
//! a keyframe at the current time on an animated camera, as `camera.orbit`, `camera.pan` and
//! `camera.dolly` make it), or the 3D view itself when the viewer shows Front, Left, Top or a
//! Custom View (the person's view: no undo step). A composition without 3D layers, or showing the
//! Active Camera without a camera layer, says so in the status bar and does nothing.

use serde_json::json;

use kition_kit::tools::ToolMods;

use super::{Gesture, Pt, Tool, ToolState};
use crate::MotionDocument;

/// Degrees the Orbit tool turns per screen pixel dragged: about half a turn across a viewer
/// 1100 pixels wide, so a short drag turns the view a little (0.4 swung the camera to the side
/// with a 150-pixel drag).
const ORBIT_DEGREES_PER_PIXEL: f64 = 0.16;
/// Screen pixels a Shift-drag goes before it keeps to the direction it went most.
const AXIS_AFTER: f64 = 4.0;
/// Composition pixels the Dolly tool moves per screen pixel dragged.
const DOLLY_PER_PIXEL: f64 = 4.0;

impl MotionDocument {
    /// Why the camera tools cannot act now, or None.
    fn camera_refusal(&self) -> Option<&'static str> {
        let comp = self.session.active_comp()?;
        if !comp.has_3d() {
            return Some("The camera tools move the view of 3D layers: turn on a layer's 3D switch first");
        }
        let id = self.session.active_comp_id()?;
        let active_view = self.session.state.views3d.get(&id).is_none_or(|views| views.current == kition_motion_engine::render::three_d::View3D::ActiveCamera);
        if active_view && comp.active_camera(self.session.time()).is_none() {
            return Some("No camera layer to move: add one (Layer ▸ New ▸ Camera) or choose another 3D view");
        }
        None
    }

    pub(super) fn camera_down(&mut self, st: &mut ToolState, p: Pt) {
        if let Some(why) = self.camera_refusal() {
            st.status = Some(why.to_owned());
            return;
        }
        let merge = st.merge_key("camera");
        st.gesture = Gesture::Camera { start: p, last: p, axis: None, merge };
    }

    /// A drag's move. With Shift held it keeps to one direction (across or up and down), the one
    /// it went most once it has gone a few pixels: Orbit turns about one axis only.
    pub(super) fn camera_drag(&mut self, st: &mut ToolState, p: Pt, mods: ToolMods) {
        let zoom = st.zoom;
        let Gesture::Camera { start, last, axis, merge } = &mut st.gesture else { return };
        // Screen pixels dragged (the pointer arrives in composition pixels).
        let (mut dx, mut dy) = ((p[0] - last[0]) * zoom, (p[1] - last[1]) * zoom);
        if mods.shift {
            let (tx, ty) = ((p[0] - start[0]) * zoom, (p[1] - start[1]) * zoom);
            if axis.is_none() && tx.hypot(ty) >= AXIS_AFTER {
                *axis = Some(tx.abs() >= ty.abs());
            }
            match axis {
                Some(true) => dy = 0.0,
                Some(false) => dx = 0.0,
                // Not far enough yet to know the direction: nothing moves.
                None => return,
            }
        } else {
            *axis = None;
        }
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        *last = p;
        let merge = merge.clone();
        let (command, params) = match st.tool {
            Tool::Orbit => ("camera.orbit", json!({ "yaw": dx * ORBIT_DEGREES_PER_PIXEL, "pitch": -dy * ORBIT_DEGREES_PER_PIXEL, "merge": merge })),
            Tool::Track => ("camera.pan", json!({ "dx": dx / st.zoom, "dy": dy / st.zoom, "merge": merge })),
            _ => ("camera.dolly", json!({ "amount": (dx - dy) * DOLLY_PER_PIXEL / st.zoom, "merge": merge })),
        };
        if let Err(error) = self.tool_run(command, params) {
            st.status = Some(error);
        }
    }
}
