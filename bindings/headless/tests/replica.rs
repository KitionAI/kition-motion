//! `kition-motion` opens a sync room's state with no file, as a server's headless kit pool does
//! (`model.open`, `model.import`, `model.export`, `model.save`; kition-sdk `docs/platform/reference/object-model.md`).

use kition_headless::{Headless, KitInfo};
use kition_kit::KitDocument;
use kition_kit_server::{ErrorReason, KitBackend};
use kition_motion_kit::MotionDocument;
use serde_json::json;

const NAME: &str = "Motion.kmotion";

fn kit() -> Headless<MotionDocument> {
    Headless::with_replica_port(KitInfo::from_manifest(include_str!("../../../kit.json"), &[], &[]).unwrap())
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        for k in 0..4 {
            out.push(if k <= chunk.len() { char::from(BASE64[((n >> (18 - 6 * k)) & 63) as usize]) } else { '=' });
        }
    }
    out
}

fn decode(text: &str) -> Vec<u8> {
    let (mut out, mut buffer, mut bits) = (Vec::new(), 0u32, 0u32);
    for byte in text.trim_end_matches('=').bytes() {
        let value = BASE64.iter().position(|known| *known == byte).expect("base64");
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    out
}

fn layer_names(doc: &MotionDocument) -> Vec<String> {
    let info = doc.inspect().unwrap();
    let mut names: Vec<String> = info["info"]["layers"].as_array().unwrap().iter().map(|layer| layer["name"].as_str().unwrap_or_default().to_owned()).collect();
    names.sort();
    names
}

/// The room's project, as a collaborator holds it.
fn peer() -> MotionDocument {
    let mut doc = MotionDocument::create(NAME, 64, 36).unwrap();
    doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap();
    doc.version().unwrap();
    doc
}

#[test]
fn opens_a_room_without_a_file_and_imports_exports_and_saves_it() {
    let mut kit = kit();
    let features = kit.capabilities()["features"].clone();
    for feature in ["model.open", "model.import", "model.export", "model.save"] {
        assert!(features.as_array().unwrap().iter().any(|listed| listed == feature), "{feature} in {features}");
    }

    let mut peer = peer();
    let (snapshot, blobs) = (peer.snapshot().unwrap(), peer.blobs().unwrap());
    let opened = kit.model("model.open", json!({ "name": NAME, "snapshot": encode(&snapshot), "blobs": encode(&blobs) })).unwrap();
    let id = opened["documentId"].as_str().unwrap().to_owned();
    assert_eq!(opened["format"], "motion");

    // A collaborator's change comes in.
    let since = peer.version().unwrap();
    peer.execute("layer.newSolid", json!({ "name": "Green", "color": "#00ff00" })).unwrap();
    let change = peer.export_changes(&since).unwrap();
    kit.model("model.import", json!({ "documentId": id, "changes": encode(&change) })).unwrap();

    // The kit edits on top of it, and the collaborator takes what it exports.
    let batch = json!({ "documentId": id, "ops": [{ "op": "command", "id": "layer.newSolid", "params": { "name": "Blue", "color": "#0000ff" } }] });
    kit.model("model.batch", batch).unwrap();
    let exported = kit.model("model.export", json!({ "documentId": id, "since": encode(&peer.version().unwrap()) })).unwrap();
    peer.import_changes(&decode(exported["changes"].as_str().unwrap())).unwrap();
    assert_eq!(layer_names(&peer), ["Blue", "Green", "Red"]);
    let everything = kit.model("model.export", json!({ "documentId": id })).unwrap();
    let mut joined = MotionDocument::open_with_replica(NAME, &[], &decode(everything["changes"].as_str().unwrap()), &[]).unwrap();
    assert_eq!(joined.version().unwrap(), peer.version().unwrap(), "every change the document has");

    let saved = kit.model("model.save", json!({ "documentId": id })).unwrap();
    assert_eq!((saved["name"].as_str(), saved["format"].as_str()), (Some(NAME), Some("motion")));
    let file = MotionDocument::open(NAME, &decode(saved["data"].as_str().unwrap())).unwrap();
    assert_eq!(layer_names(&file), ["Blue", "Green", "Red"]);
}

#[test]
fn never_opens_an_empty_project_for_a_replica_it_cannot_read() {
    let mut kit = kit();
    for snapshot in [&b"not a replica"[..], &b""[..]] {
        let error = kit.model("model.open", json!({ "name": NAME, "snapshot": encode(snapshot) })).unwrap_err();
        assert_eq!(error.code, ErrorReason::UnsupportedFormat.code(), "{error:?}");
    }
}

/// A layer whose engine id is not a whole number is something the kit's merge repair
/// (`validate`) cannot repair, nor the engine read: the changes are refused, and the document is
/// closed.
#[test]
fn refuses_a_change_validate_cannot_repair_and_closes_the_document() {
    use kition_kit::kdm::{ActorId, Origin, Replica, Schema};
    use std::sync::Arc;

    let mut kit = kit();
    let snapshot = peer().snapshot().unwrap();
    let opened = kit.model("model.open", json!({ "name": NAME, "snapshot": encode(&snapshot) })).unwrap();
    let id = opened["documentId"].as_str().unwrap().to_owned();

    let schema = Arc::new(Schema::parse(include_str!("../../../crates/kdm/kdm-model/model/schema.json")).unwrap());
    let mut raw = Replica::load(schema, &snapshot, ActorId::from_u64(77).unwrap()).unwrap();
    let since = raw.version();
    raw.transact(&Origin::person("user:b"), |tx| {
        let layer = tx.objects_of_type("Layer").into_iter().next().ok_or_else(|| kition_kit::kdm::KdmError::NotFound("a layer".into()))?;
        tx.set(&layer, "id", json!(-5))
    })
    .unwrap();
    let refused = raw.export_changes(&since).unwrap();

    let error = kit.model("model.import", json!({ "documentId": id, "changes": encode(&refused) })).unwrap_err();
    assert_eq!(error.code, ErrorReason::InvalidParams.code(), "{error:?}");
    assert_eq!(error.data.as_ref().unwrap()["refused"], true);
    assert!(error.data.as_ref().unwrap()["detail"].as_str().unwrap().contains("not a whole number"), "{error:?}");
    let closed = kit.model("model.export", json!({ "documentId": id })).unwrap_err();
    assert_eq!(closed.code, ErrorReason::DocumentNotFound.code());
}
