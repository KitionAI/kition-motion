#![allow(clippy::unwrap_used, clippy::panic)]

use kition_motion_engine::Session;
use serde_json::json;

#[test]
fn effect_reset_uses_the_addressed_compositions_current_time() {
    let mut session = Session::default();
    let comp = session.execute("comp.new", json!({"name":"Source","width":16,"height":16,"duration":5,"frameRate":30})).unwrap()["comp"].as_u64().unwrap();
    let layer = session.execute("layer.newNull", json!({"name":"Source layer"})).unwrap()["layer"].as_u64().unwrap();
    let effect = session.execute("effect.apply", json!({"layers":[layer],"effect":"Gaussian Blur"})).unwrap()["effects"][0].as_u64().unwrap();
    let path = "effects/#1/blurriness";
    for (time, value) in [(0, 10), (4, 20)] {
        session.execute("prop.addKey", json!({"layer":layer,"path":path,"time":time,"value":value})).unwrap();
    }
    session.execute("time.set", json!({"time":2})).unwrap();
    session.execute("comp.new", json!({"name":"Other","width":16,"height":16})).unwrap();
    session.execute("effect.reset", json!({"comp":comp,"layer":layer,"effect":effect})).unwrap();
    let result = session.execute("prop.get", json!({"comp":comp,"layer":layer,"path":path})).unwrap();
    let keys = result["keys"].as_array().unwrap();
    let times: Vec<_> = keys.iter().map(|key| key["compTime"].as_f64().unwrap()).collect();
    assert_eq!(times, [0.0, 2.0, 4.0]);
    let values: Vec<_> = keys.iter().map(|key| key["value"].as_f64().unwrap()).collect();
    assert_eq!(values, [10.0, 0.0, 20.0]);
}
