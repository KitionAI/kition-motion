//! The root reaches the whole object model (KHP A16: an agent reads objects "by following fields
//! from the root"): `Project.items` lists every item, an item's `comp` its composition, a
//! composition's `layers` its layers, a layer's `props` its property tree, a property's `keys`
//! its keyframes, and `Project.renderQueue` the Render Queue. The list is a KDM `list` (a movable
//! list): two replicas adding items offline keep both; a delete unlists the item in the same
//! transaction; and the repair after a merge lists an item no list holds, and drops a second
//! listing or a listing of a deleted item, the same way on every replica.

mod common;

use std::collections::{BTreeSet, HashSet};

use common::{actor, add_layer, demo, first, peer, person, record, solid, sync};
use kdm_model::{MotionModel, MotionSession};
use kition_kit::kdm::{DocId, FieldKind, KdmError, ObjectId, Origin, Replica, Schema};
use kition_kit::session::KitModel;
use kition_motion_project::LayerSource;
use kition_motion_project::render_queue::RenderQueueItem;
use serde_json::json;

fn root() -> ObjectId {
    ObjectId::from_string("project")
}

fn items(replica: &Replica) -> Vec<ObjectId> {
    replica.list(&root(), "items").unwrap()
}

/// Every live object reached from the root by following `list` and `ref` fields, with its type.
fn reachable(replica: &Replica) -> Vec<(ObjectId, String)> {
    let schema = replica.schema().clone();
    let mut seen: HashSet<ObjectId> = HashSet::new();
    let mut out = Vec::new();
    let mut queue = vec![root()];
    while let Some(object) = queue.pop() {
        if !replica.exists(&object) || !seen.insert(object.clone()) {
            continue;
        }
        let ty = replica.object_type(&object).unwrap();
        for (field, def) in &schema.types[&ty].fields {
            match def.kind {
                FieldKind::List => queue.extend(replica.list(&object, field).unwrap()),
                FieldKind::Ref => {
                    if let Some(target) = replica.get(&object, field).unwrap().as_str() {
                        queue.push(ObjectId::from_string(target));
                    }
                }
                _ => {}
            }
        }
        out.push((object, ty));
    }
    out
}

/// A demo project with a Render Queue entry, a marker, a guide and a Graphics Templates control,
/// so every type of the schema has an object.
fn full_demo() -> common::Demo {
    let mut d = demo();
    d.project.render_queue.push(RenderQueueItem::new(1, d.main));
    if let Some(c) = d.project.comp_mut(d.main) {
        c.markers.push(kition_motion_project::Marker { comment: "start".into(), ..Default::default() });
        c.guides.push(kition_motion_project::Guide { vertical: true, position: 10.0 });
        c.graphics_template = Some(kition_motion_project::template_props::GraphicsTemplate {
            name: "Main".into(),
            controls: vec![kition_motion_project::template_props::EgControl {
                id: 9001,
                name: "Note".into(),
                kind: kition_motion_project::template_props::EgKind::Comment { text: "hi".into() },
                as_type: None,
            }],
        });
    }
    d
}

#[test]
fn the_schema_is_version_1_and_the_root_lists_items() {
    let schema = MotionModel::schema();
    assert_eq!(schema.schema_version, 1);
    let field = &schema.types["Project"].fields["items"];
    assert_eq!((field.kind, field.of.as_deref()), (FieldKind::List, Some("Item")));
    let kit: serde_json::Value = serde_json::from_str(include_str!("../../../../kit.json")).unwrap();
    assert_eq!(kit["model"]["schemaVersion"], 1, "kit.json names the schema's version");
}

#[test]
fn every_object_and_every_type_is_reachable_from_the_root() {
    let d = full_demo();
    let (replica, _) = common::new_replica(&d.project);
    assert_eq!(items(&replica).len(), d.project.items.len(), "one listing per item");
    let reached = reachable(&replica);
    let types: BTreeSet<&str> = reached.iter().map(|(_, ty)| ty.as_str()).collect();
    let declared: BTreeSet<&str> = replica.schema().types.keys().map(String::as_str).collect();
    assert_eq!(types, declared, "every type the schema declares");
    let objects: BTreeSet<&ObjectId> = reached.iter().map(|(object, _)| object).collect();
    for ty in &declared {
        for object in replica.objects_of_type(ty) {
            assert!(objects.contains(&object), "{ty} {object} is not reachable from the root");
        }
    }
}

