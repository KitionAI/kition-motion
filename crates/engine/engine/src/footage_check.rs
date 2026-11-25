//! Lazy project open: footage is checked in the background after File ▸ Open instead of while
//! the project loads (M13.14).
//!
//! A project file already carries every footage item's metadata, so opening it never touches
//! the media: decoding starts when a frame is first needed, thumbnails when the Project panel
//! shows one, expressions compile on first evaluation and system fonts are scanned only when a
//! text layer asks for a family that isn't bundled. What open used to skip entirely is noticing
//! that files went missing (or that an item was saved without metadata). `footage.check` does
//! that as a background task listed in the Progress panel ("Checking footage"): it looks for
//! every file (each frame of an image sequence), probes items that have no metadata, and then
//! flags missing items (`missing`, File ▸ Dependencies ▸ Find Missing Footage). The desktop app
//! starts it after every open ([`Session::check_footage_on_open`]); agents and the CLI run it
//! with `wait: true`.

use kition_motion_project::{Footage, FootageKind, ItemId, ItemKind};
use serde_json::{Value, json};

use crate::Session;

/// One item to check.
struct Job {
    id: ItemId,
    footage: Footage,
}

/// What the check found for one item.
enum Found {
    Missing(bool),
    Probed(Box<Footage>),
}

fn exists(s: &dyn crate::Services, path: &str) -> bool {
    !path.is_empty() && s.exists(path)
}

/// Start the check (all footage, or `items`). `wait` runs it to completion first (always on
/// wasm32); the result is then `{checked, missing, probed}`.
pub fn start(s: &mut Session, items: Option<Vec<ItemId>>, wait: bool) -> crate::Result<Value> {
    let jobs: Vec<Job> = s
        .project
        .items
        .values()
        .filter(|i| items.as_ref().is_none_or(|v| v.contains(&i.id)))
        .filter_map(|i| match &i.kind {
            ItemKind::Footage(f) if f.kind != FootageKind::Data => Some(Job { id: i.id, footage: f.clone() }),
            _ => None,
        })
        .collect();
    if jobs.is_empty() {
        return Ok(json!({"checked": 0, "missing": 0, "probed": 0}));
    }
    let services = s.services.clone();
    let importer = s.importer.clone();
    let rate = s.sequence_rate();
    let n = jobs.len();
    s.spawn_task("footageCheck", format!("Checking footage ({n} items)"), wait, move |ctl| {
        let mut found: Vec<(ItemId, Footage, Found)> = Vec::with_capacity(jobs.len());
        let total = jobs.len() as u64;
        for (k, j) in jobs.into_iter().enumerate() {
            if !ctl.progress(k as u64, total) {
                return Err(crate::render_queue::CANCELLED.into());
            }
            let f = &j.footage;
            let present = if f.sequence.is_empty() {
                exists(services.as_ref(), &f.path)
            } else {
                // A sequence is there while any of its frames is (missing frames render as
                // placeholders, as is usual).
                exists(services.as_ref(), &f.path) || f.sequence.iter().any(|p| exists(services.as_ref(), p))
            };
            // Saved without metadata (a hand-written or converted project): probe it now.
            let needs_probe = present && f.width == 0 && f.height == 0 && f.kind != FootageKind::Audio;
            let probed = importer.as_ref().filter(|_| needs_probe).map(|imp| match f.kind {
                // A sequence keeps its files; its first one gives the metadata.
                FootageKind::Sequence if !f.sequence.is_empty() => imp.probe(&f.path).map(|one| {
                    let mut nf = Footage {
                        width: one.width,
                        height: one.height,
                        pixel_aspect: one.pixel_aspect,
                        has_video: one.has_video,
                        codec: one.codec,
                        ..f.clone()
                    };
                    nf.sync_sequence_duration();
                    nf
                }),
                _ => crate::sequence::probe_path(imp.as_ref(), services.as_ref(), &f.path, rate),
            });
            match probed {
                Some(Ok(mut nf)) => {
                    nf.alpha = f.alpha;
                    nf.loop_count = f.loop_count;
                    found.push((j.id, j.footage, Found::Probed(Box::new(nf))));
                }
                _ => found.push((j.id, j.footage, Found::Missing(!present))),
            }
        }
        ctl.progress(total, total);
        let probe_available = importer.is_some();
        let apply: crate::jobs::Apply = Box::new(move |s: &mut Session| apply(s, found, probe_available));
        Ok(apply)
    })
}

