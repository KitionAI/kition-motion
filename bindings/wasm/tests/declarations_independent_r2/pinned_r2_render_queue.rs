use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row { command: "renderQueue.list", path: "PINNED FROM DECLARATION AFTER REVIEW: polling query applies completed render", resolves: &["running render queue job and its captured entries"], reads: &["pending render completion and current queue entry identities", "captured render/post-render input dependency closure (unresolved)"], checks: &["job completion or no-op while pending"], writes: &["render status/time/output log", "post-render footage/proxy/source replacements through poll_render"], state: &["render job"], source: "crates/engine/engine/src/commands/render_queue.rs:690; crates/engine/engine/src/render_queue.rs:542; handler read before review but omitted from rows" },
];
