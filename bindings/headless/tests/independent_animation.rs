//! C8-motion-read: animation premises against an independent saved-file writer.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::*;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn animated() -> (Vec<u8>, u64, u64, u64) {
    let mut doc = MotionDocument::open(FILE, &project(2)).unwrap();
    let ids = layer_ids(&doc.inspect().unwrap()["info"]);
    let source = ids[0];
    for (time, value) in [(0, [70, 80, 90]), (1, [100, 120, 140]), (2, [130, 170, 210])] {
        doc.execute("prop.addKey", json!({"layer":source,"path":"transform/scale","time":time,"value":value})).unwrap();
    }
    let uid = doc.execute("prop.get", json!({"layer":source,"path":"transform/scale"})).unwrap()["uid"].as_u64().unwrap();
    (doc.save(None).unwrap(), source, ids[1], uid)
}

fn property<'a>(project: &'a mut Value, layer: u64, name: &str) -> &'a mut Value {
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
        .find(|group| group["match"] == "transform")
        .unwrap()["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|property| property["match"] == name)
        .unwrap()
}

fn key_edit(layer: u64, uid: u64, address: &str, new_time: Option<f64>) -> Value {
    let mut params = json!({"layer":layer,"time":1,"value":[105,125,145]});
    match address {
        "prop" => params["prop"] = json!(uid),
        "uid" => params["path"] = json!(format!("@{uid}")),
        "path" => params["path"] = json!("transform/scale"),
        _ => panic!("unknown fixture address {address}"),
    }
    if let Some(time) = new_time {
        params["newTime"] = json!(time);
    }
    params
}

#[tokio::test]
async fn moved_key_holds_comp_frame_rate_but_value_only_does_not() {
    let (bytes, source, _, uid) = animated();
    for address in ["prop", "uid", "path"] {
        for moving in [true, false] {
            let f = Fixture::from_bytes(&bytes, Default::default());
            let (_, comp, basis) = f.inspect().await;
            f.colleague(&[("comp.settings", json!({"frameRate":24}))]);
            let answer =
                f.call("run", json!({"basis":basis,"steps":[{"command":"keys.set","params":key_edit(source,uid,address,moving.then_some(1.61))}]})).await;
            if moving {
                assert_conflict(&answer.unwrap_err(), &comp.to_string(), "changed");
            } else {
                assert!(answer.is_ok(), "value-only key edit refused frame rate: {answer:?}");
            }
            f.shutdown().await;
        }
    }
}

#[tokio::test]
async fn moved_key_holds_both_neighbor_clamp_bounds() {
    let (bytes, source, _, uid) = animated();
    for address in ["prop", "uid", "path"] {
        for (neighbor, changed_time, destination) in [(0, 203_212_800_000_i64, 0.1), (2, 304_819_200_000_i64, 1.6)] {
            let f = Fixture::from_bytes(&bytes, Default::default());
            let (_, _, basis) = f.inspect().await;
            rewrite(&f.file(), |project| property(project, source, "scale")["keys"][neighbor]["time"] = json!(changed_time));
            let refused =
                f.refused("run", json!({"basis":basis,"steps":[{"command":"keys.set","params":key_edit(source,uid,address,Some(destination))}]})).await;
            assert_conflict(&refused, &source.to_string(), "changed");
            f.shutdown().await;
        }
    }
}

#[tokio::test]
async fn key_edits_hold_endpoint_roving_flags_they_overwrite() {
    let (bytes, source, _, uid) = animated();
    for endpoint in [0, 2] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        rewrite(&f.file(), |project| property(project, source, "scale")["keys"][endpoint]["roving"] = json!(true));
        let refused = f.refused("run", json!({"basis":basis,"steps":[{"command":"keys.set","params":key_edit(source,uid,"uid",None)}]})).await;
        assert_conflict(&refused, &source.to_string(), "changed");
        f.shutdown().await;
    }
}

#[tokio::test]
async fn key_edits_accept_unrelated_opacity_on_the_same_layer() {
    let (bytes, source, _, uid) = animated();
    for address in ["prop", "uid", "path"] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[opacity(source, 23.0)]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"keys.set","params":key_edit(source,uid,address,Some(1.6))}]})).await;
        assert!(answer.is_ok(), "key edit refused unrelated opacity: {answer:?}");
        assert_eq!(opacity_on_disk(&f.file(), source), 23.0);
        f.shutdown().await;
    }
}

#[tokio::test]
async fn explicit_expression_holds_expr_without_holding_static_value() {
    let (bytes, source, _, _) = animated();
    for relevant in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        if relevant {
            f.colleague(&[("prop.setExpression", json!({"layer":source,"path":OPACITY,"expression":"value + 2"}))]);
        } else {
            f.colleague(&[opacity(source, 23.0)]);
        }
        let answer = f
            .call("run", json!({"basis":basis,"steps":[{"command":"prop.setExpression","params":{"layer":source,"path":OPACITY,"expression":"value + 1"}}]}))
            .await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &source.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "explicit expression refused static value: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn scalar_pick_whip_holds_target_name_without_target_opacity() {
    let (bytes, source, target, _) = animated();
    for relevant in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if relevant { ("layer.rename", json!({"layer":target,"name":"Changed target"})) } else { opacity(target, 23.0) }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"prop.pickWhip","params":{"layer":source,"path":OPACITY,"target":{"layer":target,"path":"transform/rotation"}}}]})).await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &target.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "pick whip refused target opacity: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn pick_whip_named_target_comp_holds_name_without_background() {
    let (bytes, source, target, _) = animated();
    let mut doc = MotionDocument::open(FILE, &bytes).unwrap();
    let rotation = doc.execute("prop.get", json!({"layer":target,"path":"transform/rotation"})).unwrap()["uid"].as_u64().unwrap();
    for relevant in [true, false] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("comp.settings", if relevant { json!({"name":"Renamed Main"}) } else { json!({"background":"#123456"}) })]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"prop.pickWhip","params":{"layer":source,"path":OPACITY,"target":{"comp":"Main","layer":target,"prop":rotation}}}]})).await;
        if relevant {
            let refused = answer.unwrap_err();
            assert_eq!(refused["error"]["code"], "conflict", "renamed target composition reached the engine: {refused}");
        } else {
            assert!(answer.is_ok(), "pick whip refused target composition background: {answer:?}");
        }
        f.shutdown().await;
    }
}

#[tokio::test]
async fn outgoing_linear_ease_holds_next_value_without_previous_value() {
    let (bytes, source, _, uid) = animated();
    for (relevant, key) in [(true, 2), (false, 0)] {
        let f = Fixture::from_bytes(&bytes, Default::default());
        let (_, _, basis) = f.inspect().await;
        rewrite(&f.file(), |project| property(project, source, "scale")["keys"][key]["value"]["v"] = json!([71, 82, 93]));
        let answer = f
            .call(
                "run",
                json!({"basis":basis,"steps":[{"command":"keys.setEase","params":{"layer":source,"prop":uid,"time":1,"side":"out","dim":0,"speed":12}}]}),
            )
            .await;
        if relevant {
            assert_conflict(&answer.unwrap_err(), &source.to_string(), "changed");
        } else {
            assert!(answer.is_ok(), "outgoing ease refused previous value: {answer:?}");
        }
        f.shutdown().await;
    }
}
