//! C8-time-4: receipts describe the saved file, and marker premises follow their actual inputs.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::*;
use kition_kit::KitDocument;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn real_query_follows_items_to_layers() {
    let f = Fixture::new(2);
    let (info, _, _) = f.inspect().await;
    let followed = f.ok("query", json!({"follow": "items.0.comp.layers"})).await;
    assert_eq!(ids(&followed["objects"]), layer_ids(&info).iter().map(u64::to_string).collect::<Vec<_>>(), "{followed}");
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn project_items_have_no_user_order() {
    let f = Fixture::new(2);
    f.inspect().await;
    f.refused("batch", json!({"ops": [{"op": "list.move", "target": "root", "field": "items", "from": 0, "to": 3}]})).await;
    let schema = f.ok("query", json!({"describe": "*"})).await;
    assert_eq!(schema["describe"]["types"]["Project"]["fields"]["items"]["sample"], false);
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_property_cannot_acknowledge_enabled() {
    let f = Fixture::new(2);
    let (info, _, _) = f.inspect().await;
    let layer = layer_ids(&info)[0];
    let before = std::fs::read(f.file()).unwrap();
    f.refused("batch", json!({"ops": [{"op": "set", "target": layer, "field": "transform/opacity.enabled", "value": false}]})).await;
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    // A subsequent valid write also proves the refused value did not poison the replica.
    f.ok("run", json!({"steps": [set_opacity(layer, 75.0)]})).await;
    assert_eq!(opacity_on_disk(&f.file(), layer), 75.0);
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_a_comp_receipts_every_owned_layer() {
    let f = Fixture::new(2);
    let (info, comp, _) = f.inspect().await;
    let mut expected = layer_ids(&info).into_iter().chain([comp]).map(|n| n.to_string()).collect::<Vec<_>>();
    expected.sort();
    let ran = f.ok("batch", json!({"ops": [{"op": "delete", "target": comp}], "changes": "all"})).await;
    let mut deleted = ids(&ran["changes"]["deleted"]);
    deleted.sort();
    assert_eq!(deleted, expected, "{ran}");
    assert!(on_disk(&f.file()).inspect().unwrap()["info"]["layers"].as_array().is_none_or(Vec::is_empty));
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn opacity_outside_its_range_is_refused() {
    let f = Fixture::new(2);
    let (info, _, _) = f.inspect().await;
    let layer = layer_ids(&info)[0];
    f.refused("batch", json!({"ops": [{"op": "set", "target": layer, "field": "transform/opacity.value.v", "value": 101}]})).await;
    assert_eq!(opacity_on_disk(&f.file(), layer), 100.0);
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn layer_marker_needs_its_time_not_its_stack_index() {
    for change in ["above", "time", "marker"] {
        let f = Fixture::new(3);
        let (info, _, basis) = f.inspect().await;
        let layer = layer_ids(&info)[1];
        f.colleague(&[match change {
            "above" => ("layer.newSolid", json!({"name": "Theirs"})),
            "time" => ("layer.timing", json!({"layers": [layer], "delta": 1})),
            _ => ("layer.addMarker", json!({"layer": layer, "time": 2, "comment": "theirs"})),
        }]);
        let answer = f
            .call("run", json!({"basis": basis, "steps": [{"command": "layer.addMarker", "params": {"layer": layer, "comment": "mine"}, "at": {"time": 2}}]}))
            .await;
        if change == "above" {
            answer.unwrap_or_else(|e| panic!("{e}"));
            let mut disk = on_disk(&f.file());
            let markers = disk.execute("marker.list", json!({"layer": layer})).unwrap();
            assert!(markers.to_string().contains("mine"), "{markers}");
        } else {
            assert_conflict(&answer.expect_err(change), &layer.to_string(), if change == "time" { "relocated" } else { "changed" });
        }
        f.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn new_comp_marker_names_the_markers_only() {
    for command in ["comp.addMarker", "marker.set"] {
        for change in ["layer", "marker"] {
            let f = Fixture::new(3);
            let (_, comp, basis) = f.inspect().await;
            f.colleague(&[if change == "layer" {
                ("layer.newSolid", json!({"name": "Theirs"}))
            } else {
                ("comp.addMarker", json!({"time": 2, "comment": "theirs"}))
            }]);
            let params = if command == "marker.set" { json!({"new": true, "comment": "mine"}) } else { json!({"comment": "mine"}) };
            let answer = f.call("run", json!({"basis": basis, "steps": [{"command": command, "params": params, "at": {"time": 2}}]})).await;
            if change == "layer" {
                answer.unwrap_or_else(|e| panic!("{command}: {e}"));
                let mut disk = on_disk(&f.file());
                let markers = disk.execute("marker.list", json!({})).unwrap();
                assert!(markers.to_string().contains("mine"), "{markers}");
            } else {
                assert_conflict(&answer.expect_err(change), &comp.to_string(), "changed");
            }
            f.shutdown().await;
        }
    }
}
