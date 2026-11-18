//! C8-motion-read: scalar mask premises checked against an independent saved-file writer.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn masks() -> (Vec<u8>, u64, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
    let mut ids = Vec::new();
    for x in [0, 30] {
        ids.push(doc.execute("mask.new", json!({"layer":layer,"vertices":[[x,0],[x+20,0],[x+20,20]],"closed":true})).unwrap()["mask"].as_u64().unwrap());
    }
    (doc.save(None).unwrap(), layer, ids[0], ids[1])
}

fn mask_children(project: &mut Value, layer: u64) -> &mut Vec<Value> {
    project["items"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .find_map(|item| item["kind"]["layers"].as_array_mut())
        .unwrap()
        .iter_mut()
        .find(|item| item["id"] == layer)
        .unwrap()["props"]["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|group| group["match"] == "masks")
        .unwrap()["children"]
        .as_array_mut()
        .unwrap()
}

#[tokio::test]
async fn scalar_mask_assignments_hold_their_field_and_allow_layer_opacity() {
    let (bytes, layer, mask, _) = masks();
    for (command, mine, other_command, theirs) in [
        ("path.convertToBezier", json!({}), "path.rotoBezier", json!({"value":true})),
        ("layer.mask.motionBlur", json!({"mode":"off"}), "layer.mask.motionBlur", json!({"mode":"on"})),
        ("layer.mask.featherFalloff", json!({"mode":"smooth"}), "layer.mask.featherFalloff", json!({"mode":"linear"})),
    ] {
        for relevant in [true, false] {
            let f = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = f.inspect().await;
            let mut mine = mine.clone();
            mine["layer"] = json!(layer);
            mine["mask"] = json!(mask);
            if relevant {
                let mut theirs = theirs.clone();
                theirs["layer"] = json!(layer);
                theirs["mask"] = json!(mask);
                f.colleague(&[(other_command, theirs)]);
            } else {
                f.colleague(&[opacity(layer, 23.0)]);
            }
            let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":mine}]})).await;
            if relevant {
                assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
            } else {
                assert!(answer.is_ok(), "{command} refused unrelated same-layer opacity: {answer:?}");
                assert_eq!(opacity_on_disk(&f.file(), layer), 23.0);
            }
            f.shutdown().await;
        }
    }
}

#[tokio::test]
async fn mask_ordinal_insertion_is_relevant_but_uid_insertion_is_not() {
    let (bytes, layer, mask, _) = masks();
    for ordinal in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("mask.new", json!({"layer":layer,"vertices":[[50,0],[60,0],[60,10]],"closed":true}))]);
        rewrite(&f.file(), |project| {
            let children = mask_children(project, layer);
            let inserted = children.pop().unwrap();
            children.insert(0, inserted);
        });
        let answer = f
            .call(
                "run",
                json!({"basis":basis,"steps":[{"command":"layer.mask.motionBlur","params":{"layer":layer,"mask":if ordinal {1} else {mask},"mode":"off"}}]}),
            )
            .await;
        if ordinal {
            let refused = answer.unwrap_err();
            assert_eq!(refused["error"]["code"], "conflict", "mask ordinal changed: {refused}");
        } else {
            assert!(answer.is_ok(), "stable mask uid refused an insertion: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn mask_name_reassignment_is_relevant() {
    let (bytes, layer, _, _) = masks();
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    rewrite(&f.file(), |project| {
        let children = mask_children(project, layer);
        children[0]["name"] = json!("Former mask");
        children[1]["name"] = json!("Mask 1");
    });
    let refused = f
        .refused("run", json!({"basis":basis,"steps":[{"command":"layer.mask.featherFalloff","params":{"layer":layer,"mask":"Mask 1","mode":"linear"}}]}))
        .await;
    assert_eq!(refused["error"]["code"], "conflict", "mask name now resolves to another mask: {refused}");
    f.shutdown().await;
}

#[tokio::test]
async fn all_masks_hold_the_assigned_fields_without_holding_layer_opacity() {
    let (bytes, layer, _, second) = masks();
    for relevant in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if relevant { ("layer.mask.motionBlur", json!({"layer":layer,"mask":second,"mode":"on"})) } else { opacity(layer, 23.0) }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.mask.motionBlur","params":{"layer":layer,"mode":"off"}}]})).await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &layer.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "all-mask assignment refused layer opacity: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn all_mask_assignment_refuses_a_new_mask_appended_after_basis() {
    let (bytes, layer, _, _) = masks();
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("mask.new", json!({"layer":layer,"vertices":[[50,0],[60,0],[60,10]],"closed":true}))]);
    let refused = f.refused("run", json!({"basis":basis,"steps":[{"command":"layer.mask.motionBlur","params":{"layer":layer,"mode":"off"}}]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "all-mask membership changed: {refused}");
    f.shutdown().await;
}

#[tokio::test]
async fn string_layer_mask_ordinal_holds_only_its_selector_window() {
    let (bytes, layer, _, _) = masks();
    for before in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("mask.new", json!({"layer":layer,"vertices":[[50,0],[60,0],[60,10]],"closed":true}))]);
        if before {
            rewrite(&f.file(), |project| {
                let children = mask_children(project, layer);
                let inserted = children.pop().unwrap();
                children.insert(0, inserted);
            });
        }
        let answer = f
            .call("run", json!({"basis":basis,"steps":[{"command":"layer.mask.motionBlur","params":{"layer":layer.to_string(),"mask":1,"mode":"off"}}]}))
            .await;
        if before {
            let refused = answer.unwrap_err();
            assert_eq!(refused["error"]["code"], "conflict", "ordinal names another mask: {refused}");
        } else {
            assert!(answer.is_ok(), "appending after the first mask does not change mask #1: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn all_mask_removal_should_refuse() {
    let (bytes, layer, first, _) = masks();
    let f = Fixture::from_bytes(&bytes, Default::default());
    let (_, _, basis) = f.inspect().await;
    f.colleague(&[("mask.remove", json!({"layer":layer,"mask":first}))]);
    let refused =
        f.refused("run", json!({"basis":basis,"steps":[{"command":"layer.mask.motionBlur","params":{"layer":layer.to_string(),"mode":"off"}}]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "all-mask membership changed: {refused}");
    f.shutdown().await;
}

#[tokio::test]
async fn all_mask_assignment_accepts_geometry_and_member_reordering() {
    let (bytes, layer, _, _) = masks();
    for reorder in [true, false] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        rewrite(&fixture.file(), |project| {
            let children = mask_children(project, layer);
            if reorder {
                children.swap(0, 1);
            } else {
                children[0]["children"][0]["value"]["v"]["vertices"][0][0] = json!(71);
            }
        });
        let answer = fixture.call("run", json!({"basis":basis,"steps":[{"command":"layer.mask.motionBlur","params":{"layer":layer,"mode":"off"}}]})).await;
        assert!(answer.is_ok(), "mask scalar assignment does not consume member order or path geometry: {answer:?}");
        fixture.shutdown().await;
    }
}

#[tokio::test]
async fn all_mask_membership_is_private_and_holds_another_agents_deletion() {
    let (bytes, layer, first, _) = masks();
    for compound in [true, false] {
        let fixture = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = fixture.inspect().await;
        let mut steps = Vec::new();
        if compound {
            steps.push(json!({"command":"comp.settings","params":{"background":"#123456"}}));
        }
        steps.push(json!({"command":"mask.remove","params":{"layer":layer,"mask":first}}));
        let other = fixture.call_by("other", "run", json!({"steps":steps})).await;
        assert!(!other["changes"].to_string().contains("~maskMembers"), "private guard facts leaked: {other}");
        let saved = std::fs::read(fixture.file()).unwrap();
        let error = fixture.refused("run", json!({"basis":basis,"steps":[{"command":"layer.mask.motionBlur","params":{"layer":layer,"mode":"off"}}]})).await;
        assert_eq!(error["error"]["code"], "conflict", "{error}");
        assert!(!error["others"].to_string().contains("~maskMembers"), "private guard facts leaked: {error}");
        assert_eq!(std::fs::read(fixture.file()).unwrap(), saved);
        fixture.shutdown().await;
    }
}
