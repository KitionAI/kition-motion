//! The KHP conformance fixtures (`conformance/`) and the check catalogue (`agent/checks.json`,
//! from [`crate::check::CATALOGUE`]): `basic.kmotion` (no diagnostics), `edit.json` (steps that
//! change it), one `check-<code>.kmotion` per catalogue code, and `golden/` previews of `basic`.
//! Each fixture starts from a new project made with the engine's own commands; what a project can
//! hold but the commands never make (a layer whose source, parent or matte is gone, footage whose
//! file moved away) is made by editing the project, as a project damaged elsewhere would be.
//!
//! `KITION_WRITE_CONFORMANCE=1 cargo test -p kition-motion-kit conformance` writes them; without the
//! variable the test only checks that every fixture it would write reports its code.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kition_kit::KitDocument;
use kition_motion_project::{Comp, ItemId, ItemKind, LayerId, LayerSource, TrackMatte};
use kition_motion_time::Tick;
use serde_json::{Value, json};

use crate::MotionDocument;
use crate::check::{CATALOGUE, PER_CODE, catalogue_json};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(document: &mut MotionDocument, command: &str, params: Value) -> Value {
    document.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn comp_mut(document: &mut MotionDocument) -> (ItemId, &mut Comp) {
    let id = document.session.active_comp_id().expect("a composition");
    let item = Arc::make_mut(&mut document.session.project).items.get_mut(&id).expect("comp item");
    let ItemKind::Comp(comp) = &mut item.kind else { panic!("a composition") };
    (id, Arc::make_mut(comp))
}

/// A 320 × 180 composition with a blue background solid and a title layer over it.
fn basic() -> MotionDocument {
    let mut document = MotionDocument::create("Title.kmotion", 320, 180).expect("project");
    run(&mut document, "layer.newSolid", json!({ "name": "Background", "color": "#1d3557" }));
    run(&mut document, "layer.newSolid", json!({ "name": "Band", "color": "#5645d4", "width": 320, "height": 60 }));
    document
}

fn fixture(code: &str) -> MotionDocument {
    let mut document = basic();
    match code {
        "motion.missing-footage" => {
            let id = ItemId(document.session.project.next_id + 1000);
            let footage: kition_motion_project::Footage = serde_json::from_value(json!({
                "path": "clips/beach.mp4", "kind": "Video", "width": 320, "height": 180, "pixel_aspect": 1.0,
                "frame_rate": { "num": 30, "den": 1 }, "duration": 0, "has_video": true, "has_audio": false, "alpha": "Straight",
            }))
            .expect("footage");
            let comp = document.session.active_comp_id().expect("a composition");
            let mut item = document.session.project.items[&comp].clone();
            (item.id, item.name, item.kind) = (id, "beach.mp4".into(), ItemKind::Footage(footage));
            Arc::make_mut(&mut document.session.project).items.insert(id, item);
        }
        "motion.missing-source" => comp_mut(&mut document).1.layers[1].source = LayerSource::Footage { item: ItemId(987_654) },
        "motion.missing-parent" => comp_mut(&mut document).1.layers[1].parent = Some(LayerId(123_456)),
        "motion.missing-matte" => {
            comp_mut(&mut document).1.layers[1].track_matte = Some(TrackMatte { layer: LayerId(123_457), kind: Default::default() });
        }
        "motion.zero-duration" => {
            let layer = &mut comp_mut(&mut document).1.layers[1];
            layer.out_point = layer.in_point;
        }
        "motion.outside-comp" => {
            let (_, comp) = comp_mut(&mut document);
            let duration = comp.duration;
            let layer = &mut comp.layers[1];
            layer.in_point = duration;
            layer.out_point = Tick(duration.0 + 10);
        }
        "motion.empty-comp" => comp_mut(&mut document).1.layers.clear(),
        "motion.truncated" => {
            let (_, comp) = comp_mut(&mut document);
            let template = comp.layers[1].clone();
            for n in 0..=PER_CODE as u64 {
                let mut layer = template.clone();
                layer.id = LayerId(10_000 + n);
                layer.parent = Some(LayerId(9_999));
                comp.layers.push(layer);
            }
        }
        // The design advice: a title over the flat background, each made plainer in one way.
        "motion.flat-background" | "motion.no-depth" | "motion.thin-type" | "motion.static-text" => {
            run(&mut document, "layer.newText", json!({ "text": "Launch", "size": 24, "fill": "#ffffff" }));
        }
        "motion.small-title" => _ = run(&mut document, "layer.newText", json!({ "text": "Launch", "size": 10, "fill": "#ffffff" })),
        "motion.weak-hierarchy" => {
            run(&mut document, "layer.newText", json!({ "text": "Launch", "size": 24, "fill": "#ffffff" }));
            run(&mut document, "layer.newText", json!({ "text": "Tagline", "size": 20, "fill": "#ffffff", "position": [160, 130] }));
        }
        "motion.low-contrast" => _ = run(&mut document, "layer.newText", json!({ "text": "Launch", "size": 24, "fill": "#24406a" })),
        "motion.small-text" => _ = run(&mut document, "layer.newText", json!({ "text": "Launch", "size": 4, "fill": "#ffffff" })),
        "motion.missing-font" => {
            run(&mut document, "layer.newText", json!({ "text": "Launch", "size": 24, "fill": "#ffffff", "font": "No Such Family 123" }));
        }
        other => panic!("no fixture for {other}"),
    }
    document
}

fn edit() -> Value {
    json!({ "steps": [
        { "command": "layer.newSolid", "params": { "name": "Red", "color": "#ff0000", "width": 80, "height": 80 } },
    ] })
}

fn png(document: &MotionDocument, scale: f64) -> Vec<u8> {
    let (width, height) = ((f64::from(document.width()) * scale) as u32, (f64::from(document.height()) * scale) as u32);
    let rgba = document.render(0.0, 0.0, scale, width, height).expect("render");
    let picture = image::RgbaImage::from_raw(width, height, rgba).expect("pixels");
    let mut out = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut out, image::ImageFormat::Png).expect("png");
    out.into_inner()
}

