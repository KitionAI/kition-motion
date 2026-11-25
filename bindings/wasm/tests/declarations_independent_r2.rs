//! Fresh handler-derived expectations. Family files are committed before declaration review.

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use std::collections::BTreeSet;

#[allow(dead_code)]
struct Row {
    command: &'static str,
    path: &'static str,
    resolves: &'static [&'static str],
    reads: &'static [&'static str],
    checks: &'static [&'static str],
    writes: &'static [&'static str],
    state: &'static [&'static str],
    source: &'static str,
}

macro_rules! families {
    ($($family:ident),+ $(,)?) => {
        $(mod $family {
            include!(concat!("declarations_independent_r2/", stringify!($family), ".rs"));
        })+
        fn families() -> Vec<(&'static str, &'static [Row])> {
            vec![$((stringify!($family), $family::ROWS)),+]
        }
    };
}

families!(
    align,
    align_data,
    anim,
    anim_tools,
    animation,
    app_more,
    autotrace,
    batch,
    camera_cmds,
    comp,
    comp_more,
    content_fill,
    create,
    edit,
    effect,
    expr_tools,
    file,
    file_more,
    focus,
    footage_panel,
    frame_export,
    frontend,
    key_labels,
    key_transform,
    keys_more,
    layer,
    layer_menu,
    layer_time,
    learn,
    link,
    liquify,
    lottie,
    markers,
    mask,
    mask_interp,
    model3d,
    models,
    paint,
    panels,
    path_nulls,
    paths,
    pinned_r2_animation,
    pinned_r2_render_queue,
    project_items,
    promo_camera,
    promo_carry,
    promo_ease,
    promo_light,
    promo_moves,
    prop,
    prop_groups,
    proxy,
    puppet,
    relink,
    render_queue,
    rig3d,
    roto_cmds,
    scene_detect,
    scripts,
    settings,
    shape_stroke,
    shape_tool,
    storage,
    styles,
    template_props,
    template_props_sync,
    templates,
    text_anim,
    text_edit,
    three_d,
    timeline,
    track,
    view,
    vpe,
    vr_editor,
    warp_cmds,
    watch_folder,
);

#[path = "declarations_r2_comparison/mod.rs"]
mod comparison;

#[test]
fn write_registry_inventory() {
    let doc = MotionDocument::create("blind-r2", 64, 64).unwrap();
    let writes: BTreeSet<_> = doc.commands().into_iter().filter(|command| command.mutates).map(|c| c.id).collect();
    let mut covered = BTreeSet::new();
    for (family, rows) in families() {
        let mut commands = BTreeSet::new();
        for row in rows {
            assert!(!row.path.is_empty() && !row.source.is_empty(), "{}", row.command);
            commands.insert(row.command);
            covered.insert(row.command.to_owned());
        }
        println!("FAMILY {family}: {} commands / {} paths", commands.len(), rows.len());
    }
    let missing: Vec<_> = writes.difference(&covered).collect();
    let extras: Vec<_> = covered.difference(&writes).collect();
    println!("REGISTRY {} covered {}; nonadvertised/editor entries {extras:?}", writes.len(), writes.intersection(&covered).count());
    assert!(missing.is_empty(), "unread advertised writes: {missing:?}");
}

#[test]
fn polling_and_batch_commands_advertise_the_writes_their_handlers_can_apply() {
    let document = MotionDocument::create("pending", 64, 64).unwrap();
    let commands = document.commands();
    for id in ["engine.batch", "job.list", "job.wait", "renderQueue.list", "camera.solveStatus", "roto.status", "track.status", "warp.status"] {
        assert!(commands.iter().find(|c| c.id == id).unwrap().mutates, "{id} can apply project edits");
    }
}
