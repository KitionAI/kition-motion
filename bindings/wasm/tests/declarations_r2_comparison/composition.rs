//! Literal document projection of the frozen composition rows. SDK defaults are not inferred.
use super::super::{Row, families};
use super::{facts, premise};
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};
use std::collections::BTreeSet;

struct Case<'a> {
    row: &'a Row,
    scope: String,
    params: Value,
    required: BTreeSet<String>,
}

fn cases(comp: u64) -> Vec<Case<'static>> {
    let mut cases = Vec::new();
    for (_, rows) in families() {
        for row in rows {
            if row.command == "comp.settings" && row.path == "supplied scalar settings" {
                // Classic renderer stops the geometry-options branch. Each assignment still
                // holds its old field under the card's same-field decision.
                for (parameter, value, field) in [
                    ("name", json!("Renamed"), "name"),
                    ("frameRate", json!(24), "comp.attrs.frame_rate"),
                    ("startTime", json!(2), "comp.attrs.display_start"),
                    ("renderer", json!("classic3D"), "comp.attrs.renderer"),
                    ("pixelAspect", json!(2), "comp.attrs.pixel_aspect"),
                    ("background", json!("#123456"), "comp.attrs.background"),
                    ("shutterAngle", json!(90), "comp.attrs.shutter_angle"),
                    ("shutterPhase", json!(-90), "comp.attrs.shutter_phase"),
                    ("motionBlurSamples", json!(8), "comp.attrs.motion_blur_samples"),
                    ("adaptiveSampleLimit", json!(32), "comp.attrs.motion_blur_adaptive_limit"),
                    ("preserveFrameRate", json!(true), "comp.attrs.preserve_frame_rate"),
                    ("preserveResolution", json!(true), "comp.attrs.preserve_resolution"),
                ] {
                    let mut params = json!({"comp":comp});
                    params[parameter] = value;
                    cases.push(Case { row, scope: format!("classic/{parameter}"), params, required: premise(json!(comp), &[field, "comp.attrs.renderer"]) });
                }
            } else if row.command == "comp.settings" && row.path == "startTimecode without startTime" {
                cases.push(Case {
                    row,
                    scope: "classic/timecode".to_owned(),
                    params: json!({"comp":comp,"startTimecode":"00:00:02:00"}),
                    required: premise(json!(comp), &["comp.attrs.display_start", "comp.attrs.frame_rate", "comp.attrs.renderer"]),
                });
            } else if row.command == "comp.settings" && row.path == "duration supplied" {
                cases.push(Case {
                    row,
                    scope: "classic/duration".to_owned(),
                    params: json!({"comp":comp,"duration":2}),
                    required: premise(json!(comp), &["comp.attrs.duration", "comp.attrs.work_area", "comp.attrs.frame_rate", "comp.attrs.renderer"]),
                });
            } else if row.command == "comp.setSwitch" {
                for (name, field) in [
                    ("hideShy", "comp.attrs.hide_shy"),
                    ("motionBlur", "comp.attrs.enable_motion_blur"),
                    ("frameBlending", "comp.attrs.enable_frame_blending"),
                    ("draft3d", "comp.attrs.draft_3d"),
                ] {
                    let mut params = json!({"comp":comp,"switch":name});
                    if row.path == "explicit value" {
                        params["value"] = json!(true);
                    }
                    cases.push(Case { row, scope: name.to_owned(), params, required: premise(json!(comp), &[field]) });
                }
            } else if row.command == "comp.setPosterTime" {
                cases.push(Case {
                    row,
                    scope: "current-time".to_owned(),
                    params: json!({"comp":comp}),
                    required: premise(json!(comp), &["comp.attrs.poster_time"]),
                });
            }
        }
    }
    cases
}

pub(super) fn analyze() -> Value {
    let mut document = MotionDocument::create("r2-composition", 64, 64).unwrap();
    let comp = document.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    document.execute("layer.newSolid", json!({"name":"Source"})).unwrap();
    let mut report = Vec::new();
    let mut signatures = BTreeSet::new();
    let mut rows = BTreeSet::new();
    for case in cases(comp) {
        let actual = facts(document.under(case.row.command, &case.params).unwrap());
        // "comp exists" is `Project::comp(id)`, which is true only for a composition item:
        // the item's kind is that eligibility check, for every ID-resolved composition.
        let mut required = case.required.clone();
        required.insert(format!("field:{comp}:kind"));
        let case = Case { required, ..case };
        let less: Vec<_> = case.required.difference(&actual).cloned().collect();
        let more: Vec<_> = actual.difference(&case.required).cloned().collect();
        let id = format!("{}:{}:{}", case.row.command, case.row.path, case.scope);
        for (kind, differences) in [("less", &less), ("more", &more)] {
            signatures.extend(differences.iter().map(|fact| format!("{id}|{kind}|{fact}")));
        }
        rows.insert(format!("{}:{}", case.row.command, case.row.path));
        report.push(json!({
            "command":case.row.command,"path":case.row.path,"scope":case.scope,"params":case.params,
            "source":case.row.source,"required":case.required,"actual":actual,
            "literal_less":less,"literal_more":more,
            "complete_path":false,
            "remaining":"Canonical classic-3D ID fixture only; compound assignments, address spellings, advanced geometry synchronization and SDK static-role defaults are not certified. Literal differences are not safety LESS/MORE counts."
        }));
    }
    json!({"scope":"Literal Under projection, separate from SDK's effective premises.","probes":report,"rows":rows,"signatures":signatures})
}

#[test]
fn composition_projection_preserves_every_named_baseline_probe() {
    let report = analyze();
    if let Ok(path) = std::env::var("MOTION_R2_COMPOSITION_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert_eq!(report["probes"].as_array().unwrap().len(), 23);
    assert_eq!(report["rows"].as_array().unwrap().len(), 6);
    assert!(report["signatures"].as_array().unwrap().is_empty(), "{}", report["signatures"]);
}
