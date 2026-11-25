use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "keyframe.wiggle",
        path: "replace sampled selected run",
        resolves: &["selected_keys grouped layer/property/exact first-last times in active comp"],
        reads: &[
            "comp.frame_rate",
            "layer.start_time/stretch",
            "property.value kind/interpolates",
            "property.spatial",
            "keys in run and interpolation neighbours used by evaluate",
            "property uid seeds stream",
        ],
        checks: &[
            "at least two selected references of one property",
            "noise/apply/dimensions recognized",
            "frequency positive",
            "property interpolates and endpoint keys exist",
        ],
        writes: &["property.keys inside selected bounds replaced with seeded sampled deviations", "selected_keys replaced with run keys"],
        state: &["active_comp", "selected_keys"],
        source: "commands/anim_tools.rs:255,311; commands/anim_tools.rs:97; keyframe evaluate",
    },
    Row {
        command: "keyframe.smooth",
        path: "replace sampled selected run",
        resolves: &["selected_keys grouped layer/property/exact endpoint times in active comp"],
        reads: &[
            "comp.frame_rate",
            "layer.start_time/stretch",
            "property.value kind/interpolates",
            "property.spatial",
            "keys in run and interpolation neighbours used by evaluate",
        ],
        checks: &["at least three selected references of one property", "property interpolates and endpoints exist"],
        writes: &["property.keys inside selected bounds replaced with minimal auto-Bezier approximant", "selected_keys replaced with run keys"],
        state: &["active_comp", "selected_keys"],
        source: "commands/anim_tools.rs:255,316; commands/anim_tools.rs:169,198",
    },
    Row {
        command: "motion.sketch",
        path: "sample recording into position keys",
        resolves: &["comp/layer through layer_p", "start supplied else current time", "points parameter [x,y] uses comp frame duration"],
        reads: &[
            "comp.frame_rate/duration",
            "layer.start_time/stretch",
            "transform.position.value kind and preserved Z",
            "position keys in destination range for overwritten assignment; outside kept",
        ],
        checks: &["comp/layer exists", "at least two valid recording points", "position exists"],
        writes: &["position.keys in recording time range", "selected_keys"],
        state: &["active_comp", "selected_layers default", "current time when start omitted"],
        source: "commands/anim_tools.rs:326,366; commands/anim_tools.rs:169",
    },
];
