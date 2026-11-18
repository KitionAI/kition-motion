use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "camera.linkFocusToPoi", path: "camera explicit or first selected camera; assign fixed expression",
        resolves: &["comp: id/numeric-string/name or active", "camera: layer id/numeric-string/#index/name or first selected camera", "layer if supplied resolves even though result unused"],
        reads: &["camera.source determines is_camera", "camera.props path cameraOptions/focusDistance exists", "camera.props.cameraOptions.focusDistance.expr (same-field assignment)"],
        checks: &["camera exists and is camera", "optional explicit layer exists", "focusDistance property exists", "camera_selected enablement unless explicit-target bypass"],
        writes: &["camera.props.cameraOptions.focusDistance.expr = fixed enabled expression"],
        state: &["active_comp when comp omitted", "selected_layers when camera omitted or enablement needed"],
        source: "crates/engine/engine/src/commands/focus.rs:15; crates/engine/engine/src/commands/focus.rs:39; crates/engine/engine/src/commands/mod.rs:453; crates/engine/engine/src/lib.rs:609",
    },
    Row {
        command: "camera.linkFocusToLayer", path: "explicit or selected target; expression quotes target name",
        resolves: &["comp as comp_id", "camera via refs", "layer: id/numeric-string/#index/name or first selected non-camera-id layer"],
        reads: &["camera.source", "target.name copied into expression", "camera.props.cameraOptions.focusDistance existence and expr assignment"],
        checks: &["camera is camera", "target required", "focusDistance exists", "camera_selected enablement unless explicit-target bypass"],
        writes: &["camera.props.cameraOptions.focusDistance.expr = enabled expression containing target.name"],
        state: &["active_comp if comp omitted", "selected_layers for omitted refs or enablement"],
        source: "crates/engine/engine/src/commands/focus.rs:15; crates/engine/engine/src/commands/focus.rs:61; crates/engine/engine/src/commands/focus.rs:39",
    },
    Row {
        command: "camera.setFocusToLayer", path: "evaluate depth; static focusDistance",
        resolves: &["comp as comp_id", "camera and target as refs", "parents followed by id up to 64 levels", "expression references resolved by expression host when enabled"],
        reads: &["camera.source", "camera parent chain and evaluated position/orientation/rotationX/rotationY/rotation/poi and auto_orient", "target parent chain and evaluated transform including separated position, anchor, scale, rotations, orientation, auto_orient", "target source pixel_aspect and comp.pixel_aspect", "comp.width/height for missing camera transform defaults", "transform keyframes/expressions/start_time/stretch for evaluations", "active camera and parent chain when target auto_orient=TowardsCamera", "position at time +/-0.005 for AlongPath", "focusDistance.keys.is_empty/hold_only/value (assigned)", "expression result dependencies including referenced layers/comps/media"],
        checks: &["camera type", "target exists", "depth > 0", "focusDistance exists"],
        writes: &["camera.props.cameraOptions.focusDistance.value = evaluated depth"],
        state: &["active_comp fallback", "selected_layers fallback and enablement", "time_of resolved comp"],
        source: "crates/engine/engine/src/commands/focus.rs:69; crates/engine/engine/src/commands/focus.rs:81; crates/render/render/src/eval.rs:52; crates/render/render/src/eval.rs:135; crates/render/render/src/eval.rs:192; crates/render/render/src/three_d/camera.rs:203; crates/engine/project/src/props.rs:133",
    },
    Row {
        command: "camera.setFocusToLayer", path: "animated focusDistance",
        resolves: &["same comp/camera/target/evaluation references as static path", "key destination = camera.layer_time(time_of(comp))"],
        reads: &["all result-dependent depth inputs of static path", "camera.start_time/stretch for key address", "focusDistance.keys membership determines animated branch", "focusDistance.hold_only", "focusDistance.keys[time=destination] assigned entry including ease defaults"],
        checks: &["same camera/target/depth/property eligibility as static path"],
        writes: &["focusDistance.keys[time=destination] new/replaced key with depth and default/hold interpolation"],
        state: &["active_comp and selected_layers fallbacks", "time_of resolved comp"],
        source: "crates/engine/engine/src/commands/focus.rs:81; crates/engine/project/src/props.rs:133; crates/engine/project/src/lib.rs:857",
    },
];
