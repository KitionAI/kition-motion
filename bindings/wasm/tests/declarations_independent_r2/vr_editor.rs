use super::Row;

pub(super) const ROWS: &[Row] = &[Row {
    command: "comp.vr.setView",
    path: "environment selected by output/cube/face numeric comp or active comp; sole-environment fallback",
    resolves: &[
        "comp:as_u64 only, else active_comp",
        "output comp->layer with Comp source and VR converter effect->cube comp->six face comps by cell->first camera per face",
    ],
    reads: &[
        "all project comp/layer membership/source/effect kind for environment discovery and sole-environment fallback",
        "cube.width determines cell size",
        "cube child layer transform.position.value locates face cell",
        "face child source selects first camera",
        "front camera.transform.orientation.value for omitted components",
        "each camera orientation property keys emptiness/hold_only/type/ui via prop.set",
        "each camera.start_time/stretch and comp.frame_rate via prop.set time conversion",
    ],
    checks: &[
        "one matching environment or unique environment",
        "all6 faces exist",
        "orientation array entries numeric",
        "each prop.set property/typing eligibility",
    ],
    writes: &["all6 cameras transform.orientation value or key through prop.set"],
    state: &["part for environment fallback", "time consumed by nested prop.set per face", "grouped undo"],
    source: "commands/vr_editor.rs:27-152; commands/prop.rs prop.set downstream",
}];
