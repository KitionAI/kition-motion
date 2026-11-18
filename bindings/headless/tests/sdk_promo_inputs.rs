mod common;
use common::*;
use serde_json::{Value, json};

fn set_comp(project: &mut Value, comp: u64, field: &str, value: Value) {
    project["items"][comp.to_string()]["kind"][field] = value;
}

#[tokio::test]
async fn gather_holds_the_motion_blur_it_assigns_unless_motion_blur_is_off() {
    for (shutter, motion_blur) in [(true, true), (false, true), (true, false)] {
        let f = Fixture::new(2);
        let (info, comp, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        rewrite(&f.file(), |project| {
            if shutter {
                set_comp(project, comp, "shutter_angle", json!(90.0));
            } else {
                set_comp(project, comp, "background", json!([1.0, 0.0, 0.0]));
            }
        });
        let saved = std::fs::read(f.file()).unwrap();
        let params = json!({"layers":layers,"start":0,"center":[10,10],"motionBlur":motion_blur});
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.gather","params":params}]})).await;
        if shutter && motion_blur {
            assert_eq!(answer.expect_err("gather assigned an unread shutter angle")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn gather_holds_the_frame_size_only_for_its_default_centre() {
    for (width, centre) in [(true, false), (false, false), (true, true)] {
        let f = Fixture::new(2);
        let (info, comp, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        rewrite(&f.file(), |project| {
            if width {
                set_comp(project, comp, "width", json!(640));
            } else {
                set_comp(project, comp, "background", json!([1.0, 0.0, 0.0]));
            }
        });
        let saved = std::fs::read(f.file()).unwrap();
        let mut params = json!({"layers":layers,"start":0,"motionBlur":false});
        if centre {
            params["center"] = json!([10, 10]);
        }
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.gather","params":params}]})).await;
        if width && !centre {
            assert_eq!(answer.expect_err("gather centred on an unread frame size")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn handheld_holds_the_duration_only_for_its_default_end() {
    for (duration, end) in [(true, false), (false, false), (true, true)] {
        let f = Fixture::new(1);
        let (info, comp, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        rewrite(&f.file(), |project| {
            if duration {
                set_comp(project, comp, "duration", json!(254_016_000_000_i64 * 4));
            } else {
                set_comp(project, comp, "background", json!([1.0, 0.0, 0.0]));
            }
        });
        let saved = std::fs::read(f.file()).unwrap();
        let mut params = json!({"layer":layer});
        if end {
            params["end"] = json!(3);
        }
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.handheld","params":params}]})).await;
        if duration && !end {
            assert_eq!(answer.expect_err("handheld ended at an unread duration")["error"]["code"], "conflict");
            assert_eq!(std::fs::read(f.file()).unwrap(), saved);
        } else {
            assert!(answer.is_ok(), "{answer:?}");
        }
        f.shutdown().await;
    }
}