/// An item's and a layer's name are `string` fields of their own (not keys of `attrs`): a free
/// string an agent renames with one `set`, as KHP's round trip does.
#[test]
fn item_and_layer_names_are_string_fields() {
    let d = demo();
    let (replica, ids) = common::new_replica(&d.project);
    assert_eq!(replica.schema().types["Item"].fields["name"].kind, FieldKind::String);
    assert_eq!(replica.schema().types["Layer"].fields["name"].kind, FieldKind::String);
    assert_eq!(replica.get(&ids.items[&d.main], "name").unwrap(), json!("Main"));
    assert!(replica.map_get(&ids.items[&d.main], "attrs", "name").unwrap().is_null(), "the name is not in attrs too");
    let red = ids.layers.iter().find(|(key, _)| key.id == d.red).map(|(_, l)| l.object.clone()).unwrap();
    assert_eq!(replica.get(&red, "name").unwrap(), json!("Red"));
    assert!(replica.map_get(&red, "attrs", "name").unwrap().is_null());
}

#[test]
fn a_rename_through_the_name_field_reaches_the_project() {
    let d = demo();
    let mut a = first(&d.project);
    let snapshot = a.snapshot().unwrap();
    let mut raw = Replica::load(MotionModel::schema(), &snapshot, actor(5)).unwrap();
    let since = raw.version();
    let red = a.model().ids().layers.iter().find(|(key, _)| key.id == d.red).map(|(_, l)| l.object.clone()).unwrap();
    let main = a.model().ids().items[&d.main].clone();
    raw.transact(&person("agent"), |tx| {
        tx.set(&red, "name", json!("Crimson"))?;
        tx.set(&main, "name", json!("Opening"))
    })
    .unwrap();
    a.import_changes(&raw.export_changes(&since).unwrap()).unwrap();
    let p = &a.model().project;
    assert_eq!(common::layer(p, d.main, d.red).name, "Crimson");
    assert_eq!(p.items[&d.main].name, "Opening");
}

#[test]
fn a_new_item_is_listed_and_a_deleted_one_unlisted_in_the_same_transaction() {
    let d = demo();
    let mut a = first(&d.project);
    let mut after = a.model().project.clone();
    let extra = solid(&mut after, "Extra", [0.2, 0.2, 0.2]);
    record(&mut a, &person("a"), &after);
    let extra_object = a.model().ids().items[&extra].clone();
    assert_eq!(items(a.replica()).last(), Some(&extra_object), "a new item goes at the end");

    let mut after = a.model().project.clone();
    after.items.remove(&extra);
    let before = a.version();
    record(&mut a, &person("a"), &after);
    assert!(!a.replica().exists(&extra_object));
    assert!(!items(a.replica()).contains(&extra_object), "no dangling entry");
    assert_eq!(items(a.replica()).len(), a.model().project.items.len());
    // One transaction did both: undoing it brings the item back, listed.
    assert_ne!(a.version(), before);
    a.undo().unwrap();
    assert!(a.replica().exists(&extra_object));
    assert!(items(a.replica()).contains(&extra_object));
}

#[test]
fn items_added_on_two_replicas_offline_are_both_listed_alike() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);

    let mut after_a = a.model().project.clone();
    let from_a = solid(&mut after_a, "From A", [1.0, 1.0, 0.0]);
    add_layer(&mut after_a, d.main, "A layer", LayerSource::Solid { item: from_a });
    record(&mut a, &person("a"), &after_a);
    let mut after_b = b.model().project.clone();
    let from_b = solid(&mut after_b, "From B", [0.0, 1.0, 1.0]);
    let folder = after_b.add_item("Folder B", kition_motion_color::Label::None, None, kition_motion_project::ItemKind::Folder);
    record(&mut b, &person("b"), &after_b);

    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    assert_eq!(items(a.replica()), items(b.replica()), "one order on both");
    let listed = items(a.replica());
    assert_eq!(listed.len(), d.project.items.len() + 3);
    assert_eq!(listed.iter().collect::<HashSet<_>>().len(), listed.len(), "each once");
    for id in [from_a, from_b, folder] {
        assert!(a.model().project.items.contains_key(&id), "{id:?} on a");
        assert!(b.model().project.items.contains_key(&id), "{id:?} on b");
    }
    common::assert_same_content(&a.model().project, &b.model().project, "converged");
}

