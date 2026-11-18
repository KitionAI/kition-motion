//! `kition-motion` reads and writes only inside its workspace (KHP A29, §10.6): whatever an agent
//! runs, nothing appears in the process's current folder, its home folder or its temporary
//! folder, and the workspace holds only what the host saved. An agent-ready self-test found
//! `Kition Motion Auto-Save/basic auto-save 1.kmotion` in the current folder: `file.autoSave`
//! wrote the engine's auto-save next to the document's name, on the real file system.
//!
//! The test runs itself again as a child process whose current folder, `HOME` and `TMPDIR` are
//! fresh empty folders, so every by-path write the engine makes lands where it can be seen; a
//! folder beside them (`outside/`) holds files the agent names by path, which it must neither
//! list, change nor read (into a render).

use std::path::{Path, PathBuf};

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit::KitDocument;
use kition_kit_server::protocol::Origin;
use kition_kit_server::{KitBackend, OpenRequest};
use kition_motion_kit::MotionDocument;
use serde_json::{Map, Value, json};

/// Set in the child: the folder holding `workspace/`.
const CHILD: &str = "KITION_MOTION_TEST_SANDBOX";
const NAME: &str = "basic.kmotion";

/// Every file under `dir`, relative to it.
fn files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(&next).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            out.push(path.strip_prefix(dir).unwrap_or(&path).to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}

fn params(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// A 3D LUT that turns every colour black.
const BLACK_LUT: &str = "LUT_3D_SIZE 2\n0 0 0\n0 0 0\n0 0 0\n0 0 0\n0 0 0\n0 0 0\n0 0 0\n0 0 0\n";

/// What the agent does in the child: the auto-save family by name, commands that name folders
/// (through `engine.batch`, which runs commands the agent is not offered), scripts, effect
/// parameters naming files, then every command the kit offers the agent with no parameters.
/// Errors are fine; reaching files outside the workspace is not.
fn act(root: &Path) {
    let workspace = &root.join("workspace");
    let outside = root.join("outside");
    let mut doc = MotionDocument::create(NAME, 64, 36).unwrap();
    doc.execute("comp.settings", json!({ "duration": 0.2 })).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
    std::fs::write(workspace.join(NAME), doc.save(None).unwrap()).unwrap();

    let mut kit = Headless::<MotionDocument>::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &["file.import"], &[]).unwrap());
    let folder = HostFolder::new(workspace).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file(NAME).unwrap(), read_only: false, create: None }).unwrap().id;
    let run = |kit: &mut Headless<MotionDocument>, command: &str, value: Value| kit.execute(&id, command, params(value), Origin::Agent).map(|done| done.result);
    let _ = run(&mut kit, "layer.newSolid", json!({ "name": "Blue", "color": "#0000ff" }));
    // Settings that name folders: a relative one lands in the current folder.
    let folders = json!({ "values": {
        "disk.diskCacheEnabled": true,
        "disk.diskCacheFolder": "Disk Cache",
        "disk.mediaCacheFolder": "Media Cache",
        "disk.conformedMediaFolder": "Conformed Audio",
    } });
    let _ = run(&mut kit, "prefs.set", folders);
    let _ = run(&mut kit, "file.autoSave", json!({}));
    let _ = run(&mut kit, "prefs.set", json!({ "values": { "autoSave.location": "custom", "autoSave.folder": "Custom Auto-Save" } }));
    let _ = run(&mut kit, "file.autoSave", json!({}));
    let _ = run(&mut kit, "file.recoveryInfo", json!({}));
    let _ = run(&mut kit, "prefs.set", json!({ "values": { "autoSave.enabled": true, "autoSave.saveOnRenderStart": true } }));
    let _ = run(&mut kit, "layer.newSolid", json!({ "name": "Green", "color": "#00ff00" }));
    let _ = run(&mut kit, "renderQueue.add", json!({}));
    let _ = run(&mut kit, "renderQueue.render", json!({}));
    let _ = run(&mut kit, "file.exportMovie", json!({ "format": "mp4" }));
    let _ = run(&mut kit, "help.enableLogging", json!({}));
    let _ = run(&mut kit, "help.revealLogFile", json!({}));
    // Scripts, once the agent lets them write files: nothing by path.
    let gate = run(&mut kit, "prefs.set", json!({ "key": "scripting.allowScriptsWriteFiles", "value": "true" }));
    assert!(gate.is_ok(), "{gate:?}");
    let _ = run(&mut kit, "script.run", json!({ "code": "new Folder('Script Folder').create()" }));
    let made = outside.join("made by a script").to_string_lossy().into_owned();
    let _ = run(&mut kit, "script.run", json!({ "code": format!("new Folder({made:?}).create()") }));
    for folder in [outside.to_string_lossy().into_owned(), ".".to_owned()] {
        let listed = run(&mut kit, "script.run", json!({ "code": format!("new Folder({folder:?}).getFiles().length") }));
        let ran = listed.as_ref().ok().and_then(Option::as_ref).is_some_and(|answer| answer["ok"] == json!(true));
        assert!(!ran, "a script listed {folder}, outside the workspace: {listed:?}");
    }
    // Commands that name folders, which the agent is not offered, run through engine.batch.
    let named = outside.to_string_lossy().into_owned();
    for (command, params) in [
        ("mediaBrowser.list", json!({ "path": named })),
        ("file.watchFolder", json!({ "folder": named })),
        ("file.relinkFootage", json!({ "folder": named })),
        ("file.collectFiles", json!({ "folder": outside.join("collected").to_string_lossy() })),
    ] {
        let answer = run(&mut kit, "engine.batch", json!({ "steps": [{ "command": command, "params": params }] }));
        assert!(answer.is_err(), "{command} reached a folder outside the workspace: {answer:?}");
    }
    kit.save(&id, None, None).unwrap();
    let offered: Vec<String> = kit.commands(Some(&id)).unwrap().into_iter().map(|command| command.id).collect();
    for command in offered {
        // Quitting or closing ends the session the rest of the sweep runs in; the two renders ran
        // above on the short composition (by now the sweep has made long ones, a minute each).
        if matches!(command.as_str(), "app.quit" | "file.close" | "file.closeProject" | "renderQueue.render" | "file.exportMovie") {
            continue;
        }
        let _ = run(&mut kit, &command, json!({}));
    }

    // An effect parameter naming a file outside the workspace reads nothing: the red stays red.
    let mut doc = MotionDocument::create("lut.kmotion", 8, 8).unwrap();
    let layer = doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap()["layer"].clone();
    doc.execute("effect.apply", json!({ "layers": [layer], "effect": "ec.utility.applylut" })).unwrap();
    doc.execute("prop.set", json!({ "layer": layer, "path": "effects/#1/lut", "value": outside.join("black.cube").to_string_lossy() })).unwrap();
    let pixels = doc.render(0.0, 0.0, 1.0, 8, 8).unwrap();
    assert!(pixels[0] > 200, "the LUT outside the workspace was read: {:?}", &pixels[..4]);
}

