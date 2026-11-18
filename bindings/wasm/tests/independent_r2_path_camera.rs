use kition_motion_engine::Session;
use kition_motion_project::{ItemId, LayerId};
use serde_json::json;

fn masked() -> (Session, ItemId, LayerId, u64, u64) {
    let mut session = Session::default();
    let comp = ItemId(session.execute("comp.new", json!({"name":"Paths","width":64,"height":64,"frameRate":30})).unwrap()["comp"].as_u64().unwrap());
    let layer = LayerId(session.execute("layer.newSolid", json!({})).unwrap()["layer"].as_u64().unwrap());
    let mask = session.execute("mask.new", json!({"layer":layer.0,"vertices":[[0,0],[20,0],[20,20]],"closed":true})).unwrap()["mask"].as_u64().unwrap();
    let path = session.project.comp(comp).unwrap().layer(layer).unwrap().props.find_group(mask).unwrap().get("path").unwrap().uid;
    (session, comp, layer, mask, path)
}

#[test]
fn first_vertex_keeps_the_explicit_composition() {
    let (mut session, comp, layer, mask, _) = masked();
    session.execute("comp.new", json!({"name":"Other"})).unwrap();
    session.execute("path.setFirstVertex", json!({"comp":comp.0,"layer":layer.0,"mask":mask,"index":1})).unwrap();
    let property = session.project.comp(comp).unwrap().layer(layer).unwrap().props.find_group(mask).unwrap().get("path").unwrap();
    let path = property.value.as_path().unwrap();
    assert_eq!(path.vertices, [[20.0, 0.0], [20.0, 20.0], [0.0, 0.0]]);
}

#[test]
fn free_transform_uses_the_addressed_compositions_time() {
    let (mut session, comp, layer, mask, path) = masked();
    for time in [0, 4] {
        session.execute("prop.addKey", json!({"layer":layer.0,"prop":path,"time":time})).unwrap();
    }
    session.execute("time.set", json!({"time":3})).unwrap();
    session.execute("comp.new", json!({"name":"Other"})).unwrap();
    session.execute("path.freeTransform", json!({"comp":comp.0,"layer":layer.0,"mask":mask,"offset":[10,0]})).unwrap();
    let property = session.project.comp(comp).unwrap().layer(layer).unwrap().props.find(path).unwrap();
    let times: Vec<_> = property.keys.iter().map(|key| key.time.seconds()).collect();
    assert_eq!(times, [0.0, 3.0, 4.0]);
}

#[test]
fn look_at_converts_comp_time_to_layer_time_once() {
    for command in ["view.lookAtSelected", "view.lookAtAll"] {
        let mut session = Session::default();
        let comp = ItemId(session.execute("comp.new", json!({"name":"Camera","width":64,"height":64,"frameRate":30})).unwrap()["comp"].as_u64().unwrap());
        let target = session.execute("layer.newSolid", json!({})).unwrap()["layer"].as_u64().unwrap();
        let camera = LayerId(session.execute("layer.newCamera", json!({"position":[32,32,-1000],"poi":[32,32,0]})).unwrap()["layer"].as_u64().unwrap());
        session.execute("layer.timing", json!({"layers":[camera.0],"start":1})).unwrap();
        for path in ["transform/position", "transform/poi"] {
            for time in [1, 5] {
                session.execute("prop.addKey", json!({"layer":camera.0,"path":path,"time":time,"timeBase":"comp"})).unwrap();
            }
        }
        session.execute("time.set", json!({"time":3})).unwrap();
        session.execute(command, json!({"layers":[target]})).unwrap();
        let layer = session.project.comp(comp).unwrap().layer(camera).unwrap();
        for path in ["transform/position", "transform/poi"] {
            let times: Vec<_> = layer.props.prop(path).unwrap().keys.iter().map(|key| key.time.seconds()).collect();
            assert_eq!(times, [0.0, 2.0, 4.0], "{command} {path}");
        }
    }
}
