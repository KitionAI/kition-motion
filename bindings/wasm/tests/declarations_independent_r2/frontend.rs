use super::Row;

macro_rules! frontend_rows {
    ($($id:literal => $check:literal),*) => { &[$(Row { command: $id, path: "frontend event only in engine; headless ignores event", resolves: &["params passed unchanged to frontend"], reads: &[], checks: &[$check, "Session::execute explicit-target enablement bypass applies"], writes: &["Event::Frontend(command,params)"], state: &["selected_layers and active_comp only where has_layers enablement required"], source: "crates/engine/engine/src/commands/frontend.rs:9; crates/engine/engine/src/commands/mod.rs:418; crates/engine/engine/src/lib.rs:609" },)*] };
}

pub(super) const ROWS: &[Row] = frontend_rows!(
    "layer.style.options" => "has_layers",
    "layer.openLayer" => "has_layers",
    "effect.manage" => "always",
    "animation.browsePresets" => "always",
    "project.deleteWithoutConfirmation" => "always"
);
