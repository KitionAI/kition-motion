//! File precision is settled inside the batch transaction, before its receipt is calculated.
use kition_kit::kdm::Tx;
use kition_motion_keyframe::Value as PropertyValue;
use serde_json::Value;

use crate::{KdmResult, kdm_err};

pub fn settle_batch(tx: &mut Tx<'_>) -> KdmResult<()> {
    let mut created_item = false;
    for object in tx.objects_of_type("Item") {
        if tx.get(&object, "id")?.is_null() {
            if tx.get(&object, "kind")?.as_str() == Some("comp")
                && !tx.get(&object, "comp")?.as_str().is_some_and(|id| tx.exists(&kition_kit::kdm::ObjectId::from_string(id)))
            {
                return Err(kdm_err("creating a comp Item requires its owned `comp` reference to a live Comp object"));
            }
            tx.set(&object, "id", serde_json::json!(crate::repair_id(&object)))?;
            created_item = true;
        }
        if tx.map_get(&object, "attrs", "label")?.is_null() {
            tx.map_set(&object, "attrs", "label", serde_json::json!("None"))?;
        }
        if tx.map_get(&object, "attrs", "comment")?.is_null() {
            tx.map_set(&object, "attrs", "comment", serde_json::json!(""))?;
        }
    }
    if created_item {
        let root = crate::project_id();
        let mut items = tx.list_items(&root, "items")?;
        let mut keyed = Vec::with_capacity(items.len());
        for object in items.drain(..) {
            keyed.push((tx.get(&object, "id")?.as_u64().ok_or_else(|| kdm_err("an item's id is not a whole number"))?, object));
        }
        keyed.sort();
        crate::list::align(tx, &root, "items", &keyed.into_iter().map(|(_, object)| object).collect::<Vec<_>>())?;
    }
    for (ty, field, keys) in [("Comp", "attrs", &["background"][..]), ("Item", "data", &["color", "premul_color"][..])] {
        for object in tx.objects_of_type(ty) {
            for key in keys {
                let before = tx.map_get(&object, field, key)?;
                if before.is_null() {
                    continue;
                }
                let color: [f32; 3] = serde_json::from_value(before.clone()).map_err(kdm_err)?;
                if color.iter().any(|channel| !channel.is_finite()) {
                    return Err(kdm_err("color is outside the stored floating-point range"));
                }
                let after = serde_json::to_value(color).map_err(kdm_err)?;
                if before != after {
                    tx.map_set(&object, field, key, after)?;
                }
            }
        }
    }
    for ty in ["Node", "Key"] {
        for object in tx.objects_of_type(ty) {
            if ty == "Node" {
                let node = tx.get(&object, "node")?;
                let forbidden = match node.as_str() {
                    Some("prop") => "children",
                    Some("group") => "keys",
                    _ => return Err(kdm_err("a property-tree node must be a property or a group")),
                };
                if !tx.list_items(&object, forbidden)?.is_empty() {
                    return Err(kdm_err(format!(
                        "a {} node cannot own `{forbidden}`: groups own children and properties own keys",
                        node.as_str().unwrap_or_default()
                    )));
                }
            }
            let before = tx.get(&object, "value")?;
            if before == Value::Null {
                continue;
            }
            let value: PropertyValue = serde_json::from_value(before.clone()).map_err(kdm_err)?;
            let after = serde_json::to_value(value).map_err(kdm_err)?;
            if before != after {
                tx.set(&object, "value", after)?;
            }
        }
    }
    Ok(())
}
