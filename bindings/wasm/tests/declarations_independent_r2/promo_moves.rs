use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "layer.shake",
        path: "explicit layer/camera or camera=true picks top enabled camera without time eligibility",
        resolves: &["comp:id|name|active", "layer:ref wins; else camera:true top source Camera with video=true; else camera:ref must Camera"],
        reads: &[
            "chosen layer.source",
            "earlier camera.source/video for camera=true",
            "transform.position/poi(if camera)/rotation property existence and existing enabled expr.text",
        ],
        checks: &[
            "has_comp",
            "explicit target valid",
            "camera ref is camera",
            "at least one writable transform property",
            "finite/nonnegative options and seed range",
        ],
        writes: &["existing transform.position.expr", "existing camera transform.poi.expr", "existing transform.rotation.expr"],
        state: &["part", "time if frame absent (frame parameter is seconds)"],
        source: "commands/promo_moves.rs:22-156",
    },
    Row {
        command: "layer.handheld",
        path: "same target resolution; float over explicit/default comp duration",
        resolves: &["same target resolution as shake"],
        reads: &["shake target/expr facts", "comp.duration only for default end"],
        checks: &[
            "has_comp",
            "at least one writable transform property",
            "end after start",
            "holds finite ordered ranges",
            "finite/nonnegative options and seed range",
        ],
        writes: &["existing transform.position.expr", "existing camera transform.poi.expr", "existing transform.rotation.expr"],
        state: &["part"],
        source: "commands/promo_moves.rs:22-108,160-212",
    },
    Row {
        command: "layer.hop",
        path: "flight then squash/ring-out",
        resolves: &["comp", "layer:refs|first-selected", "to:numeric point in position space"],
        reads: &[
            "comp.frame_rate",
            "layer.start_time/stretch",
            "transform.position/anchor/scale at start value/keys (no expressions in tr3)",
            "local_rect text glyph/shape/effect bounds closure",
            "position/scale existing keys outside planned range preserved; hold_only; keys emptiness",
        ],
        checks: &["has_comp", "to valid point", "finite/nonnegative option ranges"],
        writes: &[
            "transform.position/scale keys within planned interval and base if originally static",
            "unless motionBlur=false layer.switches.motion_blur and comp.enable_motion_blur/shutter_angle/motion_blur_samples",
        ],
        state: &["part", "selection if layer absent", "time if start absent", "writes selected_layers"],
        source: "commands/promo_moves.rs:216-280; promo_carry.rs:59-142; promo_ease.rs:365-395",
    },
    Row {
        command: "layer.gather",
        path: "ordered explicit layers gather onto ring",
        resolves: &["comp", "layers:nonempty explicit array refs in ring slot order"],
        reads: &[
            "comp.frame_rate",
            "comp.width/height for default center",
            "each layer.start_time/stretch",
            "each transform.position.value/keys at staggered start (no expression)",
            "position keys outside planned range preserved; hold_only; keys emptiness",
        ],
        checks: &["has_comp", "nonempty explicit layers", "finite/nonnegative options"],
        writes: &[
            "each transform.position keys within planned interval and base if originally static",
            "unless motionBlur=false switches.motion_blur and comp.enable_motion_blur/shutter_angle/motion_blur_samples",
        ],
        state: &["part", "time if start absent", "writes selected_layers"],
        source: "commands/promo_moves.rs:284-344; promo_carry.rs:92-142; promo_ease.rs:365-395",
    },
];
