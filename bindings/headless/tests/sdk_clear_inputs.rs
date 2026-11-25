mod common;
use common::*;
use serde_json::{Value, json};

/// Top first: L2, L1, L0. Deleting L2 orphans whatever is parented to it and drops mattes on it.
async fn stale(command: &str, change: impl FnOnce(&Fixture, &[u64]), setup: &[(&str, Value)]) -> Result<Value, Value> {
    let f = Fixture::new(3);
    let (info, _, _) = f.inspect().await;
    let ids = layer_ids(&info);
    if !setup.is_empty() {
        let steps: Vec<Value> = setup.iter().map(|(command, params)| json!({"command":command,"params":params})).collect();
        f.ok("run", json!({"steps":steps})).await;
    }
    let (_, _, basis) = f.inspect().await;
    change(&f, &ids);
    let saved = std::fs::read(f.file()).unwrap();
    let answer = f.call("run", json!({"basis":basis,"steps":[{"command":command,"params":{"layers":[ids[0]]}}]})).await;
    if answer.is_err() {
        assert_eq!(std::fs::read(f.file()).unwrap(), saved);
    }
    f.shutdown().await;
    answer
}

#[tokio::test]
async fn deleting_a_layer_holds_its_children_and_mattes_but_not_unrelated_layers() {
    for command in ["edit.delete", "edit.cut"] {
        // A colleague parents L1 to the layer being deleted: the deletion would unparent and
        // compensate a child the agent never saw.
        let parented = stale(command, |f, ids| drop(f.colleague(&[("layer.setParent", json!({"layers":[ids[1]],"parent":ids[0]}))])), &[]).await;
        assert_eq!(parented.expect_err("a new child would be orphaned unseen")["error"]["code"], "conflict", "{command}");
        // A colleague moves an existing child: its compensation would be computed from it.
        let ids_setup = {
            let f = Fixture::new(3);
            let (info, _, _) = f.inspect().await;
            let ids = layer_ids(&info);
            f.shutdown().await;
            ids
        };
        let child = [("layer.setParent", json!({"layers":[ids_setup[1]],"parent":ids_setup[0]}))];
        let moved =
            stale(command, |f, ids| drop(f.colleague(&[("prop.set", json!({"layer":ids[1],"path":"transform/position","value":[10,20]}))])), &child).await;
        assert_eq!(moved.expect_err("the orphan's position is assigned")["error"]["code"], "conflict", "{command}");
        let matte = stale(command, |f, ids| drop(f.colleague(&[("layer.setTrackMatte", json!({"layer":ids[2],"matte":ids[0]}))])), &[]).await;
        assert_eq!(matte.expect_err("a new matte on the deleted layer would be dropped unseen")["error"]["code"], "conflict", "{command}");
        let unrelated = stale(command, |f, ids| drop(f.colleague(&[opacity(ids[2], 23.0)])), &[]).await;
        assert!(unrelated.is_ok(), "{command}: {unrelated:?}");
    }
}

/// The layer ids of a fresh `Fixture::new(3)`, top first, as `stale` sees them.
async fn fixture_ids() -> Vec<u64> {
    let f = Fixture::new(3);
    let (info, _, _) = f.inspect().await;
    f.shutdown().await;
    layer_ids(&info)
}

/// The layers a deletion depends on are the ones whose parent or matte is the deleted layer (a
/// derived fact of that layer), never the composition's whole membership: a layer created after
/// the read and parented or matted to the deleted layer refuses; an unrelated new layer, and an
/// unrelated reparenting, apply.
#[tokio::test]
async fn a_new_layer_tied_to_the_deleted_layer_refuses_and_an_unrelated_one_applies() {
    let deleted = fixture_ids().await[0];
    for command in ["edit.delete", "edit.cut"] {
        let tied = |relation: &'static str| {
            move |f: &Fixture, ids: &[u64]| {
                let added = f.colleague(&[("layer.newSolid", json!({"name":"Late","color":"#00ff00"}))]);
                let late = added[0]["layer"].as_u64().unwrap();
                let step = match relation {
                    "parent" => ("layer.setParent", json!({"layers":[late],"parent":ids[0]})),
                    _ => ("layer.setTrackMatte", json!({"layer":late,"matte":ids[0]})),
                };
                drop(f.colleague(&[step]));
            }
        };
        for relation in ["parent", "matte"] {
            let answer = stale(command, tied(relation), &[]).await;
            let refused = answer.expect_err("a late layer tied to the deleted layer");
            assert_eq!(refused["error"]["code"], "conflict", "{command} {relation}");
            // Refused because the deleted layer's dependents changed, not because it moved.
            let conflicts = refused["conflicts"].as_array().unwrap();
            assert!(conflicts.iter().any(|conflict| conflict["id"] == json!(deleted) && conflict["reason"] == "changed"), "{command} {relation}: {refused}");
        }
        let unrelated_new = stale(
            command,
            |f, ids| {
                let added = f.colleague(&[("layer.newSolid", json!({"name":"Late","color":"#00ff00"}))]);
                let late = added[0]["layer"].as_u64().unwrap();
                drop(f.colleague(&[("layer.setParent", json!({"layers":[late],"parent":ids[2]}))]));
            },
            &[],
        )
        .await;
        assert!(unrelated_new.is_ok(), "{command}: {unrelated_new:?}");
        let unrelated_parent = stale(command, |f, ids| drop(f.colleague(&[("layer.setParent", json!({"layers":[ids[1]],"parent":ids[2]}))])), &[]).await;
        assert!(unrelated_parent.is_ok(), "{command}: {unrelated_parent:?}");
    }
}
