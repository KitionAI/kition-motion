//! C8-motion-read: source-derived dependencies, changed by an independent saved-file writer.
#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

async fn pair(bytes: &[u8], command: &str, params: Value, relevant: (&str, Value), unrelated: (&str, Value)) {
    for (must_refuse, change) in [(true, relevant), (false, unrelated)] {
        let f = Fixture::from_bytes(bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[change]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
        if must_refuse {
            let refused = answer.unwrap_err();
            assert_eq!(refused["error"]["code"], "conflict", "{command}: {refused}");
            assert!(refused["conflicts"].as_array().unwrap().iter().any(|c| c["reason"] == "changed"), "{command}: {refused}");
        } else {
            assert!(answer.is_ok(), "{command} refused an unrelated change: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn assigned_project_comment_holds_only_its_field() {
    pair(
        &project(2),
        "project.setProjectComment",
        json!({"comment":"mine"}),
        ("project.setProjectComment", json!({"comment":"theirs"})),
        ("comp.settings", json!({"background":"#123456"})),
    )
    .await;
}

#[tokio::test]
async fn guide_default_holds_the_dimension_it_reads() {
    pair(
        &project(2),
        "view.addGuide",
        json!({"orientation":"vertical"}),
        ("comp.settings", json!({"width":640})),
        ("comp.settings", json!({"background":"#123456"})),
    )
    .await;
}

#[tokio::test]
async fn explicit_guide_holds_guides_without_dimension_defaults() {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    doc.execute("view.addGuide", json!({"position":12})).unwrap();
    pair(
        &doc.save(None).unwrap(),
        "view.moveGuide",
        json!({"index":0,"position":30}),
        ("view.moveGuide", json!({"index":0,"position":20})),
        ("comp.settings", json!({"width":640})),
    )
    .await;
}

#[tokio::test]
async fn camera_expression_holds_the_referenced_name_and_camera_eligibility() {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let target = layer_ids(&doc.inspect().unwrap()["info"])[0];
    let camera = doc.execute("layer.newCamera", json!({"name":"Camera"})).unwrap()["layer"].as_u64().unwrap();
    pair(
        &doc.save(None).unwrap(),
        "camera.linkFocusToLayer",
        json!({"camera":camera,"layer":target}),
        ("layer.rename", json!({"layer":target,"name":"Moved name"})),
        opacity(target, 23.0),
    )
    .await;
}

#[tokio::test]
async fn field_assignments_accept_other_changes_to_the_same_layer() {
    let bytes = project(2);
    let doc = MotionDocument::open(FILE, &bytes).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    for (command, mine, theirs) in [
        ("layer.setComment", json!({"comment":"mine"}), json!({"comment":"theirs"})),
        ("layer.setBlendMode", json!({"mode":"Multiply"}), json!({"mode":"Screen"})),
        ("layer.quality", json!({"quality":"draft"}), json!({"quality":"wireframe"})),
        ("layer.sampling", json!({"sampling":"bilinear"}), json!({"sampling":"bicubic"})),
        ("layer.frameBlending", json!({"mode":"frameMix"}), json!({"mode":"pixelMotion"})),
    ] {
        let mut mine = mine;
        let mut theirs = theirs;
        mine["layers"] = json!([layer]);
        theirs["layers"] = json!([layer]);
        pair(&bytes, command, mine, (command, theirs), opacity(layer, 23.0)).await;
    }
}

#[tokio::test]
async fn camera_link_merges_dependencies_when_camera_and_target_are_the_same() {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let camera = doc.execute("layer.newCamera", json!({"name":"Camera"})).unwrap()["layer"].as_u64().unwrap();
    pair(
        &doc.save(None).unwrap(),
        "camera.linkFocusToLayer",
        json!({"camera":camera,"layer":camera}),
        ("prop.setExpression", json!({"layer":camera,"path":"cameraOptions/focusDistance","expression":"42"})),
        ("layer.setComment", json!({"layer":camera,"comment":"unrelated"})),
    )
    .await;
}

#[tokio::test]
async fn an_item_list_is_an_object_reference_despite_an_untyped_schema() {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let item = doc.execute("project.newFolder", json!({"name":"Folder"})).unwrap()["item"].as_u64().unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    pair(
        &doc.save(None).unwrap(),
        "project.delete",
        json!({"items":[item]}),
        ("project.rename", json!({"item":item,"name":"Colleague"})),
        opacity(layer, 23.0),
    )
    .await;
}

#[tokio::test]
async fn work_area_holds_frame_rate_used_by_its_clamp() {
    pair(&project(2), "comp.workArea", json!({"start":9.99}), ("comp.settings", json!({"frameRate":15})), ("comp.settings", json!({"background":"#123456"})))
        .await;
}

#[tokio::test]
async fn uncompensated_parenting_holds_ancestor_cycle_checks() {
    let mut doc = MotionDocument::open(FILE, &project(1)).unwrap();
    let child = layer_ids(&doc.inspect().unwrap()["info"])[0];
    let parent = doc.execute("layer.newNull", json!({"name":"Parent"})).unwrap()["layer"].as_u64().unwrap();
    let ancestor = doc.execute("layer.newNull", json!({"name":"Ancestor"})).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.setParent", json!({"layers":[parent],"parent":ancestor,"compensate":false})).unwrap();
    pair(
        &doc.save(None).unwrap(),
        "layer.setParent",
        json!({"layers":[child],"parent":parent,"compensate":false}),
        ("layer.setParent", json!({"layers":[ancestor],"parent":child,"compensate":false})),
        opacity(ancestor, 23.0),
    )
    .await;
}

#[tokio::test]
async fn composition_background_holds_assignment_and_renderer_eligibility() {
    let bytes = project(2);
    let comp = MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    for params in [json!({"background":"#112233"}), json!({"comp":comp,"background":"#112233"})] {
        pair(&bytes, "comp.settings", params, ("comp.settings", json!({"background":"#445566"})), ("comp.settings", json!({"frameRate":15}))).await;
    }
}
