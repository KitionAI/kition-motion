//! Schema 1 (task schema-freeze): what people add and edit one at a time is replicated one at a
//! time. A layer's switches are one register each (a `map`); a composition's and a layer's
//! markers, a composition's guides and its Graphics Templates controls are lists of objects of
//! their own, a marker's comment a `text` field merged by character; and what one machine's
//! renders leave on the Render Queue, and its Render Queue preferences, stay on that machine.
//! Each case with a draft: the document and a draft forked from it edit, the draft merges (data
//! model 6.6), and both edits survive; typing in one text on both sides is refused (rule D5).

mod common;

use common::{assert_same_content, assert_view_is_replica, demo, draft, first, layer, layer_mut, merge, new_replica, person, read, record, seconds, try_merge};
use kdm_model::MotionModel;
use kition_kit::kdm::{FieldKind, ObjectId};
use kition_kit::session::KitModel;
use kition_motion_color::Label;
use kition_motion_project::render_queue::{RenderQueueItem, RenderStatus};
use kition_motion_project::template_props::{EgControl, EgKind, GraphicsTemplate};
use kition_motion_project::{Guide, ItemId, Marker, Project};

fn comp_mut(project: &mut Project, id: ItemId) -> &mut kition_motion_project::Comp {
    project.comp_mut(id).unwrap_or_else(|| panic!("no comp {id:?}"))
}

fn marker(time: f64, comment: &str) -> Marker {
    Marker { time: seconds(time), comment: comment.into(), ..Default::default() }
}

fn note(id: u64, text: &str) -> EgControl {
    EgControl { id, name: format!("Note {id}"), kind: EgKind::Comment { text: text.into() }, as_type: None }
}

#[test]
fn the_schema_is_version_1_with_lists_maps_and_text_for_what_is_edited_one_at_a_time() {
    let schema = MotionModel::schema();
    assert_eq!(schema.schema_version, 1);
    let kit: serde_json::Value = serde_json::from_str(include_str!("../../../../kit.json")).unwrap();
    assert_eq!(kit["model"]["schemaVersion"], 1, "kit.json names the schema's version");
    let field = |ty: &str, name: &str| {
        let def = &schema.types[ty].fields[name];
        (def.kind, def.of.clone())
    };
    assert_eq!(field("Comp", "markers"), (FieldKind::List, Some("Marker".into())));
    assert_eq!(field("Comp", "guides"), (FieldKind::List, Some("Guide".into())));
    assert_eq!(field("Comp", "controls"), (FieldKind::List, Some("Control".into())));
    assert_eq!(field("Control", "children"), (FieldKind::List, Some("Control".into())));
    assert_eq!(field("Layer", "markers"), (FieldKind::List, Some("Marker".into())));
    assert_eq!(field("Layer", "switches").0, FieldKind::Map);
    assert_eq!(field("Marker", "comment").0, FieldKind::Text);
}

#[test]
fn none_of_it_is_left_in_attrs() {
    let mut d = demo();
    layer_mut(&mut d.project, d.main, d.red).markers = vec![marker(1.0, "beat")];
    let c = comp_mut(&mut d.project, d.main);
    c.markers = vec![marker(0.0, "start")];
    c.guides = vec![Guide { vertical: true, position: 10.0 }];
    c.graphics_template = Some(GraphicsTemplate { name: "T".into(), controls: vec![note(9001, "hi")] });
    d.project.render_prefs.notify = true;
    let (replica, ids) = new_replica(&d.project);
    let red = ids.layers.iter().find(|(key, _)| key.id == d.red).map(|(_, l)| l.object.clone()).unwrap();
    for key in ["markers", "switches"] {
        assert!(replica.map_get(&red, "attrs", key).unwrap().is_null(), "Layer.attrs.{key}");
    }
    assert_eq!(replica.map_get(&red, "switches", "video").unwrap(), serde_json::json!(true), "one register per switch");
    assert_eq!(replica.list(&red, "markers").unwrap().len(), 1);
    let comp = ids.comps[&d.main].clone();
    for key in ["markers", "guides"] {
        assert!(replica.map_get(&comp, "attrs", key).unwrap().is_null(), "Comp.attrs.{key}");
    }
    assert_eq!(replica.map_get(&comp, "attrs", "graphics_template").unwrap(), serde_json::json!({ "name": "T" }), "the template keeps only its name");
    assert_eq!(replica.list(&comp, "markers").unwrap().len(), 1);
    assert_eq!(replica.list(&comp, "guides").unwrap().len(), 1);
    assert_eq!(replica.list(&comp, "controls").unwrap().len(), 1);
    let start = replica.list(&comp, "markers").unwrap()[0].clone();
    assert_eq!(replica.text(&start, "comment").unwrap(), "start");
    assert!(replica.map_get(&ObjectId::from_string("project"), "attrs", "render_prefs").unwrap().is_null(), "Render Queue preferences stay local");
}

