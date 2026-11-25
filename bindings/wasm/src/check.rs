//! What [`KitDocument::check`](kition_kit::KitDocument::check) reports on a project (KHP
//! A21): footage whose file is missing, layers whose source, parent, or track matte is gone,
//! layers that never show inside their composition's time, and empty compositions. Project items
//! are named as `item:<id>` and layers as `comp:<id>/layer:<id>` (the ids the engine's commands
//! take in `comp` and `layers`).
//!
//! Codes: `motion.missing-footage`, `motion.missing-source`, `motion.missing-parent`,
//! `motion.missing-matte`, `motion.zero-duration`, `motion.outside-comp`, `motion.empty-comp`, the
//! design advice on the active composition (`motion.flat-background`, `motion.no-depth`, … at
//! Info), and `motion.truncated` when a code is found more often than [`PER_CODE`].

use std::collections::{BTreeMap, BTreeSet};

use kition_kit::diagnostics::{Diagnostic, Severity, Target};
use kition_motion_project::{Comp, FootageKind, ItemId, ItemKind, LayerSource, Project};
use kition_motion_time::Tick;
use serde_json::json;

/// At most this many diagnostics of one code; the rest are counted in one `motion.truncated`.
pub const PER_CODE: usize = 100;

/// One code the check reports: its severity and a one-sentence summary for the catalogue.
pub struct Entry {
    pub code: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
}

const fn entry(code: &'static str, severity: Severity, summary: &'static str) -> Entry {
    Entry { code, severity, summary }
}

/// Every code [`check`] reports, errors first: the catalogue shipped as `agent/checks.json` (KHP
/// A38), with one conformance fixture per code (`conformance/check-<name>.kmotion`).
pub const CATALOGUE: &[Entry] = &[
    entry("motion.missing-footage", Severity::Error, "Footage cannot find its file, so the layers that use it show nothing."),
    entry("motion.missing-source", Severity::Error, "A layer uses a project item that no longer exists, so it shows nothing."),
    entry("motion.missing-parent", Severity::Error, "A layer is parented to a layer that no longer exists."),
    entry("motion.missing-matte", Severity::Error, "A layer uses a track matte layer that no longer exists."),
    entry("motion.zero-duration", Severity::Warning, "A layer ends where it starts, so it never shows."),
    entry("motion.outside-comp", Severity::Warning, "A layer lies outside its composition's duration, so it never shows."),
    entry("motion.empty-comp", Severity::Info, "A composition has no layers, so it renders only its background."),
    entry("motion.flat-background", Severity::Info, "The frame is one colour or a linear gradient, with no radial light or vignette, which reads as plain."),
    entry("motion.no-depth", Severity::Info, "No text or shape has a glow or a shadow, so everything sits flat on the background."),
    entry("motion.small-title", Severity::Info, "The largest text is under 7% of the frame height, small for a title in motion graphics."),
    entry("motion.weak-hierarchy", Severity::Info, "The two largest texts are too close in size to read as title and subtitle."),
    entry("motion.thin-type", Severity::Info, "A text is set in a light or regular weight, which reads faint on video."),
    entry("motion.low-contrast", Severity::Info, "A text has less than 4.5:1 contrast with what is behind it (3:1 for large text: 24 px, or 18.66 px bold)."),
    entry("motion.small-text", Severity::Info, "A text is under 3% of the frame height, hard to read in a video."),
    entry("motion.missing-font", Severity::Info, "A text asks for a font that is not installed, so it is drawn with a fallback."),
    entry("motion.static-text", Severity::Info, "A text never moves."),
    entry("motion.truncated", Severity::Info, "One code was found more often than the check lists; the rest are counted in one diagnostic."),
];

