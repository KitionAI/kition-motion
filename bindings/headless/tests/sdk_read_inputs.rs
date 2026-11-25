mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

/// The comp `Main` is item 1 in `project(..)`.
const COMP: &str = "1";

/// The agent runs `setup` on a two-solid project and reads it; another program changes the file
/// (`change`: a JSON rewrite or a colleague's engine), and the agent runs `step` against its
/// basis. The file is unchanged when the step is refused.
async fn stale(setup: &[(&str, Value)], change: impl FnOnce(&std::path::Path), step: Value) -> Result<Value, Value> {
    let f = Fixture::new(2);
    if !setup.is_empty() {
        let steps: Vec<Value> = setup.iter().map(|(command, params)| json!({"command":command,"params":params})).collect();
        f.ok("run", json!({"steps":steps})).await;
    }
    let (_, _, basis) = f.inspect().await;
    change(&f.file());
    let saved = std::fs::read(f.file()).unwrap();
    let answer = f.call("run", json!({"basis":basis,"steps":[step]})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
    }
    f.shutdown().await;
    answer
}

fn conflict(answer: Result<Value, Value>, what: &str) {
    let refused = answer.expect_err(what);
    assert_eq!(refused["error"]["code"], "conflict", "{what}: {}", refused["error"]);
}

/// The two solids of `project(2)`, top first, and the solid item of the top one.
fn solids() -> (Vec<u64>, u64) {
    let bytes = project(2);
    let ids = layer_ids(&MotionDocument::open(FILE, &bytes).unwrap().inspect().unwrap()["info"]);
    let project: Value = serde_json::from_slice(&bytes).unwrap();
    let item =
        project["items"][COMP]["kind"]["layers"].as_array().unwrap().iter().find(|layer| layer["id"] == ids[0]).unwrap()["source"]["item"].as_u64().unwrap();
    (ids, item)
}

fn reframe(file: &std::path::Path) {
    rewrite(file, |project| project["items"][COMP]["kind"]["width"] = json!(640));
}

fn rename_comp(file: &std::path::Path) {
    rewrite(file, |project| project["items"][COMP]["name"] = json!("Other"));
}

/// Composition attributes: the frame rate a sheen counts its frames in, and the frame a reset
/// Position takes its centre from, refuse when changed; the composition's name does not.
#[tokio::test]
async fn composition_attributes_a_step_reads_refuse_and_others_apply() {
    let (ids, _) = solids();
    let sheen = json!({"command":"light.sheen","params":{"layer":ids[0]}});
    let rerated = stale(&[], |file| drop(colleague(file, &[("comp.settings", json!({"frameRate":24}))])), sheen.clone()).await;
    conflict(rerated, "light.sheen counts frames at the frame rate");
    let renamed = stale(&[], rename_comp, sheen).await;
    assert!(renamed.is_ok(), "{renamed:?}");
    let reset = json!({"command":"prop.reset","params":{"layer":ids[0],"path":"transform/position"}});
    conflict(stale(&[], reframe, reset.clone()).await, "a reset Position is the frame's centre");
    let renamed = stale(&[], rename_comp, reset).await;
    assert!(renamed.is_ok(), "{renamed:?}");
    // Opacity's default is a constant: the frame does not enter it.
    let opacity = json!({"command":"prop.reset","params":{"layer":ids[0],"path":"transform/opacity"}});
    let reframed = stale(&[], reframe, opacity).await;
    assert!(reframed.is_ok(), "{reframed:?}");
}

