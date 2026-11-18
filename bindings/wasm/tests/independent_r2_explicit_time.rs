use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use kition_motion_project::{ItemId, LayerId, Project};
use serde_json::json;

fn other_active_comp() -> (MotionDocument, u64, u64) {
    let mut document = MotionDocument::create("Time.kmotion", 16, 16).unwrap();
    let comp = document.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let layer = document.execute("layer.newSolid", json!({"name": "Source"})).unwrap()["layer"].as_u64().unwrap();
    document.execute("time.set", json!({"time": 3})).unwrap();
    document.execute("comp.new", json!({"name": "Other", "width": 16, "height": 16})).unwrap();
    (document, comp, layer)
}

#[test]
fn split_uses_the_addressed_compositions_time() {
    let (mut document, comp, layer) = other_active_comp();
    let result = document.execute("edit.splitLayer", json!({"comp": comp, "layers": [layer]})).unwrap();
    assert_eq!(result.as_array().unwrap().len(), 1, "{result}");
    let project = Project::from_json(std::str::from_utf8(&document.save(None).unwrap()).unwrap()).unwrap();
    assert_eq!(project.comp(ItemId(comp)).unwrap().layer(LayerId(layer)).unwrap().out_point.seconds(), 3.0);
}

#[test]
fn layered_frame_exports_use_the_addressed_compositions_time() {
    for (command, path) in [("comp.saveFrameAsPsd", "Frame.psd"), ("comp.saveFrameAsExr", "Frame.exr")] {
        let (mut document, comp, _) = other_active_comp();
        let result = document.execute(command, json!({"comp": comp, "path": path})).unwrap();
        assert_eq!(result["time"], 3.0, "{command}: {result}");
    }
}
