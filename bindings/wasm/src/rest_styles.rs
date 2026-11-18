//! Style insertion consumes eligibility, style layout and the selected style, not Transform.
use kition_kit::{DerivedValues, Premise, Under};
use kition_motion_engine::{Session, commands::resolve_layer};
use kition_motion_project::{ItemId, Layer, styles};
use serde_json::{Value, json};

fn names(command: &str, params: &Value) -> Option<Vec<&'static str>> {
    match command {
        "layer.style.showAll" => Some(styles::STYLES.iter().map(|(id, _)| *id).collect()),
        "layer.style.add" => Some(vec![styles::style_id(params.get("style")?.as_str()?)?]),
        _ => Some(vec![styles::style_id(command.strip_prefix("layer.style.")?)?]),
    }
}

fn layout(layer: &Layer) -> Result<Value, String> {
    if layer.props.children.len() > 65_536 || layer.layer_styles().is_some_and(|group| group.children.len() > 65_536) {
        return Err("too many property groups to guard style insertion; simplify the layer".to_owned());
    }
    let existing = layer.layer_styles();
    let gap = if existing.is_none() {
        let index =
            layer.props.children.iter().position(|node| node.match_id() == "transform").map_or(layer.props.children.len(), |index| index.saturating_add(1));
        json!([
            index.checked_sub(1).and_then(|index| layer.props.children.get(index)).map(|node| node.uid()),
            layer.props.children.get(index).map(|node| node.uid())
        ])
    } else {
        Value::Null
    };
    Ok(json!({
        "eligible":layer.can_have_styles(),
        "gap":gap,
        "styles":existing.map(|group| json!({
            "uid":group.uid,"enabled":group.enabled,
            "children":group.children.iter().map(|node| json!([node.uid(),node.match_id(),node.as_group().is_some()])).collect::<Vec<_>>()
        }))
    }))
}

pub(crate) fn derived(session: &Session, ids: &[Value], names: &[String]) -> Result<Vec<DerivedValues>, String> {
    if !names.is_empty() && !names.iter().any(|name| name == "styleLayout") {
        return Ok(Vec::new());
    }
    let mut answer = Vec::new();
    for layer in session.project.items.values().filter_map(|item| item.as_comp()).flat_map(|comp| &comp.layers) {
        if !ids.is_empty() && !ids.iter().any(|id| id.as_u64().or_else(|| id.as_str()?.parse().ok()) == Some(layer.id.0)) {
            continue;
        }
        answer.push(DerivedValues { id: json!(layer.id.0), values: serde_json::Map::from_iter([("styleLayout".to_owned(), layout(layer)?)]) });
    }
    Ok(answer)
}

pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Result<Option<Under>, String> {
    let Some(names) = names(command, params) else { return Ok(None) };
    let cid =
        match params.get("comp").filter(|value| !value.is_null()) {
            Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()).map(ItemId).filter(|id| session.project.comp(*id).is_some()).or_else(|| {
                session.project.items.values().find(|item| value.as_str() == Some(item.name.as_str()) && item.as_comp().is_some()).map(|item| item.id)
            }),
            None => session.active_comp_id(),
        };
    let Some((cid, comp)) = cid.and_then(|id| Some((id, session.project.comp(id)?))) else { return Ok(None) };
    let target_values = params.get("layers").and_then(Value::as_array);
    let count = target_values.map_or(session.state.selected_layers.len(), Vec::len);
    if count > 4096 {
        return Err("too many style recipients to guard; split the selection".to_owned());
    }
    let targets = target_values
        .map(|values| values.iter().filter_map(|value| resolve_layer(comp, value)).collect::<Vec<_>>())
        .unwrap_or_else(|| session.state.selected_layers.clone());
    let mut answer = Under::default();
    for layer in targets.iter().filter_map(|id| comp.layer(*id)) {
        layout(layer)?;
        let mut fields = vec!["~styleLayout".to_owned()];
        if target_values.is_some_and(|values| {
            values.iter().any(|value| {
                value.as_str().is_some_and(|name| name.parse::<u64>().is_err() && !name.starts_with('#')) && resolve_layer(comp, value) == Some(layer.id)
            })
        }) {
            fields.push("name".to_owned());
        }
        if let Some(group) = layer.layer_styles() {
            for name in &names {
                if group.sub(name).is_some() {
                    fields.push(format!("layerStyles/{name}"));
                }
            }
        } else if layer.can_have_styles() {
            answer.premises.push(Premise::on(json!(cid.0)).fields(["comp.attrs.global_light"]).positions(Vec::<String>::new()));
        }
        answer.premises.push(Premise::on(json!(layer.id.0)).fields(fields).positions(Vec::<String>::new()));
    }
    Ok(Some(answer))
}
