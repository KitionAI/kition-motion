//! `document.text` (`KitText 1.0`) on the document `kition-motion` serves: the text layers of
//! every composition as plain text, in layer order, one part per composition, cut at `limit`
//! characters, and nothing changed. The headless executable lists the capability
//! `document-text` and answers the method.

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit::KitDocument;
use kition_kit::testing::check_text;
use kition_kit::text::{DEFAULT_LIMIT, DocumentText};
use kition_kit_server::{KitBackend, OpenRequest};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};

fn effect() -> MotionDocument {
    MotionDocument::create("Title.kmotion", 640, 360).unwrap()
}

fn run(doc: &mut MotionDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn layer(made: &Value) -> u64 {
    made["layer"].as_u64().unwrap_or_else(|| panic!("no layer in {made}"))
}

/// Comp 1 with two text layers (the newer on top) and a solid; a second composition, "Lower
/// third", with one text layer of two lines.
fn titled() -> MotionDocument {
    let mut doc = effect();
    run(&mut doc, "layer.newText", json!({ "text": "Subtitle here" }));
    run(&mut doc, "layer.newSolid", json!({ "color": "#336699" }));
    run(&mut doc, "layer.newText", json!({ "text": "Main Title" }));
    run(&mut doc, "comp.new", json!({ "name": "Lower third", "width": 640, "height": 360, "duration": 5, "open": true }));
    run(&mut doc, "layer.newText", json!({ "text": "Ana Lima\rDirector" }));
    doc
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

fn piece(answer: &DocumentText, index: usize) -> String {
    let part = &answer.parts[index];
    answer.text.chars().skip(part.start).take(part.end - part.start).collect()
}

#[test]
fn text_layers_per_composition_in_layer_order() {
    let doc = titled();
    let answer = check_text(&doc).unwrap();
    assert_eq!(answer.text, "Main Title\nSubtitle here\nAna Lima\nDirector", "{answer:?}");
    assert!(!answer.truncated);
    let labels: Vec<&str> = answer.parts.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["Comp 1", "Lower third"]);
    let pieces: Vec<String> = (0..answer.parts.len()).map(|index| piece(&answer, index)).collect();
    assert_eq!(pieces, ["Main Title\nSubtitle here", "Ana Lima\nDirector"]);
}

#[test]
fn the_limit_cuts_the_text_and_the_parts_stay_within_it() {
    let doc = titled();
    let whole = doc.text(DEFAULT_LIMIT).unwrap().text;
    for limit in 0..=chars(&whole) + 2 {
        let answer = doc.text(limit).unwrap();
        assert!(chars(&answer.text) <= limit, "limit {limit}");
        assert!(whole.starts_with(&answer.text));
        assert_eq!(answer.truncated, limit < chars(&whole), "limit {limit}");
        assert_eq!(answer.text == whole, !answer.truncated, "limit {limit}");
        for p in &answer.parts {
            assert!(p.start < p.end && p.end <= chars(&answer.text), "limit {limit}: {p:?} outside {:?}", answer.text);
        }
    }
}

#[test]
fn characters_are_counted_not_bytes() {
    let mut doc = effect();
    run(&mut doc, "layer.newText", json!({ "text": "标题 é" }));
    let answer = doc.text(2).unwrap();
    assert_eq!(answer.text, "标题");
    assert!(answer.truncated);
    assert_eq!(answer.to_json()["parts"], json!([{ "label": "Comp 1", "start": 0, "end": 2 }]));
    check_text(&doc).unwrap();
}

#[test]
fn reading_the_text_changes_nothing() {
    let doc = titled();
    let (revision, dirty, saved) = (doc.revision(), doc.dirty(), doc.save(None).unwrap());
    for limit in [0, 5, DEFAULT_LIMIT] {
        doc.text(limit).unwrap();
    }
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.dirty(), dirty);
    assert_eq!(doc.save(None).unwrap(), saved);
}

#[test]
fn a_hidden_layer_is_left_out() {
    let mut doc = effect();
    let hidden = layer(&run(&mut doc, "layer.newText", json!({ "text": "Hidden" })));
    run(&mut doc, "layer.setSwitch", json!({ "layers": [hidden], "switch": "video", "value": false }));
    assert_eq!(check_text(&doc).unwrap().text, "");
}

#[test]
fn keyframed_source_text_gives_each_keyframe_once() {
    let mut doc = effect();
    let title = layer(&run(&mut doc, "layer.newText", json!({ "text": "First" })));
    run(&mut doc, "prop.addKey", json!({ "layer": title, "path": "text/sourceText", "time": 0.0 }));
    run(&mut doc, "prop.addKey", json!({ "layer": title, "path": "text/sourceText", "time": 1.0, "value": "Second" }));
    run(&mut doc, "prop.addKey", json!({ "layer": title, "path": "text/sourceText", "time": 2.0, "value": "Second" }));
    assert_eq!(check_text(&doc).unwrap().text, "First\nSecond");
}

#[test]
fn an_empty_project_has_no_text() {
    let doc = effect();
    let answer = check_text(&doc).unwrap();
    assert_eq!(answer.to_json(), json!({ "text": "", "truncated": false }));
    let reopened = MotionDocument::open("Title.kmotion", &doc.save(None).unwrap()).unwrap();
    assert_eq!(reopened.text(DEFAULT_LIMIT).unwrap(), answer);
}

#[test]
fn the_headless_kit_lists_the_capability_and_answers_document_text() {
    let root = std::env::temp_dir().join(format!("kition-motion-text-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let file = root.join("Title.kmotion");
    std::fs::write(&file, titled().save(None).unwrap()).unwrap();
    let info = KitInfo::from_manifest(include_str!("../../../kit.json"), &["file.import"], kition_motion_kit::EDITOR_STATE).unwrap();
    let mut kit = Headless::<MotionDocument>::with_replica_port(info);
    assert_eq!(kit.capabilities()["capabilities"], json!([kition_kit::text::CAPABILITY]));
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let id = kit.open(OpenRequest { file: &folder.file("Title.kmotion").unwrap(), read_only: false, create: None }).unwrap().id;
    let answer = kit.text(&id, 1_048_576).unwrap();
    assert_eq!(answer["text"], json!("Main Title\nSubtitle here\nAna Lima\nDirector"));
    assert_eq!(answer["truncated"], json!(false));
    assert_eq!(answer["parts"].as_array().map(Vec::len), Some(2));
    let cut = kit.text(&id, 4).unwrap();
    assert_eq!(cut["text"], json!("Main"));
    assert_eq!(cut["truncated"], json!(true));
    assert!(!kit.document(&id).unwrap().dirty, "reading changes nothing");
    let _ = std::fs::remove_dir_all(root);
}
