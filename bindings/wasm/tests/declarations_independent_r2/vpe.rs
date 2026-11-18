use super::Row;

pub(super) const ROWS: &[Row] = &[Row {
    command: "file.importVanishingPoint",
    path: "always unsupported",
    resolves: &[],
    reads: &[],
    checks: &["enabled always returns undocumented-format error; handler also always errors"],
    writes: &[],
    state: &[],
    source: "commands/vpe.rs:11-27",
}];
