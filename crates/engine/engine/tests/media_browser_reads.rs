//! `mediaBrowser.import` with `addToComp` (files dragged from the Media Browser into the Timeline
//! or the Composition viewer) puts the new layers into the active composition as `layer.addItem`
//! does: above its first selected layer and starting at its current time, unless the call gives an
//! `index` and a `time` (panels_cmds.rs `browser_import` → file.rs `add_to_comp` → layer.rs
//! `add_item`, `insert_layer_at`; the current time when Settings ▸ General ▸ Create New Layers at
//! Composition Start Time is off). So it reads the active composition, the layer selection and the
//! current time, the editor state `bindings/wasm/src/reads.json` declares for it (task
//! acts-known-differences, case regressions/issue-acts-under-declared-reads).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use kition_motion_engine::{Importer, Session};
use kition_motion_project::{Footage, FootageKind};
use kition_motion_time::Tick;
use serde_json::json;

/// Every file is a 64 x 32 still.
struct Probe;

impl Importer for Probe {
    fn probe(&self, path: &str) -> Result<Footage, String> {
        Ok(Footage { path: path.into(), kind: FootageKind::Still, width: 64, height: 32, has_video: true, ..Default::default() })
    }
}

/// A session with comp `Main` holding `Top` over `Bottom`, with `selected` selected, the current
/// time at `time` seconds, and new layers starting at the current time.
fn session(selected: &str, time: f64) -> Session {
    let mut s = Session { importer: Some(Arc::new(Probe)), ..Default::default() };
    s.prefs.general.create_layers_at_comp_start = false;
    s.execute("comp.new", json!({"name": "Main", "width": 320, "height": 180, "frameRate": 30, "duration": 4})).unwrap();
    s.execute("layer.newSolid", json!({"name": "Bottom"})).unwrap();
    s.execute("layer.newSolid", json!({"name": "Top"})).unwrap();
    s.execute("layer.select", json!({"layers": [selected]})).unwrap();
    s.execute("time.set", json!({"time": time})).unwrap();
    s
}

/// The active comp's layer names, top first, and where the dropped layer starts.
fn dropped(s: &Session) -> (Vec<String>, Tick) {
    let comp = s.active_comp().unwrap();
    let start = comp.layers.iter().find(|l| l.name == "drop.png").map(|l| l.in_point).unwrap();
    (comp.layers.iter().map(|l| l.name.clone()).collect(), start)
}

#[test]
fn a_media_browser_drop_lands_above_the_selected_layer_at_the_current_time() {
    let drop = json!({"paths": ["/media/drop.png"], "sequence": false, "addToComp": true});
    let mut s = session("Bottom", 2.0);
    s.execute("mediaBrowser.import", drop.clone()).unwrap();
    assert_eq!(dropped(&s), (vec!["Top".into(), "drop.png".into(), "Bottom".into()], Tick::from_seconds_f64(2.0)));
    // The same call with another selection and current time lands elsewhere: both are inputs.
    let mut s = session("Top", 1.0);
    s.execute("mediaBrowser.import", drop.clone()).unwrap();
    assert_eq!(dropped(&s), (vec!["drop.png".into(), "Top".into(), "Bottom".into()], Tick::from_seconds_f64(1.0)));
    // Given an index and a time, neither is read.
    let mut given = drop;
    given["index"] = json!(3);
    given["time"] = json!(0.5);
    for (selected, time) in [("Bottom", 2.0), ("Top", 1.0)] {
        let mut s = session(selected, time);
        s.execute("mediaBrowser.import", given.clone()).unwrap();
        assert_eq!(dropped(&s), (vec!["Top".into(), "Bottom".into(), "drop.png".into()], Tick::from_seconds_f64(0.5)));
    }
}
