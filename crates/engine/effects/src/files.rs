//! Files effect parameters name: a LUT, an OCIO config or file transform, tracked shape data.
//! Each parameter also takes the file's text itself; a path is read from the file system only
//! while this process may reach files by path. A kit, whose host owns every file (KHP A29),
//! turns that off with [`deny_paths`] and gives the text instead.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static PATHS: AtomicBool = AtomicBool::new(true);

/// From now on, effect parameters naming files read nothing from the file system (for the whole
/// process: a kit's process serves only documents whose files belong to the host).
pub fn deny_paths() {
    PATHS.store(false, Ordering::Relaxed);
}

/// Whether effect parameters may name files on the file system.
pub fn paths_allowed() -> bool {
    PATHS.load(Ordering::Relaxed)
}

/// The text of the file at `path`, unless paths are denied ([`deny_paths`]).
pub fn read_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    if !paths_allowed() {
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "effect parameters name no files here: give the file's text instead"));
    }
    std::fs::read_to_string(path)
}

/// Whether a file exists at `path` (never, when paths are denied).
pub fn exists(path: &Path) -> bool {
    paths_allowed() && path.exists()
}