/// Project items: comp.new picks the first free name among every item's, a new solid files into
/// the first folder named Solids, and project.rename assigns the selected item's name.
#[tokio::test]
async fn project_items_a_step_reads_refuse_and_others_apply() {
    let (_, item) = solids();
    let recolour = move |file: &std::path::Path| rewrite(file, |project| project["items"][item.to_string()]["kind"]["color"] = json!([0.0, 0.0, 1.0]));
    // `Main` is taken, so the new comp is `Main 2` unless another item took that name since.
    let new_comp = json!({"command":"comp.new","params":{"name":"Main","open":false}});
    let taken = stale(&[], move |file| rewrite(file, |project| project["items"][item.to_string()]["name"] = json!("Main 2")), new_comp.clone()).await;
    conflict(taken, "the free name changed");
    let recoloured = stale(&[], recolour, new_comp).await;
    assert!(recoloured.is_ok(), "{recoloured:?}");
    // The Solids folder a new solid files into.
    let solid = json!({"command":"layer.newSolid","params":{"name":"Late"}});
    let folder = {
        let project: Value = serde_json::from_slice(&project(2)).unwrap();
        project["items"].as_object().unwrap().iter().find(|(_, item)| item["name"] == "Solids").map(|(id, _)| id.clone()).unwrap()
    };
    let moved = stale(&[], move |file| rewrite(file, |project| project["items"][folder.as_str()]["name"] = json!("Old Solids")), solid.clone()).await;
    conflict(moved, "the Solids folder was renamed");
    let recoloured = stale(&[], recolour, solid).await;
    assert!(recoloured.is_ok(), "{recoloured:?}");
    // project.rename without `item` renames the selected item: its name is the field it assigns.
    // The selection is the agent's own editor state, made in the same run.
    let rename = |at: u64, theirs: Value| async move {
        let f = Fixture::new(2);
        let (_, _, basis) = f.inspect().await;
        rewrite(&f.file(), |project| {
            for (key, value) in theirs.as_object().unwrap() {
                project["items"][at.to_string()][key] = value.clone();
            }
        });
        let steps = json!([{"command":"project.select","params":{"items":[item]}},{"command":"project.rename","params":{"name":"Mine"}}]);
        let answer = f.call("run", json!({"basis":basis,"steps":steps})).await;
        f.shutdown().await;
        answer
    };
    conflict(rename(item, json!({"name":"Theirs"})).await, "the selected item's name changed");
    // Another item's name is not read (project.select itself holds the items it selects).
    let other = rename(1, json!({"name":"Other"})).await;
    assert!(other.is_ok(), "{other:?}");
}

/// Other layers: a selection by id holds that the layer exists; parenting with compensation
/// holds the new parent's ancestors' transforms; an unrelated rename applies.
#[tokio::test]
async fn other_layers_a_step_reads_refuse_and_others_apply() {
    let (ids, _) = solids();
    let select = json!({"command":"layer.select","params":{"layers":[ids[1]]}});
    let deleted = stale(&[], |file| drop(colleague(file, &[("edit.delete", json!({"layers":[ids[1]]}))])), select.clone()).await;
    conflict(deleted, "the selected layer was deleted");
    let renamed = stale(&[], |file| drop(colleague(file, &[("layer.rename", json!({"layer":ids[0],"name":"Other"}))])), select).await;
    assert!(renamed.is_ok(), "{renamed:?}");
    // A null parented to the top solid: parenting the bottom solid to the null keeps it in place
    // through the null's parent's transform.
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let parent_null = doc.execute("layer.newNull", json!({"name":"Null"})).unwrap()["layer"].as_u64().unwrap();
    let setup = [("layer.newNull", json!({"name":"Null"})), ("layer.setParent", json!({"layers":[parent_null],"parent":ids[0]}))];
    let parent = json!({"command":"layer.setParent","params":{"layers":[ids[1]],"parent":parent_null}});
    let moved =
        stale(&setup, |file| drop(colleague(file, &[("prop.set", json!({"layer":ids[0],"path":"transform/position","value":[10,20]}))])), parent.clone()).await;
    conflict(moved, "the new parent's parent moved");
    let renamed = stale(&setup, |file| drop(colleague(file, &[("layer.rename", json!({"layer":ids[0],"name":"Other"}))])), parent).await;
    assert!(renamed.is_ok(), "{renamed:?}");
}

fn opacity_of(layer: u64) -> impl FnOnce(&std::path::Path) {
    move |file| drop(colleague(file, &[("prop.set", json!({"layer":layer,"path":"transform/opacity","value":40}))]))
}

