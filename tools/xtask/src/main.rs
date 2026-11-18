//! Workspace automation: `cargo xtask <layers|assets|wasm|ci>`.
//!
//! - `layers`: enforces the dependency layering of `docs/architecture.md` §1 (downward-only edges,
//!   listed same-layer edges, no UI/OS crates below L5).
//! - `wasm`: `cargo check --target wasm32-unknown-unknown` for every crate in L0–L4 (the engine
//!   crates behind `bindings/wasm`).
//! - `assets`: every asset file (image, icon, font, LUT, audio, video…) has a complete
//!   `<file>.attribution` sidecar and an entry in `docs/attribution.md` (AGENTS.md §1).
//! - `ci`: fmt check, clippy -D warnings, tests, layers, assets, wasm.

mod version;

use std::process::{Command, ExitCode};

use serde_json::Value;

/// (crate name without the `kition-motion-` prefix, layer). See `docs/architecture.md` §1.
const LAYERS: &[(&str, u8)] = &[
    ("time", 0),
    ("geom", 0),
    ("color", 0),
    ("vp9enc", 0),
    ("testkit", 0),
    ("opusenc", 0),
    ("hevcenc", 0),
    ("av1enc", 0),
    ("raster", 1),
    ("keyframe", 1),
    ("path", 1),
    ("psd", 1),
    ("segment", 1),
    ("project", 2),
    ("text", 2),
    ("effects", 2),
    ("track", 2),
    ("svg", 2),
    ("pdf", 2),
    ("model", 2),
    ("render", 3),
    ("media", 3),
    ("expr", 3),
    ("export", 3),
    ("lottie", 3),
    ("format", 3),
    ("plugin", 3),
    ("interchange", 3),
    ("kdm-model", 3),
    ("engine", 4),
    ("script", 4),
    ("host", 4),
    ("kit", 5),
    ("kition-motion", 6),
];

/// Allowed same-layer edges (from, to).
const SAME_LAYER: &[(&str, &str)] = &[
    ("path", "keyframe"),
    ("path", "raster"),
    ("text", "path"),
    ("pdf", "svg"),
    ("pdf", "text"),
    ("effects", "project"),
    ("effects", "text"),
    ("effects", "path"),
    ("effects", "track"),
    ("media", "render"),
    ("expr", "render"),
    ("export", "render"),
    ("export", "media"),
    ("lottie", "format"),
    ("host", "engine"),
    ("script", "engine"),
    ("host", "script"),
];

/// Crates that must not appear below L5 (UI toolkits, windowing, OS audio/menus).
const UI_ONLY: &[&str] = &["egui", "eframe", "egui-wgpu", "winit", "rfd", "cpal", "muda"];

fn short(name: &str) -> &str {
    name.strip_prefix("kition-motion-").unwrap_or(name)
}

fn layer_of(name: &str) -> Option<u8> {
    LAYERS.iter().find(|(n, _)| *n == short(name)).map(|(_, l)| *l)
}

fn metadata() -> Result<Value, String> {
    let out = Command::new(env!("CARGO")).args(["metadata", "--format-version", "1", "--no-deps"]).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

fn workspace_crates(md: &Value) -> Vec<(String, Vec<String>)> {
    md["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["name"] != "xtask")
        .map(|p| {
            let deps = p["dependencies"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|d| d["kind"].is_null() || d["kind"] == "build")
                .filter_map(|d| d["name"].as_str().map(str::to_string))
                .collect();
            (p["name"].as_str().unwrap_or_default().to_string(), deps)
        })
        .collect()
}

fn layers() -> Result<(), String> {
    let md = metadata()?;
    let mut errors = Vec::new();
    for (name, deps) in workspace_crates(&md) {
        let Some(l) = layer_of(&name) else {
            errors.push(format!("{name}: not assigned a layer (add it to xtask LAYERS and docs/architecture.md §1)"));
            continue;
        };
        for d in &deps {
            if l < 5 && UI_ONLY.contains(&d.as_str()) {
                errors.push(format!("{name} (L{l}) depends on UI/OS crate `{d}`"));
            }
            if !d.starts_with("kition-motion-") {
                continue;
            }
            let Some(dl) = layer_of(d) else { continue };
            let (a, b) = (short(&name), short(d));
            if dl > l {
                errors.push(format!("{a} (L{l}) depends upward on {b} (L{dl})"));
            } else if dl == l && l > 0 && !SAME_LAYER.contains(&(a, b)) {
                errors.push(format!("{a} → {b}: same-layer edge (L{l}) not in the allowed list"));
            }
        }
    }
    if errors.is_empty() {
        println!("layers: ok");
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// File extensions that count as assets (AGENTS.md §1).
const ASSET_EXT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "icns", "bmp", "tif", "tiff", "heic", "avif", "exr", "ttf", "otf", "ttc", "woff", "woff2", "cube",
    "3dl", "lut", "look", "wav", "mp3", "aac", "flac", "ogg", "opus", "m4a", "aif", "aiff", "mp4", "mov", "m4v", "mkv", "webm", "avi", "mxf", "psd", "ai",
    "eps", "pdf", "prproj", "ffx", "prfpset", "mogrt", "aep",
];

/// Sidecar fields that must be present and non-empty.
const REQUIRED_FIELDS: &[&str] = &["asset", "title", "author", "source", "license", "added"];

fn repo_files() -> Result<Vec<String>, String> {
    let out = Command::new("git").args(["ls-files", "--cached", "--others", "--exclude-standard"]).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).filter(|f| std::path::Path::new(f).exists()).collect())
}

fn is_asset(path: &str) -> bool {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).is_some_and(|e| ASSET_EXT.contains(&e.to_ascii_lowercase().as_str()))
}

