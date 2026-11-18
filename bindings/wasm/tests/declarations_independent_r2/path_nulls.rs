use super::Row;

macro_rules! path {
    ($command:literal,$path:literal,$reads:expr,$writes:expr) => {
        Row { command: $command, path: $path,
            resolves: &["comp id/name/default active", "explicit path/prop via prop_ref validates BASE Path kind; else selected_leaf_props (active comp) searched in resolved comp; else first visible Path in explicit/default selected layer tree order"],
            reads: $reads,
            checks: &["has_layers", "target Path at sampled time", "nonempty vertices", "property reference expressible"],
            writes: $writes,
            state: &["active_comp", "selected_layers/props", "current time for sampling", "layer insertion preferences"],
            source: "commands/path_nulls.rs:38,89,182,201,236,276; render/src/eval.rs:52,220; commands/link.rs:reference" }
    };
}
macro_rules! pin {
    ($command:literal,$writes:expr) => {
        Row { command: $command, path: "Puppet pin branch",
            resolves: &["pin/pins explicit UID or exact name via pins_p; else selected active-comp pins (ignores explicit comp); else all pins on target layer only if no valid path", "only Position/Advanced moving pin kinds retained"],
            reads: &["pin hierarchy/membership/kind eligibility", "pin Position sampled base/keys/expression dependency closure", "layer transform and parent chain at CTI; 3D projection camera/dimensions where applicable", "source layer/pin/property ancestor names for generated reference and null name", "all comp layer names unique naming", "layer.newNull comp dimensions/duration/default transform and insertion gap/preferences", "project.next_id"],
            checks: &["has_layers", "at least one moving pin with Position", "reference expressible"],
            writes: $writes,
            state: &["active_comp", "selected_layers/props", "current time", "layer insertion preferences"],
            source: "commands/path_nulls.rs:141,182,201,236; commands/puppet.rs:186,191,218,230; render/src/eval.rs:52,220" }
    };
}

pub(super) const ROWS: &[Row] = &[
    path!(
        "paths.nullsFollowPoints",
        "path vertex branch",
        &[
            "sampled full Path and expression dependency closure",
            "layer and ancestor transforms at CTI; 3D camera projection/comp dimensions",
            "source layer and property ancestor names determine reference and null names",
            "all comp layer names determine unique names",
            "comp dimensions/duration and insertion gap/preferences for newNull",
            "project.next_id"
        ],
        &[
            "one new null per vertex with world-space Position base and generated Position expression",
            "comp.layers insertion",
            "project.next_id",
            "selection/new-layer state from nested commands"
        ]
    ),
    pin!(
        "paths.nullsFollowPoints",
        &[
            "new null per moving pin with sampled Position and generated following expression",
            "comp.layers insertion",
            "project.next_id",
            "nested command selection state"
        ]
    ),
    path!(
        "paths.pointsFollowNulls",
        "path vertex branch",
        &[
            "sampled full Path and expression dependencies: vertices determine null positions; tangents/closed embedded in path expression",
            "layer and ancestor transforms/camera projection at CTI",
            "source layer/property names and all comp layer names",
            "comp dimensions/duration and insertion gap/preferences",
            "existing path.expr assigned field",
            "project.next_id"
        ],
        &[
            "one new null per vertex with Position",
            "source path.expr rebuilding observed tangents/closed",
            "comp.layers insertion",
            "project.next_id",
            "nested command selection state"
        ]
    ),
    pin!(
        "paths.pointsFollowNulls",
        &[
            "new null per moving pin with sampled Position",
            "each source pin Position.expr",
            "comp.layers insertion",
            "project.next_id",
            "nested command selection state"
        ]
    ),
    path!(
        "paths.tracePath",
        "path only, optional loop",
        &[
            "sampled path/expression dependency closure for first vertex position",
            "layer and ancestor transform/camera projection at CTI",
            "source layer/property names and all comp layer names",
            "source in_point/out_point determine Progress key times",
            "comp dimensions/duration/frame_rate and insertion gap/preferences",
            "Slider effect defaults and effect child ordering",
            "project.next_id"
        ],
        &[
            "new null Position base and expression",
            "Progress Slider effect/name, 0/100 keys at source span times",
            "optional slider.expr loopOut",
            "comp.layers insertion",
            "project.next_id",
            "nested command selection state"
        ]
    ),
];
