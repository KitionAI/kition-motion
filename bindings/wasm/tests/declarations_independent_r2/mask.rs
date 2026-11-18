use super::Row;

macro_rules! edit_path {
    ($command:literal, $path:literal, $reads:expr, $checks:expr, $writes:expr) => {
        Row {
            command: $command,
            path: $path,
            resolves: &[
                "comp/layer through layer_p",
                "mask numeric uid then 1-based mask index, string exact mask name; fallback numeric shape Path group uid",
                "vertex/segment/feather index names sampled path position",
            ],
            reads: $reads,
            checks: $checks,
            writes: $writes,
            state: &["active_comp default", "selected_layers default", "current time for animated path", "preserve_constant_vertex_count for counted edits"],
            source: "commands/mask.rs:25,48,120; project/src/props.rs:126; keyframe evaluate; commands/paths.rs:19",
        }
    };
}

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "mask.new",
        path: "append new mask",
        resolves: &["comp/layer through layer_p"],
        reads: &["layer masks group membership/count", "project.next_id"],
        checks: &["vertices nonempty", "layer has masks group", "enabled requires selected layer"],
        writes: &["masks.children append new mask with count-derived name/color", "project.next_id", "selected_vertices last new vertex"],
        state: &["active_comp default", "selected_layers default", "prefs.appearance.cycle_mask_colors"],
        source: "commands/mask.rs:149; project/src/build.rs:72",
    },
    edit_path!(
        "mask.addVertex",
        "insert explicit point at index or append",
        &[
            "sampled path vertices/tangents/feather and topology",
            "path keys interpolation/spatial when animated",
            "layer start_time/stretch when animated",
            "group roto_bezier",
            "all other path key values when preserve count"
        ],
        &["point valid", "editable path exists"],
        &[
            "path vertices/tangents insert point",
            "feather segment references adjusted",
            "roto tangents recomputed if enabled",
            "path key at current time if animated else base value",
            "other key paths matching insert if preserve count",
            "selected_vertices inserted point"
        ]
    ),
    edit_path!(
        "mask.setVertex",
        "assign supplied vertex coordinates and/or tangents",
        &[
            "sampled path topology",
            "sampled path fields retained by partial assignment",
            "path interpolation and layer start_time/stretch if animated",
            "group roto_bezier and all vertices when roto"
        ],
        &["vertex index within vertices", "editable path exists"],
        &["supplied vertex point/in/out", "all tangents when roto", "base path value or animated key at current time"]
    ),
    edit_path!(
        "mask.setClosed",
        "assign closed flag",
        &[
            "sampled path.closed assigned",
            "path animated membership/interpolation and layer start_time/stretch if animated",
            "group roto_bezier and all vertices if roto"
        ],
        &["editable path exists"],
        &["sampled path.closed", "tangents if roto", "base value or animated current-time key"]
    ),
    Row {
        command: "mask.selectVertices",
        path: "editor selection only",
        resolves: &["active comp layer refs; mask uid and index accepted without existence check"],
        reads: &["layer membership for explicit refs"],
        checks: &["each vertex supplies layer/mask/index"],
        writes: &["selected_vertices replace or add/toggle"],
        state: &["active_comp", "selected_vertices when add"],
        source: "commands/mask.rs:226,244",
    },
    Row {
        command: "mask.moveVertices",
        path: "move explicit or selected vertices grouped by path",
        resolves: &[
            "active comp even comp parameter supplied",
            "vertices[].layer resolve_layer; mask exact group uid; index zero-based",
            "vertices omitted selected_vertices",
        ],
        reads: &["path sampled vertices/tangents/feather", "group kind and roto_bezier", "layer start_time/stretch and key interpolation when animated"],
        checks: &["enabled requires selected_vertices even explicit vertices", "delta valid", "missing layer/path or out-of-range vertex skipped"],
        writes: &["selected vertex coordinates", "all path tangents if roto", "base path or animated key at current time"],
        state: &["active_comp", "selected_vertices default", "current time for animated path"],
        source: "commands/mask.rs:226,283,313",
    },
    Row {
        command: "mask.deleteVertices",
        path: "remove explicit or selected vertices",
        resolves: &["active comp", "vertices[].layer ref, mask exact uid, index zero-based; omitted selection"],
        reads: &[
            "sampled path vertices/tangents/feather/topology/closed",
            "roto_bezier",
            "layer timing and interpolation if animated",
            "all other key paths if preserve count",
        ],
        checks: &["selected_vertices nonempty for enabled", "invalid path refs skipped", "vertex out of range skipped"],
        writes: &[
            "sampled path remove vertices/tangents and adjust feather",
            "closed false if fewer than 3 vertices",
            "other path key count when preserve enabled",
            "selected_vertices cleared",
        ],
        state: &["active_comp", "selected_vertices", "current time if animated", "prefs.general.preserve_constant_vertex_count"],
        source: "commands/mask.rs:327",
    },
    edit_path!(
        "mask.convertVertex",
        "toggle or force smooth/corner",
        &[
            "sampled path vertex topology and closed",
            "existing target tangents if smooth omitted",
            "neighbour vertices when smoothing",
            "group roto_bezier",
            "key interpolation and layer timing if animated"
        ],
        &["vertex index valid"],
        &["target in/out tangents; resize tangent lists", "all tangents if roto", "base or current-time path key"]
    ),
    edit_path!(
        "mask.insertVertex",
        "split cubic segment",
        &[
            "sampled path vertices, segment tangents, closed/topology/feather",
            "group roto_bezier",
            "key interpolation/layer timing if animated",
            "other path key values when preserving count"
        ],
        &["segment exists (last only if closed)"],
        &[
            "inserted vertex/tangents; neighbouring segment tangents; feather references",
            "roto tangents",
            "base/current-time path key",
            "other key paths insert at matching count position when preserve",
            "selected_vertices"
        ]
    ),
    edit_path!(
        "mask.featherPoint.add",
        "explicit segment",
        &["sampled path segment topology", "existing feather list", "group roto_bezier", "key interpolation/layer timing if animated"],
        &["segment exists"],
        &["append feather point (segment,t,radius,tension)", "roto tangents if enabled", "base/current-time path key"]
    ),
    edit_path!(
        "mask.featherPoint.add",
        "nearest point and inferred signed radius",
        &["sampled path all segment geometry and closed winding", "existing feather list", "group roto_bezier", "key interpolation/layer timing if animated"],
        &["path has segments", "point supplied when segment omitted"],
        &["append nearest-segment feather; radius from signed distance unless supplied", "roto tangents", "base/current-time path key"]
    ),
    edit_path!(
        "mask.featherPoint.set",
        "partial feather assignment, point/toward optional",
        &[
            "sampled path feather index/record",
            "segment geometry when point or toward supplied",
            "closed/winding when toward supplied",
            "segment count for explicit segment",
            "group roto_bezier",
            "key interpolation/layer timing if animated"
        ],
        &["feather index exists", "explicit segment exists", "path position for toward exists"],
        &["supplied feather segment/t/radius/tension; toward overrides radius", "roto tangents", "base/current-time path key"]
    ),
    edit_path!(
        "mask.featherPoint.remove",
        "one index or all",
        &["sampled path feather index/list", "group roto_bezier", "key interpolation/layer timing if animated"],
        &["index exists unless all"],
        &["remove feather or clear feather list", "roto tangents", "base/current-time path key"]
    ),
    Row {
        command: "mask.remove",
        path: "remove addressed mask",
        resolves: &["layer_p", "mask numeric uid then 1-based index or exact name"],
        reads: &["masks membership/index/name resolution"],
        checks: &["mask exists"],
        writes: &["masks.children remove complete group", "selected_vertices drop target layer"],
        state: &["active_comp default", "selected_layers default", "selected_vertices"],
        source: "commands/mask.rs:612",
    },
    Row {
        command: "mask.removeAll",
        path: "clear masks",
        resolves: &["layer_p"],
        reads: &["layer masks group membership"],
        checks: &["layer has masks group"],
        writes: &["masks.children clear", "selected_vertices drop layer"],
        state: &["active_comp default", "selected_layers default", "selected_vertices"],
        source: "commands/mask.rs:612",
    },
];
