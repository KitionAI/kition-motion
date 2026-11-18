use super::Row;

macro_rules! frame {
    ($id:literal) => { Row { command: $id, path: "layered frame host export", resolves: &["comp via comp_id", "time seconds else active s.time() even explicit other comp", "path host destination"], reads: &["comp settings and layer order", "layer video/active/adjustment/guide/environment/solo eligibility", "layer name/blend/opacity at layer_time", "render/evaluation source/precomp/effect/expression/media closure (not fully enumerated)", "PSD cropped alpha bounds or EXR name uniqueness"], checks: &["has_comp active composition eligibility", "path supplied", "comp exists", "encoder and host output success"], writes: &["host bytes only"], state: &["active_comp fallback/enablement", "active comp current time when time omitted", "nested_switches and realtime_shadows preferences"], source: "crates/engine/engine/src/commands/frame_export.rs:32; crates/engine/engine/src/commands/frame_export.rs:45; crates/engine/engine/src/commands/frame_export.rs:218" } };
}
pub(super) const ROWS: &[Row] = &[frame!("comp.saveFrameAsPsd"), frame!("comp.saveFrameAsExr")];
