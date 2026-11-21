//! `kition-motion` exports the active composition as an MP4 (KHP A41) both ways a headless agent gets
//! one: `file.exportMovie` answers the file's bytes (for `<prefix>_export`), run as the agent, and the
//! host's `document.export` writes it as a new file. The Render Queue's commands name files and are
//! disabled headless. Neither changes the project.

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit::KitDocument;
use kition_kit_server::protocol::Origin;
use kition_kit_server::{KitBackend, OpenRequest};
use kition_motion_kit::MotionDocument;
use serde_json::{Map, json};

fn decode(text: &str) -> Vec<u8> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let (mut out, mut buffer, mut bits) = (Vec::new(), 0u32, 0u32);
    for byte in text.trim_end_matches('=').bytes() {
        buffer = (buffer << 6) | TABLE.iter().position(|known| *known == byte).expect("base64") as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    out
}

#[test]
fn the_agent_exports_the_active_composition_as_an_mp4() {
    let root = std::env::temp_dir().join(format!("kition-motion-export-agent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("title.kmotion");
    let mut doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    doc.execute("comp.settings", json!({ "duration": 0.5 })).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
    std::fs::write(&file, doc.save(None).unwrap()).unwrap();

    // As main.rs serves it (run_manifest_model): the agent's calls go through the replica port as the agent.
    let mut kit =
        Headless::<MotionDocument>::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_motion_kit::EDITOR_STATE).unwrap());
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let listed = kit.commands(Some(&id)).unwrap();
    assert!(listed.iter().any(|command| command.id == "file.exportMovie"), "offered to the agent: it names no file");
    let answer = kit.execute(&id, "file.exportMovie", Map::from_iter([("format".into(), json!("mp4"))]), Origin::Agent).unwrap();
    let result = answer.result.unwrap();
    let movie = decode(result["data"].as_str().unwrap());
    assert_eq!(&movie[4..8], b"ftyp", "an MP4");
    assert_eq!(result["bytes"], json!(movie.len()));
    assert!(!kit.document(&id).unwrap().dirty, "exporting leaves the project as it is");
    assert!(kit.execute(&id, "file.exportMovie", Map::from_iter([("format".into(), json!("gif"))]), Origin::Agent).is_err());
}

#[test]
fn the_host_exports_the_active_composition_as_a_new_mp4() {
    let root = std::env::temp_dir().join(format!("kition-motion-export-host-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("title.kmotion");
    let mut doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    doc.execute("comp.settings", json!({ "duration": 0.5 })).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
    std::fs::write(&file, doc.save(None).unwrap()).unwrap();

    let mut kit = Headless::<MotionDocument>::new(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_motion_kit::EDITOR_STATE).unwrap());
    assert_eq!(kit.capabilities()["formats"]["export"], json!(["mp4"]), "the host is told the kit exports MP4");
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let target = folder.export("title.mp4").unwrap();
    kit.export(&id, &target, "mp4", None).unwrap();
    let movie = std::fs::read(root.join("title.mp4")).unwrap();
    assert_eq!(&movie[4..8], b"ftyp", "an MP4 file");
    assert!(!kit.document(&id).unwrap().dirty, "exporting leaves the project as it is");
    assert!(kit.export(&id, &target, "mp4", None).is_err(), "an export never replaces a file");
    assert!(folder.export("title.mp4").is_err(), "the host mints no export handle on an existing file");
    assert!(kit.export(&id, &folder.export("title.gif").unwrap(), "gif", None).is_err(), "only the manifest's exports");
    assert!(!root.join("title.gif").exists());
}

/// Every `exports` entry of the manifest names the command `agent.export` names, the one every
/// format runs (kition-sdk manifest rule K21), and that command answers the entry's format as
/// the agent-ready check passes it (`{format: <extension>}`, as the agent). The manifest named
/// `renderQueue.render`, which refuses `format`.
#[test]
fn every_export_entry_runs_the_agent_export_command() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("../../../kit.json")).unwrap();
    let command = manifest["agent"]["export"]["command"].as_str().unwrap();
    let root = std::env::temp_dir().join(format!("kition-motion-export-entries-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let mut doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    doc.execute("comp.settings", json!({ "duration": 0.2 })).unwrap();
    std::fs::write(root.join("title.kmotion"), doc.save(None).unwrap()).unwrap();
    let mut kit =
        Headless::<MotionDocument>::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_motion_kit::EDITOR_STATE).unwrap());
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let entries = manifest["exports"].as_array().unwrap();
    assert!(!entries.is_empty());
    for entry in entries {
        assert_eq!(entry["command"].as_str(), Some(command), "exports[] and agent.export name one command: {entry}");
        let extension = entry["extension"].as_str().unwrap();
        let answer = kit.execute(&id, entry["command"].as_str().unwrap(), Map::from_iter([("format".into(), json!(extension))]), Origin::Agent);
        let result = answer.map(|done| done.result).unwrap_or_else(|e| panic!("export {extension}: {e:?}")).unwrap();
        assert_eq!(result["mime"], entry["mime"], "{entry}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
