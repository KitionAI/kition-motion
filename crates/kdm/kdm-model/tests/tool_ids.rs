//! Tool ids (KitObjectModel OM9, task card C8-motion): every item and layer holds the id the
//! engine gave it (the `.kmotion` file's id) in its read-only `id` field, which the schema names
//! as the type's tool id; a comp's composition and a layer's property tree are held by refs
//! declared `owns`, so a change inside them is told under the item or layer. The repair after a
//! draft's merge numbers an item or layer that has no id (a batch created it) or one another item
//! or layer of the project holds.

mod common;

use common::{demo, draft, first, merge, person};
use kition_kit::kdm::ObjectId;
use serde_json::{Map, Value, json};

/// The schema names `id` as the tool id of items and layers, read-only and optional (a batch may
/// create either without it), and the composition and property tree as owned.
#[test]
fn the_schema_declares_tool_ids_and_what_items_and_layers_own() {
    let schema: Value = serde_json::from_str(include_str!("../model/schema.json")).unwrap();
    for ty in ["Item", "Layer"] {
        assert_eq!(schema["types"][ty]["toolId"], json!({ "field": "id" }), "{ty}");
        assert_eq!(schema["types"][ty]["fields"]["id"], json!({ "kind": "integer", "optional": true, "readOnly": true }), "{ty}");
    }
    assert_eq!(schema["types"]["Item"]["fields"]["comp"]["owns"], true);
    assert_eq!(schema["types"]["Layer"]["fields"]["props"]["owns"], true);
    assert!(schema["types"]["Node"].get("toolId").is_none(), "properties are told under their layer");
    let parsed = <kdm_model::MotionModel as kition_kit::session::KitModel>::schema();
    assert_eq!(parsed.schema_version, 1, "project membership is a query view without manual order in schema 1");
}

/// A layer a draft's batch created without an id is given one by the repair when the draft
/// merges, and the engine reads the layer with it.
#[test]
fn a_layer_a_draft_created_without_an_id_is_numbered_when_it_merges() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let layer = a.replica().objects_of_type("Layer").into_iter().next().unwrap();
    let comp = a.replica().objects_of_type("Comp").into_iter().next().unwrap();
    let mut values = Map::new();
    for field in ["name", "startTime", "inPoint", "outPoint", "attrs", "sourceItem"] {
        values.insert(field.to_owned(), a.replica().get(&layer, field).unwrap_or(Value::Null));
    }
    values.remove("attrs");
    let attrs = a.replica().map_entries(&layer, "attrs").unwrap();
    let root = ObjectId::from_string(a.replica().get(&layer, "props").unwrap().as_str().unwrap());
    let mut node = Map::new();
    for field in ["uid", "node", "meta"] {
        node.insert(field.to_owned(), a.replica().get(&root, field).unwrap());
    }
    b.record(&person("b"), |tx| {
        // A property tree of its own: the copied layer's root group, empty.
        let tree = tx.create("Node", node.clone())?;
        let mut values = values.clone();
        values.insert("props".to_owned(), json!(tree.as_str()));
        let made = tx.create("Layer", values)?;
        for (key, value) in &attrs {
            tx.map_set(&made, "attrs", key, value.clone())?;
        }
        tx.list_insert(&comp, "layers", 0, &made)?;
        Ok(())
    })
    .unwrap();
    merge(&mut a, &b, &base);
    let made: Vec<ObjectId> = a.replica().objects_of_type("Layer");
    assert_eq!(made.len(), b.replica().objects_of_type("Layer").len(), "the draft's layer merged");
    let mut stored: Vec<u64> = made.iter().map(|object| a.replica().get(object, "id").unwrap().as_u64().expect("every layer numbered")).collect();
    stored.sort_unstable();
    let mut unique = stored.clone();
    unique.dedup();
    assert_eq!(unique, stored, "every layer has its own id");
}

/// A layer whose id an item of the project holds (a draft's batch that numbered it so) is given a
/// repair id when the draft merges: items and layers share one id space.
#[test]
fn a_layer_holding_an_items_id_is_given_another() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let item = a.replica().objects_of_type("Item").into_iter().next().unwrap();
    let item_id = a.replica().get(&item, "id").unwrap();
    let layer = a.replica().objects_of_type("Layer").into_iter().next().unwrap();
    b.record(&person("b"), |tx| Ok(tx.set(&layer, "id", item_id.clone())?)).unwrap();
    merge(&mut a, &b, &base);
    let now = a.replica().get(&layer, "id").unwrap();
    assert_ne!(now, item_id, "the layer was given an id of its own");
    assert!(now.as_u64().is_some(), "{now}");
}
