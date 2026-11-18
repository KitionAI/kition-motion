mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn project_settings_hold_assigned_fields_but_ignore_layer_opacity() {
    for settings in [true, false] {
        let f = Fixture::new(1);
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        f.colleague(&[if settings { ("file.projectSettings", json!({"bitDepth":16})) } else { opacity(layer, 23.0) }]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"file.projectSettings","params":{"bitDepth":32}}]})).await;
        if settings {
            assert_eq!(answer.expect_err("same-field assignment must refuse")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
        }
        f.shutdown().await;
    }
}