#[test]
fn agent_commands_write_nothing_outside_the_workspace() {
    if let Some(root) = std::env::var_os(CHILD) {
        act(&PathBuf::from(root));
        return;
    }
    let root = std::env::temp_dir().join(format!("kition-motion-workspace-only-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for dir in ["workspace", "cwd", "home", "tmp", "outside"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    let root = std::fs::canonicalize(root).unwrap();
    std::fs::write(root.join("outside/black.cube"), BLACK_LUT).unwrap();
    std::fs::write(root.join("outside/planted.kmotion"), MotionDocument::create("planted.kmotion", 16, 16).unwrap().save(None).unwrap()).unwrap();
    let planted = files(&root.join("outside"));
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "agent_commands_write_nothing_outside_the_workspace", "--nocapture", "--test-threads=1"])
        .current_dir(root.join("cwd"))
        .env(CHILD, &root)
        .env("HOME", root.join("home"))
        .env("TMPDIR", root.join("tmp"))
        .env("XDG_CONFIG_HOME", root.join("home/.config"))
        .env_remove("KITION_MOTION_CONFIG_DIR")
        .status()
        .unwrap();
    assert!(status.success(), "the agent's session ran: {status}");
    let outside: Vec<(&str, Vec<String>)> =
        ["cwd", "home", "tmp"].into_iter().map(|dir| (dir, files(&root.join(dir)))).filter(|(_, found)| !found.is_empty()).collect();
    assert!(outside.is_empty(), "the kit wrote outside its workspace: {outside:?}");
    assert_eq!(files(&root.join("workspace")), [NAME], "the workspace holds only the document the host saved");
    assert_eq!(files(&root.join("outside")), planted, "the folder outside the workspace is as it was");
    let _ = std::fs::remove_dir_all(&root);
}
