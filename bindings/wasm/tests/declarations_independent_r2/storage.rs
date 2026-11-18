use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row { command: "storage.persist", path: "host request; unsupported when no browser StorageHost", resolves: &[], reads: &["storage host availability"], checks: &["has_host enablement and host()"], writes: &["external host persistence request only"], state: &[], source: "crates/engine/engine/src/storage.rs:61; crates/engine/engine/src/storage.rs:41" },
    Row { command: "storage.clear", path: "external host cache/media/projects/autosaves/all", resolves: &["what enum external storage collection, not document object"], reads: &["storage host availability"], checks: &["what in CLEARABLE", "host supplied", "host clear result"], writes: &["host-owned storage deleted", "toast event"], state: &[], source: "crates/engine/engine/src/storage.rs:65; crates/engine/engine/src/storage.rs:41" },
];
