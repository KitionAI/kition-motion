mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn comp_space_nudge_holds_parent_transform_and_allows_parent_opacity() {
    let mut document = MotionDocument::open(FILE, &project(1)).unwrap();
    let child = layer_ids(&document.inspect().unwrap()["info"])[0];
    let parent = document.execute("layer.newNull", json!({"name":"Parent"})).unwrap()["layer"].as_u64().unwrap();
    document.execute("prop.set", json!({"layer":parent,"path":"transform/scale","value":[200,200,100]})).unwrap();
    document.execute("layer.setParent", json!({"layers":[child],"parent":parent,"compensate":false})).unwrap();
    let before = document.execute("prop.get", json!({"layer":child,"path":"transform/position"})).unwrap()["value"].clone();
    let bytes = document.save(None).unwrap();
    for transform_changed in [true, false] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        fixture.colleague(&[if transform_changed {
            ("prop.set", json!({"layer":parent,"path":"transform/scale","value":[150,150,100]}))
        } else {
            opacity(parent, 23.0)
        }]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"layer.nudge","params":{"layers":[child],"x":10}}]})).await;
        if transform_changed {
            assert_conflict(&answer.unwrap_err(), &parent.to_string(), "changed");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "parent opacity is not used by a comp-space nudge: {answer:?}");
            let position = value_on_disk(&fixture.file(), child, "transform/position");
            assert_eq!(position[0].as_f64().unwrap(), before[0].as_f64().unwrap() + 5.0);
            assert_eq!(opacity_on_disk(&fixture.file(), parent), 23.0);
        }
        fixture.shutdown().await;
    }
}
