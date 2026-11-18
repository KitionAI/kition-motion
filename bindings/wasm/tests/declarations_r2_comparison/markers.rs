//! Field projection of the frozen bulk-marker rows (layer_menu.rs:577-603).
use super::super::families;
use super::facts;
use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn bulk_layer_markers_have_field_comparisons_after_their_classes_match() {
    let mut document = MotionDocument::create("Marker comparison", 64, 64).unwrap();
    let layer = document.execute("layer.newSolid", json!({})).unwrap()["layer"].as_u64().unwrap();
    document.execute("layer.addMarker", json!({"layer":layer,"time":1,"comment":"Observed"})).unwrap();
    let mut compared = BTreeSet::new();
    for (_, rows) in families() {
        for row in rows {
            if !matches!(row.command, "layer.markersLock" | "layer.deleteAllMarkers") {
                continue;
            }
            // markers_lock assigns the lock flag even for an explicit value. Deletion reads
            // that flag before clearing the complete marker array; other layer content is unused.
            let params = if row.command == "layer.markersLock" { json!({"layers":[layer],"value":false}) } else { json!({"layers":[layer]}) };
            let mut required = BTreeSet::from([format!("exists:{layer}"), format!("positions:{layer}:*"), format!("field:{layer}:attrs.markers_locked")]);
            if row.command == "layer.deleteAllMarkers" {
                required.insert(format!("field:{layer}:attrs.markers"));
            }
            assert_eq!(facts(document.under(row.command, &params).unwrap()), required, "{} {}", row.command, row.path);
            compared.insert(row.command);
        }
    }
    assert_eq!(compared, BTreeSet::from(["layer.markersLock", "layer.deleteAllMarkers"]));
}
