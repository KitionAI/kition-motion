use super::Row;

macro_rules! mask_option {
    ($command:literal,$field:literal) => {
        Row {
            command: $command,
            path: "mask option assignment",
            resolves: &[
                "selected vertex masks if no mask/layer; numeric shape Path uid accepted by masks_for but edit_mask_groups subsequently searches masks only",
                "else layer_menu::target_masks: explicit mask uid/index/name across layers; selected groups/properties else all masks",
            ],
            reads: &["target group/mask membership and group.kind"],
            checks: &["target list nonempty", "mode recognized"],
            writes: &[$field],
            state: &["active_comp", "selected_layers default", "selected_props", "selected_vertices"],
            source: "commands/paths.rs:61,79,133; commands/layer_menu.rs:294",
        }
    };
}

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "path.rotoBezier",
        path: "explicit or toggle all masks",
        resolves: &["masks_for selected vertex masks, shape Path uid else target_masks selected or explicit masks"],
        reads: &["target mask roto_bezier flags when value omitted (all-on toggle)", "base and every keyed path vertices/closed when enabling"],
        checks: &["targets nonempty", "edit only groups found under masks even if shape Path target was resolved"],
        writes: &["mask.kind.roto_bezier", "all base/key path tangents when enabling"],
        state: &["active_comp", "selected_layers", "selected_props", "selected_vertices"],
        source: "commands/paths.rs:61,79,99; commands/layer_menu.rs:294",
    },
    Row {
        command: "path.convertToBezier",
        path: "clear roto flag",
        resolves: &["masks_for as rotoBezier"],
        reads: &["mask group membership/kind"],
        checks: &["target list nonempty"],
        writes: &["mask.kind.roto_bezier false"],
        state: &["active_comp", "selected_layers", "selected_props", "selected_vertices"],
        source: "commands/paths.rs:117",
    },
    mask_option!("layer.mask.motionBlur", "mask.kind.motion_blur"),
    mask_option!("layer.mask.featherFalloff", "mask.kind.feather_falloff"),
    Row {
        command: "layer.mask.hideLocked",
        path: "editor only",
        resolves: &[],
        reads: &["hide_locked_masks when value omitted"],
        checks: &["active comp"],
        writes: &["editor hide_locked_masks"],
        state: &["hide_locked_masks", "active_comp"],
        source: "commands/paths.rs:158",
    },
    Row {
        command: "path.setFirstVertex",
        path: "rotate all mask path values",
        resolves: &["explicit layer resolved via layer_p but write comp is ACTIVE comp", "mask exact uid", "index explicit or exactly one selected vertex"],
        reads: &["base/every key path topology/closed and vertex/tangent arrays"],
        checks: &["exactly one selected vertex for enabled", "mask exists under active comp", "base path must be closed; out-of-range key path index skipped"],
        writes: &["base/key path vertex and tangent order rotated", "selected_vertices becomes vertex zero"],
        state: &["active_comp", "selected_vertices", "selected_layers default"],
        source: "commands/paths.rs:171",
    },
    Row {
        command: "path.freeTransform",
        path: "selected points or whole addressed mask",
        resolves: &["masks_for", "selected_vertices valid indices on each target else every point"],
        reads: &[
            "sampled path vertices/tangents and topology",
            "bbox of selected points if anchor omitted",
            "roto flag",
            "layer timing and key interpolation if animated",
        ],
        checks: &["scale/rotation/offset transform nonidentity", "editable path group exists; otherwise skipped"],
        writes: &["sampled selected vertex/tangent affine transform", "roto all tangents if enabled", "base path or current-time key"],
        state: &["active_comp", "selected_layers", "selected_props", "selected_vertices", "current time if animated"],
        source: "commands/paths.rs:210",
    },
    Row {
        command: "path.groupShapes",
        path: "group same-parent items in first target layer",
        resolves: &[
            "active comp; explicit items uids use layer_p layer but active comp retained",
            "else selected shape-content item groups",
            "first item's layer only",
        ],
        reads: &[
            "layer.source for selected-items branch",
            "contents tree ancestry",
            "item sibling membership/order",
            "sibling group count for default name",
            "project.next_id",
        ],
        checks: &["selected shape item for enabled", "items nonempty and siblings share parent"],
        writes: &["parent children moved into new group contents at first source position", "project.next_id", "selected_props new group"],
        state: &["active_comp", "selected_layers", "selected_props"],
        source: "commands/paths.rs:296,333; project/src/build.rs shape_group",
    },
    Row {
        command: "path.ungroupShapes",
        path: "bake static translation into children",
        resolves: &["same selected_shape_items as grouping; only match_id group"],
        reads: &[
            "group contents full records",
            "transform keys/expr eligibility",
            "transform anchor/position/scale/rotation/skew/opacity",
            "position values and key values in immediate child shape items",
            "parent membership/order",
        ],
        checks: &["group selected", "group transform unanimated unit-scale/zero-rotation/zero-skew/full-opacity"],
        writes: &[
            "remove group; insert inner children at same index",
            "offset child position/path/gradient/twist values and keys by position-anchor",
            "selected_props inner groups",
        ],
        state: &["active_comp", "selected_layers", "selected_props"],
        source: "commands/paths.rs:372,407",
    },
];
