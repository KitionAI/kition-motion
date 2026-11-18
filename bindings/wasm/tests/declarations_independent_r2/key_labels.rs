use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "keys.setLabel",
        path: "selected stored keys",
        resolves: &["active comp", "selected_keys: layer id, property uid, exact stored time"],
        reads: &[
            "property.keys membership and selected key times",
            "property.spatial",
            "all keys roving flags; when roving: boundary times, values and spatial handles",
        ],
        checks: &["active comp and selected keys nonempty", "label name or index 0..16", "missing layer/property/key skipped"],
        writes: &[
            "selected key.label",
            "retime_roving: endpoint roving flags and roving key times/in_interp/out_interp",
            "selected_keys remapped after roving",
        ],
        state: &["active_comp", "selected_keys"],
        source: "commands/key_labels.rs:24; commands/prop.rs:850; keyframe/src/lib.rs:431",
    },
    Row {
        command: "keys.selectLabelGroup",
        path: "selection only",
        resolves: &["active comp", "selected_keys exact layer/property/time", "scope selected|all|visibleSelected|visibleAll; visible property uids"],
        reads: &["selected keys labels", "layer/property/key membership", "candidate key labels and times"],
        checks: &["active comp", "at least one selected key resolves", "scope recognized"],
        writes: &["editor selected_keys"],
        state: &["active_comp", "selected_keys", "selected_layers"],
        source: "commands/key_labels.rs:38",
    },
];
