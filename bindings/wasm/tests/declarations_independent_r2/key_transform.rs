use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "keyframe.transform",
        path: "transform current selected keys",
        resolves: &["active comp", "selected_keys layer/property/exact time", "timeAnchor default earliest selected key in comp time"],
        reads: &[
            "comp.frame_rate",
            "selected layers.start_time/stretch",
            "selected property.keys: selected records, destination collisions and roving runs",
            "selected property.spatial",
        ],
        checks: &["active comp and nonempty selected keys", "timeScale finite and positive", "missing layer/property skipped"],
        writes: &[
            "selected keys.time",
            "selected keys.value if valueScale != 1 or valueOffset != 0",
            "destination key records replaced on collision",
            "property.keys order/dedup",
            "endpoint roving flags and roving time/interpolation",
            "editor selected_keys",
        ],
        state: &["active_comp", "selected_keys", "history.merge_key", "key_move"],
        source: "commands/key_transform.rs:25; keyframe/src/lib.rs:431",
    },
    Row {
        command: "keyframe.transform",
        path: "fromStart merged gesture",
        resolves: &["active comp", "key_move.from selection against history undo base"],
        reads: &[
            "entire history undo base project replaces current project",
            "base comp.frame_rate",
            "base layers.start_time/stretch",
            "base keys as current-selection branch",
        ],
        checks: &["fromStart true, merge present and matches history/key_move", "timeScale finite positive"],
        writes: &["entire project restored from undo base then transformed keys", "key_move", "selected_keys"],
        state: &["active_comp", "history.undo last project", "history.merge_key", "key_move"],
        source: "commands/key_transform.rs:28,69",
    },
    Row {
        command: "keyframe.setSpatialTangents",
        path: "nearest key, both sides explicit or mirror unspecified side",
        resolves: &["prop_ref comp/layer/property uid or path", "time: layer seconds, exact match else nearest stored key"],
        reads: &[
            "property.spatial",
            "key times and nearest ordering",
            "key.spatial_auto and neighbouring values when auto",
            "existing spatial_in/out when break preserves missing side",
            "roving runs, boundary times and spatial arc geometry",
        ],
        checks: &["active comp", "at least one valid in/out vector", "property spatial", "property has keys"],
        writes: &[
            "chosen key.spatial_auto",
            "chosen key.spatial_continuous",
            "chosen key.spatial_in/out",
            "retime_roving: endpoint roving flags and roving time/interpolation",
        ],
        state: &["active_comp", "selected_layers when omitted"],
        source: "commands/key_transform.rs:128; commands/prop.rs:13; keyframe/src/lib.rs:332,431",
    },
];
