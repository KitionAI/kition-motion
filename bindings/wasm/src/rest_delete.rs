//! Item deletion consumes erased content and the references it clears throughout the project.
use kition_kit::{Premise, Under, Within};
use kition_motion_engine::Session;
use kition_motion_project::ItemId;
use serde_json::{Value, json};
use std::collections::HashSet;

fn fields(id: Value, names: &[&str]) -> Premise {
    Premise::on(id).fields(names.iter().copied()).positions(Vec::<String>::new())
}
fn limit(session: &Session) -> Result<(), String> {
    let mut count = session.project.items.len().saturating_add(session.project.render_queue.len());
    for item in session.project.items.values() {
        if let Some(comp) = item.as_comp() {
            count = count.saturating_add(comp.layers.len());
        }
    }
    if count > 65_536 {
        return Err("too many item-deletion dependencies; split or simplify the project".to_owned());
    }
    Ok(())
}
fn within(session: &Session, start: ItemId, parent: ItemId) -> bool {
    let mut next = Some(start);
    // Match the engine's bounded folder walk, including its conservative cycle repair.
    for _ in 0..=256 {
        let Some(id) = next else { return false };
        if id == parent {
            return true;
        }
        next = session.project.item(id).and_then(|item| item.parent);
    }
    true
}
fn resolve(session: &Session, value: &Value) -> Option<ItemId> {
    match value {
        Value::Number(number) => number.as_u64().map(ItemId).filter(|id| session.project.item(*id).is_some()),
        Value::String(name) => session.project.find_by_name(name).map(|item| item.id),
        _ => None,
    }
}
pub(crate) fn check(session: &Session, command: &str, params: &Value) -> Result<(), String> {
    if matches!(command, "project.delete" | "model.itemDelete") {
        limit(session)?;
        if params.get("items").and_then(Value::as_array).is_some_and(|ids| ids.len() > 65_536) {
            return Err("too many item-deletion targets; split the selection".to_owned());
        }
    }
    Ok(())
}
pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Result<Option<Under>, String> {
    if !matches!(command, "project.delete" | "model.itemDelete") {
        return Ok(None);
    }
    check(session, command, params)?;
    let raw = command == "model.itemDelete";
    let ids: Vec<ItemId> = if raw {
        let target = params.get("target").ok_or("raw item deletion requires its target")?;
        let id = target
            .as_u64()
            .or_else(|| target.as_str()?.parse().ok())
            .map(ItemId)
            .filter(|id| session.project.item(*id).is_some())
            .or_else(|| session.kdm()?.ids().items.iter().find(|(_, id)| target.as_str() == Some(id.as_str())).map(|(id, _)| *id));
        vec![id.ok_or("raw item deletion requires a retained item")?]
    } else if let Some(values) = params.get("items").and_then(Value::as_array) {
        values.iter().map(|value| resolve(session, value).ok_or("item deletion target does not resolve")).collect::<Result<_, _>>()?
    } else if let Some(value) = params.get("item") {
        vec![resolve(session, value).ok_or("item deletion target does not resolve")?]
    } else {
        session.state.project_selection.clone()
    };
    let doomed: HashSet<_> =
        session.project.items.keys().copied().filter(|id| if raw { ids.contains(id) } else { ids.iter().any(|root| within(session, *id, *root)) }).collect();
    let mut answer = Under::default();
    for (field, len) in [("items", session.project.items.len()), ("renderQueue", session.project.render_queue.len())] {
        answer.within.push(Within { parent: json!("project"), field: field.to_owned(), from: 0, to: len.saturating_add(1), ..Default::default() });
    }
    for item in session.project.items.values() {
        if doomed.contains(&item.id) {
            answer.premises.push(Premise::on(json!(item.id.0)).positions(Vec::<String>::new()));
        } else {
            answer.premises.push(fields(json!(item.id.0), &["kind", "parent"]));
        }
        if let Some(comp) = item.as_comp() {
            answer.premises.push(fields(json!(item.id.0), &["~layerMembers"]));
            for layer in &comp.layers {
                if doomed.contains(&item.id) || layer.source.item().is_some_and(|id| doomed.contains(&id)) {
                    answer.premises.push(Premise::on(json!(layer.id.0)));
                } else {
                    answer.premises.push(fields(json!(layer.id.0), &["attrs.source", "sourceItem", "parent", "matte", "matteLayer"]));
                }
            }
        }
    }
    for item in &session.project.render_queue {
        let id = format!("render:{}", item.id);
        answer.premises.push(if doomed.contains(&item.comp) { Premise::on(id) } else { fields(json!(id), &["item.comp"]) });
    }
    crate::place::merge_premises(&mut answer);
    Ok(Some(answer))
}
