mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

/// A solid with three effects (Gaussian Blur, Fill, Tint) and three comp markers: the agent
/// reads, a colleague's engine changes the file, and the agent runs `step` against its basis.
async fn stale(change: &[(&str, Value)], step: Value) -> Result<Value, Value> {
    let f = Fixture::new(1);
    let (info, _, _) = f.inspect().await;
    let layer = layer_ids(&info)[0];
    let mut steps = Vec::new();
    for effect in ["Gaussian Blur", "Fill", "Tint"] {
        steps.push(json!({"command":"effect.apply","params":{"effect":effect,"layers":[layer]}}));
    }
    for (time, comment) in [(1, "C1"), (4, "C4"), (8, "C8")] {
        steps.push(json!({"command":"comp.addMarker","params":{"time":time,"comment":comment}}));
    }
    f.ok("run", json!({"steps":steps})).await;
    let (_, _, basis) = f.inspect().await;
    drop(f.colleague(change));
    let saved = std::fs::read(f.file()).unwrap();
    let answer = f.call("run", json!({"basis":basis,"steps":[step]})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
    }
    f.shutdown().await;
    answer
}

fn layer() -> u64 {
    layer_ids(&MotionDocument::open(FILE, &project(1)).unwrap().inspect().unwrap()["info"])[0]
}

/// Removing an effect by uid holds that effect, not its siblings: another effect deleted since
/// applies; the named effect changed since refuses.
#[tokio::test]
async fn removing_an_effect_by_uid_holds_that_effect_only() {
    let layer = layer();
    // The second effect's uid, as the same setup gives it.
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let fill = ["Gaussian Blur", "Fill", "Tint"]
        .map(|effect| doc.execute("effect.apply", json!({"effect":effect,"layers":[layer]})).unwrap()["effects"][0].as_u64().unwrap())[1];
    let remove = json!({"command":"effect.remove","params":{"layer":layer,"effect":fill}});
    for sibling in [1, 3] {
        let other = stale(&[("effect.remove", json!({"layer":layer,"effect":sibling}))], remove.clone()).await;
        assert!(other.is_ok(), "deleting effect #{sibling}: {other:?}");
    }
    let changed = stale(&[("prop.set", json!({"layer":layer,"path":"effects/#2/opacity","value":50}))], remove).await;
    assert_eq!(changed.expect_err("the named effect changed")["error"]["code"], "conflict");
}

/// Setting a comp marker by index holds that marker: a marker added after it applies; the
/// marker itself edited since refuses.
#[tokio::test]
async fn setting_a_marker_by_index_holds_that_marker_only() {
    let set = json!({"command":"marker.set","params":{"index":1,"comment":"Mine"}});
    let later = stale(&[("comp.addMarker", json!({"time":9,"comment":"C9"}))], set.clone()).await;
    assert!(later.is_ok(), "{later:?}");
    let edited = stale(&[("marker.set", json!({"index":1,"comment":"Theirs"}))], set).await;
    assert_eq!(edited.expect_err("the marker was edited")["error"]["code"], "conflict");
}
