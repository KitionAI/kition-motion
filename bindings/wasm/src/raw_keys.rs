//! Raw key contracts use the typed owning Node.keys list. The SDK maps raw operations to
//! these premise hooks; execution still uses the original replica operation and settler.
use kition_kit::{Covered, CoveredAsk, CoveredAt, Covers, DerivedValues, Premise, Under, Within};
use kition_motion_engine::Session;
use kition_motion_project::{ItemId, Layer};
use serde_json::{Value, json};

const LIMIT: usize = 65_536;
fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned).or_else(|| value.as_u64().map(|n| n.to_string()))
}
fn requested(ids: &[Value], id: &Value) -> bool {
    ids.is_empty() || ids.iter().any(|value| text(value) == text(id))
}

// Prefix compression is exact, with no hash or generated identity. It keeps the ordinary
// small key lists' coverage history within the SDK's eager-fact retention bound.
pub(crate) fn pack(keys: &[Value]) -> Option<Value> {
    let first = keys.first()?.as_str()?;
    let mut prefix = first.to_owned();
    for key in keys {
        let key = key.as_str()?;
        while !key.starts_with(&prefix) {
            prefix.pop()?;
        }
    }
    let suffixes: Option<Vec<_>> = keys.iter().map(|key| Some(json!(key.as_str()?.strip_prefix(&prefix)?))).collect();
    Some(json!([prefix, suffixes?]))
}
pub(crate) fn unpack(value: &Value) -> Option<Vec<Value>> {
    let prefix = value.get(0)?.as_str()?;
    let suffixes = value.get(1)?.as_array()?;
    if suffixes.len() > LIMIT {
        return None;
    }
    suffixes.iter().map(|suffix| Some(json!(format!("{prefix}{}", suffix.as_str()?)))).collect()
}

pub(crate) fn derived(session: &Session, ids: &[Value], names: &[String]) -> Result<Vec<DerivedValues>, String> {
    let named = |name: &str| names.is_empty() || names.iter().any(|given| given == name);
    if !named("container") && !named("keyOrder") {
        return Ok(Vec::new());
    }
    let Some(replica) = session.kdm() else { return Ok(Vec::new()) };
    let mut answer = Vec::new();
    let mut budget = LIMIT;
    for item in session.project.items.values() {
        let Some(comp) = item.as_comp() else { continue };
        for layer in &comp.layers {
            if !requested(ids, &json!(layer.id.0)) {
                continue;
            }
            budget = budget.checked_sub(1).ok_or("too many raw key facts to retain; split the project")?;
            let mut values = serde_json::Map::new();
            if named("container") {
                values.insert("container".to_owned(), json!(item.id.0));
            }
            if named("keyOrder") {
                let mut orders = serde_json::Map::new();
                if let Some(objects) = replica.ids().layers.get(&kdm_model::LayerKey { id: layer.id, nth: 0 }) {
                    for (node, keys) in &objects.keys {
                        if keys.is_empty() {
                            continue;
                        }
                        budget = budget.checked_sub(keys.len()).ok_or("too many raw key facts to retain; split the project")?;
                        let keys: Vec<_> = keys.iter().map(|key| json!(key.as_str())).collect();
                        let packed = pack(&keys).ok_or("raw key identity cannot be retained")?;
                        orders.insert(node.uid.to_string(), packed);
                    }
                }
                values.insert("keyOrder".to_owned(), json!(orders));
            }
            answer.push(DerivedValues { id: json!(layer.id.0), values });
        }
    }
    Ok(answer)
}

pub(crate) fn content(ask: &kition_kit::ContentAsk) -> Option<bool> {
    if !matches!(ask.premise.field.as_str(), "~rawValue" | "~rawRecord") || !ask.premise.region.is_null() {
        return None;
    }
    let key = text(&ask.premise.id)?;
    let transitions = ask.before.get("history")?.as_array()?;
    for transition in transitions {
        let before = transition.get("before")?.get("objects")?.get(&key);
        let after = transition.get("after")?.get("objects")?.get(&key);
        // A key no outside checkpoint holds was created later, by this run's own earlier step.
        let (Some(before), Some(after)) = (before, after) else {
            if before.is_none() && after.is_none() {
                continue;
            }
            return None;
        };
        for field in if ask.premise.field == "~rawValue" { &["value"][..] } else { &["time", "value", "data"][..] } {
            if before.get(*field)? != after.get(*field)? {
                return Some(true);
            }
        }
    }
    Some(false)
}

