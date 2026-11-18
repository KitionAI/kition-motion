use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "keys.audioToKeyframes",
        path: "sample work-area audible mix and create amplitude null",
        resolves: &["comp id/name/default active", "all recursively audible layers and source item references"],
        reads: &[
            "comp.frame_rate/work_area/width/height/duration",
            "layer names/order for new null insertion/name",
            "audio eligibility source/switches.audio/solo/effects/stretch/in_point/out_point",
            "source footage has_audio/path/interpretation and lent audio samples",
            "nested comp audio graph depth<=16",
            "layer.start_time/stretch",
            "audio levels keys/expr and enabled audio effect parameters including expression dependencies",
            "slider effect registry defaults",
            "project.next_id",
        ],
        checks: &["comp exists", "slider effect registered", "cycle/depth audio branches silent", "new null effects group exists"],
        writes: &[
            "comp.layers insert Audio Amplitude null",
            "new layer effects: Left/Right/Both Channels with per-frame slider keys",
            "project.next_id",
            "selected_layers",
        ],
        state: &["active_comp default", "work_area", "footage provider and expr host", "new null preferences"],
        source: "commands/keys_more.rs:71; render/src/audio.rs:26,229; commands/layer.rs:244",
    },
    Row {
        command: "keys.rpfCameraImport",
        path: "parse lent JSON/CSV and create keyed camera",
        resolves: &["comp id/name/default active", "path lent file"],
        reads: &[
            "comp.frame_rate default when input lacks frameRate",
            "comp.width for FOV-to-zoom",
            "comp.height/duration and names/order through layer.newCamera",
            "input bytes",
            "new camera defaults",
            "project.next_id",
        ],
        checks: &["rpf/rla unsupported", "file readable", "JSON/CSV parseable and camera samples nonempty"],
        writes: &[
            "comp.layers insert one-node camera",
            "new camera position/orientation keys",
            "zoom keys when samples supply zoom/fov",
            "project.next_id",
            "selected_layers",
        ],
        state: &["active_comp default", "new camera preferences"],
        source: "commands/keys_more.rs:133,204; commands/three_d.rs new_camera",
    },
];
