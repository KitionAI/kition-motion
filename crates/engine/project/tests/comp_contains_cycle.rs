#![allow(clippy::unwrap_used, clippy::panic)]

use kition_motion_color::Label;
use kition_motion_project::{Comp, ItemKind, LayerSource, Project, build};
use kition_motion_time::{FrameRate, Tick};

#[test]
fn comp_contains_terminates_on_a_loaded_cycle() {
    const CHILD: &str = "KITION_C8_COMPOSITION_CYCLE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let mut project = Project::default();
        let comp = Comp::new(16, 16, FrameRate::FPS_30, Tick::from_seconds_f64(1.0));
        let a = project.add_item("A", Label::Sandstone, None, ItemKind::Comp(comp.clone().into()));
        let b = project.add_item("B", Label::Sandstone, None, ItemKind::Comp(comp.clone().into()));
        let unrelated = project.add_item("Unrelated", Label::Sandstone, None, ItemKind::Comp(comp.clone().into()));
        for (outer, inner) in [(a, b), (b, a)] {
            let layer = build::layer(&mut project, &comp, "Nested", LayerSource::Comp { item: inner }, (16, 16), None);
            project.comp_mut(outer).unwrap().layers.push(layer);
        }
        assert!(project.comp_contains(a, a));
        assert!(project.comp_contains(a, b));
        assert!(!project.comp_contains(a, unrelated));
        return;
    }
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "comp_contains_terminates_on_a_loaded_cycle", "--nocapture"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(child.status.success(), "cyclic composition traversal failed: {}", String::from_utf8_lossy(&child.stderr));
}