#[test]
fn two_switches_of_one_layer_toggled_by_a_draft_and_the_document_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after = a.model().project.clone();
    layer_mut(&mut after, d.main, d.red).switches.locked = true;
    record(&mut a, &person("a"), &after);
    let mut after = b.model().project.clone();
    layer_mut(&mut after, d.main, d.red).switches.shy = true;
    record(&mut b, &person("b"), &after);
    merge(&mut a, &b, &base);
    let switches = &layer(&a.model().project, d.main, d.red).switches;
    assert!(switches.locked && switches.shy, "both switches");
}

#[test]
fn composition_markers_added_by_a_draft_and_the_document_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after = a.model().project.clone();
    comp_mut(&mut after, d.main).markers.push(marker(2.0, "a"));
    record(&mut a, &person("a"), &after);
    let mut after = b.model().project.clone();
    comp_mut(&mut after, d.main).markers.push(marker(1.0, "b"));
    record(&mut b, &person("b"), &after);
    merge(&mut a, &b, &base);
    let comments: Vec<String> = a.model().project.comp(d.main).unwrap().markers.iter().map(|m| m.comment.clone()).collect();
    assert_eq!(comments, ["b", "a"], "both, in time order");
}

#[test]
fn layer_markers_added_by_a_draft_and_the_document_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after = a.model().project.clone();
    layer_mut(&mut after, d.main, d.green).markers.push(marker(0.5, "a"));
    record(&mut a, &person("a"), &after);
    let mut after = b.model().project.clone();
    layer_mut(&mut after, d.main, d.green).markers.push(marker(1.5, "b"));
    record(&mut b, &person("b"), &after);
    merge(&mut a, &b, &base);
    let comments: Vec<String> = layer(&a.model().project, d.main, d.green).markers.iter().map(|m| m.comment.clone()).collect();
    assert_eq!(comments, ["a", "b"]);
}

#[test]
fn one_marker_edited_by_a_draft_and_the_document_merges_field_by_field() {
    let mut d = demo();
    comp_mut(&mut d.project, d.main).markers = vec![marker(1.0, "hello world"), marker(3.0, "other")];
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after = a.model().project.clone();
    comp_mut(&mut after, d.main).markers[0].comment = "hello brave world".into();
    record(&mut a, &person("a"), &after);
    let mut after = b.model().project.clone();
    let m = &mut comp_mut(&mut after, d.main).markers[0];
    m.label = Label::Blue;
    m.duration = seconds(0.5);
    record(&mut b, &person("b"), &after);
    merge(&mut a, &b, &base);
    let markers = &a.model().project.comp(d.main).unwrap().markers;
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].comment, "hello brave world", "the document's comment edit");
    assert_eq!((markers[0].label, markers[0].duration), (Label::Blue, seconds(0.5)), "the draft's other fields");
    assert_eq!(markers[1].comment, "other");
}

/// Rule D5: a draft that typed in a marker's comment the document also changed is refused as a
/// whole; the document keeps its own text.
#[test]
fn typing_in_one_comment_on_both_sides_is_refused() {
    let mut d = demo();
    comp_mut(&mut d.project, d.main).markers = vec![marker(1.0, "hello world")];
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after = a.model().project.clone();
    comp_mut(&mut after, d.main).markers[0].comment = "hello brave world".into();
    record(&mut a, &person("a"), &after);
    let mut after = b.model().project.clone();
    comp_mut(&mut after, d.main).markers[0].comment = "hello world!".into();
    record(&mut b, &person("b"), &after);
    let version = a.version();
    assert!(try_merge(&mut a, &b, &base).is_err(), "both typed in one text");
    assert_eq!(a.version(), version, "nothing changed");
    assert_eq!(a.model().project.comp(d.main).unwrap().markers[0].comment, "hello brave world");
    assert_view_is_replica(&a);
}

#[test]
fn a_marker_moved_in_time_keeps_its_object() {
    let mut d = demo();
    comp_mut(&mut d.project, d.main).markers = vec![marker(1.0, "one"), marker(2.0, "two")];
    let mut a = first(&d.project);
    let comp = a.model().ids().comps[&d.main].clone();
    let before = a.replica().list(&comp, "markers").unwrap();
    let mut after = a.model().project.clone();
    comp_mut(&mut after, d.main).markers[0].time = seconds(1.5);
    record(&mut a, &person("a"), &after);
    assert_eq!(a.replica().list(&comp, "markers").unwrap(), before, "the same two objects");
    assert_same_content(&read(a.replica()), &after, "the replica holds the move");
}

