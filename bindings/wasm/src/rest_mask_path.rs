//! Static mask path assignment by explicit layer and mask uid; other branches stay general.
use kition_kit::{Premise, Under};
use kition_motion_engine::{Session, keyframe::Value as PropValue};
use kition_motion_project::{GroupKind, ItemId, LayerId};
use serde_json::{Value, json};

fn id(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Option<Under> {
    if command != "mask.setClosed" {
        return None;
    }
    let comp = match params.get("comp") {
        Some(value) => ItemId(id(value)?),
        None => session.active_comp_id()?,
    };
    let layer = session.project.comp(comp)?.layer(LayerId(id(params.get("layer")?)?))?;
    let mask = params.get("mask")?.as_u64()?;
    let (root_index, masks) =
        layer.props.children.iter().enumerate().filter_map(|(index, node)| Some((index, node.as_group()?))).find(|(_, group)| group.match_id == "masks")?;
    let group = masks.children.iter().find(|node| node.uid() == mask)?.as_group()?;
    if !matches!(group.kind, GroupKind::Mask { .. }) {
        return None;
    }
    let (property_index, property) =
        group.children.iter().enumerate().filter_map(|(index, node)| Some((index, node.as_prop()?))).find(|(_, property)| property.match_id == "path")?;
    if !property.keys.is_empty() || !matches!(property.value, PropValue::Path(_)) {
        return None;
    }
    let group = format!("props.children.{root_index}.children[uid={mask}]");
    let mut fields = vec!["attrs.stretch".to_owned()];
    for (index, node) in layer.props.children.iter().enumerate().take(root_index.saturating_add(1)) {
        if node.as_group().is_some() {
            fields.push(format!("props.children.{index}.meta.match"));
        }
    }
    fields.extend([format!("{group}.uid"), format!("{group}.meta.kind.kind"), format!("{group}.meta.kind.roto_bezier")]);
    for (index, node) in masks.children.iter().find(|node| node.uid() == mask)?.as_group()?.children.iter().enumerate().take(property_index.saturating_add(1)) {
        if node.as_prop().is_some() {
            fields.push(format!("{group}.children.{index}.meta.match"));
        }
    }
    fields.extend([format!("{group}.children.{property_index}.value"), format!("{group}.children.{property_index}.keys")]);
    Some(Under { premises: vec![Premise::on(json!(layer.id.0)).fields(fields).positions(["startTime"])], ..Default::default() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_explicit_uid_static_mask_branch_is_narrowed() {
        let mut session = Session::default();
        session.execute("comp.new", json!({"name":"Mask path","width":32,"height":32,"duration":2})).unwrap();
        let layer = session.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
        let mask = session.execute("mask.new", json!({"layer":layer,"vertices":[[0,0],[20,0],[20,20]],"closed":false})).unwrap()["mask"].as_u64().unwrap();
        let params = json!({"layer":layer,"mask":mask,"closed":true});
        assert!(under(&session, "mask.setClosed", &params).is_some());
        assert!(under(&session, "mask.setClosed", &json!({"layer":"Source","mask":mask,"closed":true})).is_none());
        assert!(under(&session, "mask.setClosed", &json!({"layer":layer,"mask":1,"closed":true})).is_none());
        assert!(under(&session, "mask.setClosed", &json!({"mask":mask,"closed":true})).is_none());
        session.execute("prop.addKey", json!({"layer":layer,"path":"masks/#1/path","time":0})).unwrap();
        assert!(under(&session, "mask.setClosed", &params).is_none());
    }
}
