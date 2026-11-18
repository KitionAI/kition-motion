mod common;

use common::*;
use kition_kit::KitDocument;
use kition_kits::tools::CallOptions;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn fixture() -> (Fixture, u64, u64) {
    let mut document = MotionDocument::create(FILE, 64, 64).unwrap();
    let text = document.execute("layer.newText", json!({"text":"abcdef"})).unwrap()["layer"].as_u64().unwrap();
    let unrelated = document.execute("layer.newSolid", json!({"name":"Unrelated"})).unwrap()["layer"].as_u64().unwrap();
    (Fixture::from_bytes(&document.save(None).unwrap(), CallOptions::default()), text, unrelated)
}

#[tokio::test]
async fn text_range_write_refuses_a_colleagues_changed_source_text() {
    let (fixture, text, _) = fixture();
    let (_, comp, basis) = fixture.inspect().await;
    fixture.colleague(&[("layer.setText", json!({"layer":text,"text":"uvwxyz"}))]);
    let error = fixture
        .refused(
            "run",
            json!({"basis":basis,"target":{"comp":comp},"steps":[
                {"command":"text.setSelection","params":{"layer":text,"start":1,"end":4}},
                {"command":"text.insert","params":{"layer":text,"text":"Q"}}
            ]}),
        )
        .await;
    assert_conflict(&error, &text.to_string(), "changed");
    assert_eq!(value_on_disk(&fixture.file(), text, "text/sourceText"), "uvwxyz");
    fixture.shutdown().await;
}

#[tokio::test]
async fn text_range_write_accepts_a_colleagues_unrelated_layer_edit() {
    let (fixture, text, unrelated) = fixture();
    let (_, comp, basis) = fixture.inspect().await;
    fixture.colleague(&[opacity(unrelated, 23.0)]);
    fixture
        .ok(
            "run",
            json!({"basis":basis,"target":{"comp":comp},"steps":[
                {"command":"text.setSelection","params":{"layer":text,"start":1,"end":4}},
                {"command":"text.insert","params":{"layer":text,"text":"Q"}}
            ]}),
        )
        .await;
    assert_eq!(value_on_disk(&fixture.file(), text, "text/sourceText"), "aQef");
    assert_eq!(opacity_on_disk(&fixture.file(), unrelated), 23.0);
    fixture.shutdown().await;
}
