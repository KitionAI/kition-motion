//! Composition settings and whole-comp edits, including their child-layer writes.
use std::collections::{BTreeMap, BTreeSet};

use kition_kit::{DerivedValues, Premise, Under, Within};
use kition_motion_engine::Session;
use kition_motion_project::{Comp, ItemId, Layer, Property, Renderer};
use serde_json::{Value, json};

#[derive(Default)]
struct Reads {
    answer: Under,
    fields: BTreeMap<u64, BTreeSet<String>>,
}

impl Reads {
    fn field(&mut self, id: u64, field: impl Into<String>) {
        self.fields.entry(id).or_default().insert(field.into());
    }

    fn finish(mut self) -> Under {
        self.answer.premises.extend(self.fields.into_iter().map(|(id, fields)| Premise::on(json!(id)).fields(fields).positions(Vec::<String>::new())));
        self.answer
    }
}

fn composition<'a>(session: &'a Session, params: &Value, reads: &mut Reads) -> Option<(ItemId, &'a Comp)> {
    let id = match params.get("comp").filter(|value| !value.is_null()) {
        Some(Value::Number(number)) => ItemId(number.as_u64()?),
        Some(Value::String(text)) => match text.parse().ok().map(ItemId).filter(|id| session.project.comp(*id).is_some()) {
            Some(id) => id,
            None => {
                let mut found = None;
                for item in session.project.items.values() {
                    reads.field(item.id.0, "name");
                    if item.name == *text {
                        reads.field(item.id.0, "kind");
                        if item.as_comp().is_some() {
                            found = Some(item.id);
                            break;
                        }
                    }
                }
                reads.answer.within.push(Within {
                    parent: json!("project"),
                    field: "items".to_owned(),
                    from: 0,
                    to: session.project.items.len().saturating_add(1),
                    ..Default::default()
                });
                found?
            }
        },
        Some(_) => return None,
        None => session.active_comp_id()?,
    };
    reads.field(id.0, "kind");
    Some((id, session.project.comp(id)?))
}

fn stored_values(property: Option<&Property>) -> Value {
    property.map_or(Value::Null, |property| {
        json!({
            "uid":property.uid,
            "value":property.value,
            "keys":property.keys.iter().map(|key| json!([key.time.0.to_string(),key.value])).collect::<Vec<_>>()
        })
    })
}

fn resize_inputs(layer: &Layer) -> Value {
    // offset_position never samples curves, evaluates expressions, or retimes keys.
    if layer.parent.is_some() {
        return json!({"unparented":false});
    }
    let transform = layer.transform();
    json!({
        "unparented":true,
        "transform":transform.map(|group| group.uid),
        "position":stored_values(transform.and_then(|group| group.get("position"))),
        "positionX":stored_values(transform.and_then(|group| group.get("positionX"))),
        "positionY":stored_values(transform.filter(|group| group.get("positionX").is_some()).and_then(|group| group.get("positionY")))
    })
}

pub(crate) fn derived(session: &Session, ids: &[Value], names: &[String]) -> Vec<DerivedValues> {
    let requested = |id: u64| ids.is_empty() || ids.iter().any(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()) == Some(id));
    let named = |name: &str| names.is_empty() || names.iter().any(|value| value == name);
    let mut answer = Vec::new();
    let mut mask_budget = 65_536usize;
    for item in session.project.items.values() {
        let Some(comp) = item.as_comp() else { continue };
        if requested(item.id.0) && named("layerMembers") {
            let mut members: Vec<_> = comp.layers.iter().map(|layer| layer.id.0).collect();
            members.sort_unstable();
            answer.push(DerivedValues { id: json!(item.id.0), values: serde_json::Map::from_iter([("layerMembers".to_owned(), json!(members))]) });
        }
        if requested(item.id.0) && named("maskMembers") {
            let members = crate::rest_masks::snapshot(comp, &mut mask_budget);
            answer.push(DerivedValues { id: json!(item.id.0), values: serde_json::Map::from_iter([("maskMembers".to_owned(), members)]) });
        }
        if named("dependents") {
            // The layers a deletion of this layer rewrites (edit.rs clear: orphans it unparents
            // and compensates, mattes it drops), by id; a layer tied to none has none.
            let mut dependents: std::collections::HashMap<u64, Vec<u64>> = std::collections::HashMap::new();
            for layer in &comp.layers {
                for tie in [layer.parent, layer.track_matte.as_ref().map(|matte| matte.layer)].into_iter().flatten() {
                    dependents.entry(tie.0).or_default().push(layer.id.0);
                }
            }
            answer.extend(comp.layers.iter().filter(|layer| requested(layer.id.0)).map(|layer| {
                let mut ids = dependents.remove(&layer.id.0).unwrap_or_default();
                ids.sort_unstable();
                ids.dedup();
                DerivedValues { id: json!(layer.id.0), values: serde_json::Map::from_iter([("dependents".to_owned(), json!(ids))]) }
            }));
        }
        if named("resizeInputs") {
            answer.extend(
                comp.layers.iter().filter(|layer| requested(layer.id.0)).map(|layer| DerivedValues {
                    id: json!(layer.id.0),
                    values: serde_json::Map::from_iter([("resizeInputs".to_owned(), resize_inputs(layer))]),
                }),
            );
        }
    }
    answer
}

