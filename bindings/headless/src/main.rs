//! `kition-motion`: the Motion kit for Kition's agent. The same document the in-page editor
//! runs (`kition-motion-kit`), served headless over the Kit Protocol by `kition-headless`, as the
//! kit's `kit.json` declares it, with the object model
//! (`model.describe`, `model.batch`) over the document's change exchange.

// The commands in `SANDBOXED` name files but read them through lent files (the workspace,
// never the process's file system), so the agent may use them.

const SANDBOXED: &[&str] = &["file.import", "anim.applyRecentPreset"];
/// `inspect()` keys about the editing session rather than the file.
const VOLATILE: &[&str] = kition_motion_kit::EDITOR_STATE;

fn main() -> std::process::ExitCode {
    kition_headless::run_manifest_model::<kition_motion_kit::MotionDocument>(include_str!("../../../kit.json"), SANDBOXED, VOLATILE)
}
