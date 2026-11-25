//! A write that names a layer holds only the fields its handler reads and assigns (task card
//! C8-motion-pin, run 4): another writer's unrelated change of that layer (its name, its opacity)
//! applies; a change of a field the step reads or assigns is refused as `changed` (and the
//! layer's deletion as `deleted`).
mod common;
use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

struct Base {
    bytes: Vec<u8>,
    /// The top solid, with a Gaussian Blur and a mask; the solid below it.
    top: u64,
    below: u64,
    effect: u64,
    mask: u64,
    /// A text layer with one text animator.
    text: u64,
}

fn base() -> Base {
    let mut document = MotionDocument::open(FILE, &project(2)).unwrap();
    let ids = layer_ids(&document.inspect().unwrap()["info"]);
    let (top, below) = (ids[0], ids[1]);
    let effect = document.execute("effect.apply", json!({"effect":"Gaussian Blur","layers":[top]})).unwrap()["effects"][0].as_u64().unwrap();
    let mask = document.execute("mask.new", json!({"layer":top,"vertices":[[0,0],[100,0],[100,100]],"closed":true})).unwrap()["mask"].as_u64().unwrap();
    // `below` takes the layer above it as its matte, for layer.trackMatte `none`.
    document.execute("layer.trackMatte", json!({"layers":[below],"op":"alpha"})).unwrap();
    let text = document.execute("layer.newText", json!({"text":"Title"})).unwrap()["layer"].as_u64().unwrap();
    document.execute("layer.addTextAnimator", json!({"layer":text,"properties":["opacity"]})).unwrap();
    Base { bytes: document.save(None).unwrap(), top, below, effect, mask, text }
}

/// The agent reads the file; `theirs` (a colleague's engine) changes it; the agent runs `step`
/// against its basis. A refusal leaves the file as the colleague saved it.
async fn after(base: &Base, theirs: &[(&str, Value)], step: &Value) -> Result<Value, Value> {
    let fixture = Fixture::from_bytes(&base.bytes, Default::default());
    let (_, _, basis) = fixture.inspect().await;
    fixture.colleague(theirs);
    let saved = std::fs::read(fixture.file()).unwrap();
    let answer = fixture.call("run", json!({"basis":basis,"steps":[step]})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
    }
    fixture.shutdown().await;
    answer
}

/// An unrelated change of `layer`: its name and its opacity, which none of these steps reads.
fn unrelated(layer: u64) -> Vec<(&'static str, Value)> {
    vec![("layer.rename", json!({"layer":layer,"name":"Theirs"})), opacity(layer, 37.0)]
}

