//! `kition-motion`: the Motion kit for Kition's agent. The same document the in-page editor
//! runs (`kition-motion-kit`), served headless over the Kit Protocol by `kition-headless`, as the
//! kit's `kit.json` declares it, with the object model
//! (`model.describe`, `model.batch`) over the document's change exchange, and the identity a file
//! carries (`document.identity`, `document.fork`): a `.kmotion` is the project as JSON and embeds no
//! replica (the kit makes one under a new id each time it opens a file), so a file carries no
//! identity and a fork is the same bytes.

use kition_headless::{FileIdentity, Headless, KitInfo};

// The commands in `SANDBOXED` name files but read them through lent files (the workspace,
// never the process's file system), so the agent may use them.

const SANDBOXED: &[&str] = &["file.import", "animation.applyRecentPreset"];
/// `inspect()` keys about the editing session rather than the file.
const VOLATILE: &[&str] = kition_motion_kit::EDITOR_STATE;

fn main() -> std::process::ExitCode {
    let kit = match KitInfo::from_manifest(include_str!("../../../kit.json"), SANDBOXED, VOLATILE) {
        Ok(kit) => kit,
        Err(error) => {
            eprintln!("kit: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let backend = Headless::<kition_motion_kit::MotionDocument>::with_replica_port(kit.clone()).with_file_identity(FileIdentity::NotEmbedded);
    kition_headless::serve_backend(&kit, backend)
}
