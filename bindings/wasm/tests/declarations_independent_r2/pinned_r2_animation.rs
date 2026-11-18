use super::Row;

// These registry omissions were added after declaration exposure and are not blind evidence.
pub(super) const ROWS: &[Row] = &[
    Row {
        command: "shape.newPath",
        path: "PINNED FROM DECLARATION AFTER REVIEW: existing shape layer",
        resolves: &[
            "comp explicit/default active",
            "explicit layer id/index/name else first selected unlocked Shape; name without layer forces new layer",
            "vertices/tangents layer space unless space comp",
        ],
        reads: &[
            "target Shape/implicit unlocked eligibility",
            "unsupplied shape_tool paint defaults",
            "comp-space layer/parent/camera projection at time_of(comp); singular inverse falls back identity",
            "contents group names and insertion gap",
            "project.next_id",
        ],
        checks: &["nonempty valid input vertices", "valid paint inputs", "shape target contents exists"],
        writes: &["new shape group/path/paint inserted at top of contents", "project.next_id", "selected_layers", "selected_vertices last new point"],
        state: &["active_comp", "selected_layers default", "shape_tool", "time_of(comp) for comp-space transform"],
        source: "commands/viewer_cmds.rs:221; commands/shape_tool.rs:107,267,317; newly read after review",
    },
    Row {
        command: "shape.newPath",
        path: "PINNED FROM DECLARATION AFTER REVIEW: new shape layer",
        resolves: &["new layer when name without layer or no eligible target", "comp-space default; omitted space assumes centred untransformed new layer"],
        reads: &[
            "comp width/height for coordinate conversion and layer defaults",
            "comp.duration",
            "unsupplied shape_tool paint defaults",
            "comp layer names/source counts and insertion gap",
            "project.next_id",
        ],
        checks: &["nonempty vertices", "valid paint inputs"],
        writes: &["comp.layers insert Shape layer with path/group/paint", "project.next_id", "selected_layers/vertices"],
        state: &["active_comp", "selected_layers", "shape_tool", "layer insertion preferences"],
        source: "commands/viewer_cmds.rs:221; commands/shape_tool.rs:283,317; newly read after review",
    },
    Row {
        command: "text.fontFeatures",
        path: "PINNED FROM DECLARATION AFTER REVIEW: query-only runtime write classification",
        resolves: &["explicit font/style else text target: explicit layer in comp, edited layer, first selected Text; layer_doc uses active comp"],
        reads: &[
            "sampled sourceText at CTI and layer timing when font omitted",
            "text_edit.range.start chooses character style",
            "resolved font face/features",
        ],
        checks: &["font supplied or Text target"],
        writes: &[],
        state: &["active_comp", "selected_layers/text_edit when font omitted", "current time when font omitted"],
        source: "commands/text_edit.rs:43,53,475; handler read blind but row omitted before review; no document write",
    },
];
