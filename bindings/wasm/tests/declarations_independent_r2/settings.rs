use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row { command: "file.openRecent", path: "explicit path, zero-based recent index, or first recent", resolves: &["path host file; else index into prefs.recent_projects (default0)"], reads: &["recent_projects nonempty eligibility even explicit path", "indexed recent path when omitted", "file.open whole-project replacement and host parse dependencies"], checks: &["has_recent", "index in bounds", "file.open checks"], writes: &["file.open writes", "remove failed missing recent path outside host-only mode"], state: &["recent_projects preferences", "file.open editor reset"], source: "crates/engine/engine/src/commands/settings.rs:212; crates/engine/engine/src/commands/file.rs:40" },
    Row { command: "file.clearRecent", path: "clear preference list", resolves: &[], reads: &["recent_projects assigned list; nonempty eligibility"], checks: &["has_recent"], writes: &["prefs.recent_projects empty; prefs_revision; saved prefs"], state: &["recent_projects"], source: "crates/engine/engine/src/commands/settings.rs:231" },
    Row { command: "file.autoSave", path: "kit rejects host-only auto-save", resolves: &[], reads: &["host_files_only"], checks: &["autosave_now returns unavailable in kit"], writes: &[], state: &[], source: "crates/engine/engine/src/commands/settings.rs:243; crates/engine/engine/src/session_settings.rs:233" },
];
