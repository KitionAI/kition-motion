//! Effect ordinals name nested resources. Retain their actual replica identities privately
//! and replay occupancy through coverage, including deleted earlier recipients.
use kition_kit::{Covered, CoveredAsk, CoveredAt, Covers, DerivedValues, Premise, Under};
use kition_motion_engine::Session;
use kition_motion_project::{Layer, LayerId};
use serde_json::{Value, json};

fn order(session: &Session, layer: &Layer) -> Option<Vec<Value>> {
    let effects = layer.effects()?;
    if effects.children.len() > 65_536 {
        return None;
    }
    let replica = session.kdm()?;
    effects.groups().map(|effect| Some(json!(replica.ids().node(layer.id, effect.uid)?.as_str()))).collect()
}

pub(crate) fn derived(session: &Session, ids: &[Value], names: &[String]) -> Vec<DerivedValues> {
    if !names.is_empty() && !names.iter().any(|name| name == "effectOrder") {
        return Vec::new();
    }
    session
        .project
        .items
        .values()
        .filter_map(|item| item.as_comp())
        .flat_map(|comp| &comp.layers)
        .filter(|layer| ids.is_empty() || ids.iter().any(|id| id.as_u64().or_else(|| id.as_str()?.parse().ok()) == Some(layer.id.0)))
        .map(|layer| DerivedValues {
            id: json!(layer.id.0),
            values: serde_json::Map::from_iter([("effectOrder".to_owned(), order(session, layer).map_or(Value::Null, |order| json!(order)))]),
        })
        .collect()
}

fn target<'a>(session: &'a Session, command: &str, params: &Value) -> Option<(&'a Layer, usize)> {
    if command != "effect.remove" {
        return None;
    }
    let number = params.get("effect")?.as_u64()?;
    let id = params.get("layer")?.as_u64()?;
    let layer = session.project.items.values().filter_map(|item| item.as_comp()).find_map(|comp| comp.layer(LayerId(id)))?;
    if layer.effects()?.groups().any(|effect| effect.uid == number) {
        return None;
    }
    Some((layer, usize::try_from(number).ok()?.checked_sub(1)?))
}

pub(crate) fn extend(session: &Session, command: &str, params: &Value, answer: &mut Option<Under>) {
    let Some((layer, index)) = target(session, command, params) else { return };
    let answer = answer.get_or_insert_with(Under::default);
    if let Some(recipient) = order(session, layer).and_then(|order| order.get(index).cloned()) {
        answer.objects.push(recipient.clone());
        // Its place within the effect list matters; movement of the containing layer does not.
        answer.premises.push(Premise::on(recipient).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
    }
    answer.covers = Some(Covers { fields: vec!["~effectOrder".to_owned()] });
}

pub(crate) fn covered(session: &Session, command: &str, params: &Value, objects: &[CoveredAsk]) -> Option<CoveredAt> {
    let (layer, index) = target(session, command, params)?;
    let current = order(session, layer)?;
    let mut under = current.get(index).cloned().into_iter().collect::<Vec<_>>();
    let mut covered = Vec::new();
    for object in objects {
        let same = object.id.as_u64().or_else(|| object.id.as_str()?.parse().ok()) == Some(layer.id.0);
        if same {
            if object.unknown.iter().any(|field| field == "~effectOrder") {
                return None;
            }
            if let Some(before) = object.before.get("~effectOrder") {
                under = before.as_array()?.get(index).cloned().into_iter().collect();
            }
        }
        // A layer carries the nested effect. If its occupant changed, the SDK reports the
        // layer along with the departed/current resource as relocated.
        covered.push(Covered { id: object.id.clone(), covered: Some(same), ..Default::default() });
    }
    Some(CoveredAt { under, covered })
}