fn codes(document: &MotionDocument) -> Vec<String> {
    document.check().into_iter().map(|diagnostic| diagnostic.code).collect()
}

#[test]
fn every_fixture_reports_its_code_and_basic_is_clean() {
    let write = std::env::var_os("KITION_WRITE_CONFORMANCE").is_some();
    let out = repo().join("conformance");
    let basic = basic();
    assert_eq!(codes(&basic), Vec::<String>::new(), "basic.kmotion is clean");
    if write {
        std::fs::create_dir_all(out.join("golden")).expect("conformance/golden");
        std::fs::create_dir_all(repo().join("agent")).expect("agent");
        std::fs::write(out.join("basic.kmotion"), basic.save(None).expect("save")).expect("basic");
        std::fs::write(out.join("edit.json"), format!("{:#}\n", edit())).expect("edit.json");
        std::fs::write(out.join("golden/basic.png"), png(&basic, 1.0)).expect("golden");
        std::fs::write(out.join("golden/basic-scale-2.png"), png(&basic, 2.0)).expect("golden scale 2");
        let tolerance = json!({
            "maxChannelDelta": 2,
            "maxDifferentPixels": 300,
            "note": "The engine's own compositor draws the preview at the composition's current time; edges may differ by a step between platforms."
        });
        std::fs::write(out.join("golden/tolerance.json"), format!("{tolerance:#}\n")).expect("tolerance");
        std::fs::write(repo().join("agent/checks.json"), format!("{:#}\n", catalogue_json())).expect("checks.json");
    }
    let mut failures = Vec::new();
    for entry in CATALOGUE {
        let bytes = fixture(entry.code).save(None).expect("save");
        let reopened = MotionDocument::open("fixture.kmotion", &bytes).expect("reopen");
        let found = codes(&reopened);
        if !found.iter().any(|code| code == entry.code) {
            failures.push(format!("{}: {found:?}", entry.code));
        }
        if write {
            std::fs::write(out.join(format!("check-{}.kmotion", entry.code.trim_start_matches("motion."))), bytes).expect("fixture");
        }
    }
    assert!(failures.is_empty(), "fixtures that lose their code on save: {failures:#?}");
}