async fn check(base: &Base, name: &str, layer: u64, step: Value, related: Vec<(&'static str, Value)>, reason: &str) -> Vec<String> {
    let mut failures = Vec::new();
    match after(base, &related, &step).await {
        Err(refused) if refused["error"]["code"] == "conflict" => {
            let found = refused["conflicts"].as_array().and_then(|list| list.iter().find(|conflict| id_text(&conflict["id"]) == Some(layer.to_string())));
            if found.map(|conflict| &conflict["reason"]) != Some(&json!(reason)) {
                failures.push(format!("{name}: a change it depends on refused without `{reason}` on {layer}: {refused}"));
            }
        }
        other => failures.push(format!("{name}: a change it depends on was not refused: {other:?}")),
    }
    let applied = after(base, &unrelated(layer), &step).await;
    if let Err(refused) = applied {
        failures.push(format!("{name}: an unrelated change of the layer refused it: {refused}"));
    }
    failures
}

#[tokio::test]
async fn a_named_layer_is_held_by_the_fields_a_step_reads_and_assigns() {
    let base = base();
    let (top, below, effect, mask, text) = (base.top, base.below, base.effect, base.mask, base.text);
    let step = |command: &str, params: Value| json!({"command":command,"params":params});
    let cases: Vec<(&str, u64, Value, Vec<(&str, Value)>)> = vec![
        ("edit.label", top, step("edit.label", json!({"layers":[top],"label":"Yellow"})), vec![("edit.label", json!({"layers":[top],"label":"Blue"}))]),
        (
            "layer.trackMatte none",
            below,
            step("layer.trackMatte", json!({"layers":[below],"op":"none"})),
            vec![("layer.trackMatte", json!({"layers":[below],"op":"luma"}))],
        ),
        (
            "effect.toggle",
            top,
            step("effect.toggle", json!({"layer":top,"effect":effect,"value":false})),
            vec![("effect.toggle", json!({"layer":top,"effect":effect,"value":false}))],
        ),
        (
            "effect.toggle after its removal",
            top,
            step("effect.toggle", json!({"layer":top,"effect":effect,"value":false})),
            vec![("effect.remove", json!({"layer":top,"effect":effect}))],
        ),
        (
            "prop.renameGroup",
            top,
            step("prop.renameGroup", json!({"layer":top,"prop":effect,"name":"Mine"})),
            vec![("prop.renameGroup", json!({"layer":top,"prop":effect,"name":"Theirs"}))],
        ),
        (
            "prop.setGroupEnabled",
            top,
            step("prop.setGroupEnabled", json!({"layer":top,"prop":effect,"value":false})),
            vec![("prop.setGroupEnabled", json!({"layer":top,"prop":effect,"value":false}))],
        ),
        ("effect.removeAll", top, step("effect.removeAll", json!({"layers":[top]})), vec![("effect.apply", json!({"effect":"Gaussian Blur","layers":[top]}))]),
        ("mask.removeAll", top, step("mask.removeAll", json!({"layer":top})), vec![("layer.mask.mode", json!({"layer":top,"mask":mask,"mode":"Subtract"}))]),
        (
            "layer.mask.removeAll",
            top,
            step("layer.mask.removeAll", json!({"layers":[top]})),
            vec![("mask.new", json!({"layer":top,"vertices":[[0,0],[50,0],[50,50]],"closed":true}))],
        ),
        (
            "layer.mask.mode",
            top,
            step("layer.mask.mode", json!({"layer":top,"mask":mask,"mode":"Subtract"})),
            vec![("layer.mask.mode", json!({"layer":top,"mask":mask,"mode":"Difference"}))],
        ),
        (
            "layer.mask.invert",
            top,
            step("layer.mask.invert", json!({"layer":top,"mask":mask})),
            vec![("layer.mask.invert", json!({"layer":top,"mask":mask,"value":true}))],
        ),
        (
            "layer.mask.lock",
            top,
            step("layer.mask.lock", json!({"layer":top,"mask":mask})),
            vec![("layer.mask.lock", json!({"layer":top,"mask":mask,"value":true}))],
        ),
        (
            "layer.transform flipH",
            top,
            step("layer.transform", json!({"layers":[top],"op":"flipH"})),
            vec![("prop.set", json!({"layer":top,"path":"transform/scale","value":[50,50,100]}))],
        ),
        (
            "layer.transform flipV after a lock",
            top,
            step("layer.transform", json!({"layers":[top],"op":"flipV"})),
            vec![("layer.setSwitch", json!({"layers":[top],"switch":"locked","value":true}))],
        ),
        ("prop.select", top, step("prop.select", json!({"layer":top,"prop":mask})), vec![("mask.remove", json!({"layer":top,"mask":mask}))]),
        (
            "prop.deselect",
            top,
            step("prop.deselect", json!({"layer":top,"prop":mask})),
            vec![("layer.mask.shape", json!({"layer":top,"mask":mask,"rect":[5,5,20,20]}))],
        ),
        (
            "text.removeAllAnimators",
            text,
            step("text.removeAllAnimators", json!({"layers":[text]})),
            vec![("layer.addTextAnimator", json!({"layer":text,"properties":["scale"]}))],
        ),
        (
            "layer.autoOrient",
            top,
            step("layer.autoOrient", json!({"layers":[top],"mode":"alongPath"})),
            vec![("layer.autoOrient", json!({"layers":[top],"mode":"towardsCamera"}))],
        ),
        (
            "layer.setTransform rotation",
            top,
            step("layer.setTransform", json!({"layers":[top],"prop":"rotation","value":30})),
            vec![("prop.set", json!({"layer":top,"path":"transform/rotation","value":10}))],
        ),
        (
            "layer.setTransform position",
            top,
            step("layer.setTransform", json!({"layers":[top],"prop":"position","value":[10,20]})),
            vec![("prop.separateDimensions", json!({"layer":top,"value":true}))],
        ),
    ];
    let mut failures = Vec::new();
    for (name, layer, step, related) in cases {
        failures.extend(check(&base, name, layer, step, related, "changed").await);
    }
    // paint.setCloneSource needs only that the layer is there.
    let clone = step("paint.setCloneSource", json!({"layer":top,"point":[50,50]}));
    failures.extend(check(&base, "paint.setCloneSource", top, clone, vec![("edit.delete", json!({"layers":[top]}))], "deleted").await);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Undo and a jump by steps restore the agent's own history state rebased on others' edits: a
/// colleague's later rename is never overwritten (the step applies and leaves it) and their rename
/// of another layer survives. A jump by history index or id puts back a stored state as it was
/// (history.rs:453 `self.project = a.state[target]`), which carries none of the colleague's
/// later edits, so any of them would be reverted: it holds the whole document and refuses after a
/// colleague's rename of either layer (the conservative hold of run 3, kept).
#[tokio::test]
async fn history_jumps_never_revert_a_colleagues_later_edit() {
    let jumps = [
        ("edit.history.goto", json!({"index":0}), true),
        ("edit.history.goto", json!({"steps":-1}), false),
        ("edit.undo", json!({}), false),
        ("edit.history", json!({"steps":1}), false),
    ];
    let mut failures = Vec::new();
    for (command, params, whole) in jumps {
        for related in [true, false] {
            let fixture = Fixture::new(2);
            let (info, _, _) = fixture.inspect().await;
            let ids = layer_ids(&info);
            let (top, other) = (ids[0], ids[1]);
            fixture.ok("run", json!({"steps":[{"command":"layer.rename","params":{"layer":top,"name":"Mine"}}]})).await;
            let (_, _, basis) = fixture.inspect().await;
            let theirs = if related { top } else { other };
            fixture.colleague(&[("layer.rename", json!({"layer":theirs,"name":"Theirs"}))]);
            let saved = std::fs::read(fixture.file()).unwrap();
            let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":command,"params":params}]})).await;
            let names: Vec<String> = on_disk(&fixture.file()).inspect().unwrap()["info"]["layers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|layer| layer["name"].as_str().unwrap().to_owned())
                .collect();
            let refused = answer.as_ref().err().is_some_and(|refused| {
                refused["error"]["code"] == "conflict"
                    && refused["conflicts"].as_array().is_some_and(|list| list.iter().any(|conflict| conflict["reason"] == "changed"))
            });
            match answer {
                Err(_) if refused => {
                    if std::fs::read(fixture.file()).unwrap() != saved {
                        failures.push(format!("{command} {params}: a refusal changed the file"));
                    }
                    if !whole && !related {
                        failures.push(format!("{command} {params}: the other layer's rename refused it"));
                    }
                }
                Err(error) => failures.push(format!("{command} {params}: failed: {error}")),
                Ok(_) if whole => failures.push(format!("{command} {params}: applied after a colleague's edit, leaving {names:?}")),
                Ok(_) => {
                    let kept = if related { names[0] == "Theirs" } else { names[1] == "Theirs" && names[0] != "Mine" };
                    if !kept {
                        failures.push(format!("{command} {params}: left {names:?}"));
                    }
                }
            }
            fixture.shutdown().await;
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
