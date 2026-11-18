mod common;
use common::*;
use serde_json::{Value, json};

/// Another agent of the same server editing the marker a content premise names (`~markerList`,
/// region `{index}`) refuses the write as `changed`, as an outside writer's edit is
/// (sdk_content_regions). Up to kition-sdk 5c1253a it was refused as `unknown` ("Content basis
/// unavailable: the complete prior field value was not retained"), because the SDK kept no
/// derived-fact basis for that revision; since 45bf962 it does, and this test passes.
#[tokio::test]
async fn waiting_for_sdk_another_agents_edit_of_the_indexed_marker_refuses_as_changed() {
    let f = Fixture::new(1);
    let steps: Vec<Value> =
        [(1, "C1"), (4, "C4")].iter().map(|(time, comment)| json!({"command":"comp.addMarker","params":{"time":time,"comment":comment}})).collect();
    f.ok("run", json!({"steps":steps})).await;
    let (_, comp, basis) = f.inspect().await;
    f.call_by("agent-other", "run", json!({"steps":[{"command":"markers.set","params":{"comp":comp,"index":1,"comment":"Theirs"}}]})).await;
    let refused = f.refused("run", json!({"basis":basis,"steps":[{"command":"markers.set","params":{"index":1,"comment":"Mine"}}]})).await;
    assert_conflict(&refused, &comp.to_string(), "changed");
    f.shutdown().await;
}
