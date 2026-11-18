use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "engine.batch",
        path: "successful steps, including result references",
        resolves: &["steps[].command or id -> command registry", "$N.member.index -> earlier step result", "all target references of substituted nested params"],
        reads: &["each nested command's result-dependent facts in execution order", "project and history snapshots for grouped undo", "earlier step results used by substitutions"],
        checks: &["steps is array", "each command exists and is enabled unless explicit-target bypass", "reject nested engine.batch/file.runScript/script.run", "references name an existing earlier result and member", "each nested command's parameter and eligibility checks"],
        writes: &["union of nested command writes", "history.undo folded to one step", "nested command editor state writes", "journal entries of nested commands"],
        state: &["union of nested command editor-state reads"],
        source: "crates/engine/engine/src/commands/batch.rs:54; crates/engine/engine/src/lib.rs:609; crates/engine/engine/src/lib.rs:676",
    },
    Row {
        command: "engine.batch",
        path: "failed nested step; atomic defaults true, atomic:false preserves prefix",
        resolves: &["substituted references of steps preceding first failure"],
        reads: &["project/history/editor snapshot before first step", "result-dependent facts of successful prefix and failed step"],
        checks: &["same nested parameter and eligibility checks as success path"],
        writes: &["atomic:true restores project/history/editor snapshot and bumps revision", "atomic:false retains successful prefix and folds its undo", "nested external/job/event side effects are not restored by project rollback"],
        state: &["nested command reads", "editor state snapshot"],
        source: "crates/engine/engine/src/commands/batch.rs:58; crates/engine/engine/src/commands/batch.rs:78; crates/engine/engine/src/commands/batch.rs:97",
    },
];
