mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn large_nearest_membership_accepts_an_external_distant_insertion() {
    large_history_probe(false, false).await;
}

#[tokio::test]
async fn large_nearest_membership_accepts_another_agents_distant_insertion() {
    large_history_probe(true, false).await;
}

#[tokio::test]
async fn large_exact_key_history_refuses_an_addressed_keys_deletion() {
    for same_server in [false, true] {
        large_history_probe(same_server, true).await;
    }
}

async fn large_history_probe(same_server: bool, deletion: bool) {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[0];
    for frame in 0..50 {
        document.execute("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":frame as f64/30.0,"value":frame})).unwrap();
    }
    document.execute("keys.setEase", json!({"layer":layer,"path":OPACITY,"time":1,"side":"out","speed":1})).unwrap();
    let uid = document.execute("prop.get", json!({"layer":layer,"path":OPACITY})).unwrap()["uid"].clone();
    assert!(document.derived(&[], &[]).unwrap()[0].values["keyTimes"].to_string().len() > 200);
    let fixture = Fixture::from_bytes(&document.save(None).unwrap(), Default::default());
    let (_, _, basis) = fixture.inspect().await;
    let changes = if deletion {
        vec![("keys.select", json!({"keys":[{"layer":layer,"path":OPACITY,"time":1}]})), ("keys.delete", json!({}))]
    } else {
        vec![("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":3,"value":30}))]
    };
    if same_server {
        let changes: Vec<_> = changes.into_iter().map(|(command, params)| json!({"command":command,"params":params})).collect();
        fixture.call_by("other", "run", json!({"steps":changes})).await;
    } else {
        fixture.colleague(&changes);
    }
    let answer = fixture
        .call("run", json!({"basis":basis,"steps":[{"command":"keys.setEase","params":{"layer":layer,"prop":uid,"time":1,"side":"out","speed":2}}]}))
        .await;
    if deletion {
        assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
    } else {
        assert!(answer.is_ok(), "a distant insertion keeps the addressed key and stored ease unchanged: {answer:?}");
    }
    fixture.shutdown().await;
}
