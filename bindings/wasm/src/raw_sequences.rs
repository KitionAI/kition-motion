//! Raw container edits guard the engine's retained list identities, not serialized positions.
use kition_kit::{Covered, CoveredAsk, CoveredAt, Covers, DerivedValues, Premise, Under};
use kition_motion_engine::Session;
use serde_json::{Value, json};
const LIMIT: usize = 65_536;
fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned).or_else(|| value.as_u64().map(|id| id.to_string()))
}
fn wanted(ids: &[Value], id: u64) -> bool {
    ids.is_empty() || ids.iter().any(|value| text(value).and_then(|value| value.parse::<u64>().ok()) == Some(id))
}

pub(crate) fn derived(session: &Session, ids: &[Value], names: &[String]) -> Result<Vec<DerivedValues>, String> {
    let named = |name: &str| names.is_empty() || names.iter().any(|given| given == name);
    if !named("nodeOrder") && !named("layerOrder") {
        return Ok(Vec::new());
    }
    let Some(replica) = session.kdm() else { return Ok(Vec::new()) };
    let mut answer = Vec::new();
    let mut budget = LIMIT;
    for item in session.project.items.values() {
        let Some(comp) = item.as_comp() else { continue };
        if named("layerOrder") && wanted(ids, item.id.0) {
            budget = budget.checked_sub(comp.layers.len().saturating_add(1)).ok_or("too many layer identities to retain; split the project")?;
            answer.push(DerivedValues {
                id: json!(item.id.0),
                values: serde_json::Map::from_iter([("layerOrder".to_owned(), json!(comp.layers.iter().map(|layer| layer.id.0).collect::<Vec<_>>()))]),
            });
        }
        if !named("nodeOrder") {
            continue;
        }
        for layer in &comp.layers {
            if !wanted(ids, layer.id.0) {
                continue;
            }
            let Some(objects) = replica.ids().layers.get(&kdm_model::LayerKey { id: layer.id, nth: 0 }) else { continue };
            budget = budget.checked_sub(objects.nodes.len().saturating_add(1)).ok_or("too many property identities to retain; split the project")?;
            let mut orders = serde_json::Map::new();
            for (node, object) in &objects.nodes {
                if node.nth != 0 {
                    return Err("property insertion requires unique property UIDs".to_owned());
                }
                let children = replica.replica().list(object, "children").map_err(|error| error.to_string())?;
                if children.is_empty() {
                    continue;
                }
                budget = budget.checked_sub(children.len()).ok_or("too many property identities to retain; split the project")?;
                let children: Vec<_> = children.iter().map(|id| json!(id.as_str())).collect();
                orders.insert(node.uid.to_string(), crate::raw_keys::pack(&children).ok_or("property identities cannot be retained")?);
            }
            answer.push(DerivedValues { id: json!(layer.id.0), values: serde_json::Map::from_iter([("nodeOrder".to_owned(), json!(orders))]) });
        }
    }
    Ok(answer)
}
struct Address {
    carrier: Value,
    comp: Value,
    owner: Value,
    order: Vec<Value>,
    from: Option<usize>,
    to: usize,
    uid: Option<u64>,
}
fn address(session: &Session, command: &str, params: &Value) -> Result<Option<Address>, String> {
    if !matches!(command, "model.layerMove" | "model.nodeCreate") {
        return Ok(None);
    }
    let index = |name| {
        params.get(name).and_then(Value::as_u64).and_then(|number| usize::try_from(number).ok()).ok_or_else(|| format!("raw container edit requires `{name}`"))
    };
    let replica = session.kdm().ok_or("raw container edit requires the live replica")?;
    if command == "model.layerMove" {
        let owner = params.get("target").ok_or("raw layer move needs its composition reference")?;
        let (cid, object) =
            replica.ids().comps.iter().find(|(_, object)| owner.as_str() == Some(object.as_str())).ok_or("raw layer move requires a retained composition")?;
        let comp = session.project.comp(*cid).ok_or("raw layer move composition is missing")?;
        if comp.layers.len() > LIMIT {
            return Err("too many layers to guard; split the composition".to_owned());
        }
        return Ok(Some(Address {
            carrier: json!(cid.0),
            comp: json!(cid.0),
            owner: json!(object.as_str()),
            order: comp.layers.iter().map(|layer| json!(layer.id.0)).collect(),
            from: Some(index("from")?),
            to: index("to")?,
            uid: None,
        }));
    }
    let Some(owner) = params.get("in").and_then(Value::as_str).and_then(|place| place.strip_suffix(".children")) else {
        return Err("raw property creation requires its retained owning list".to_owned());
    };
    for (key, objects) in &replica.ids().layers {
        if let Some((node, object)) = objects.nodes.iter().find(|(_, object)| object.as_str() == owner) {
            if key.nth != 0 || node.nth != 0 || objects.nodes.len() > LIMIT {
                return Err("raw property creation requires bounded, unique property UIDs".to_owned());
            }
            let cid = session
                .project
                .items
                .values()
                .find(|item| item.as_comp().is_some_and(|comp| comp.layer(key.id).is_some()))
                .map(|item| item.id)
                .ok_or("raw property creation has no owning composition")?;
            if replica.replica().get(object, "node").map_err(|error| error.to_string())?.as_str() != Some("group") {
                return Err("raw property creation requires a group as its children owner".to_owned());
            }
            let children = replica.replica().list(object, "children").map_err(|error| error.to_string())?;
            if children.len() > LIMIT {
                return Err("too many property children to guard; split the group".to_owned());
            }
            return Ok(Some(Address {
                carrier: json!(key.id.0),
                comp: json!(cid.0),
                owner: json!(owner),
                order: children.iter().map(|id| json!(id.as_str())).collect(),
                from: None,
                to: index("at")?,
                uid: Some(node.uid),
            }));
        }
    }
    Err("raw property creation requires a retained owner".to_owned())
}
fn occupants(address: &Address, order: &[Value], comp: Value) -> Vec<Value> {
    let mut objects = vec![comp];
    if let Some(from) = address.from {
        objects.extend(order.get(from).cloned());
    }
    objects.extend(address.to.checked_sub(1).and_then(|left| order.get(left)).cloned());
    objects.extend(order.get(address.to).cloned());
    objects.sort_by_key(Value::to_string);
    objects.dedup();
    objects
}
pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Result<Option<Under>, String> {
    let Some(address) = address(session, command, params)? else { return Ok(None) };
    let fact = if address.uid.is_some() { "~nodeOrder" } else { "~layerOrder" };
    let fields = if address.uid.is_some() { vec!["~container".to_owned(), fact.to_owned()] } else { vec![fact.to_owned()] };
    let objects = occupants(&address, &address.order, address.comp.clone());
    let mut answer = Under { objects: objects.clone(), covers: Some(Covers { fields }), ..Default::default() };
    for id in objects.into_iter().chain([address.carrier]) {
        answer.premises.push(Premise::on(id).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
    }
    let kind: Vec<&str> = if address.uid.is_some() { vec!["node"] } else { Vec::new() };
    answer.premises.push(Premise::on(address.owner).fields(kind).positions(Vec::<String>::new()));
    crate::place::merge_premises(&mut answer);
    Ok(Some(answer))
}
pub(crate) fn covered(session: &Session, command: &str, params: &Value, objects: &[CoveredAsk]) -> Option<CoveredAt> {
    let address = address(session, command, params).ok()??;
    let fact = if address.uid.is_some() { "~nodeOrder" } else { "~layerOrder" };
    let mut order = address.order.clone();
    let mut comp = address.comp.clone();
    let mut covered = Vec::new();
    for object in objects {
        let same = text(&object.id) == text(&address.carrier);
        if same {
            if object.unknown.iter().any(|field| field == fact || field == "~container") {
                return None;
            }
            if let Some(before) = object.before.get("~container") {
                comp = before.clone();
            }
            if let Some(before) = object.before.get(fact) {
                order = if let Some(uid) = address.uid {
                    match before.get(uid.to_string()) {
                        Some(order) => crate::raw_keys::unpack(order)?,
                        None => Vec::new(),
                    }
                } else {
                    let before = before.as_array()?;
                    if before.len() > LIMIT {
                        return None;
                    }
                    before.clone()
                };
                if order.len() > LIMIT {
                    return None;
                }
            }
        }
        covered.push(Covered { id: object.id.clone(), covered: Some(same), ..Default::default() });
    }
    Some(CoveredAt { under: occupants(&address, &order, comp), covered })
}