struct Address<'a> {
    layer: &'a Layer,
    comp: ItemId,
    uid: u64,
    owner: String,
    keys: Vec<Value>,
    from: Option<usize>,
    to: Option<usize>,
    value: Option<Value>,
}
fn address<'a>(session: &'a Session, command: &str, params: &Value) -> Result<Option<Address<'a>>, String> {
    let (source, insertion) = match command {
        "model.keyRemove" => (true, false),
        "model.keyCreate" | "model.keyInsert" => (false, true),
        "model.keyMove" => (true, true),
        "model.keyValue" => (false, false),
        _ => return Ok(None),
    };
    let location = params.get("in").and_then(Value::as_str).and_then(|place| place.strip_suffix(".keys"));
    let Some(target) = params.get("target").and_then(Value::as_str).or(location) else { return Ok(None) };
    let index = |name: &str| {
        params.get(name).and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok()).ok_or_else(|| format!("raw key operation requires `{name}` as an index"))
    };
    let from = if command == "model.keyMove" {
        Some(index("from")?)
    } else if source {
        Some(index("index")?)
    } else {
        None
    };
    let to = if command == "model.keyMove" {
        Some(index("to")?)
    } else if command == "model.keyCreate" {
        Some(index("at")?)
    } else if insertion {
        Some(index("index")?)
    } else {
        None
    };
    let replica = session.kdm().ok_or("raw key premise requires the live replica")?;
    for (layer, objects) in &replica.ids().layers {
        for (node, object) in &objects.nodes {
            let keys = replica.ids().keys(layer.id, node.uid);
            let by_value = command == "model.keyValue" && keys.iter().any(|key| key.as_str() == target);
            if object.as_str() != target && !by_value {
                continue;
            }
            if keys.len() > LIMIT {
                return Err("too many keys to guard a structural operation; split the property".to_owned());
            }
            let (cid, layer) = session
                .project
                .items
                .values()
                .filter_map(|item| Some((item.id, item.as_comp()?.layer(layer.id)?)))
                .next()
                .ok_or("raw key owner has no layer")?;
            return Ok(Some(Address {
                layer,
                comp: cid,
                uid: node.uid,
                owner: object.as_str().to_owned(),
                keys: keys.iter().map(|key| json!(key.as_str())).collect(),
                from,
                to,
                value: by_value.then(|| json!(target)),
            }));
        }
    }
    Err("raw key operation names no retained property".to_owned())
}

fn occupants(address: &Address<'_>, keys: &[Value], comp: Value) -> Vec<Value> {
    let mut objects = vec![comp];
    if let Some(value) = &address.value {
        objects.push(value.clone());
        return objects;
    }
    if let Some(from) = address.from {
        objects.extend(keys.get(from).cloned());
    }
    if let Some(to) = address.to {
        objects.extend(to.checked_sub(1).and_then(|left| keys.get(left)).cloned());
        objects.extend(keys.get(to).cloned());
    }
    objects.sort_by_key(Value::to_string);
    objects.dedup();
    objects
}

pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Result<Option<Under>, String> {
    if matches!(command, "model.compAttrs" | "model.layerAttrs") {
        return attrs_under(session, command, params);
    }
    if !command.starts_with("model.key") {
        return Ok(None);
    }
    let Some(target) = address(session, command, params)? else {
        if command == "model.keyCreate" && params.get("in").is_none() {
            return Ok(Some(Under {
                premises: vec![Premise::on("project").fields(Vec::<String>::new()).positions(Vec::<String>::new())],
                ..Default::default()
            }));
        }
        return Err("raw key operation requires a retained owner".to_owned());
    };
    let objects = occupants(&target, &target.keys, json!(target.comp.0));
    let mut answer =
        Under { objects: objects.clone(), covers: Some(Covers { fields: vec!["~container".to_owned(), "~keyOrder".to_owned()] }), ..Default::default() };
    for object in objects {
        answer.premises.push(Premise::on(object).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
    }
    answer.premises.push(Premise::on(target.owner.clone()).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
    if let Some(key) = &target.value {
        answer.premises.push(Premise::on(key.clone()).fields(["~rawValue"]).positions(Vec::<String>::new()));
    } else {
        let property = target.layer.props.find(target.uid).ok_or("raw key owner is not a property")?;
        let path = target.layer.props.match_path_of(target.uid).ok_or("raw key owner has no property path")?;
        let consumed = target.from.or_else(|| target.to?.checked_sub(1));
        if let Some(key) = consumed.and_then(|index| property.keys.get(index)) {
            answer.premises.push(Premise::on(json!(target.layer.id.0)).fields([format!("{path}.keys[time={}]", key.time.0)]).positions(Vec::<String>::new()));
        }
    }
    if let Some(to) = target.to {
        answer.within.push(Within { parent: json!(target.owner), field: "keys".to_owned(), from: to, to, ..Default::default() });
    }
    Ok(Some(answer))
}

/// The SDK supplies the actual isolated prefix changes. A key created by that prefix has no
/// outside basis to compare; an existing inserted key must keep all its stored content.
pub(crate) fn prefix(command: &str, params: &Value, step: &kition_kit::PremiseStep, answer: &mut Option<Under>) {
    if command != "model.keyInsert" {
        return;
    }
    let Some(item) = params.get("item") else { return };
    let created = step.changes.get("created").and_then(Value::as_array).is_some_and(|objects| objects.iter().any(|object| text(&object["id"]) == text(item)));
    let Some(answer) = answer.as_mut() else { return };
    let fields = if created { Vec::new() } else { vec!["~rawRecord".to_owned()] };
    answer.premises.push(Premise::on(item.clone()).fields(fields).positions(Vec::<String>::new()));
}

pub(crate) fn covered(session: &Session, command: &str, params: &Value, objects: &[CoveredAsk]) -> Option<CoveredAt> {
    if command == "model.layerAttrs" {
        return attrs_covered(session, params, objects);
    }
    let target = address(session, command, params).ok()??;
    let mut keys = target.keys.clone();
    let mut comp = json!(target.comp.0);
    let mut covered = Vec::new();
    for object in objects {
        let layer = object.id.as_u64().or_else(|| object.id.as_str()?.parse().ok()) == Some(target.layer.id.0);
        if layer {
            if object.unknown.iter().any(|field| field == "~container") {
                return None;
            }
            if let Some(before) = object.before.get("~container") {
                comp = before.clone();
            }
            if object.unknown.iter().any(|field| field == "~keyOrder") {
                return None;
            }
            if let Some(before) = object.before.get("~keyOrder") {
                keys = match before.get(target.uid.to_string()) {
                    Some(packed) => unpack(packed)?,
                    None => Vec::new(),
                };
            }
        }
        covered.push(Covered { id: object.id.clone(), covered: Some(layer), ..Default::default() });
    }
    Some(CoveredAt { under: occupants(&target, &keys, comp), covered })
}

fn layer_holder<'a>(session: &'a Session, params: &Value) -> Option<(ItemId, &'a Layer)> {
    let id = params.get("target")?.as_u64().or_else(|| params.get("target")?.as_str()?.parse().ok())?;
    session.project.items.values().find_map(|item| Some((item.id, item.as_comp()?.layer(kition_motion_project::LayerId(id))?)))
}
fn attrs_under(session: &Session, command: &str, params: &Value) -> Result<Option<Under>, String> {
    let key = params.get("key").and_then(Value::as_str).ok_or("a raw attribute assignment needs its literal map key")?;
    let plain = key.chars().all(|character| character.is_ascii_alphanumeric() || character == '_');
    let key = key.replace('~', "~0").replace('/', "~1");
    let mut answer = Under::default();
    let target = params.get("target").ok_or("a raw attribute assignment needs its target")?;
    if command == "model.layerAttrs" {
        let (comp, layer) = layer_holder(session, params).ok_or("raw layer attributes name no retained layer")?;
        answer.objects.push(json!(comp.0));
        answer.covers = Some(Covers { fields: vec!["~container".to_owned()] });
        answer.premises.push(
            Premise::on(json!(layer.id.0)).fields([if plain { format!("attrs.{key}") } else { format!("/attrs/{key}") }]).positions(Vec::<String>::new()),
        );
        answer.premises.push(Premise::on(json!(comp.0)).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
    } else {
        let raw = target.as_str().ok_or("raw composition attributes need an object reference")?;
        let replica = session.kdm().ok_or("raw composition attributes need the live replica")?;
        let comp = replica.ids().comps.iter().find(|(_, object)| object.as_str() == raw).map(|(id, _)| *id).ok_or("raw composition reference has no item")?;
        answer.objects.push(json!(comp.0));
        answer.premises.push(Premise::on(target.clone()).fields(Vec::<String>::new()).positions(Vec::<String>::new()));
        answer.premises.push(
            Premise::on(json!(comp.0)).fields([if plain { format!("comp.attrs.{key}") } else { format!("/comp/attrs/{key}") }]).positions(Vec::<String>::new()),
        );
    }
    Ok(Some(answer))
}
fn attrs_covered(session: &Session, params: &Value, objects: &[CoveredAsk]) -> Option<CoveredAt> {
    let (cid, layer) = layer_holder(session, params)?;
    let mut comp = json!(cid.0);
    let mut covered = Vec::new();
    for object in objects {
        let same = object.id.as_u64().or_else(|| object.id.as_str()?.parse().ok()) == Some(layer.id.0);
        if same {
            if object.unknown.iter().any(|field| field == "~container") {
                return None;
            }
            if let Some(before) = object.before.get("~container") {
                comp = before.clone();
            }
        }
        covered.push(Covered { id: object.id.clone(), covered: Some(same), ..Default::default() });
    }
    Some(CoveredAt { under: vec![comp], covered })
}
