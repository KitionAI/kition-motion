mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn seed() -> (Vec<u8>, u64) {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let source = doc.execute("file.importPlaceholder", json!({"name":"Source","width":320,"height":180,"duration":5})).unwrap()["item"].as_u64().unwrap();
    (doc.save(None).unwrap(), source)
}

#[tokio::test]
async fn comp_from_selection_holds_queue_allocation_and_templates() {
    let (bytes, source) = seed();
    for change in ["queue", "template", "opacity"] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        match change {
            "queue" => {
                f.colleague(&[("renderQueue.add", json!({}))]);
            }
            "template" => {
                f.colleague(&[
                    ("renderQueue.saveTemplate", json!({"kind":"renderSettings","name":"Draft","params":{"quality":"draft"}})),
                    ("renderQueue.setTemplateDefault", json!({"kind":"renderSettings","slot":"movie","name":"Draft"})),
                ]);
            }
            _ => {
                f.colleague(&[opacity(layer, 23.0)]);
            }
        }
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f
            .call(
                "run",
                json!({"basis":basis,"steps":[
                    {"command":"project.select","params":{"items":[source]}},
                    {"command":"file.newCompFromSelection","params":{"addToRenderQueue":true}}
                ]}),
            )
            .await;
        if change == "opacity" {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
        } else {
            assert_eq!(answer.expect_err("queued composition consumed unread queue inputs")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        }
        f.shutdown().await;
    }
}