/// The catalogue as `agent/checks.json` holds it: code, severity, summary, and its fixture.
pub fn catalogue_json() -> serde_json::Value {
    serde_json::Value::Array(
        CATALOGUE
            .iter()
            .map(|entry| {
                json!({
                    "code": entry.code,
                    "severity": entry.severity,
                    "summary": entry.summary,
                    "fixture": format!("check-{}.kmotion", entry.code.trim_start_matches("motion.")),
                })
            })
            .collect(),
    )
}

/// Everything worth fixing in the project, errors first, and the design advice for the `active`
/// composition (the one `_export` renders; [`crate::design::diagnostics`]). `exists` says whether
/// a footage file can be read here.
pub fn check(project: &Project, active: Option<ItemId>, exists: &mut dyn FnMut(&str) -> bool) -> Vec<Diagnostic> {
    let mut found = Found::default();
    if let Some((id, comp)) = active.and_then(|id| project.comp(id).map(|comp| (id, comp))) {
        for diagnostic in crate::design::diagnostics(project, id, comp) {
            found.push(diagnostic);
        }
    }
    for (id, item) in &project.items {
        match &item.kind {
            ItemKind::Footage(footage) => {
                let files: Vec<&str> = if footage.kind == FootageKind::Sequence && !footage.sequence.is_empty() {
                    footage.sequence.iter().map(String::as_str).collect()
                } else {
                    vec![footage.path.as_str()]
                };
                // Footage made in the project (a placeholder, data typed in) has no file.
                let needs_file = !footage.path.is_empty() || !footage.sequence.is_empty();
                if footage.missing || (needs_file && files.iter().any(|file| file.is_empty() || !exists(file))) {
                    found.push(
                        Diagnostic::new(
                            "motion.missing-footage",
                            Severity::Error,
                            format!("Footage \"{}\" cannot find its file \"{}\", so the layers that use it show nothing.", item.name, footage.path),
                        )
                        .on(Target { path: Some(format!("item:{}", id.0)), ..Target::default() }),
                    );
                }
            }
            ItemKind::Comp(comp) => layers(project, *id, &item.name, comp, &mut found),
            ItemKind::Folder | ItemKind::Solid(_) => {}
        }
    }
    found.finish()
}

#[derive(Default)]
struct Found {
    list: Vec<Diagnostic>,
    counts: BTreeMap<String, usize>,
}

impl Found {
    fn push(&mut self, diagnostic: Diagnostic) {
        let count = self.counts.entry(diagnostic.code.clone()).or_default();
        *count += 1;
        if *count <= PER_CODE {
            self.list.push(diagnostic);
        }
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        for (code, count) in &self.counts {
            if *count > PER_CODE {
                self.list.push(Diagnostic::new("motion.truncated", Severity::Info, format!("{} more `{code}` diagnostics are not listed.", count - PER_CODE)));
            }
        }
        self.list.sort_by_key(|diagnostic| diagnostic.severity);
        self.list
    }
}

/// The project item a layer's source names, and whether that item is of the kind the source needs.
fn source_ok(project: &Project, source: &LayerSource) -> bool {
    let wanted = |item: &ItemId, ok: fn(&ItemKind) -> bool| project.items.get(item).is_some_and(|found| ok(&found.kind));
    match source {
        LayerSource::Footage { item } | LayerSource::Model { item } => wanted(item, |kind| matches!(kind, ItemKind::Footage(_))),
        LayerSource::Comp { item } => wanted(item, |kind| matches!(kind, ItemKind::Comp(_))),
        LayerSource::Solid { item } => wanted(item, |kind| matches!(kind, ItemKind::Solid(_))),
        _ => true,
    }
}

