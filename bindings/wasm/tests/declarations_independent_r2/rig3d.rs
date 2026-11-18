use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "camera.stereoRig",
        path: "active camera at CTI or create master, two eye comps and output comp",
        resolves: &[
            "comp_id; active_camera at CTI (topmost active camera), no explicit camera param",
            "new master insertion above selection if absent; controls gap above master",
            "new project item names global unique; parent folder from source comp",
        ],
        reads: &[
            "source comp settings cloned except layers/markers; source item.name/parent",
            "camera eligibility/source/switches/in/out and stack order for active camera",
            "master evaluated position/poi/orientation/rotations/zoom and parent transforms at CTI, start/stretch and expression reference closure",
            "master auto_orient/poi presence selects two-node expression formula; all layer/project names for new names",
            "stereo/glasses effect defaults; source item label for nested build defaults",
        ],
        checks: &["comp exists; stereo option enums valid; control/glasses specs exist; no camera locks check"],
        writes: &[
            "optional new master and new control null/effect in source comp",
            "two new eye comps with copied settings, camera+collapsed source nest; expressions naming source comp/master/control",
            "new output comp with eye layers/glasses references; project.next_id",
        ],
        state: &[
            "active_comp if omitted; CTI always; selected_layers insertion default for absent master; writes selection during insertion and project_selection=output",
        ],
        source: "crates/engine/engine/src/commands/rig3d.rs:196; crates/engine/engine/src/commands/rig3d.rs:76; crates/render/render/src/three_d/camera.rs:281; crates/render/render/src/three_d/camera.rs:294",
    },
    Row {
        command: "camera.orbitNull",
        path: "new null at evaluated view target, compensate stored/keyed camera position/poi",
        resolves: &["comp_id; layer/camera resolve_layer else first selected camera else active_camera at CTI", "gap directly above camera"],
        reads: &[
            "target source camera and parent=None eligibility",
            "camera evaluated view frame and poi/zoom focus at CTI including transform keys/expressions, timing and expression reference closure",
            "camera stored and EVERY-key position/poi values",
            "comp width/height/duration and all layer names for null defaults/name",
        ],
        checks: &["target camera; camera unparented; no lock/expression check"],
        writes: &[
            "new 3D visible null at target",
            "camera.parent=new null; stored and every-key position/poi translated into null space; expressions and key tangents retained",
            "project.next_id",
        ],
        state: &["active_comp if omitted; selected_layers for camera fallback; CTI always; writes selected_layers=[null]"],
        source: "crates/engine/engine/src/commands/rig3d.rs:313; crates/engine/engine/src/commands/rig3d.rs:56; crates/render/render/src/three_d/camera.rs:281",
    },
    Row {
        command: "light.controlWithCamera",
        path: "on true, absolute parent/static transform assignment",
        resolves: &[
            "comp_id; layers/layer resolve_layer scalar/array, unresolved members filtered; absent selected LIGHT layers",
            "camera explicit resolve_layer or active_camera at CTI",
        ],
        reads: &[
            "all target source Light and target camera source Camera eligibility",
            "first target.parent only if on omitted to toggle",
            "active camera eligibility/order/in/out at CTI when camera omitted",
        ],
        checks: &["nonempty targets, all lights; camera exists/is_camera; no locks check"],
        writes: &[
            "light.parent=camera; auto_orient Off",
            "position/poi/orientation/rotationX/rotationY/rotation static values; clears those keys and expressions",
        ],
        state: &["active_comp/selected_layers defaults; CTI for camera fallback; light frame evaluation also occurs but on=true result does not use it"],
        source: "crates/engine/engine/src/commands/rig3d.rs:359; crates/engine/engine/src/commands/rig3d.rs:25",
    },
    Row {
        command: "light.controlWithCamera",
        path: "on false releases only lights parented to chosen camera",
        resolves: &["same light/camera addresses as on path"],
        reads: &[
            "target.parent==camera eligibility per target; first parent for omitted on",
            "released light world view frame at CTI, own and parent transform chains, timing/auto-orient/expressions",
            "comp width/height for poi distance; transform poi property presence selects auto-orient",
        ],
        checks: &["same light/camera validation; differently-parented lights skipped"],
        writes: &[
            "released target.parent=None; position/poi static world values clearing their keys/expressions",
            "auto_orient TowardsPointOfInterest when poi exists; orientation/rotation retained",
        ],
        state: &["same address defaults; CTI always determines released placement"],
        source: "crates/engine/engine/src/commands/rig3d.rs:375; crates/engine/engine/src/commands/rig3d.rs:401; crates/render/render/src/three_d/camera.rs:281",
    },
    Row {
        command: "camera.fromModel",
        path: "host model rest-pose cameras transformed through model layer at CTI",
        resolves: &[
            "comp_id; layers/layer scalar/array resolve_layer filters invalid members; absent selected MODEL layers",
            "source item ID from each Model source; insertion above selection",
        ],
        reads: &[
            "all target source Model eligibility; source Footage and host model bytes",
            "model reachable node hierarchy rest TRS/matrices and camera projections/names (animation clip None, time0)",
            "model layer unitScale and world transform/parent ancestry/auto-orient at CTI with expressions/timing",
            "comp width/height/duration and names for camera defaults/unique names",
        ],
        checks: &["nonempty all-model targets; missing footage skipped; unreadable model errors; at least one reachable camera; no locks check"],
        writes: &["new camera layers position/poi/zoom/focusDistance from model; project.next_id"],
        state: &["active_comp/selected_layers when omitted; selected_layers insertion gap; CTI always; writes layer selection as inserted"],
        source: "crates/engine/engine/src/commands/rig3d.rs:436; crates/engine/model/src/lib.rs:646; crates/render/render/src/three_d/adv/scene.rs:431",
    },
    Row {
        command: "light.fromModel",
        path: "host model rest-pose lights transformed through model layer at CTI",
        resolves: &["same model target/source/insertion addresses as camera.fromModel"],
        reads: &[
            "same model eligibility/world transform dependencies as camera.fromModel",
            "reachable rest nodes and light kind/name/color/intensity/spot inner+outer/range",
            "comp width/height for poi distance, defaults/duration and names",
        ],
        checks: &["same nonempty/model-readable checks; at least one reachable light"],
        writes: &["new light layers with evaluated position/poi and model kind/color/intensity/cone/falloff options; project.next_id"],
        state: &["active_comp/selected_layers defaults and insertion; CTI always; writes inserted selection"],
        source: "crates/engine/engine/src/commands/rig3d.rs:436; crates/engine/engine/src/commands/rig3d.rs:485; crates/engine/model/src/lib.rs:646",
    },
    Row {
        command: "light.environmentBackground",
        path: "source property reference else first environment layer, append backdrop",
        resolves: &[
            "comp_id; layer resolve_layer must environment light; absent first selected environment light else first in comp",
            "lightOptions/source.value_at(light.layer_time(CTI)) Layer ID else FIRST layer.environment",
            "bottom stack insertion gap",
        ],
        reads: &[
            "environment light source kind; source property keys/value and light start/stretch (no expressions evaluated for source)",
            "candidate environment switches/stack order for fallback",
            "resolved source layer.source must footage/comp/solid; source layer.name and item dimensions/label",
            "comp width/height/duration and layer names for build defaults",
        ],
        checks: &["environment light found; source found; source allowed kind; no locks check"],
        writes: &["new 3D environment_background layer with same source, defaults (does not clone source transforms/timing); project.next_id"],
        state: &["active_comp if omitted; selected_layers light default; CTI always; writes selected_layers=[new backdrop]"],
        source: "crates/engine/engine/src/commands/rig3d.rs:520; crates/engine/project/src/build.rs:737",
    },
];
