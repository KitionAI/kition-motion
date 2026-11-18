//! Validate object-model writes before importing them into the live document.
use kition_kit::kdm::{ObjectId, Replica};
use serde_json::Value;

use crate::{KdmResult, kdm_err};

pub fn check_model_edits(before: &Replica, after: &Replica) -> KdmResult<()> {
    let root = crate::project_id();
    let listed = before.list(&root, "items")?;
    let wanted: Vec<_> = listed.iter().filter(|item| after.exists(item)).cloned().collect();
    let actual = after.list(&root, "items")?;
    // New objects are listed by the repair, after their engine ids are assigned. Only the
    // removal of deleted objects can change this query view through a batch.
    let surviving: Vec<_> = actual.iter().filter(|item| before.exists(item)).cloned().collect();
    if surviving != wanted {
        return Err(kdm_err("Project.items has no manual order; create or delete items, or use project.move to change their folder"));
    }
    for node in after.objects_of_type("Node") {
        if after.get(&node, "node")?.as_str() != Some("prop") {
            continue;
        }
        if !after.get(&node, "enabled")?.is_null() {
            return Err(kdm_err("enabled belongs to a property group, not a property"));
        }
        let meta = after.get(&node, "meta")?;
        // These are the engine's declared hard limits, not its narrower slider range.
        // Percent alone is insufficient: scale, offset and intensity can exceed 100.
        let ui = &meta["ui"];
        if ui["kind"] != "Slider" || ui["min"].as_f64() != Some(0.0) || ui["max"].as_f64() != Some(100.0) {
            continue;
        }
        check_percentage(before, after, &node, &meta)?;
        for key in after.list(&node, "keys")? {
            if after.exists(&key) {
                check_percentage(before, after, &key, &meta)?;
            }
        }
    }
    Ok(())
}

fn check_percentage(before: &Replica, after: &Replica, object: &ObjectId, meta: &Value) -> KdmResult<()> {
    let value = after.get(object, "value")?;
    if before.exists(object) && before.get(object, "value")? == value {
        return Ok(());
    }
    if let Some(number) = value.get("v").and_then(Value::as_f64)
        && !(0.0..=100.0).contains(&number)
    {
        return Err(kdm_err(format!("{} must be between 0 and 100", meta["name"].as_str().unwrap_or("percentage"))));
    }
    Ok(())
}