#[test]
fn guides_added_by_a_draft_and_the_document_are_both_kept() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after = a.model().project.clone();
    comp_mut(&mut after, d.main).guides.push(Guide { vertical: true, position: 100.0 });
    record(&mut a, &person("a"), &after);
    let mut after = b.model().project.clone();
    comp_mut(&mut after, d.main).guides.push(Guide { vertical: false, position: 50.0 });
    record(&mut b, &person("b"), &after);
    merge(&mut a, &b, &base);
    assert_eq!(a.model().project.comp(d.main).unwrap().guides.len(), 2, "both guides");
}

#[test]
fn template_controls_added_and_renamed_by_a_draft_and_the_document_are_all_kept() {
    let mut d = demo();
    let group = EgControl { id: 9100, name: "Group".into(), kind: EgKind::Group { children: vec![note(9101, "inside")] }, as_type: None };
    comp_mut(&mut d.project, d.main).graphics_template = Some(GraphicsTemplate { name: "Lower third".into(), controls: vec![note(9001, "first"), group] });
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let mut after = a.model().project.clone();
    {
        let template = comp_mut(&mut after, d.main).graphics_template.as_mut().unwrap();
        template.controls.push(note(9002, "from a"));
        template.controls[0].name = "Renamed".into();
    }
    record(&mut a, &person("a"), &after);
    let mut after = b.model().project.clone();
    {
        let template = comp_mut(&mut after, d.main).graphics_template.as_mut().unwrap();
        template.controls.push(note(9003, "from b"));
        if let EgKind::Group { children } = &mut template.controls[1].kind {
            children.push(note(9102, "also inside"));
        }
    }
    record(&mut b, &person("b"), &after);
    merge(&mut a, &b, &base);
    let template = a.model().project.comp(d.main).unwrap().graphics_template.clone().unwrap();
    assert_eq!(template.name, "Lower third");
    let ids: Vec<u64> = template.controls.iter().map(|c| c.id).collect();
    assert_eq!(ids.len(), 4, "{ids:?}");
    assert!(ids.contains(&9002) && ids.contains(&9003), "both added controls: {ids:?}");
    assert_eq!(template.controls[0].name, "Renamed");
    let Some(EgKind::Group { children }) = template.controls.iter().find(|c| c.id == 9100).map(|c| &c.kind) else { panic!("the group") };
    assert_eq!(children.iter().map(|c| c.id).collect::<Vec<_>>(), [9101, 9102], "b's child of the group");
}

#[test]
fn a_render_on_one_machine_stays_there() {
    let mut d = demo();
    d.project.render_queue.push(RenderQueueItem::new(1, d.main));
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    let version = a.version();
    let mut after = a.model().project.clone();
    after.render_queue[0].status = RenderStatus::Done;
    after.render_queue[0].started = Some(1_700_000_000);
    after.render_queue[0].last_output = Some("/tmp/out.mov".into());
    after.render_prefs.notify = true;
    record(&mut a, &person("a"), &after);
    assert_eq!(a.version(), version, "a render's status and the preferences write nothing to the replica");
    // A draft's edit merges: this machine's status and preferences stay as they were.
    let mut theirs = b.model().project.clone();
    theirs.render_queue[0].settings.name = "Best".into();
    record(&mut b, &person("b"), &theirs);
    merge(&mut a, &b, &base);
    let mine = &a.model().project;
    assert_eq!(mine.render_queue[0].status, RenderStatus::Done);
    assert_eq!(mine.render_queue[0].last_output.as_deref(), Some("/tmp/out.mov"));
    assert!(mine.render_prefs.notify);
    assert_eq!(mine.render_queue[0].settings.name, "Best", "the draft's change of the definition merged");
    let other = &b.model().project;
    assert_eq!(other.render_queue[0].status, RenderStatus::Queued, "not rendered on b");
    assert!(other.render_queue[0].last_output.is_none() && !other.render_prefs.notify);
    assert_same_content(mine, other, "the same document");
}

#[test]
fn render_queue_ids_are_minted_per_author() {
    let d = demo();
    let mut a = first(&d.project);
    let (mut b, base) = draft(&a, 2);
    for (session, name) in [(&mut a, "a"), (&mut b, "b")] {
        let mut after = session.model().project.clone();
        let id = after.alloc();
        after.render_queue.push(RenderQueueItem::new(id, d.main));
        record(session, &person(name), &after);
    }
    merge(&mut a, &b, &base);
    let ids: Vec<u64> = a.model().project.render_queue.iter().map(|r| r.id).collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1], "each author's own id");
    assert!(ids.iter().all(|id| *id < 1 << 52), "neither needed a repair id: {ids:?}");
}