/// A peer's raw changes (an object-model batch, or another kit build) can leave the list wrong;
/// the repair every replica runs puts it right, and both replicas end equal.
#[test]
fn the_repair_lists_unlisted_items_and_drops_second_and_dead_listings() {
    let d = demo();
    let mut a = first(&d.project);
    let mut b = peer(&a, 2);
    let snapshot = a.snapshot().unwrap();
    let mut raw = Replica::load(MotionModel::schema(), &snapshot, actor(7)).unwrap();
    let since = raw.version();
    let inner = a.model().ids().items[&d.inner].clone();
    let main = a.model().ids().items[&d.main].clone();
    let unlisted = raw
        .transact(&person("script"), |tx| {
            // An item no list holds.
            let folder = tx.create("Item", json!({ "id": 900, "kind": "folder", "name": "Loose" }).as_object().cloned().unwrap())?;
            for (key, value) in tx.map_entries(&main, "attrs")? {
                tx.map_set(&folder, "attrs", &key, value)?;
            }
            // A second listing of `main`.
            let len = tx.list_items(&root(), "items")?.len();
            tx.list_insert(&root(), "items", len, &main)?;
            // A deleted item still listed (`Inner` is used by no layer).
            tx.delete(&inner)?;
            Ok(folder)
        })
        .unwrap();
    let changes = raw.export_changes(&since).unwrap();
    a.import_changes(&changes).unwrap();
    b.import_changes(&changes).unwrap();
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    for session in [&a, &b] {
        let listed = items(session.replica());
        assert!(listed.contains(&unlisted), "the loose item is listed");
        assert_eq!(listed.iter().filter(|o| **o == main).count(), 1, "main once");
        assert!(!listed.contains(&inner), "the deleted item is unlisted");
        assert!(listed.iter().all(|o| session.replica().exists(o)));
        assert_eq!(listed.len(), session.replica().objects_of_type("Item").len());
        assert_eq!(listed.len(), session.model().project.items.len());
        assert_eq!(session.model().project.items[&kition_motion_project::ItemId(900)].name, "Loose");
    }
}

/// No migration: a replica made on an earlier schema (here a version 0 without `Project.items`)
/// is refused with an error that says what to do, never read as an empty project. Its room is
/// re-imported from the `.kmotion`.
#[test]
fn an_earlier_schema_replica_is_refused_not_read_empty() {
    let text = include_str!("../model/schema.json")
        .replace("\"schemaVersion\": 1", "\"schemaVersion\": 0")
        .replace(",\n        \"items\": { \"kind\": \"list\", \"of\": \"Item\", \"sample\": false }", "")
        .replace("\n        \"name\": { \"kind\": \"string\" },", "");
    let old = std::sync::Arc::new(Schema::parse(&text).unwrap());
    assert!(!old.types["Project"].fields.contains_key("items"), "the old schema has no items list");
    assert!(!old.types["Item"].fields.contains_key("name") && !old.types["Layer"].fields.contains_key("name"), "nor name fields");
    let (replica, ()) = Replica::create_with(old, DocId::new(), actor(1), &Origin::system("create"), |tx| {
        tx.create_with_id("Project", &root(), serde_json::Map::new())?;
        tx.create("Item", json!({ "id": 1, "kind": "folder" }).as_object().cloned().unwrap())?;
        Ok::<(), KdmError>(())
    })
    .unwrap();
    let error = MotionSession::open_snapshot(&replica.snapshot().unwrap()).unwrap_err().to_string();
    assert!(error.contains("schema 0") && error.contains(".kmotion"), "{error}");
}

#[test]
fn every_item_stays_listed_through_random_whole_project_writes() {
    let d = full_demo();
    let mut a = first(&d.project);
    let mut p = a.model().project.clone();
    for n in 0..6 {
        let item = solid(&mut p, &format!("S{n}"), [0.1, 0.1, 0.1]);
        add_layer(&mut p, d.inner, &format!("L{n}"), LayerSource::Solid { item });
        if n % 2 == 1 {
            p.items.remove(&item);
            if let Some(comp) = p.comp_mut(d.inner) {
                comp.layers.retain(|l| l.source.item() != Some(item));
            }
        }
        record(&mut a, &person("a"), &p);
        let listed = items(a.replica());
        assert_eq!(listed.len(), p.items.len(), "step {n}");
        assert_eq!(reachable(a.replica()).len(), live_objects(a.replica()), "step {n}: nothing unreachable");
    }
}

/// How many live objects the replica holds, of every type.
fn live_objects(replica: &Replica) -> usize {
    replica.schema().types.keys().map(|ty| replica.objects_of_type(ty).len()).sum()
}
