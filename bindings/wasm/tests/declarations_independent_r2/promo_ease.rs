use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "keyframe.applyEase",
        path: "curve; segments whose both endpoints lie in snapped range",
        resolves: &[
            "comp:id|name|active",
            "layer:id|numeric-index-fallback|#index|name|first-selected",
            "prop:uid or prop/path string; bare path tries transform prefix",
            "from/to:layer seconds default or comp seconds by timeBase; snap in comp grid then convert layer time",
        ],
        reads: &[
            "property.value.interpolates",
            "property.hold_only",
            "property.keys count/times",
            "property.spatial",
            "each eligible segment endpoint value/time/out_interp",
            "spatial_segment_length uses spatial tangents and neighbor keys for auto tangents",
            "layer.start_time/stretch",
            "comp.frame_rate",
        ],
        checks: &[
            "has_comp",
            "property exists",
            "shape preset valid",
            "non-hold interpolating property",
            "at least two keys",
            "finite time and nonzero finite stretch",
            "two consecutive keys in range",
        ],
        writes: &[
            "segment start out_interp/out_ease/auto_bezier/continuous",
            "segment end in_interp/in_ease/auto_bezier/continuous",
            "overshoot spatial branch inserts turning-point keys with chord values",
        ],
        state: &["part", "selection if layer absent", "writes selected_keys clear"],
        source: "commands/promo_ease.rs:223-279,423-469; commands/prop.rs:13-47,163-200; keyframe::spatial_segment_length closure",
    },
    Row {
        command: "keyframe.applyEase",
        path: "spring expression",
        resolves: &["same property and range resolution as curve; range computed even though spring does not consume range"],
        reads: &[
            "property.value.interpolates",
            "property.hold_only",
            "property.keys count",
            "layer.start_time/stretch and comp.frame_rate for parsed from/to validation",
        ],
        checks: &["has_comp", "property exists", "valid spring preset", "non-hold interpolating property", "at least two keys", "key time inputs valid"],
        writes: &["property.expr text/enabled"],
        state: &["part", "selection if layer absent"],
        source: "commands/promo_ease.rs:396-469; commands/prop.rs:13-47,163-200",
    },
];
