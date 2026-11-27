//! `file.identity` and `file.fork` (Kit Process Protocol section 5.6, KitModel M7a)
//! through the real `kition-motion` process: a `.kmotion` is the project as JSON (`Project::to_json`)
//! and embeds no replica; the kit makes one under a new id each time it opens a file
//! (`MotionModel::import`). So a file carries no identity, and a fork is the same bytes with no id,
//! written to a new file only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use kition_kit::KitDocument;
use kition_kit_host::{HeadlessKit, LaunchOptions};
use kition_motion_kit::MotionDocument;

const LIMIT: Duration = Duration::from_secs(30);

fn workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kition-motion-identity-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
}

async fn launch(dir: &Path) -> HeadlessKit {
    let mut options = LaunchOptions::new(env!("CARGO_BIN_EXE_kition-motion"));
    options.workspace = Some(dir.to_path_buf());
    HeadlessKit::launch(options).await.unwrap()
}

/// A saved project: one composition, as a person's first save writes it.
fn project() -> Vec<u8> {
    MotionDocument::create("Cut.kmotion", 1920, 1080).unwrap().save(None).unwrap()
}

#[tokio::test]
async fn a_motion_project_carries_no_identity_and_forks_into_the_same_bytes() {
    let dir = workspace("plain");
    let bytes = project();
    std::fs::write(dir.join("Cut.kmotion"), &bytes).unwrap();
    // An empty `.kmotion` (a file made for the kit before it saved a project there).
    std::fs::write(dir.join("New.kmotion"), b"").unwrap();
    let kit = launch(&dir).await;
    assert!(kit.serves("file.identity"), "{}", kit.capabilities());

    // No file carries an identity, a byte copy neither.
    std::fs::copy(dir.join("Cut.kmotion"), dir.join("Copy.kmotion")).unwrap();
    for path in ["Cut.kmotion", "Copy.kmotion", "New.kmotion"] {
        assert_eq!(kit.identity(path, LIMIT).await.unwrap().doc_id, None, "{path} embeds no replica");
    }

    // A fork is the same bytes, with no id, in a new file that opens as the project.
    let fork = kit.fork("Cut.kmotion", "Fork.kmotion", LIMIT).await.unwrap();
    assert_eq!(fork.doc_id, None);
    assert_eq!(fork.name, "Fork.kmotion");
    assert!(!fork.version.is_empty());
    assert_eq!(std::fs::read(dir.join("Fork.kmotion")).unwrap(), bytes, "a file without a replica forks unchanged");
    assert_eq!(std::fs::read(dir.join("Cut.kmotion")).unwrap(), bytes, "the source is unchanged");
    let state = kit.open("Fork.kmotion", true, LIMIT).await.unwrap();
    assert!(state["documentId"].is_string(), "{state}");
    let empty = kit.fork("New.kmotion", "NewCopy.kmotion", LIMIT).await.unwrap();
    assert_eq!(empty.doc_id, None);
    assert_eq!(std::fs::read(dir.join("NewCopy.kmotion")).unwrap(), b"");

    // A fork never replaces a file.
    let taken = kit.fork("Copy.kmotion", "Fork.kmotion", LIMIT).await.unwrap_err();
    assert_eq!(taken.reason(), Some("file_conflict"), "{taken}");
    assert_eq!(std::fs::read(dir.join("Fork.kmotion")).unwrap(), bytes, "the first fork is kept");
    kit.shutdown(Duration::from_secs(5)).await;
    let _ = std::fs::remove_dir_all(dir);
}