/// A new layer named among its siblings (camera.orbitNull's null), and the active camera the 3D
/// view commands find by scanning the stack.
#[tokio::test]
async fn names_and_the_active_camera_a_step_finds_refuse_and_others_apply() {
    let (ids, _) = solids();
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let camera = doc.execute("layer.newCamera", json!({"name":"Cam"})).unwrap()["layer"].as_u64().unwrap();
    let setup = [("layer.newCamera", json!({"name":"Cam"}))];
    let orbit = json!({"command":"camera.orbitNull","params":{"layer":camera}});
    let taken = stale(&setup, |file| drop(colleague(file, &[("layer.rename", json!({"layer":ids[1],"name":"Cam Orbit Null"}))])), orbit.clone()).await;
    conflict(taken, "the null's name was taken");
    let faded = stale(&setup, opacity_of(ids[1]), orbit).await;
    assert!(faded.is_ok(), "{faded:?}");
    for command in ["view.reset3DView", "view.lookAtAll"] {
        let step = json!({"command":command,"params":{}});
        let above = stale(&setup, |file| drop(colleague(file, &[("layer.newCamera", json!({"name":"Other"}))])), step.clone()).await;
        conflict(above, "another camera became active");
        let faded = stale(&setup, opacity_of(ids[1]), step).await;
        assert!(faded.is_ok(), "{command}: {faded:?}");
    }
}

/// An enabled expression may read any layer: converting it to keyframes holds every layer; a
/// project setting it cannot read applies.
#[tokio::test]
async fn an_expression_closure_refuses_a_referenced_layer_and_applies_a_project_setting() {
    let (ids, _) = solids();
    let expression = "thisComp.layer(\"L0\").transform.opacity";
    let setup = [("prop.setExpression", json!({"layer":ids[0],"path":"transform/opacity","expression":expression}))];
    let convert = json!({"command":"prop.convertExpressionToKeyframes","params":{"layer":ids[0],"path":"transform/opacity"}});
    conflict(stale(&setup, opacity_of(ids[1]), convert.clone()).await, "the referenced layer changed");
    let setting = stale(&setup, |file| rewrite(file, |project| project["settings"]["time_display"] = json!("Frames")), convert).await;
    assert!(setting.is_ok(), "{setting:?}");
}

/// engine.batch holds each literal step's own inputs: layer.newText builds its layer in the frame.
#[tokio::test]
async fn a_batch_holds_its_steps_inputs() {
    let batch = json!({"command":"engine.batch","params":{"steps":[{"command":"layer.newText","params":{"text":"Hi"}}]}});
    conflict(stale(&[], reframe, batch.clone()).await, "the new text layer is centred in the frame");
    let renamed = stale(&[], rename_comp, batch).await;
    assert!(renamed.is_ok(), "{renamed:?}");
}

/// Undo restores the agent's previous state rebased on outside rereads: what it assigns is what
/// differs from that state, so a colleague's later value of the same field is never reverted,
/// and a change elsewhere applies and is kept.
#[tokio::test]
async fn undo_never_reverts_a_colleagues_later_edit() {
    let (ids, _) = solids();
    for (layer, colleague_changed_mine) in [(ids[0], true), (ids[1], false)] {
        let f = Fixture::new(2);
        f.ok("run", json!({"steps":[{"command":"prop.set","params":{"layer":ids[0],"path":OPACITY,"value":37}}]})).await;
        let (_, _, basis) = f.inspect().await;
        opacity_of(layer)(&f.file());
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"edit.undo","params":{}}]})).await;
        assert_eq!(opacity_on_disk(&f.file(), layer), 40.0, "the colleague's value stays ({answer:?})");
        if !colleague_changed_mine {
            assert!(answer.is_ok(), "{answer:?}");
            assert_eq!(opacity_on_disk(&f.file(), ids[0]), 100.0, "the agent's own step is undone");
        }
        f.shutdown().await;
    }
}

/// comp.cropToLayerBounds shifts every unparented layer's Position by the crop offset: another
/// layer's Position refuses, its opacity applies.
#[tokio::test]
async fn cropping_to_layer_bounds_holds_every_position_it_shifts() {
    let (ids, _) = solids();
    let setup = [("prop.set", json!({"layer":ids[0],"path":"transform/scale","value":[50,50]}))];
    let crop = json!({"command":"comp.cropToLayerBounds","params":{"layers":[ids[0]]}});
    let moved =
        stale(&setup, |file| drop(colleague(file, &[("prop.set", json!({"layer":ids[1],"path":"transform/position","value":[10,20]}))])), crop.clone()).await;
    conflict(moved, "a shifted layer's Position changed");
    let faded = stale(&setup, opacity_of(ids[1]), crop).await;
    assert!(faded.is_ok(), "{faded:?}");
}
