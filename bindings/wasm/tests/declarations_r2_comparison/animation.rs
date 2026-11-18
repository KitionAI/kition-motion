//! Literal translations of frozen animation rows, with incomplete branches kept explicit.

use super::super::{Row, families};
use super::{Probe, facts, premise};
use kition_kit::{CallContext, KitDocument, Within};
use kition_motion_engine::Session;
use kition_motion_kit::MotionDocument;
use kition_motion_project::{ItemId, LayerId, PropGroup, Property};
use serde_json::{Value, json};
use std::collections::BTreeSet;

struct Fixture {
    session: Session,
    comp: u64,
    layer: u64,
}

impl Fixture {
    fn new(animated: bool) -> Self {
        let mut session = Session::default();
        let comp =
            session.execute("comp.new", json!({"name":"Comparison","width":320,"height":180,"frameRate":30,"duration":8})).unwrap()["comp"].as_u64().unwrap();
        let layer = session.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
        if animated {
            for (time, value) in [(0, 10), (1, 20), (2, 30)] {
                session.execute("prop.addKey", json!({"layer":layer,"path":"transform/opacity","time":time,"value":value})).unwrap();
            }
        }
        Self { session, comp, layer }
    }

    fn property(&self) -> &Property {
        self.session.project.comp(ItemId(self.comp)).unwrap().layer(LayerId(self.layer)).unwrap().props.prop("transform/opacity").unwrap()
    }

    fn params(&self) -> Value {
        json!({"layer":self.layer,"prop":self.property().uid})
    }

    fn selected_key(&mut self) {
        self.session.execute("keys.select", json!({"keys":[{"layer":self.layer,"prop":self.property().uid,"time":1}]})).unwrap();
    }

    fn document(&self) -> MotionDocument {
        let mut document = MotionDocument::open("R2Animation.kmotion", self.session.project.to_json().as_bytes()).unwrap();
        document.set_context(&CallContext { part: Some(json!({"comp":self.comp})), time: Some(1.0), on: Vec::new() }).unwrap();
        document.execute("layer.select", json!({"layers":[self.layer]})).unwrap();
        if !self.session.state.selected_keys.is_empty() {
            document.execute("keys.select", json!({"keys":[{"layer":self.layer,"prop":self.property().uid,"time":1}]})).unwrap();
        }
        document
    }

    fn required(&self, names: &[&str]) -> BTreeSet<String> {
        premise(json!(self.layer), names)
    }
}

fn field(required: &mut BTreeSet<String>, id: u64, name: impl AsRef<str>) {
    required.insert(format!("field:{id}:{}", name.as_ref()));
}

fn position(required: &mut BTreeSet<String>, id: u64, names: &[&str]) {
    required.remove(&format!("positions:{id}:[]"));
    let mut names: Vec<_> = names.to_vec();
    names.sort();
    required.insert(format!("positions:{id}:{}", json!(names)));
}

fn window(required: &mut BTreeSet<String>, id: u64, name: &str, from: usize, to: usize) {
    let value = Within { parent: json!(id), field: name.to_owned(), from, to, ..Default::default() };
    required.insert(format!("within:{}", serde_json::to_string(&value).unwrap()));
}

// project/props.rs find_node matches names or match IDs and counts matches from the front.
// These fixture-specific windows and metadata facts are not aliases for a whole property.
fn named_opacity(required: &mut BTreeSet<String>, fixture: &Fixture, indexed: bool, bare: bool) -> String {
    let root = &fixture.session.project.comp(ItemId(fixture.comp)).unwrap().layer(LayerId(fixture.layer)).unwrap().props;
    let transform_index = root.children.iter().position(|node| node.match_id() == "transform").unwrap();
    let transform = root.children[transform_index].as_group().unwrap();
    let opacity_index = transform.children.iter().position(|node| node.uid() == fixture.property().uid).unwrap();
    if indexed {
        window(required, fixture.layer, "props.children", transform_index, transform_index + 1);
        window(required, fixture.layer, &format!("props.children.{transform_index}.children"), opacity_index, opacity_index + 1);
        return format!("#{}/#{}", transform_index + 1, opacity_index + 1);
    }
    let scan = |required: &mut BTreeSet<String>, group: &PropGroup, prefix: &str, until: usize, wanted: &str| {
        for (index, node) in group.children.iter().take(until + 1).enumerate() {
            field(required, fixture.layer, format!("{prefix}.children.{index}.meta.match"));
            if node.match_id() != wanted {
                field(required, fixture.layer, format!("{prefix}.children.{index}.meta.name"));
            }
        }
        window(required, fixture.layer, &format!("{prefix}.children"), 0, until + 1);
    };
    if bare {
        scan(required, root, "props", root.children.len() - 1, "opacity");
    }
    scan(required, root, "props", transform_index, "transform");
    scan(required, transform, &format!("props.children.{transform_index}"), opacity_index, "opacity");
    if bare { "opacity".to_owned() } else { "transform/opacity".to_owned() }
}

