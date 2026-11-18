mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn precompose_holds_copied_settings_and_source_label_but_preserves_other_opacity() {
    for change in ["background", "label", "opacity"] {
        let fixture = Fixture::new(2);
        let (info, comp, basis) = fixture.inspect().await;
        let layers = layer_ids(&info);
        let source = fixture.ok("query", json!({"objects":[layers[0]]})).await["objects"][0]["fields"]["sourceItem"].clone();
        fixture.colleague(&[match change {
            "background" => ("comp.settings", json!({"comp":comp,"background":"#ff0000"})),
            "label" => ("project.setLabel", json!({"items":[source],"label":2})),
            _ => opacity(layers[1], 23.0),
        }]);
        let saved = std::fs::read(fixture.file()).unwrap();
        let answer = fixture
            .call("run", json!({"basis":basis,"steps":[{"command":"layer.precompose","params":{"layers":[layers[0]],"mode":"leave","name":"Inner"}}]}))
            .await;
        if change != "opacity" {
            assert_eq!(answer.expect_err("precompose copies these observed inputs")["error"]["code"], "conflict", "{change}");
            assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&fixture.file(), layers[1]), 23.0);
        }
        fixture.shutdown().await;
    }
}
