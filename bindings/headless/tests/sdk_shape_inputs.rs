mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

/// A comp with a shape layer parented to a null: (bytes, shape, null).
fn parented_shape() -> (Vec<u8>, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let shape = doc.execute("layer.newShape", json!({"kind":"rect","name":"Shape"})).unwrap()["layer"].as_u64().unwrap();
    let parent = doc.execute("layer.newNull", json!({"name":"Null"})).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.setParent", json!({"layers":[shape],"parent":parent})).unwrap();
    (doc.save(None).unwrap(), shape, parent)
}

async fn stale_run(bytes: &[u8], change: impl FnOnce(&Fixture, u64), command: &str, params: Value) -> Result<Value, Value> {
    let f = Fixture::from_bytes(bytes, Default::default());
    let (_, comp, basis) = f.inspect().await;
    change(&f, comp);
    let saved = std::fs::read(f.file()).unwrap();
    let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved, "a refused write left the file as the colleague saved it");
    }
    f.shutdown().await;
    answer
}

fn comp_field(f: &Fixture, comp: u64, field: &str, value: Value) {
    rewrite(&f.file(), |project| project["items"][comp.to_string()]["kind"][field] = value);
}

#[tokio::test]
async fn a_default_shape_position_holds_the_frame_size_but_ignores_background() {
    let (bytes, shape, _) = parented_shape();
    let params = json!({"layer":shape,"kind":"rect","size":[20,20],"space":"layer"});
    let refused = stale_run(&bytes, |f, comp| comp_field(f, comp, "width", json!(640)), "shape.newShape", params.clone()).await;
    assert_eq!(refused.expect_err("default position used an unread frame size")["error"]["code"], "conflict");
    let applied = stale_run(&bytes, |f, comp| comp_field(f, comp, "background", json!([1.0, 0.0, 0.0])), "shape.newShape", params).await;
    assert!(applied.is_ok(), "{applied:?}");
}

#[tokio::test]
async fn comp_space_shapes_hold_the_parent_transform_but_ignore_its_opacity() {
    let (bytes, shape, parent) = parented_shape();
    for (command, params) in [
        ("shape.newShape", json!({"layer":shape,"kind":"rect","size":[20,20],"position":[40,40]})),
        ("shape.newPath", json!({"layer":shape,"space":"comp","vertices":[[10,10],[40,10],[40,40]]})),
    ] {
        let refused = stale_run(
            &bytes,
            |f, _| {
                f.colleague(&[("prop.set", json!({"layer":parent,"path":"transform/position","value":[50,60]}))]);
            },
            command,
            params.clone(),
        )
        .await;
        assert_eq!(refused.expect_err("comp-space input mapped through an unread parent")["error"]["code"], "conflict", "{command}");
        let applied = stale_run(
            &bytes,
            |f, _| {
                f.colleague(&[opacity(parent, 23.0)]);
            },
            command,
            params,
        )
        .await;
        assert!(applied.is_ok(), "{command}: {applied:?}");
    }
}

#[tokio::test]
async fn a_new_pen_layer_holds_the_frame_size_and_duration_but_ignores_background() {
    let bytes = project(1);
    let params = json!({"name":"Pen","vertices":[[10,10],[40,10],[40,40]]});
    for field in ["duration", "height"] {
        let refused = stale_run(
            &bytes,
            |f, comp| {
                comp_field(f, comp, field, if field == "duration" { json!(254_016_000_000_i64 * 4) } else { json!(90) });
            },
            "shape.newPath",
            params.clone(),
        )
        .await;
        assert_eq!(refused.expect_err("a new shape layer used an unread composition attribute")["error"]["code"], "conflict", "{field}");
    }
    let applied = stale_run(&bytes, |f, comp| comp_field(f, comp, "background", json!([1.0, 0.0, 0.0])), "shape.newPath", params).await;
    assert!(applied.is_ok(), "{applied:?}");
}