fn direct_expression(scope: &'static str, existing: bool, expression: Option<&str>, enabled: Option<bool>) -> (MotionDocument, Probe, &'static str) {
    let mut fixture = Fixture::new(false);
    if existing {
        fixture.session.execute("prop.setExpression", json!({"layer":fixture.layer,"prop":fixture.property().uid,"expression":"value + 2"})).unwrap();
    }
    let mut params = fixture.params();
    if let Some(expression) = expression {
        params["expression"] = json!(expression);
    }
    if let Some(enabled) = enabled {
        params["enabled"] = json!(enabled);
    }
    let mut required = fixture.required(&["transform/opacity.expr"]);
    if expression.is_none() && !existing && enabled != Some(false) {
        for name in ["transform.meta.match", "transform/opacity.meta.match", "transform/opacity.meta.name"] {
            field(&mut required, fixture.layer, name);
        }
    }
    (fixture.document(), Probe { params, expected: required }, scope)
}

fn expression_probes(row: &Row) -> Vec<(MotionDocument, Probe, &'static str)> {
    let explicit = row.path == "explicit text including empty removal";
    if explicit {
        let mut probes = vec![
            direct_expression("uid-explicit-text", false, Some("value + 1"), None),
            direct_expression("uid-explicit-empty", true, Some(""), None),
            direct_expression("uid-explicit-disabled", false, Some("value + 1"), Some(false)),
        ];
        for scope in [
            "named-property-explicit-text",
            "index-property-explicit-text",
            "bare-property-explicit-text",
            "named-layer-explicit-text",
            "index-layer-explicit-text",
            "named-comp-explicit-text",
        ] {
            let fixture = Fixture::new(false);
            let mut required = fixture.required(&["transform/opacity.expr"]);
            let mut params = fixture.params();
            params["expression"] = json!("value + 1");
            match scope {
                "named-property-explicit-text" | "index-property-explicit-text" | "bare-property-explicit-text" => {
                    params.as_object_mut().unwrap().remove("prop");
                    params["path"] = json!(named_opacity(&mut required, &fixture, scope.starts_with("index-"), scope.starts_with("bare-")));
                }
                "named-layer-explicit-text" => {
                    params["layer"] = json!("Source");
                    field(&mut required, fixture.layer, "name");
                    window(&mut required, fixture.comp, "comp.layers", 0, 1);
                }
                "index-layer-explicit-text" => {
                    params["layer"] = json!("#1");
                    required.insert(format!("object:{}", fixture.layer));
                    window(&mut required, fixture.comp, "comp.layers", 0, 1);
                }
                "named-comp-explicit-text" => {
                    params["comp"] = json!("Comparison");
                    required.extend(premise(json!(fixture.comp), &["name", "kind"]));
                    window(&mut required, 0, "unused", 0, 0);
                    required.remove(&format!(
                        "within:{}",
                        serde_json::to_string(&Within { parent: json!(0), field: "unused".to_owned(), ..Default::default() }).unwrap()
                    ));
                    let value = Within {
                        parent: json!("project"),
                        field: "items".to_owned(),
                        from: 0,
                        to: fixture.session.project.items.len() + 1,
                        ..Default::default()
                    };
                    required.insert(format!("within:{}", serde_json::to_string(&value).unwrap()));
                }
                _ => unreachable!(),
            }
            probes.push((fixture.document(), Probe { params, expected: required }, scope));
        }
        // Selection expansion and syntax-hook paths are still unresolved for this frozen row.
        probes
    } else {
        vec![
            direct_expression("uid-create-default", false, None, None),
            direct_expression("uid-enable-default", false, None, Some(true)),
            direct_expression("uid-disable-missing", false, None, Some(false)),
            direct_expression("uid-toggle-existing", true, None, None),
            direct_expression("uid-enable-existing", true, None, Some(true)),
            direct_expression("uid-disable-existing", true, None, Some(false)),
        ]
    }
}