fn layers(project: &Project, comp_id: ItemId, comp_name: &str, comp: &Comp, found: &mut Found) {
    if comp.layers.is_empty() {
        found.push(
            Diagnostic::new("motion.empty-comp", Severity::Info, format!("Composition \"{comp_name}\" has no layers, so it renders only its background."))
                .on(Target { path: Some(format!("item:{}", comp_id.0)), ..Target::default() }),
        );
        return;
    }
    let ids: BTreeSet<u64> = comp.layers.iter().map(|layer| layer.id.0).collect();
    for layer in &comp.layers {
        let on = Target { path: Some(format!("comp:{}/layer:{}", comp_id.0, layer.id.0)), ..Target::default() };
        let name = &layer.name;
        let delete = json!({ "comp": comp_id.0, "layers": [layer.id.0] });
        if !source_ok(project, &layer.source) {
            found.push(
                Diagnostic::new(
                    "motion.missing-source",
                    Severity::Error,
                    format!("Layer \"{name}\" in \"{comp_name}\" uses a project item that no longer exists."),
                )
                .on(on.clone())
                .fix("edit.delete", Some(delete.clone())),
            );
        }
        if layer.parent.is_some_and(|parent| !ids.contains(&parent.0)) {
            found.push(
                Diagnostic::new(
                    "motion.missing-parent",
                    Severity::Error,
                    format!("Layer \"{name}\" in \"{comp_name}\" is parented to a layer that no longer exists."),
                )
                .on(on.clone())
                .fix("layer.setParent", Some(json!({ "comp": comp_id.0, "layers": [layer.id.0], "parent": null, "compensate": false }))),
            );
        }
        if layer.track_matte.as_ref().is_some_and(|matte| !ids.contains(&matte.layer.0)) {
            found.push(
                Diagnostic::new(
                    "motion.missing-matte",
                    Severity::Error,
                    format!("Layer \"{name}\" in \"{comp_name}\" uses a track matte layer that no longer exists."),
                )
                .on(on.clone())
                .fix("layer.setTrackMatte", Some(json!({ "comp": comp_id.0, "layer": layer.id.0, "matte": null }))),
            );
        }
        if layer.out_point <= layer.in_point {
            found.push(
                Diagnostic::new(
                    "motion.zero-duration",
                    Severity::Warning,
                    format!("Layer \"{name}\" in \"{comp_name}\" ends where it starts, so it never shows."),
                )
                .on(on)
                .fix("edit.delete", Some(delete)),
            );
        } else if layer.out_point <= Tick(0) || layer.in_point >= comp.duration {
            found.push(
                Diagnostic::new(
                    "motion.outside-comp",
                    Severity::Warning,
                    format!("Layer \"{name}\" in \"{comp_name}\" lies outside the composition's duration, so it never shows."),
                )
                .on(on),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MotionDocument;
    use kition_kit::KitDocument;
    use kition_motion_project::LayerId;

    fn project() -> MotionDocument {
        MotionDocument::create("Movie.kmotion", 320, 180).expect("project")
    }

    fn comp_mut(document: &mut MotionDocument) -> (ItemId, &mut Comp) {
        let id = document.session.active_comp_id().expect("a composition");
        let item = std::sync::Arc::make_mut(&mut document.session.project).items.get_mut(&id).expect("comp item");
        let ItemKind::Comp(comp) = &mut item.kind else { panic!("a composition") };
        (id, std::sync::Arc::make_mut(comp))
    }

    fn codes(document: &MotionDocument) -> Vec<String> {
        check(&document.session.project, document.session.active_comp_id(), &mut |_| true).into_iter().map(|d| d.code).collect()
    }

    fn solid(document: &mut MotionDocument) -> u64 {
        let made = document.execute("layer.newSolid", json!({ "name": "Back", "color": "#336699" })).expect("solid");
        made["layer"].as_u64().or_else(|| made["id"].as_u64()).expect("layer id")
    }

    #[test]
    fn a_new_project_reports_its_empty_composition_only() {
        let document = project();
        assert_eq!(codes(&document), ["motion.empty-comp"]);
        assert_eq!(document.check().iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["motion.empty-comp"]);
    }

    #[test]
    fn reports_layers_that_never_show() {
        let mut document = project();
        solid(&mut document);
        assert_eq!(codes(&document), Vec::<String>::new());
        let (comp_id, comp) = comp_mut(&mut document);
        let duration = comp.duration;
        let layer = &mut comp.layers[0];
        let lid = layer.id.0;
        layer.in_point = duration;
        layer.out_point = Tick(duration.0 + 10);
        let found = check(&document.session.project, document.session.active_comp_id(), &mut |_| true);
        assert_eq!(found.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["motion.outside-comp"]);
        assert_eq!(found[0].target.path.as_deref(), Some(format!("comp:{}/layer:{lid}", comp_id.0).as_str()));
        let (_, comp) = comp_mut(&mut document);
        comp.layers[0].out_point = comp.layers[0].in_point;
        let found = check(&document.session.project, document.session.active_comp_id(), &mut |_| true);
        assert_eq!(found[0].code, "motion.zero-duration");
        assert_eq!(found[0].fix.as_ref().map(|fix| fix.command.as_str()), Some("edit.delete"));
    }

    #[test]
    fn reports_missing_sources_parents_and_mattes() {
        let mut document = project();
        solid(&mut document);
        let (_, comp) = comp_mut(&mut document);
        let layer = &mut comp.layers[0];
        layer.source = LayerSource::Footage { item: ItemId(987_654) };
        layer.parent = Some(LayerId(123_456));
        layer.track_matte = Some(kition_motion_project::TrackMatte { layer: LayerId(123_457), kind: Default::default() });
        let mut found = codes(&document);
        found.sort();
        assert_eq!(found, ["motion.missing-matte", "motion.missing-parent", "motion.missing-source"]);
    }

    #[test]
    fn reports_footage_whose_file_is_gone() {
        let mut document = project();
        let id = ItemId(document.session.project.next_id + 1000);
        let footage: kition_motion_project::Footage = serde_json::from_value(json!({
            "path": "clips/beach.mp4", "kind": "Video", "width": 320, "height": 180, "pixel_aspect": 1.0,
            "frame_rate": { "num": 30, "den": 1 }, "duration": 0, "has_video": true, "has_audio": false, "alpha": "Straight",
        }))
        .expect("footage");
        let comp = document.session.active_comp_id().expect("a composition");
        let mut item = document.session.project.items[&comp].clone();
        (item.id, item.name, item.kind) = (id, "beach.mp4".into(), ItemKind::Footage(footage));
        std::sync::Arc::make_mut(&mut document.session.project).items.insert(id, item);
        let found = check(&document.session.project, document.session.active_comp_id(), &mut |file| file != "clips/beach.mp4");
        assert_eq!(found[0].code, "motion.missing-footage");
        assert_eq!(found[0].target.path.as_deref(), Some(format!("item:{}", id.0).as_str()));
        assert!(!check(&document.session.project, document.session.active_comp_id(), &mut |_| true).iter().any(|d| d.code == "motion.missing-footage"));
        kition_kit::diagnostics::check_diagnostics(&found).expect("valid diagnostics");
    }

    #[test]
    fn caps_each_code_and_says_how_many_more() {
        let mut document = project();
        solid(&mut document);
        let (_, comp) = comp_mut(&mut document);
        let template = comp.layers[0].clone();
        comp.layers = (0..PER_CODE as u64 + 3)
            .map(|n| {
                let mut layer = template.clone();
                layer.id = LayerId(10_000 + n);
                layer.parent = Some(LayerId(1));
                layer
            })
            .collect();
        let found = check(&document.session.project, document.session.active_comp_id(), &mut |_| true);
        assert_eq!(found.iter().filter(|d| d.code == "motion.missing-parent").count(), PER_CODE);
        let truncated: Vec<_> = found.iter().filter(|d| d.code == "motion.truncated").collect();
        assert_eq!(truncated.len(), 1);
        assert!(truncated[0].message.starts_with("3 more `motion.missing-parent`"));
    }
}
