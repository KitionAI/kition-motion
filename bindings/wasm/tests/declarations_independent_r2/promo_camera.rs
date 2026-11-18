use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "camera.move",
        path: "pushHover|craneRise|orbit; explicit point/default center/target center",
        resolves: &[
            "comp:id|name|active",
            "target:layer refs if supplied even with point",
            "layers:explicit layer refs else target; no selection default",
            "new camera/rig inserted at top gap",
        ],
        reads: &[
            "comp.width/height/duration/frame_rate",
            "layer names for unique camera and rig names",
            "target source and source dimensions when deriving center",
            "target sourceText value/keys/layout for text center",
            "target transforms and parent chains via layer_to_comp and expressions",
            "project.next_id",
        ],
        checks: &["has_comp", "preset valid", "explicit target/layers resolve", "point valid", "finite and nonnegative numeric options"],
        writes: &[
            "comp.layers insert camera and 3D rig",
            "explicit/default target layers.switches.three_d=true",
            "unless motionBlur=false: target switches.motion_blur, comp.enable_motion_blur/shutter_angle/motion_blur_samples",
            "project.next_id",
        ],
        state: &["part", "time when start absent", "writes selected_layers"],
        source: "commands/promo_camera.rs:35-63,174-315; project/build.rs:737-850; promo_ease.rs:337-395; EvalCtx::layer_to_comp closure",
    },
    Row {
        command: "camera.move",
        path: "parallax; first/middle/last layers",
        resolves: &["comp", "layers:at least two explicit layer refs ordered far to near"],
        reads: &[
            "comp.frame_rate",
            "each layer.start_time/stretch",
            "transform.position existence/value or keys evaluation at start/spatial",
            "position.hold_only/spatial",
            "source dimensions or comp.width/height",
            "effects membership and child names when adding depth effects",
            "effect specs defaults",
            "project.next_id",
        ],
        checks: &[
            "has_comp",
            "at least two layers",
            "speeds count matches layers and values finite",
            "direction nonzero pair",
            "position property exists",
            "numeric options valid",
            "effect specs available when effects group exists and requested depth effect nonzero",
        ],
        writes: &[
            "each transform.position.value and entire keys list",
            "first/last effects.children append requested depth blur/desaturation",
            "unless motionBlur=false: each switches.motion_blur and comp.enable_motion_blur/shutter_angle/motion_blur_samples",
            "project.next_id",
        ],
        state: &["part", "time when start absent"],
        source: "commands/promo_camera.rs:88-172; render/eval.rs:317-340; promo_ease.rs:337-395",
    },
];
