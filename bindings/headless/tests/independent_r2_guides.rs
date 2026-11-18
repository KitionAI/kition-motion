mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn guides() -> Vec<u8> {
    let mut doc = MotionDocument::create(FILE, 64, 64).unwrap();
    for orientation in ["vertical", "horizontal", "vertical"] {
        doc.execute("view.addGuide", json!({"orientation":orientation,"position":12})).unwrap();
    }
    doc.save(None).unwrap()
}

#[tokio::test]
async fn indexed_guide_write_refuses_a_different_guide_at_the_same_position() {
    for change in ["remove", "swap"] {
        let fixture = Fixture::from_bytes(&guides(), Default::default());
        let (_, comp, basis) = fixture.inspect().await;
        if change == "remove" {
            fixture.colleague(&[("view.removeGuide", json!({"index":0}))]);
        } else {
            rewrite(&fixture.file(), |project| {
                project["items"][comp.to_string()]["kind"]["guides"].as_array_mut().unwrap().swap(0, 1);
            });
        }
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"view.moveGuide","params":{"index":0,"position":30}}]})).await;
        let error = answer.expect_err("replacing the observed guide must not redirect the write to a horizontal guide");
        assert_conflict(&error, &comp.to_string(), "changed");
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn indexed_guide_write_accepts_a_change_to_a_different_guide() {
    let fixture = Fixture::from_bytes(&guides(), Default::default());
    let (_, _, basis) = fixture.inspect().await;
    fixture.colleague(&[("view.moveGuide", json!({"index":2,"position":40}))]);
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"view.moveGuide","params":{"index":0,"position":30}}]})).await;
    assert!(answer.is_ok(), "moving another guide does not change the observed guide: {answer:?}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn guide_append_accepts_an_existing_guides_position_change() {
    let fixture = Fixture::from_bytes(&guides(), Default::default());
    let (_, _, basis) = fixture.inspect().await;
    fixture.colleague(&[("view.moveGuide", json!({"index":0,"position":40}))]);
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"view.addGuide","params":{"orientation":"vertical","position":30}}]})).await;
    assert!(answer.is_ok(), "changing an existing guide leaves the trailing insertion gap intact: {answer:?}");
    fixture.shutdown().await;
}

#[tokio::test]
async fn guide_append_refuses_an_insertion_into_the_observed_gap() {
    let fixture = Fixture::from_bytes(&guides(), Default::default());
    let (_, _, basis) = fixture.inspect().await;
    fixture.colleague(&[("view.addGuide", json!({"position":40}))]);
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"view.addGuide","params":{"position":30}}]})).await;
    let error = answer.expect_err("an insertion into the observed end gap changes the address");
    assert_eq!(error["error"]["code"], "conflict", "{error}");
    fixture.shutdown().await;
}
