mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

#[tokio::test]
async fn extra_output_holds_the_copied_module_but_ignores_layer_opacity() {
    let mut d = MotionDocument::open(FILE, &project(1)).unwrap();
    let item = d.execute("renderQueue.add", json!({})).unwrap()["id"].as_u64().unwrap();
    let bytes = d.save(None).unwrap();
    for output in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        f.colleague(&[if output { ("renderQueue.setOutput", json!({"item":item,"output":"colleague_[#####].png"})) } else { opacity(layer, 23.0) }]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"render.addOutputModule","params":{"item":item}}]})).await;
        if output {
            assert_eq!(answer.expect_err("copied output module changed")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn height_only_resize_holds_its_compositions_dimensions() {
    let mut d = MotionDocument::open(FILE, &project(1)).unwrap();
    let item = d.execute("renderQueue.add", json!({})).unwrap()["id"].as_u64().unwrap();
    let bytes = d.save(None).unwrap();
    for dimensions in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("comp.settings", if dimensions { json!({"width":640,"anchor":0}) } else { json!({"background":"#ff0000"}) })]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer=f.call("run",json!({"basis":basis,"steps":[{"command":"renderQueue.setOutputModule","params":{"item":item,"resize":{"enabled":true,"lockAspect":true,"height":90}}}]})).await;
        if dimensions {
            assert_eq!(answer.expect_err("height-only output sizing must not use changed composition dimensions")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}
