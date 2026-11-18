use super::Row;

pub(super) const ROWS: &[Row] = &[
    Row {
        command: "*Session.edit",
        path: "global Template Properties link synchronization after handler closure",
        resolves: &[
            "EVERY comp graphics_template.flat Property.kind.targets(main + links), excluding Font controls and <2 targets",
            "existing source/destination layer/property IDs from control bindings",
        ],
        reads: &[
            "before/after property.value/keys determines leading edited target, main wins when several changed; otherwise main leads",
            "lead and other target value TYPE/ui determine linkable; lead.value/keys used to assign other targets",
            "lead start/stretch comp_time and destination start/stretch layer_time map every copied key",
            "all existing link definitions and referenced target existence",
        ],
        checks: &["lead exists; target exists and supported equal natural control type plus value discriminant; unsupported pair skipped"],
        writes: &["other linked property.value/keys copied from lead with key times converted; expressions retained"],
        state: &["runs after EVERY successful edit closure, before project installation; no CTI"],
        source: "crates/engine/engine/src/lib.rs:684; crates/engine/project/src/template_props.rs:336",
    },
    Row {
        command: "*Session.edit",
        path: "global edited instance mirror propagates to master and override detection",
        resolves: &["EVERY precomp instance.source comp graphics_template controls; mirror chains max8; eg<control> children in old/new instance groups"],
        reads: &[
            "source control Mirror binding; instance mirror/master before/after value/keys/expr determines changed-mirror and unchanged-master eligibility",
            "old/new instance overridden UID sets; every child before/after value/keys/expr determines fresh override",
        ],
        checks: &[
            "both comp versions and both instance groups exist; edited mirror and unchanged master; already overridden children excluded from fresh marking",
        ],
        writes: &["master instance property.value/keys/expr copied from edited mirror", "fresh changed child UIDs appended to instance overridden list"],
        state: &["every edit; no CTI"],
        source: "crates/engine/engine/src/lib.rs:684; crates/engine/project/src/template_props.rs:511; crates/engine/project/src/template_props.rs:551",
    },
    Row {
        command: "*Session.edit",
        path: "global rebuild of every precomp instance Template Properties group",
        resolves: &[
            "EVERY comp layer.source Comp item -> source.graphics_template.has_instance_controls",
            "source control tree main property/media layer IDs; mirrors resolve max8; instance previous eg<control> nodes",
        ],
        reads: &[
            "source graphics_template control IDs/names/kinds/order/group structure/as_type; main property metadata/full look with value_at(Tick::ZERO)",
            "source Media bound layer.source Footage item ID; source property existence",
            "old instance group UID/child UID/overridden state and overridden values/keys/expr",
            "source mirror master child newly built value/keys/expr and overridden status; existing templateProps group position",
        ],
        checks: &["source has instance controls else remove old group; missing bound properties/media omitted; unchanged built group skipped"],
        writes: &[
            "replace/add/remove instance templateProps group; new UIDs allocated as needed",
            "non-overridden child metadata from source, value_at(0), keys empty/expr None; static_only=false for properties",
            "overridden value/keys/expr retained, Media scalar/static-only; mirrors copy master values/keys/expr/override state; project.next_id",
        ],
        state: &["every edit; no CTI"],
        source: "crates/engine/engine/src/lib.rs:684; crates/engine/project/src/template_props.rs:402; crates/engine/project/src/template_props.rs:596",
    },
];
