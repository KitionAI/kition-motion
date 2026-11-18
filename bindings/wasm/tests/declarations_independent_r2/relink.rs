use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "file.relinkFootage",
        path: "wait=false background or wait=true inline, dryRun=false",
        resolves: &[
            "all project Footage items whose nonempty source path missing",
            "folder recursive desktop search exact basenames; no symlink traversal",
            "source sequence members mapped by basename to candidate folder",
        ],
        reads: &[
            "all footage item membership/kind/path and host existence",
            "supported footage layer/page/kind",
            "matching candidate metadata compared with source width/height/has_video/has_audio/codec/native_rate/frame_rate/duration",
            "old source sequence paths",
            "complete old Footage copied before replacing path/sequence/missing",
        ],
        checks: &[
            "desktop filesystem permitted; folder exists/is directory; importer available",
            "search <=100000 entries; task not cancelled",
            "unique exact filename candidate and compatible source metadata; full sequence exists",
            "apply rechecks target complete Footage == old, old path still absent, new path and sequence still exist",
        ],
        writes: &["matched item.kind replaces Footage with copied metadata and new path/sequence/missing=false"],
        state: &["no editor state dependency; task may complete after outer command; apply has own stale-footage check"],
        source: "crates/engine/engine/src/commands/relink.rs:19; crates/engine/engine/src/commands/relink.rs:52; crates/engine/engine/src/commands/relink.rs:75; crates/engine/engine/src/commands/relink.rs:154",
    },
    Row {
        command: "file.relinkFootage",
        path: "dryRun=true",
        resolves: &["same missing footage/candidate scan"],
        reads: &["same scan and stale-footage eligibility facts as dryRun=false"],
        checks: &["same apply-time stale checks even though no document writes"],
        writes: &[],
        state: &["background task state only"],
        source: "crates/engine/engine/src/commands/relink.rs:75; crates/engine/engine/src/commands/relink.rs:154",
    },
    Row {
        command: "file.relinkFootage",
        path: "kit binding host_files_only",
        resolves: &["folder argument validated before filesystem guard"],
        reads: &[],
        checks: &["need_file_system refuses this command in kit binding"],
        writes: &[],
        state: &[],
        source: "crates/engine/engine/src/commands/relink.rs:77; bindings/wasm/src/lib.rs:173",
    },
];
