//! Regression: a 3D primitive's Metallic and Roughness are percents
//! (0 to 100, as their sliders show) and say so, roughness visibly spreads the highlight, and a
//! material option a layer does not have (Specular Intensity is the Classic material's, not a
//! primitive's) is refused rather than ignored.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn run(doc: &mut MotionDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

/// A lit sphere of the given roughness, rendered.
fn sphere(roughness: f64) -> Vec<u8> {
    let mut doc = MotionDocument::create("planet.kmotion", 320, 240).unwrap();
    run(&mut doc, "layer.newSolid", json!({ "name": "Space", "color": "#000000" }));
    run(&mut doc, "layer.new3dPrimitive", json!({ "kind": "sphere", "radius": 90, "position": [160, 120, 0], "baseColor": "#d98a4e", "roughness": roughness }));
    let key = run(&mut doc, "layer.newLight", json!({ "kind": "Parallel", "intensity": 130 }))["layer"].clone();
    run(&mut doc, "layer.lightSettings", json!({ "layer": key, "position": [20, 30, -420], "poi": [160, 120, 0] }));
    doc.render(0.0, 0.0, 1.0, 320, 240).unwrap()
}

/// Pixels brighter than `level` in every channel.
fn bright(pixels: &[u8], level: u8) -> usize {
    pixels.as_chunks::<4>().0.iter().filter(|px| px[0] > level && px[1] > level && px[2] > level).count()
}

#[test]
fn roughness_spreads_the_highlight() {
    let smooth = sphere(25.0);
    let rough = sphere(100.0);
    let changed =
        smooth.as_chunks::<4>().0.iter().zip(rough.as_chunks::<4>().0).filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 24)).count();
    assert!(changed > 500, "roughness 5 and 100 look different ({changed} pixels differ)");
    assert!(bright(&smooth, 240) > bright(&rough, 240), "a smooth sphere has a sharp, bright highlight");
}

#[test]
fn material_options_say_their_unit_and_must_exist() {
    let mut doc = MotionDocument::create("planet.kmotion", 320, 240).unwrap();
    for command in ["layer.new3dPrimitive", "material.set"] {
        let spec = doc.commands().into_iter().find(|c| c.id == command).unwrap();
        let params = spec.params.unwrap_or_default();
        assert!(params.contains("roughness?: 0-100") && params.contains("metallic?: 0-100"), "{command} names the unit: {params}");
    }
    let planet = run(&mut doc, "layer.new3dPrimitive", json!({ "kind": "sphere", "roughness": 75 }))["layer"].clone();
    let refused = doc.execute("material.set", json!({ "layers": [planet], "specularIntensity": 10, "roughness": 90 }));
    let error = format!("{:?}", refused.expect_err("a primitive has no Specular Intensity"));
    assert!(error.contains("specularIntensity"), "{error}");
    run(&mut doc, "material.set", json!({ "layers": [planet], "roughness": 90 }));
}