/// Write the findings into the project. Not an undo step: it records the state of the disk,
/// not an edit (the same as compositing apps marking footage missing at open).
fn apply(s: &mut Session, found: Vec<(ItemId, Footage, Found)>, probe_available: bool) -> crate::Result<Value> {
    for (id, before, result) in &found {
        let current = s.project.item(*id).and_then(|item| match &item.kind {
            ItemKind::Footage(footage) => Some(footage),
            _ => None,
        });
        let unchanged = current.is_some_and(|current| match result {
            Found::Probed(_) => current == before,
            Found::Missing(missing) => {
                let probe_eligible = |footage: &Footage| footage.width == 0 && footage.height == 0 && footage.kind != FootageKind::Audio;
                current.kind != FootageKind::Data
                    && current.path == before.path
                    && current.sequence == before.sequence
                    && current.missing == before.missing
                    && (*missing || !probe_available || probe_eligible(current) == probe_eligible(before))
            }
        });
        if !unchanged {
            return Err(crate::EngineError::Other(format!("footage {} changed during its check; check it again", id.0)));
        }
    }
    let checked = found.len();
    let (mut missing, mut probed, mut changed) = (0, 0, false);
    let p = std::sync::Arc::make_mut(&mut s.project);
    for (id, _, f) in found {
        let Some(ItemKind::Footage(cur)) = p.item_mut(id).map(|i| &mut i.kind) else { continue };
        match f {
            Found::Missing(m) => {
                missing += usize::from(m);
                if cur.missing != m {
                    cur.missing = m;
                    changed = true;
                }
            }
            Found::Probed(nf) => {
                probed += 1;
                *cur = *nf;
                changed = true;
            }
        }
    }
    if changed {
        let dirty = s.is_dirty();
        s.bump();
        if !dirty {
            // Only the disk changed: the project isn't modified.
            s.mark_saved();
        }
        s.events.push(crate::Event::PurgeCaches);
    }
    if missing > 0 {
        s.toast(format!("{missing} footage item(s) missing (File ▸ Dependencies ▸ Find Missing Footage)"));
    }
    Ok(json!({"checked": checked, "missing": missing, "probed": probed}))
}

