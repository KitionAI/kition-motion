use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn tracked() -> (MotionDocument, u64, u64, u64) {
    let mut doc = MotionDocument::create("Track.kmotion", 64, 64).unwrap();
    let layer = doc.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
    let tracker = doc.execute("track.new", json!({"layer":layer,"kind":"raw","target":null})).unwrap()["tracker"].as_u64().unwrap();
    let project: Value = serde_json::from_slice(&doc.save(None).unwrap()).unwrap();
    let source =
        project["items"].as_object().unwrap().values().filter_map(|item| item["kind"]["layers"].as_array()).flatten().find(|item| item["id"] == layer).unwrap()
            ["source"]["item"]
            .as_u64()
            .unwrap();
    (doc, layer, tracker, source)
}

#[test]
fn tracker_creation_holds_source_dimensions_used_by_default_points() {
    let (doc, layer, _, source) = tracked();
    for command in ["track.motion", "track.stabilize", "track.new"] {
        let params = if command == "track.new" { json!({"layer":layer,"target":null}) } else { json!({"layer":layer}) };
        let under = doc.under(command, &params).unwrap().unwrap();
        let source =
            under.premises.iter().find(|premise| premise.id == json!(source)).unwrap_or_else(|| panic!("{command} lacks source-item premise: {under:?}"));
        let fields = source.fields.as_ref().unwrap();
        assert!(fields.iter().any(|field| field == "data.width"), "{command}: {under:?}");
        assert!(fields.iter().any(|field| field == "data.height"), "{command}: {under:?}");
        assert!(!fields.iter().any(|field| field == "attrs.comment"), "{command}: {under:?}");
    }
}

#[test]
fn tracker_option_assignment_holds_its_register_without_layer_content() {
    let (doc, layer, tracker, _) = tracked();
    let under = doc.under("track.options", &json!({"layer":layer,"tracker":tracker,"blur":7})).unwrap().unwrap();
    let premise = under.premises.iter().find(|premise| premise.id == json!(layer)).unwrap_or_else(|| panic!("missing tracker owner: {under:?}"));
    let fields = premise.fields.as_ref().unwrap();
    assert!(fields.iter().any(|field| field.ends_with(".meta.kind.settings.options.blur")), "{under:?}");
    assert!(!fields.iter().any(|field| field.contains("transform") || field == "attrs.comment" || field == "props"), "{under:?}");
    assert_eq!(premise.positions.as_deref(), Some([].as_slice()), "{under:?}");
}
