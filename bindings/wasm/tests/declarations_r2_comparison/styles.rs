//! Field projection of style insertion (commands/styles.rs:23-92, project/styles.rs:318-339).
use super::{facts, premise};
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn style_insertions_compare_fields_after_their_object_classes_match() {
    let commands = [
        "layer.style.add",
        "layer.style.showAll",
        "layer.style.dropShadow",
        "layer.style.innerShadow",
        "layer.style.outerGlow",
        "layer.style.innerGlow",
        "layer.style.bevelEmboss",
        "layer.style.satin",
        "layer.style.colorOverlay",
        "layer.style.gradientOverlay",
        "layer.style.stroke",
    ];
    let mut compared = BTreeSet::new();
    for command in commands {
        let mut doc = MotionDocument::create("Styles", 64, 64).unwrap();
        let comp = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
        let layer = doc.execute("layer.newSolid", json!({})).unwrap()["layer"].as_u64().unwrap();
        let params = if command == "layer.style.add" { json!({"layers":[layer],"style":"dropShadow"}) } else { json!({"layers":[layer]}) };
        let mut required = premise(json!(layer), &["~styleLayout"]);
        required.extend(premise(json!(comp), &["comp.attrs.global_light"]));
        assert_eq!(facts(doc.under(command, &params).unwrap()), required, "{command}: a new blending group copies global light");
        compared.insert(command);
    }
    assert_eq!(compared.len(), 11);

    let mut doc = MotionDocument::create("Existing style", 64, 64).unwrap();
    let layer = doc.execute("layer.newSolid", json!({})).unwrap()["layer"].as_u64().unwrap();
    doc.execute("layer.style.dropShadow", json!({"layers":[layer]})).unwrap();
    let required = premise(json!(layer), &["~styleLayout", "layerStyles/dropShadow"]);
    assert_eq!(
        facts(doc.under("layer.style.add", &json!({"layers":[layer],"style":"dropShadow","params":{"opacity":50}})).unwrap()),
        required,
        "an existing style returns and can assign its own values; it does not copy global light again"
    );
}
