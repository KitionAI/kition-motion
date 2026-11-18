//! C8-motion-read: an explicit composition owns both evaluation and assignment time.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[test]
fn focus_assignment_uses_the_addressed_compositions_time() {
    let mut doc = MotionDocument::create("Focus.kmotion", 320, 180).unwrap();
    let main = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let other = doc.execute("comp.new", json!({"name":"Other", "duration":5,"frameRate":30})).unwrap()["comp"].as_u64().unwrap();
    let target = doc.execute("layer.newSolid", json!({"name":"Target"})).unwrap()["layer"].as_u64().unwrap();
    let camera = doc.execute("layer.newCamera", json!({"name":"Camera"})).unwrap()["layer"].as_u64().unwrap();
    let path = "cameraOptions/focusDistance";
    for time in [0, 4] {
        doc.execute("prop.addKey", json!({"layer":camera,"path":path,"time":time,"value":100})).unwrap();
    }
    doc.execute("time.set", json!({"time":2})).unwrap();
    doc.execute("comp.open", json!({"comp":main})).unwrap();
    doc.execute("time.set", json!({"time":0})).unwrap();
    doc.execute("camera.setFocusToLayer", json!({"comp":other,"camera":camera,"layer":target})).unwrap();
    let got = doc.execute("prop.get", json!({"comp":other,"layer":camera,"path":path})).unwrap();
    let times: Vec<_> = got["keys"].as_array().unwrap().iter().map(|key| key["compTime"].as_f64().unwrap()).collect();
    assert_eq!(times, [0.0, 2.0, 4.0]);
    assert_eq!(got["keys"][0]["value"].as_f64(), Some(100.0));
}