fn key_probe(row: &Row, scope: &'static str) -> (MotionDocument, Probe, &'static str) {
    let fixture = Fixture::new(true);
    let mut params = fixture.params();
    params["time"] = json!(1.2);
    params["value"] = json!(45);
    let mut required =
        fixture.required(&["attrs.stretch", "transform/opacity.keys[time=254016000000].time", "transform/opacity.keys[time=254016000000].value"]);
    position(&mut required, fixture.layer, &["startTime"]);
    // Numeric-layer nearest membership is represented by the bounded keyTimes premise;
    // no logical/physical field equivalence is applied by this comparison.
    required.insert(format!("object:{}", fixture.layer));
    required.insert("cover:{\"fields\":[\"~keyTimes\"]}".to_owned());
    for time in [0, 508_032_000_000i64] {
        field(&mut required, fixture.layer, format!("transform/opacity.keys[time={time}].data.roving"));
    }
    if row.path == "newTime supplied" {
        params["newTime"] = json!(1.6);
        required.extend(premise(json!(fixture.comp), &["comp.attrs.frame_rate"]));
        // Existing neighbour times are used as the clamp bounds, even when no clamp fires.
        for time in [0, 508_032_000_000i64] {
            field(&mut required, fixture.layer, format!("transform/opacity.keys[time={time}].time"));
        }
    }
    if scope.starts_with("named-") {
        params.as_object_mut().unwrap().remove("prop");
        params["path"] = json!(named_opacity(&mut required, &fixture, false, false));
    } else if scope.starts_with("index-") {
        params.as_object_mut().unwrap().remove("prop");
        params["path"] = json!(named_opacity(&mut required, &fixture, true, false));
    }
    (fixture.document(), Probe { params, expected: required }, scope)
}

fn selected_probe(row: &Row) -> (MotionDocument, Probe, &'static str) {
    let mut fixture = Fixture::new(true);
    fixture.selected_key();
    let params = match row.command {
        "keys.setLabel" => json!({"label":2}),
        "keys.interpolation" => json!({"interpolation":"linear"}),
        _ => json!({}),
    };
    let mut required = fixture.required(&["transform/opacity.keys[time=254016000000].time"]);
    // edit_keys checks every interior roving flag; endpoints are unconditionally assigned.
    for time in [0, 254_016_000_000i64, 508_032_000_000i64] {
        field(&mut required, fixture.layer, format!("transform/opacity.keys[time={time}].data.roving"));
    }
    let fields: &[&str] = match row.command {
        "keys.setLabel" => &["data.label"],
        "keys.easyEase" => &["value", "data.in_interp", "data.in_ease", "data.out_interp", "data.out_ease", "data.auto_bezier"],
        "keys.easyEaseIn" => &["value", "data.in_interp", "data.in_ease", "data.auto_bezier"],
        "keys.easyEaseOut" => &["value", "data.out_interp", "data.out_ease", "data.auto_bezier"],
        "keys.toggleHold" => &["data.out_interp", "data.in_interp", "data.auto_bezier", "data.continuous", "data.out_ease"],
        "keys.interpolation" => &["value", "data.in_interp", "data.out_interp", "data.auto_bezier", "data.continuous"],
        _ => unreachable!(),
    };
    for name in fields {
        field(&mut required, fixture.layer, format!("transform/opacity.keys[time=254016000000].{name}"));
    }
    (fixture.document(), Probe { params, expected: required }, "selected-scalar-no-roving")
}

pub(super) fn probes(row: &Row) -> Vec<(MotionDocument, Probe, &'static str)> {
    match row.command {
        "prop.setExpression" => expression_probes(row),
        "keys.set" => vec![key_probe(row, "uid-key-edit"), key_probe(row, "named-key-edit"), key_probe(row, "index-key-edit")],
        "keys.easyEase" | "keys.easyEaseIn" | "keys.easyEaseOut" | "keys.toggleHold" | "keys.interpolation" | "keys.setLabel" => vec![selected_probe(row)],
        _ => Vec::new(),
    }
}

pub(super) fn complete_paths() -> BTreeSet<(&'static str, &'static str)> {
    // The expression selection/syntax-hook paths and roving/spatial branches are not yet
    // lowered. A fixture on the scalar branch does not certify a broader frozen row.
    BTreeSet::new()
}

pub(super) fn analyze() -> Value {
    let mut reports = Vec::new();
    let mut signatures = BTreeSet::new();
    for (family, rows) in families() {
        for row in rows {
            for (document, probe, scope) in probes(row) {
                let actual = facts(document.under(row.command, &probe.params).unwrap());
                let less: BTreeSet<_> = probe.expected.difference(&actual).cloned().collect();
                let more: BTreeSet<_> = actual.difference(&probe.expected).cloned().collect();
                for (kind, difference) in [("less", &less), ("more", &more)] {
                    signatures.extend(difference.iter().map(|fact| format!("{family}/{}:{}:{scope}:{kind}:{fact}", row.command, row.path)));
                }
                reports.push(json!({"family":family,"command":row.command,"path":row.path,"scope":scope,"params":probe.params,"required":probe.expected,"actual":actual,"literal_less":less,"literal_more":more}));
            }
        }
    }
    json!({"complete_paths":complete_paths(),"probes":reports,"literal_mismatch_signatures":signatures,"interpretation":"Literal Under-only comparison; static declaration roles and SDK replay must be audited before classifying these as actual held LESS or MORE."})
}

#[test]
fn animation_comparison_has_explicit_incomplete_statuses() {
    let report = analyze();
    if let Ok(path) = std::env::var("MOTION_R2_ANIMATION_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert_eq!(report["probes"].as_array().unwrap().len(), 27);
    assert!(complete_paths().is_empty());
}
