#![allow(clippy::unwrap_used, clippy::panic)]

use kition_motion_engine::Session;
use serde_json::json;

#[test]
fn effect_copy_uses_the_explicit_composition() {
    let mut session = Session::default();
    let source = session.execute("comp.new", json!({"name":"Source","width":16,"height":16})).unwrap()["comp"].as_u64().unwrap();
    let layer = session.execute("layer.newNull", json!({"name":"Source layer"})).unwrap()["layer"].as_u64().unwrap();
    let effect = session.execute("effect.apply", json!({"layers":[layer],"effect":"Gaussian Blur"})).unwrap()["effects"][0].as_u64().unwrap();
    session.execute("comp.new", json!({"name":"Other","width":16,"height":16})).unwrap();
    let copied = session.execute("effect.copy", json!({"comp":source,"layer":layer,"effect":effect})).unwrap();
    assert_eq!(copied["effects"], 1);
    let target = session.execute("layer.newNull", json!({"name":"Destination"})).unwrap()["layer"].as_u64().unwrap();
    let pasted = session.execute("effect.paste", json!({"layers":[target]})).unwrap();
    assert_eq!(pasted["effects"].as_array().unwrap().len(), 1);
}
