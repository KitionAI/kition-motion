mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn effect_ordinal_refuses_an_earlier_deletion_and_preserves_unrelated_edits() {
    let mut document = MotionDocument::open(FILE, &project(2)).unwrap();
    let layers = layer_ids(&document.inspect().unwrap()["info"]);
    let effects: Vec<_> = ["Gaussian Blur", "Fill", "Gaussian Blur"]
        .into_iter()
        .map(|name| document.execute("effect.apply", json!({"layer":layers[0],"effect":name})).unwrap()["effects"][0].as_u64().unwrap())
        .collect();
    let bytes = document.save(None).unwrap();
    for earlier in [true, false] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[if earlier { ("effect.remove", json!({"layer":layers[0],"effect":effects[0]})) } else { opacity(layers[1], 23.0) }]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"effect.remove","params":{"layer":layers[0],"effect":2}}]})).await;
        if earlier {
            let error = answer.expect_err("ordinal 2 must not delete the former third effect");
            assert_conflict(&error, &layers[0].to_string(), "relocated");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&fixture.file(), layers[1]), 23.0);
        }
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn effect_ordinal_accepts_reordering_its_containing_layer() {
    let mut document = MotionDocument::open(FILE, &project(3)).unwrap();
    let layer = layer_ids(&document.inspect().unwrap()["info"])[1];
    for effect in ["Gaussian Blur", "Fill", "Gaussian Blur"] {
        document.execute("effect.apply", json!({"layer":layer,"effect":effect})).unwrap();
    }
    let fixture = Fixture::from_bytes(&document.save(None).unwrap(), Default::default());
    let (_, _, basis) = fixture.inspect().await;
    fixture.colleague(&[("layer.arrange", json!({"layers":[layer],"to":"front"}))]);
    let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"effect.remove","params":{"layer":layer,"effect":2}}]})).await;
    assert!(answer.is_ok(), "layer movement does not change the effect ordinal: {answer:?}");
    assert_eq!(layers_on_disk(&fixture.file())[0], layer);
    fixture.shutdown().await;
}