pub(crate) fn command(s: &mut Session, p: &Value) -> crate::Result<Value> {
    let items = p.get("items").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).map(ItemId).collect());
    let wait = p.get("wait").and_then(Value::as_bool).unwrap_or(false);
    start(s, items, wait)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_with(paths: &[&str]) -> Session {
        let mut s = Session::default();
        for (i, p) in paths.iter().enumerate() {
            let f = Footage { path: p.to_string(), width: 64, height: 48, has_video: true, ..Default::default() };
            std::sync::Arc::make_mut(&mut s.project).add_item(&format!("f{i}"), crate::color::Label::None, None, ItemKind::Footage(f));
        }
        s
    }

    #[test]
    fn deferred_footage_check_preserves_changed_footage() {
        for replace_source in [true, false] {
            let mut s = session_with(&["/nonexistent/deferred-check.mov"]);
            let id = *s.project.items.keys().next().unwrap();
            start(&mut s, None, false).unwrap();
            if replace_source {
                let ItemKind::Footage(footage) = &mut std::sync::Arc::make_mut(&mut s.project).item_mut(id).unwrap().kind else { panic!() };
                footage.path = "/replacement.mov".into();
                footage.width = 999;
            } else {
                s.execute("project.rename", json!({"item": id.0, "name": "unrelated name"})).unwrap();
            }
            s.wait_jobs();
            let ItemKind::Footage(footage) = &s.project.item(id).unwrap().kind else { panic!() };
            if replace_source {
                assert!(!footage.missing);
                assert_eq!(footage.path, "/replacement.mov");
                assert_eq!(s.job_log.last().unwrap().status, "failed");
            } else {
                assert!(footage.missing);
                assert_eq!(s.job_log.last().unwrap().status, "done");
            }
        }
    }

    #[test]
    fn deferred_footage_probe_preserves_changed_interpretation() {
        struct Probe;
        impl crate::Importer for Probe {
            fn probe(&self, path: &str) -> Result<Footage, String> {
                Ok(Footage { path: path.into(), width: 320, height: 180, has_video: true, ..Default::default() })
            }
        }
        let path = std::env::temp_dir().join(format!("motion-deferred-probe-{}.mov", std::process::id()));
        std::fs::write(&path, b"synthetic probe input").unwrap();
        for change_interpretation in [true, false] {
            let mut s = session_with(&[path.to_str().unwrap()]);
            let id = *s.project.items.keys().next().unwrap();
            let ItemKind::Footage(footage) = &mut std::sync::Arc::make_mut(&mut s.project).item_mut(id).unwrap().kind else { panic!() };
            footage.width = 0;
            footage.height = 0;
            s.importer = Some(std::sync::Arc::new(Probe));
            start(&mut s, None, false).unwrap();
            if change_interpretation {
                let ItemKind::Footage(footage) = &mut std::sync::Arc::make_mut(&mut s.project).item_mut(id).unwrap().kind else { panic!() };
                footage.loop_count = 3;
            } else {
                s.execute("project.rename", json!({"item": id.0, "name": "renamed while probing"})).unwrap();
            }
            s.wait_jobs();
            let ItemKind::Footage(footage) = &s.project.item(id).unwrap().kind else { panic!() };
            if change_interpretation {
                assert_eq!(footage.loop_count, 3);
                assert_eq!(footage.width, 0);
                assert_eq!(s.job_log.last().unwrap().status, "failed");
            } else {
                assert_eq!(footage.width, 320);
                assert_eq!(s.job_log.last().unwrap().status, "done");
            }
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn deferred_footage_check_refuses_changed_kind() {
        let mut s = session_with(&["/nonexistent/deferred-now-data.csv"]);
        let id = *s.project.items.keys().next().unwrap();
        start(&mut s, None, false).unwrap();
        let ItemKind::Footage(footage) = &mut std::sync::Arc::make_mut(&mut s.project).item_mut(id).unwrap().kind else { panic!() };
        footage.kind = FootageKind::Data;
        s.wait_jobs();
        let ItemKind::Footage(footage) = &s.project.item(id).unwrap().kind else { panic!() };
        assert!(!footage.missing);
        assert_eq!(s.job_log.last().unwrap().status, "failed");
        assert!(s.job_log.last().unwrap().message.contains("changed"));
    }

    #[test]
    fn check_flags_missing_footage_without_an_undo_step_or_dirtying() {
        let dir = std::env::temp_dir().join(format!("ec-footage-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let there = dir.join("there.png");
        std::fs::write(&there, b"x").unwrap();
        let mut s = session_with(&[there.to_str().unwrap(), "/nonexistent/ec/gone.png"]);
        s.mark_saved();
        let r = s.execute("footage.check", json!({"wait": true})).unwrap();
        assert_eq!(r, json!({"checked": 2, "missing": 1, "probed": 0}));
        let missing: Vec<bool> = s
            .project
            .items
            .values()
            .filter_map(|i| match &i.kind {
                ItemKind::Footage(f) => Some(f.missing),
                _ => None,
            })
            .collect();
        assert_eq!(missing, [false, true]);
        assert!(s.history.undo.is_empty() && !s.is_dirty());
        // In the background: listed as a job until polled.
        std::fs::remove_file(&there).unwrap();
        let r = s.execute("footage.check", json!({})).unwrap();
        assert_eq!(r["started"], true);
        assert!(s.jobs().iter().any(|j| j.kind == "footageCheck"));
        let t0 = std::time::Instant::now();
        while !s.tasks.is_empty() && t0.elapsed().as_secs() < 10 {
            s.poll_jobs();
            std::thread::yield_now();
        }
        assert!(s.project.items.values().all(|i| matches!(&i.kind, ItemKind::Footage(f) if f.missing)));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// #652: footage saved with a relative path is found next to the project, wherever the
    /// app was started (the desktop app doesn't start in the project's folder, the CLI often
    /// does).
    #[test]
    fn relative_footage_is_found_next_to_the_project() {
        let dir = std::env::temp_dir().join(format!("ec-relative-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("media")).unwrap();
        let clip = dir.join("media").join("ec-relative-clip.png");
        std::fs::write(&clip, b"x").unwrap();
        let mut s = session_with(&["media/ec-relative-clip.png", "/nonexistent/ec/abs.png", "media/ec-relative-gone.png"]);
        let path = dir.join("p.kmotion");
        s.execute("file.saveAs", json!({"path": path.to_string_lossy()})).unwrap();
        let mut o = Session::default();
        o.execute("file.open", json!({"path": path.to_string_lossy()})).unwrap();
        let mut paths: Vec<String> =
            o.project.items.values().filter_map(|i| if let ItemKind::Footage(f) = &i.kind { Some(f.path.clone()) } else { None }).collect();
        paths.sort();
        assert_eq!(paths, ["/nonexistent/ec/abs.png", clip.to_str().unwrap(), "media/ec-relative-gone.png"]);
        assert!(!o.is_dirty(), "finding the files is no edit");
        let r = o.execute("footage.check", json!({"wait": true})).unwrap();
        assert_eq!(r["missing"], 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn open_starts_the_check_when_asked() {
        let mut s = session_with(&["/nonexistent/ec/a.mov"]);
        let path = std::env::temp_dir().join(format!("ec-footage-open-{}.kmotion", std::process::id()));
        s.execute("file.saveAs", json!({"path": path.to_string_lossy()})).unwrap();
        let mut o = Session { check_footage_on_open: true, ..Default::default() };
        o.execute("file.open", json!({"path": path.to_string_lossy()})).unwrap();
        assert!(o.jobs().iter().any(|j| j.kind == "footageCheck"), "{:?}", o.jobs());
        let mut plain = Session::default();
        plain.execute("file.open", json!({"path": path.to_string_lossy()})).unwrap();
        assert!(plain.jobs().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
