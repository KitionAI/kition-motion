mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn new_styles_hold_copied_global_light_but_ignore_background() {
    for changed_light in [true, false] {
        let f = Fixture::new(1);
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        f.colleague(&[if changed_light { ("layer.style.globalLight", json!({"angle":37})) } else { ("comp.settings", json!({"background":"#ff0000"})) }]);
        let saved = std::fs::read(f.file()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.style.dropShadow","params":{"layers":[layer]}}]})).await;
        if changed_light {
            assert_eq!(answer.expect_err("new style copied an unread global light")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn adding_a_style_ignores_target_opacity_and_preserves_existing_global_light() {
    for existing in [true, false] {
        let f = Fixture::new(1);
        let (info, _, _) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        if existing {
            f.ok("run", json!({"steps":[{"command":"layer.style.dropShadow","params":{"layers":[layer]}}]})).await;
        }
        let (_, _, basis) = f.inspect().await;
        let mut changes = vec![opacity(layer, 23.0)];
        if existing {
            changes.push(("layer.style.globalLight", json!({"angle":37})));
        }
        f.colleague(&changes);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.style.stroke","params":{"layers":[layer]}}]})).await;
        assert!(answer.is_ok(), "style insertion does not consume opacity or recopy an existing blending group: {answer:?}");
        assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
        f.shutdown().await;
    }
}

#[tokio::test]
async fn camera_angle_holds_width_but_explicit_zoom_ignores_it() {
    for command in ["layer.cameraSettings", "layer.settings"] {
        for angle in [true, false] {
            let f = Fixture::new(0);
            let created = f.ok("run", json!({"steps":[{"command":"layer.newCamera"}]})).await;
            let _ = created;
            let (info, _, basis) = f.inspect().await;
            let camera = layer_ids(&info)[0];
            f.colleague(&[("comp.settings", json!({"width":640,"anchor":0}))]);
            let saved = std::fs::read(f.file()).unwrap();
            let params = if angle { json!({"layer":camera,"angleOfView":50}) } else { json!({"layer":camera,"zoom":500}) };
            let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
            if angle {
                assert_eq!(answer.expect_err("angle-to-zoom conversion consumed changed width")["error"]["code"], "conflict");
                assert_eq!(std::fs::read(f.file()).unwrap(), saved);
            } else {
                assert!(answer.is_ok(), "{answer:?}");
            }
            f.shutdown().await;
        }
    }
}