fn assets() -> Result<(), String> {
    let files = repo_files()?;
    let index = std::fs::read_to_string("docs/attribution.md").map_err(|e| format!("docs/attribution.md: {e}"))?;
    let mut errors = Vec::new();
    let mut n = 0;
    for f in files.iter().filter(|f| is_asset(f)) {
        n += 1;
        let lower = f.to_ascii_lowercase();
        if lower.contains("adobe") || lower.contains("aftereffects") || lower.contains("after-effects") {
            errors.push(format!("{f}: asset paths must not reference Adobe/After Effects (AGENTS.md §1)"));
        }
        let side = format!("{f}.attribution");
        match std::fs::read_to_string(&side) {
            Err(_) => errors.push(format!("{f}: missing attribution sidecar {side}")),
            Ok(text) => {
                for field in REQUIRED_FIELDS {
                    let ok = text.lines().any(|l| l.split_once(':').is_some_and(|(k, v)| k.trim() == *field && !v.trim().is_empty()));
                    if !ok {
                        errors.push(format!("{side}: field `{field}` missing or empty"));
                    }
                }
                let lic = text.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim() == "license").map(|(_, v)| v.trim().to_ascii_lowercase()));
                if lic.is_some_and(|l| l.contains("-nc") || l.contains("-nd") || l.contains("adobe") || l.contains("proprietary")) {
                    errors.push(format!("{side}: licence not allowed (no NC/ND, Adobe or proprietary licences)"));
                }
            }
        }
        if !index.contains(&format!("`{f}`")) {
            errors.push(format!("{f}: not listed in docs/attribution.md"));
        }
    }
    for f in files.iter().filter(|f| f.ends_with(".attribution")) {
        let asset = f.trim_end_matches(".attribution");
        if !std::path::Path::new(asset).exists() {
            errors.push(format!("{f}: sidecar for a file that does not exist"));
        }
    }
    if errors.is_empty() {
        println!("assets: ok ({n} assets attributed)");
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn run(cmd: &mut Command) -> Result<(), String> {
    eprintln!("$ {cmd:?}");
    let st = cmd.status().map_err(|e| e.to_string())?;
    if st.success() { Ok(()) } else { Err(format!("failed: {cmd:?}")) }
}

fn wasm() -> Result<(), String> {
    let md = metadata()?;
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["check", "--target", "wasm32-unknown-unknown"]);
    let mut n = 0;
    for (name, _) in workspace_crates(&md) {
        if layer_of(&name).is_some_and(|l| l <= 4) {
            cmd.args(["-p", &name]);
            n += 1;
        }
    }
    if n == 0 {
        return Ok(());
    }
    run(&mut cmd)?;
    println!("wasm: ok ({n} crates)");
    Ok(())
}

fn ci() -> Result<(), String> {
    let cargo = env!("CARGO");
    run(Command::new(cargo).args(["fmt", "--check"]))?;
    run(Command::new(cargo).args(["clippy", "--workspace", "--all-targets", "--release", "--", "-D", "warnings"]))?;
    run(Command::new(cargo).args(["test", "--workspace", "--release"]))?;
    layers()?;
    assets()?;
    wasm()
}

/// The eval cases (docs/quality.md layer `eval`, decision F-QA-16), run by kition-sdk's
/// `tools/eval-cases.mjs` (KITION_SDK_DIR, or ../kition-sdk beside this repository). Only a
/// maintainer starts it; it asks at the terminal before calling any model. `--dry-run` calls none.
fn eval(args: &[String]) -> Result<(), String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let sdk = std::env::var_os("KITION_SDK_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| root.join("..").join("kition-sdk"));
    let status = Command::new("node")
        .arg(sdk.join("tools").join("eval-cases.mjs"))
        .arg("--repo")
        .arg(&root)
        .args(args)
        .status()
        .map_err(|e| format!("could not start node: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("eval cases: {status}")) }
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let r = match task.as_str() {
        "layers" => layers(),
        "wasm" => wasm(),
        "assets" => assets(),
        "ci" => ci(),
        "eval" => eval(&std::env::args().skip(2).collect::<Vec<_>>()),
        "version" => {
            let rest: Vec<String> = std::env::args().skip(2).collect();
            version::run(&root(), &rest.iter().map(String::as_str).collect::<Vec<_>>())
        }
        _ => Err("usage: cargo xtask <layers|assets|wasm|version [set X.Y.Z]|ci|eval [options]>".into()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// The workspace root (two levels above `tools/xtask/`).
pub fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).expect("tools/xtask is two levels below the root").to_path_buf()
}