pub(crate) fn under(session: &Session, command: &str, params: &Value) -> Option<Under> {
    if !matches!(command, "comp.settings" | "comp.setSwitch" | "comp.setPosterTime" | "comp.trimToWorkArea") {
        return None;
    }
    let mut reads = Reads::default();
    let (id, comp) = composition(session, params, &mut reads)?;
    match command {
        "comp.setPosterTime" => reads.field(id.0, "comp.attrs.poster_time"),
        "comp.setSwitch" => {
            let field = match params.get("switch")?.as_str()? {
                "hideShy" | "shy" => "hide_shy",
                "motionBlur" => "enable_motion_blur",
                "frameBlending" => "enable_frame_blending",
                "draft3d" | "draft3D" => "draft_3d",
                _ => return None,
            };
            reads.field(id.0, format!("comp.attrs.{field}"));
        }
        "comp.trimToWorkArea" => {
            for field in ["work_area", "duration", "frame_rate"] {
                reads.field(id.0, format!("comp.attrs.{field}"));
            }
            reads.field(id.0, "~layerMembers");
            reads.answer.premises.extend(
                comp.layers.iter().map(|layer| Premise::on(json!(layer.id.0)).fields(Vec::<String>::new()).positions(["startTime", "inPoint", "outPoint"])),
            );
        }
        "comp.settings" => {
            let renderer = match params.get("renderer").and_then(Value::as_str).map(str::to_ascii_lowercase).as_deref() {
                Some("classic3d" | "classic") => Renderer::Classic3D,
                Some("advanced3d" | "advanced") => Renderer::Advanced3D,
                _ => comp.renderer,
            };
            // Advanced geometry insertion has a separate frozen row and retains its fallback.
            if renderer != Renderer::Classic3D {
                return None;
            }
            reads.field(id.0, "comp.attrs.renderer");
            for (parameter, field) in [
                ("name", "name"),
                ("width", "comp.attrs.width"),
                ("height", "comp.attrs.height"),
                ("frameRate", "comp.attrs.frame_rate"),
                ("fps", "comp.attrs.frame_rate"),
                ("startTime", "comp.attrs.display_start"),
                ("pixelAspect", "comp.attrs.pixel_aspect"),
                ("background", "comp.attrs.background"),
                ("shutterAngle", "comp.attrs.shutter_angle"),
                ("shutterPhase", "comp.attrs.shutter_phase"),
                ("motionBlurSamples", "comp.attrs.motion_blur_samples"),
                ("adaptiveSampleLimit", "comp.attrs.motion_blur_adaptive_limit"),
                ("preserveFrameRate", "comp.attrs.preserve_frame_rate"),
                ("preserveResolution", "comp.attrs.preserve_resolution"),
            ] {
                if params.get(parameter).is_some_and(|value| !value.is_null()) {
                    reads.field(id.0, field);
                }
            }
            if params.get("startTime").and_then(Value::as_f64).is_none() && params.get("startTimecode").and_then(Value::as_str).is_some() {
                reads.field(id.0, "comp.attrs.frame_rate");
                reads.field(id.0, "comp.attrs.display_start");
            }
            if params.get("duration").and_then(Value::as_f64).is_some() {
                for field in ["duration", "frame_rate", "work_area"] {
                    reads.field(id.0, format!("comp.attrs.{field}"));
                }
            }
            let anchor = params.get("anchor").and_then(Value::as_u64).unwrap_or(4).min(8);
            let dimension = |name: &str, old: u32| params.get(name).and_then(Value::as_u64).map_or(old, |value| (value as u32).clamp(4, 30000));
            if (!anchor.is_multiple_of(3) && dimension("width", comp.width) != comp.width)
                || (anchor / 3 != 0 && dimension("height", comp.height) != comp.height)
            {
                reads.field(id.0, "~layerMembers");
                for layer in &comp.layers {
                    reads.field(layer.id.0, "~resizeInputs");
                }
            }
        }
        _ => return None,
    }
    Some(reads.finish())
}
