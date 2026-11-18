use kition_motion_engine::Session;
use kition_motion_project::{ItemId, LayerId};
use serde_json::json;

#[test]
fn cross_comp_pick_whip_does_not_resolve_target_in_source_comp() {
    let mut session = Session::default();
    let source = session.execute("comp.new", json!({"name": "Source", "width": 16, "height": 16})).unwrap()["comp"].as_u64().unwrap();
    let source_layer = session.execute("layer.newNull", json!({"name": "Source layer"})).unwrap()["layer"].as_u64().unwrap();
    session.execute("prop.setExpression", json!({"comp": source, "layer": source_layer, "path": "transform/opacity", "expression": "value"})).unwrap();
    let target = session.execute("comp.new", json!({"name": "Target", "width": 16, "height": 16})).unwrap()["comp"].as_u64().unwrap();
    let target_layer = session.execute("layer.newNull", json!({"name": "Target layer"})).unwrap()["layer"].as_u64().unwrap();
    let result = session.execute(
        "prop.pickWhip",
        json!({
            "comp": source, "layer": source_layer, "path": "transform/opacity",
            "target": {"comp": target, "layer": target_layer, "path": "transform/opacity"}
        }),
    );
    let error = result.unwrap_err().to_string();
    assert!(error.contains("cross-composition"), "{error}");
    let source = session.project.comp(ItemId(source)).unwrap().layer(LayerId(source_layer)).unwrap();
    assert_eq!(source.props.prop("transform/opacity").unwrap().expr.as_ref().unwrap().text, "value");
}
