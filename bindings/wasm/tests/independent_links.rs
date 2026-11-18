//! C8-motion-read: delegated expression edits retain the addressed composition.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[test]
fn pick_whip_assigns_in_its_explicit_composition() {
    for insert in [false, true] {
        let mut doc = MotionDocument::create("Links.kmotion", 64, 36).unwrap();
        let main = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
        let other = doc.execute("comp.new", json!({"name":"Other"})).unwrap()["comp"].as_u64().unwrap();
        let source = doc.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
        let target = doc.execute("layer.newSolid", json!({"name":"Target"})).unwrap()["layer"].as_u64().unwrap();
        doc.execute("comp.open", json!({"comp":main})).unwrap();
        let mut params = json!({"comp":other,"layer":source,"path":"transform/opacity","target":{"comp":other,"layer":target,"path":"transform/rotation"}});
        if insert {
            params["expression"] = json!("value + ");
        }
        let reply = doc.execute("prop.pickWhip", params).unwrap();
        let mut reopened = MotionDocument::open("Links.kmotion", &doc.save(None).unwrap()).unwrap();
        let got = reopened.execute("prop.get", json!({"comp":other,"layer":source,"path":"transform/opacity"})).unwrap();
        let expected = if insert { "value + thisComp.layer(\"Target\").transform.rotation" } else { "thisComp.layer(\"Target\").transform.rotation" };
        assert_eq!(reply, expected);
        assert_eq!(got["expression"], expected);
        assert_eq!(doc.inspect().unwrap()["comp"]["id"], main);
    }
}
