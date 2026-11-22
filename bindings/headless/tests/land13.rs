//! C8-land-13t: insertion gaps and the boundary between authored state and rendering.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::*;
use kition_kit::KitDocument;
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread")]
async fn layer_arrange_holds_its_destination_gap() {
    for before in [true, false] {
        let f = Fixture::new(5);
        let (info, _, basis) = f.inspect().await;
        let layers = layer_ids(&info);
        f.colleague(&[("edit.clear", json!({"layers": [layers[if before {1} else {4}]]}))]);
        let answer = f.call("run", json!({"basis": basis, "steps": [{"command": "layer.arrange", "params": {"layers": [layers[0]], "index": 3}}]})).await;
        if before {
            let error = answer.expect_err("deleting before the destination changes the gap");
            assert_eq!(error["error"]["code"], "conflict", "{error}");
            assert!(error["conflicts"].as_array().unwrap().iter().any(|c| c["reason"] == "relocated"), "{error}");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn layer_insertions_hold_only_gaps_they_precede() {
    for before in [true, false] {
        let f = Fixture::new(5);
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        let mut doc = on_disk(&f.file());
        let created = doc.execute("layer.newSolid", json!({"name":"Colleague"})).unwrap()["layer"].clone();
        if !before {
            doc.execute("layer.arrange", json!({"layers":[created],"to":"back"})).unwrap();
        }
        std::fs::write(f.file(), doc.save(None).unwrap()).unwrap();
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.arrange","params":{"layers":[layer],"index":3}}]})).await;
        if before {
            assert!(answer.is_err(), "insertion before the gap applied: {answer:?}");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn effect_reorder_ignores_changes_after_its_gap() {
    for before in [true, false] {
        let f = Fixture::new(1);
        let (info, _, _) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        let mut doc = on_disk(&f.file());
        let uids: Vec<_> =
            (0..5).map(|_| doc.execute("effect.apply", json!({"layer":layer,"effect":"Gaussian Blur"})).unwrap()["effects"][0].as_u64().unwrap()).collect();
        std::fs::write(f.file(), doc.save(None).unwrap()).unwrap();
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("effect.remove", json!({"layer":layer,"effect":uids[if before {0} else {4}]}))]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"effect.reorder","params":{"layer":layer,"effect":uids[1],"index":3}}]})).await;
        if before {
            assert!(answer.is_err(), "stale effect destination applied: {answer:?}");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_move_holds_the_interval_that_can_clamp_its_destination() {
    for dependent in [true, false] {
        let f = Fixture::new(1);
        let (info, _, _) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        for time in [0, 2, 4] {
            f.ok("run", json!({"steps":[{"command":"prop.addKey","params":{"layer":layer,"path":OPACITY,"time":time,"value":20}}]})).await;
        }
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("prop.addKey", json!({"layer":layer,"path":OPACITY,"time":if dependent {2.5} else {3.5},"value":30}))]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"keys.set","params":{"layer":layer,"path":OPACITY,"time":2,"newTime":3}}]})).await;
        if dependent {
            let error = answer.expect_err("a new neighbour would clamp the requested move");
            assert_eq!(error["error"]["code"], "conflict", "{error}");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn render_output_holds_the_queue_item_but_not_composition_pixels() {
    for dependent in [true, false] {
        let f = Fixture::new(1);
        let (info, comp, _) = f.inspect().await;
        let layer = layer_ids(&info)[0];
        f.ok("run", json!({"steps":[{"command":"renderQueue.add","params":{"comp":comp,"resolution":1}}]})).await;
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[if dependent { ("renderQueue.setRenderSettings", json!({"index":1,"quality":"draft"})) } else { opacity(layer, 40.0) }]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"renderQueue.setRenderSettings","params":{"index":1,"resolution":0.5}}]})).await;
        if dependent {
            assert!(answer.is_err(), "the queue output changed: {answer:?}");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_write_inside_a_precomp_holds_local_state_not_its_consumer() {
    for dependent in [true, false] {
        let mut doc = kition_motion_kit::MotionDocument::open(FILE, &project(1)).unwrap();
        let outer = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
        let layer = layer_ids(&doc.inspect().unwrap()["info"])[0];
        doc.execute("layer.precompose", json!({"layers":[layer],"name":"Inner","open":true})).unwrap();
        let inner = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
        let inside = layer_ids(&doc.inspect().unwrap()["info"])[0];
        doc.execute("comp.open", json!({"comp":outer})).unwrap();
        let consumer = layer_ids(&doc.inspect().unwrap()["info"])[0];
        let f = Fixture::from_bytes(&doc.save(None).unwrap(), Default::default());
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("comp.open", json!({"comp":if dependent {inner} else {outer}})), opacity(if dependent { inside } else { consumer }, 40.0)]);
        let answer = f
            .call(
                "run",
                json!({"basis":basis,"steps":[{"command":"prop.set","at":{"part":{"comp":inner}},"params":{"layer":inside,"path":OPACITY,"value":25}}]}),
            )
            .await;
        if dependent {
            assert!(answer.is_err(), "local precomp value changed");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn model_moves_and_creates_have_valid_saved_forms() {
    let f = Fixture::new(3);
    let (info, comp, _) = f.inspect().await;
    let start = layer_ids(&info);
    let loaded = f.ok("query", json!({"objects":[comp]})).await;
    let held = loaded["objects"][0]["fields"]["comp"].clone();
    let by_path = f.call("batch", json!({"ops":[{"op":"list.move","target":format!("{comp}.comp"),"field":"layers","from":0,"to":2}]})).await;
    // SDK 34dff06 resolves a list parent by its raw reference, not this held spelling.
    assert_eq!(by_path.unwrap_err()["error"]["code"], "not-found");
    let first = f.ok("batch", json!({"ops":[{"op":"list.move","target":held,"field":"layers","from":0,"to":2}]})).await;
    // `to` is a gap counted before the item leaves: the top layer goes between the other two.
    let arranged = vec![start[1], start[0], start[2]];
    assert_eq!(layer_ids(&f.inspect().await.0), arranged, "{first}");
    // The move's receipt names the list by its place on the comp's item, the saved form kit-env replays.
    assert_eq!(first["changes"]["moved"].as_array().map(Vec::len), Some(1), "{first}");
    let place = &first["changes"]["moved"][0]["to"];
    assert_eq!((&place["parent"], &place["field"]), (&json!(comp), &json!("comp.layers")), "{first}");
    // Replayed as a raw batch target and field, that place applies (SDK dcf5ca7: a number target, a nested holder path).
    let replay = f.ok("batch", json!({"ops":[{"op":"list.move","target":place["parent"],"field":place["field"],"from":1,"to":0}]})).await;
    let from = json!({"parent": comp, "field": "comp.layers", "index": 1});
    let to = json!({"parent": comp, "field": "comp.layers", "index": 0});
    assert_eq!(replay["changes"]["moved"], json!([{"id": arranged[1], "type": "Layer", "from": from, "to": to}]), "{replay}");
    assert_eq!(replay["changes"]["created"], json!([]), "{replay}");
    assert_eq!(replay["changes"]["deleted"], json!([]), "{replay}");
    // Moving the middle layer back to the top restores the original order, in memory and in the saved file.
    assert_eq!(layer_ids(&f.inspect().await.0), start, "{replay}");
    assert_eq!(layers_on_disk(&f.file()), start, "the replayed move is saved: {replay}");
    f.ok("batch", json!({"ops":[{"op":"create","type":"Item","in":"root.items","at":0,"fields":{"kind":"folder","name":"New folder"}}]})).await;
    f.shutdown().await;
}

/// A batch a model recorded against one opening of the file, replayed on the file opened again by
/// another manager. The kit builds its replica from the file on every open, so the raw object id
/// of the composition's `Comp` does not survive the reopen (kition-sdk dcf5ca7,
/// `docs/platform/data-model.md`, "Ids of a file opened again": "an `ObjectId` does not survive a
/// reopen; tool ids read from the file do"; `docs/platform/kits/call-context.md`: "reobserve after
/// restart/eviction"). The replay first proves the old id is refused, then re-observes the
/// composition by its place, the project's first item, as an agent does, and replays the move
/// against the id it reads there.
#[tokio::test(flavor = "multi_thread")]
async fn a_composition_reference_is_reobserved_after_reopening_for_model_replay() {
    let bytes = project(3);
    let source = Fixture::from_bytes(&bytes, Default::default());
    let (info, comp, _) = source.inspect().await;
    let start = layer_ids(&info);
    let loaded = source.ok("query", json!({"objects":[comp]})).await;
    let held = loaded["objects"][0]["fields"]["comp"].clone();
    assert!(held.is_string(), "the composition's Comp is a raw object reference: {loaded}");
    source.shutdown().await;

    let replay = Fixture::from_bytes(&bytes, Default::default());
    // The contract, pinned: the id from the other opening names nothing here, and nothing changes.
    let stale = replay.call("batch", json!({"ops":[{"op":"list.move","target":held,"field":"layers","from":0,"to":2}]})).await;
    let stale = stale.expect_err("an object id from another opening of the file must be refused");
    assert_eq!(stale["error"]["code"], "not-found", "{stale}");
    assert_eq!(std::fs::read(replay.file()).unwrap(), bytes, "a refused replay leaves the file as it was: {stale}");
    // Re-observe: the composition item keeps its tool id across the reopen, and its place (the
    // project's first item) gives the Comp object's id in this opening.
    let (info, again, _) = replay.inspect().await;
    assert_eq!((again, layer_ids(&info)), (comp, start.clone()), "tool ids survive the reopen");
    let followed = replay.ok("query", json!({"follow":"items.0"})).await;
    assert_eq!(followed["objects"].as_array().map(Vec::len), Some(1), "{followed}");
    assert_eq!(id_text(&followed["objects"][0]["id"]), Some(comp.to_string()), "items.0 is the composition: {followed}");
    let reobserved = followed["objects"][0]["fields"]["comp"].clone();
    assert!(reobserved.is_string() && reobserved != held, "this opening names the Comp by a fresh id: {reobserved} (held {held})");
    // The replay against the re-observed id: `to` is a gap counted before the item leaves, so the
    // top layer goes between the other two, in the running kit and in the saved file.
    let moved = replay.ok("batch", json!({"ops":[{"op":"list.move","target":reobserved,"field":"layers","from":0,"to":2}]})).await;
    let arranged = vec![start[1], start[0], start[2]];
    assert_eq!(layer_ids(&replay.inspect().await.0), arranged, "{moved}");
    assert_eq!(layers_on_disk(&replay.file()), arranged, "the replayed move is saved: {moved}");
    // The receipt tells the same order as the one move that makes it: the middle layer to the top.
    let from = json!({"parent": comp, "field": "comp.layers", "index": 1});
    let to = json!({"parent": comp, "field": "comp.layers", "index": 0});
    assert_eq!(moved["changes"]["moved"], json!([{"id": start[1], "type": "Layer", "from": from, "to": to}]), "{moved}");
    assert_eq!(moved["changes"]["created"], json!([]), "{moved}");
    assert_eq!(moved["changes"]["deleted"], json!([]), "{moved}");
    replay.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_incomplete_composition_creation_returns_an_actionable_error() {
    let f = Fixture::new(1);
    let error = f
        .call("batch", json!({"ops":[{"op":"create","type":"Item","in":"root.items","at":0,"fields":{"kind":"comp","name":"Incomplete"}}]}))
        .await
        .expect_err("a composition requires its owned Comp object");
    assert_eq!(error["error"]["code"], "invalid-value", "{error}");
    assert!(error.to_string().contains("comp"), "{error}");
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_creation_receipts_the_defaults_saved_by_the_engine() {
    let f = Fixture::new(1);
    let answer = f.ok("batch", json!({"ops":[{"op":"create","type":"Item","in":"root.items","at":0,"fields":{"kind":"folder","name":"New folder"}}]})).await;
    let created = &answer["changes"]["created"][0];
    assert_eq!(created["values"]["attrs"]["comment"], "", "{answer}");
    let reopened = Fixture::from_bytes(&std::fs::read(f.file()).unwrap(), Default::default());
    let loaded = reopened.ok("query", json!({"objects":[created["id"]]})).await;
    assert_eq!(loaded["objects"][0]["fields"]["attrs"], created["values"]["attrs"], "the receipt and saved reread must agree");
    f.shutdown().await;
    reopened.shutdown().await;
}

/// A property-tree creation a model recorded against one opening of the file, replayed on the file
/// opened again by another manager. The layer's property tree root is a raw object, minted afresh
/// on every open (kition-sdk dcf5ca7, `docs/platform/data-model.md`, "Ids of a file opened
/// again"): the replay proves the old id is refused, re-observes the root through the layer, whose
/// tool id survives, and creates the group there.
#[tokio::test(flavor = "multi_thread")]
async fn property_tree_creations_replay_on_a_reopened_document() {
    let bytes = project(1);
    let source = Fixture::from_bytes(&bytes, Default::default());
    let (info, _, _) = source.inspect().await;
    let layer = layer_ids(&info)[0];
    let loaded = source.ok("query", json!({"objects":[layer]})).await;
    let held = loaded["objects"][0]["fields"]["props"].clone();
    assert!(held.is_string(), "a layer's property tree root is a raw object reference: {loaded}");
    let props = source.ok("query", json!({"objects":[held]})).await;
    let children = props["objects"][0]["fields"]["children"].as_array().map(Vec::len).unwrap();
    let mut meta = props["objects"][0]["fields"]["meta"].clone();
    meta["name"] = json!("Extra");
    meta["match"] = json!("extra");
    source.shutdown().await;
    let create = |root: &Value| json!({"ops":[{"op":"create","type":"Node","in":format!("{}.children",root.as_str().unwrap()),"at":0,"fields":{"uid":999999,"node":"group","meta":meta,"enabled":true}}]});

    let replay = Fixture::from_bytes(&bytes, Default::default());
    // The contract, pinned: the root's id from the other opening names nothing here, and nothing
    // changes.
    let stale = replay.call("batch", create(&held)).await.expect_err("an object id from another opening of the file must be refused");
    assert_eq!(stale["error"]["code"], "not-found", "{stale}");
    assert_eq!(std::fs::read(replay.file()).unwrap(), bytes, "a refused replay leaves the file as it was: {stale}");
    // Re-observe: the layer keeps its tool id, and names its property tree root by this opening's id.
    let (info, _, _) = replay.inspect().await;
    assert_eq!(layer_ids(&info), vec![layer], "tool ids survive the reopen");
    let reloaded = replay.ok("query", json!({"objects":[layer]})).await;
    let root = reloaded["objects"][0]["fields"]["props"].clone();
    assert!(root.is_string() && root != held, "this opening names the root by a fresh id: {root} (held {held})");
    let created = replay.ok("batch", create(&root)).await;
    // A node has no tool id of its own: the receipt tells the creation as an edit of the layer
    // holding it, at the node's place in the tree.
    assert_eq!((&created["changes"]["created"], &created["changes"]["deleted"]), (&json!([]), &json!([])), "{created}");
    let changed = created["changes"]["changed"].as_array().cloned().unwrap();
    assert_eq!(changed.len(), 1, "{created}");
    assert_eq!(id_text(&changed[0]["id"]), Some(layer.to_string()), "{created}");
    let edit = &changed[0]["edits"][0];
    assert_eq!((&edit["field"], &edit["named"], &edit["before"]), (&json!("props.children.0"), &json!("extra"), &Value::Null), "{created}");
    assert_eq!(
        (&edit["after"]["node"], &edit["after"]["meta"]["name"], &edit["after"]["uid"]),
        (&json!("group"), &json!("Extra"), &json!(999999)),
        "{created}"
    );
    // The group is the root's first child in the running kit, and in the saved file as a new
    // engine reads it.
    let tree = replay.ok("query", json!({"objects":[root]})).await;
    let kids = tree["objects"][0]["fields"]["children"].as_array().cloned().unwrap();
    assert_eq!(kids.len(), children + 1, "{tree}");
    let first = replay.ok("query", json!({"objects":[kids[0].clone()]})).await;
    let fields = &first["objects"][0]["fields"];
    assert_eq!((&fields["node"], &fields["meta"]["name"], &fields["meta"]["match"]), (&json!("group"), &json!("Extra"), &json!("extra")), "{first}");
    let reopened = Fixture::from_bytes(&std::fs::read(replay.file()).unwrap(), Default::default());
    let reread = reopened.ok("query", json!({"objects":[layer]})).await;
    let saved_root = reopened.ok("query", json!({"objects":[reread["objects"][0]["fields"]["props"].clone()]})).await;
    let saved_kids = saved_root["objects"][0]["fields"]["children"].as_array().cloned().unwrap();
    assert_eq!(saved_kids.len(), children + 1, "{saved_root}");
    let saved_first = reopened.ok("query", json!({"objects":[saved_kids[0].clone()]})).await;
    let fields = &saved_first["objects"][0]["fields"];
    assert_eq!((&fields["node"], &fields["meta"]["name"], &fields["meta"]["match"]), (&json!("group"), &json!("Extra"), &json!("extra")), "{saved_first}");
    replay.shutdown().await;
    reopened.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn authored_values_do_not_hold_their_render_consumers() {
    for family in ["expression", "adjustment", "matte"] {
        for dependent in [true, false] {
            let f = Fixture::new(2);
            let (info, _, _) = f.inspect().await;
            let layers = layer_ids(&info);
            match family {
                "expression" => {
                    f.ok("run", json!({"steps":[{"command":"prop.setExpression","params":{"layer":layers[1],"path":"transform/opacity","expression":"thisComp.layer(1).transform.opacity"}}]})).await;
                }
                "adjustment" => {
                    f.ok("run", json!({"steps":[{"command":"layer.setSwitch","params":{"layers":[layers[1]],"switch":"adjustment","value":true}}]})).await;
                }
                "matte" => {
                    f.ok("run", json!({"steps":[{"command":"layer.trackMatte","params":{"layers":[layers[1]],"op":"alpha"}}]})).await;
                }
                _ => {}
            }
            let (_, _, basis) = f.inspect().await;
            f.colleague(&[opacity(layers[if dependent { 0 } else { 1 }], 40.0)]);
            let answer = f.call("run", json!({"basis":basis,"steps":[set_opacity(layers[0],25.0)]})).await;
            if dependent {
                assert_conflict(&answer.expect_err("the authored value changed"), &layers[0].to_string(), "changed");
            } else {
                answer.unwrap();
            }
            f.shutdown().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn expression_text_holds_its_authored_property_not_literal_references() {
    for reference in ["thisComp.layer(2).transform.opacity", "thisComp.layer(\"L0\").transform.opacity"] {
        for dependent in [true, false] {
            let f = Fixture::new(2);
            let (info, _, _) = f.inspect().await;
            let layers = layer_ids(&info);
            f.ok("run", json!({"steps":[{"command":"prop.setExpression","params":{"layer":layers[0],"path":OPACITY,"expression":reference}}]})).await;
            let (_, _, basis) = f.inspect().await;
            f.colleague(&[if dependent {
                ("prop.setExpression", json!({"layer":layers[0],"path":OPACITY,"expression":"value / 2"}))
            } else {
                ("layer.rename", json!({"layer":layers[1],"name":"Colleague"}))
            }]);
            let answer = f.call("run", json!({"basis":basis,"steps":[set_opacity(layers[0],25.0)]})).await;
            if dependent {
                assert!(answer.is_err(), "the edited expression is part of the authored property");
            } else {
                answer.unwrap();
            }
            f.shutdown().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn compensation_holds_ancestor_transforms_but_not_ancestor_opacity() {
    for dependent in [true, false] {
        let f = Fixture::new(4);
        let (info, _, _) = f.inspect().await;
        let layers = layer_ids(&info);
        f.ok(
            "run",
            json!({"steps":[
                {"command":"layer.setParent","params":{"layers":[layers[0]],"parent":layers[1]}},
                {"command":"layer.setParent","params":{"layers":[layers[1]],"parent":layers[2]}}
            ]}),
        )
        .await;
        let (_, _, basis) = f.inspect().await;
        f.colleague(&[("prop.set", json!({"layer":layers[2],"path":if dependent {"transform/rotation"} else {OPACITY},"value":35}))]);
        let answer = f.call("run", json!({"basis":basis,"steps":[{"command":"layer.setParent","params":{"layers":[layers[0]],"parent":layers[3]}}]})).await;
        if dependent {
            assert!(answer.is_err(), "the old grandparent transform changed");
        } else {
            answer.unwrap();
        }
        f.shutdown().await;
    }
}
