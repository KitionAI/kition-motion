#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn environment_assignment_holds_its_fields_and_accepts_unrelated_opacity() {
    for relevant in [true, false] {
        let fixture = Fixture::new(1);
        let (info, _, basis) = fixture.inspect().await;
        let layer = layer_ids(&info)[0];
        fixture.colleague(&[if relevant { ("layer.environment", json!({"layers":[layer],"on":true})) } else { opacity(layer, 23.0) }]);
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"layer.environment","params":{"layers":[layer],"on":false}}]})).await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "environment assignment refused unrelated opacity: {answer:?}");
            assert_eq!(opacity_on_disk(&fixture.file(), layer), 23.0);
        }
        fixture.shutdown().await;
    }
}
