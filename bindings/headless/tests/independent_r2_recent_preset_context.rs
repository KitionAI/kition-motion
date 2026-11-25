mod common;

use common::*;
use kition_kit::KitDocument;
use kition_kits::tools::CallOptions;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn fixture() -> (Fixture, u64, u64) {
    let mut document = MotionDocument::create(FILE, 64, 64).unwrap();
    let source = document.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
    for (time, value) in [(0, 0), (1, 100)] {
        document.execute("prop.addKey", json!({"layer":source,"path":OPACITY,"time":time,"value":value})).unwrap();
    }
    document.execute("prop.select", json!({"layer":source,"path":OPACITY})).unwrap();
    document.execute("animation.savePreset", json!({"path":"fade.ecpreset"})).unwrap();
    let preset = document.take_file("fade.ecpreset").unwrap();
    let target = document.execute("layer.newSolid", json!({"name":"Target"})).unwrap()["layer"].as_u64().unwrap();
    let unrelated = document.execute("layer.newSolid", json!({"name":"Unrelated"})).unwrap()["layer"].as_u64().unwrap();
    let fixture = Fixture::from_bytes(&document.save(None).unwrap(), CallOptions::default());
    std::fs::write(fixture.workspace.path().join("fade.ecpreset"), preset).unwrap();
    (fixture, target, unrelated)
}

#[tokio::test]
async fn recent_preset_at_a_call_time_refuses_a_changed_destination_and_accepts_an_unrelated_edit() {
    for target_changed in [true, false] {
        let (fixture, target, unrelated) = fixture();
        let (_, comp, basis) = fixture.inspect().await;
        if target_changed {
            fixture.colleague(&[("prop.addKey", json!({"layer":target,"path":OPACITY,"time":3,"value":55}))]);
        } else {
            fixture.colleague(&[opacity(unrelated, 23.0)]);
        }
        let answer = fixture
            .call(
                "run",
                json!({"basis":basis,"target":{"comp":comp},"steps":[
                {"command":"animation.applyRecentPreset","at":{"time":3},"params":{"path":"fade.ecpreset","layers":[target]}}
                ]}),
            )
            .await;
        if target_changed {
            assert_conflict(&answer.unwrap_err(), &target.to_string(), "changed");
            assert_eq!(keys_on_disk(&fixture.file(), target, OPACITY), [(3.0, json!(55.0))]);
        } else {
            assert!(answer.is_ok(), "unrelated layer edit prevented recent preset application: {answer:?}");
            assert_eq!(keys_on_disk(&fixture.file(), target, OPACITY), [(3.0, json!(0.0)), (4.0, json!(100.0))]);
            assert_eq!(opacity_on_disk(&fixture.file(), unrelated), 23.0);
        }
        fixture.shutdown().await;
    }
}
