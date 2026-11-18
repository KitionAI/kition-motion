//! Every model field typed with an engine id, and how the replica keeps it. Engine ids are the
//! objects' own data (minted from per-actor slots, `kdm_model::slot_base`), so an id stored as
//! data names the same thing on every replica; the references a merge must check or repair are
//! KDM `ref` fields as well. This lists every such field, so a new one fails here until it is
//! considered (and, if a merge can leave it dangling, repaired by `validate`).

const SOURCES: &[(&str, &str)] = &[
    ("lib.rs", include_str!("../../../engine/project/src/lib.rs")),
    ("props.rs", include_str!("../../../engine/project/src/props.rs")),
    ("template_props.rs", include_str!("../../../engine/project/src/template_props.rs")),
    ("tracking.rs", include_str!("../../../engine/project/src/tracking.rs")),
    ("render_queue.rs", include_str!("../../../engine/project/src/render_queue.rs")),
    ("render_templates.rs", include_str!("../../../engine/project/src/render_templates.rs")),
    ("styles.rs", include_str!("../../../engine/project/src/styles.rs")),
    ("sequence.rs", include_str!("../../../engine/project/src/sequence.rs")),
    ("build.rs", include_str!("../../../engine/project/src/build.rs")),
    ("keyframe.rs", include_str!("../../../engine/keyframe/src/lib.rs")),
    ("value.rs", include_str!("../../../engine/keyframe/src/value.rs")),
    ("text_doc.rs", include_str!("../../../engine/keyframe/src/text_doc.rs")),
];

/// `file: field` for every field whose type names an id, and how the bridge handles it.
const HANDLED: &[&str] = &[
    // An object's own identity: the integer `id`/`uid` field of its object, read back as is.
    "lib.rs: id: ItemId",
    "lib.rs: id: LayerId",
    "props.rs: uid: Uid",
    // Project.items: the `Item` objects themselves (keyed by their `id`).
    "lib.rs: items: BTreeMap<ItemId, Item>",
    // `ref` fields, translated to the target object's id and back: an item's folder
    // (`Item.parent`), a layer's source item (`Layer.sourceItem`, LayerSource's `item`), its
    // parent (`Layer.parent`) and track matte (`Layer.matteLayer`, TrackMatte's `layer`), a
    // Render Queue item's composition (`RenderItem.comp`). `validate` clears or removes what a
    // merge leaves dangling.
    "lib.rs: parent: Option<ItemId>",
    "lib.rs: item: ItemId",
    "lib.rs: parent: Option<LayerId>",
    "lib.rs: layer: LayerId",
    "render_queue.rs: comp: ItemId",
    // Ids stored as data inside a register: the same on every replica. A reference to a layer
    // or a property that a concurrent delete removed dangles exactly as it does after a local
    // delete (the engine resolves it to nothing): Graphics Templates controls and links, the
    // Template Properties overrides, a tracker's target layer, a Layer Control's value.
    "template_props.rs: layer: LayerId",
    "template_props.rs: prop: Uid",
    "template_props.rs: Media { layer: LayerId }",
    "props.rs: overridden: Vec<Uid>",
    "tracking.rs: target: Option<LayerId>",
    "value.rs: Layer(Option<u64>)",
    // Not model fields: parameters of functions written over several lines.
    "template_props.rs: comp: ItemId",
    "template_props.rs: over: &[Uid]",
    "template_props.rs: keep: &mut Vec<Uid>",
];

#[test]
fn every_id_field_is_handled() {
    let mut found = Vec::new();
    for (file, source) in SOURCES {
        for line in source.lines() {
            let line = line.trim();
            if line.starts_with("//")
                || line.starts_with("pub fn")
                || line.starts_with("pub(crate) fn")
                || line.starts_with("fn ")
                || line.contains("=>")
                || line.contains(" = ")
                || line.starts_with("use ")
                || line.starts_with("let ")
            {
                continue;
            }
            let line = line.strip_prefix("pub ").unwrap_or(line).trim_end_matches(',');
            if line.contains("Layer(Option<u64>)") {
                found.push(format!("{file}: {line}"));
                continue;
            }
            let names_id = ["ItemId", "LayerId", "Uid"].iter().any(|id| line.contains(id));
            if names_id
                && line.contains(": ")
                && !line.starts_with("struct ")
                && !line.starts_with("type ")
                && !line.contains("ItemId(")
                && !line.contains("LayerId(")
            {
                found.push(format!("{file}: {line}"));
            }
        }
    }
    found.sort();
    found.dedup();
    let unhandled: Vec<_> = found.iter().filter(|f| !HANDLED.contains(&f.as_str())).collect();
    assert!(unhandled.is_empty(), "model fields holding ids that kdm-model does not consider: {unhandled:#?}");
    let stale: Vec<_> = HANDLED.iter().filter(|h| !found.iter().any(|f| f == *h)).collect();
    assert!(stale.is_empty(), "listed as handled but no longer in the model (update this list): {stale:#?}");
}
