use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "layer.alignVideoToData",
        path: "explicit videoStart; explicit or first data footage",
        resolves: &[
            "comp:id|numeric-string|name|active",
            "layer:id|numeric-index-fallback|#index|name|first-selected",
            "data:numeric id|string exact item name|first Data footage item",
        ],
        reads: &[
            "layer.source.item",
            "source item Footage kind and frame_rate",
            "data item kind/data/codec",
            "first readable timestamp from requested or default time column",
            "comp.frame_rate",
            "layer.start_time",
            "layer.in_point",
            "layer.out_point",
        ],
        checks: &[
            "has_layers",
            "target source is non-Data Footage",
            "data item is Footage and has data",
            "column exists",
            "at least one parseable timestamp",
            "videoStart parses",
        ],
        writes: &["layer.start_time", "layer.in_point", "layer.out_point"],
        state: &["part", "selection if layer absent"],
        source: "commands/align_data.rs:157-237; commands/mod.rs:460-509",
    },
    Row {
        command: "layer.alignVideoToData",
        path: "videoStart absent or empty; recording date fallback",
        resolves: &["same target and data resolution as explicit branch", "source file path recording date"],
        reads: &["explicit branch facts", "source footage.path", "host_files_only", "source file creation time when host permits file access"],
        checks: &["explicit branch eligibility", "source creation time available (kit host denies file-date access)"],
        writes: &["layer.start_time", "layer.in_point", "layer.out_point"],
        state: &["part", "selection if layer absent"],
        source: "commands/align_data.rs:195-215; media_browser::file_dates",
    },
];
