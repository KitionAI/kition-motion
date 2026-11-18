use kition_motion_engine::Session;
use kition_motion_project::{ItemId, LayerId};
use kition_motion_time::Tick;
use serde_json::json;

#[test]
fn nearest_key_with_hostile_stored_ticks_uses_an_overflow_safe_distance() {
    let mut session = Session::default();
    let comp = session.execute("comp.new", json!({"name":"Extreme stored time"})).unwrap()["comp"].as_u64().unwrap();
    let layer = session.execute("layer.newNull", json!({})).unwrap()["layer"].as_u64().unwrap();
    for (time, value) in [(0, 10), (1, 20)] {
        session.execute("prop.addKey", json!({"layer":layer,"path":"transform/opacity","time":time,"value":value})).unwrap();
    }
    let project = std::sync::Arc::make_mut(&mut session.project);
    let opacity = project.comp_mut(ItemId(comp)).unwrap().layer_mut(LayerId(layer)).unwrap().props.prop_mut("transform/opacity").unwrap();
    opacity.keys[0].time = Tick(i64::MIN);
    session.execute("keys.setEase", json!({"layer":layer,"path":"transform/opacity","time":1e100,"side":"out","speed":2})).unwrap();
    let opacity = session.project.comp(ItemId(comp)).unwrap().layer(LayerId(layer)).unwrap().props.prop("transform/opacity").unwrap();
    assert!(opacity.keys[0].out_ease.is_empty());
    assert_eq!(opacity.keys[1].out_ease[0].speed, 2.0);
}
