use kition_motion_engine::Session;
use kition_motion_project::{ItemId, LayerId};
use kition_motion_time::Tick;
use serde_json::json;

#[test]
fn nearest_spatial_key_with_hostile_stored_ticks_uses_an_overflow_safe_distance() {
    let mut session = Session::default();
    let comp = session.execute("comp.new", json!({"name":"Extreme spatial time"})).unwrap()["comp"].as_u64().unwrap();
    let layer = session.execute("layer.newNull", json!({})).unwrap()["layer"].as_u64().unwrap();
    for (time, value) in [(0, [0, 0, 0]), (1, [10, 20, 0])] {
        session.execute("prop.addKey", json!({"layer":layer,"path":"transform/position","time":time,"value":value})).unwrap();
    }
    let project = std::sync::Arc::make_mut(&mut session.project);
    let position = project.comp_mut(ItemId(comp)).unwrap().layer_mut(LayerId(layer)).unwrap().props.prop_mut("transform/position").unwrap();
    position.keys[0].time = Tick(i64::MIN);
    for key in &mut position.keys {
        key.spatial_auto = false;
    }
    session.execute("keys.setSpatialTangents", json!({"layer":layer,"path":"transform/position","time":1e100,"out":[3,4,0]})).unwrap();
    let position = session.project.comp(ItemId(comp)).unwrap().layer(LayerId(layer)).unwrap().props.prop("transform/position").unwrap();
    assert_eq!(position.keys[0].spatial_out, [0.0; 3]);
    assert_eq!(position.keys[1].spatial_out, [3.0, 4.0, 0.0]);
    assert_eq!(position.keys[1].spatial_in, [-3.0, -4.0, 0.0]);
}
