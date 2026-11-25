use kition_kit::KitDocument;
use kition_motion_engine::{Session, commands::written_key_time};
use kition_motion_kit::MotionDocument;
use kition_motion_project::{ItemId, LayerId};
use serde_json::json;

#[test]
fn prop_set_premise_time_matches_the_snapped_destination() {
    let mut session = Session::default();
    let comp = session.execute("comp.new", json!({"name":"Keys","frameRate":30})).unwrap()["comp"].as_u64().unwrap();
    let layer = session.execute("layer.newNull", json!({})).unwrap()["layer"].as_u64().unwrap();
    session.execute("prop.addKey", json!({"layer":layer,"path":"transform/opacity","time":0,"value":10})).unwrap();
    let params = json!({"layer":layer,"path":"transform/opacity","time":0.017,"value":70});
    let premise_time = written_key_time(&session, &params, "prop.set").unwrap();
    session.execute("prop.set", params).unwrap();
    let property = session.project.comp(ItemId(comp)).unwrap().layer(LayerId(layer)).unwrap().props.prop("transform/opacity").unwrap();
    let written = property.keys.iter().find(|key| key.value.as_f64() == 70.0).unwrap();
    assert_eq!(premise_time, written.time);
}

#[test]
fn complete_stored_ease_holds_the_edited_key_without_the_whole_curve() {
    let mut document = MotionDocument::create("Ease.kmotion", 32, 32).unwrap();
    let layer = document.execute("layer.newNull", json!({})).unwrap()["layer"].as_u64().unwrap();
    for time in [0, 1] {
        document.execute("prop.addKey", json!({"layer":layer,"path":"transform/opacity","time":time,"value":20})).unwrap();
    }
    document.execute("keyframe.setEase", json!({"layer":layer,"path":"transform/opacity","time":1,"side":"out","speed":1})).unwrap();
    let under = document.under("keyframe.setEase", &json!({"layer":layer,"path":"transform/opacity","time":1.2,"side":"out","speed":2})).unwrap().unwrap();
    println!("{}", serde_json::to_string(&under).unwrap());
    assert!(under.premises.iter().all(|premise| premise.fields.as_ref().is_some_and(|fields| fields.iter().all(|field| !field.ends_with("/opacity.keys")))));
}
