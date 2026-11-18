use super::Row;

macro_rules! property {
    ($command:literal,$path:literal,$reads:expr,$checks:expr,$writes:expr,$state:expr) => {
        Row {
            command: $command,
            path: $path,
            resolves: &[
                "comp id/name/default active",
                "layer id first else numeric 1-based fallback/#index/name/default first selected",
                "prop numeric uid or path; bare name falls back to transform/name",
            ],
            reads: $reads,
            checks: $checks,
            writes: $writes,
            state: $state,
            source: "commands/prop.rs:13,107; project/src/props.rs:126,133,146; keyframe/src/lib.rs:599",
        }
    };
}
macro_rules! selected_ease {
    ($command:literal,$fields:literal) => {
        Row {
            command: $command,
            path: "selected exact stored keys",
            resolves: &["active comp selected_keys exact layer/property/time"],
            reads: &["key membership/time", "property.spatial", "roving flags and conditional roving-run geometry/times"],
            checks: &["active comp and selected keys nonempty", "missing layer/property/key skipped"],
            writes: &[$fields, "endpoint roving flags and roving key times/in_interp/out_interp", "selected_keys remapped"],
            state: &["active_comp", "selected_keys"],
            source: "commands/prop.rs:850,994; keyframe/src/lib.rs:431",
        }
    };
}

pub(super) const ROWS: &[Row] = &[
    property!(
        "prop.set",
        "static property absolute assignment",
        &[
            "property.value sampled at CTI: coercion kind and preserved components/structured fields",
            "property.ui slider range",
            "property.keys empty membership",
            "if explicit time supplied: layer start_time/stretch and comp.frame_rate for snapped_key_time validation/conversion"
        ],
        &["value present/coercible", "timeBase valid", "explicit time finite and layer stretch nonzero", "no static_only eligibility check"],
        &["property.value"],
        &["active_comp", "selected_layers default", "current time only if sampled keys (static branch none)"]
    ),
    property!(
        "prop.set",
        "animated property assignment",
        &[
            "sampled property at CTI determines coercion and retained components even when explicit time targets another key",
            "property.ui",
            "property.hold_only",
            "key membership and destination interpolation preservation",
            "layer.start_time/stretch",
            "comp.frame_rate for explicit time snapping"
        ],
        &["value coercible", "valid time conversion"],
        &["property.keys at snapped explicit time or CTI converted to layer time", "new spatial key tangents when default_spatial_linear preference"],
        &["active_comp", "selected_layers default", "current time for coercion sample and default destination", "prefs.general.default_spatial_linear"]
    ),
    property!(
        "prop.set",
        "separated combined Position array",
        &[
            "transform.position uid and positionX membership",
            "layer.is_3d",
            "supplied array dimensionality",
            "each axis property value/keys/ui for scalar writes",
            "axis timing as normal branch"
        ],
        &["each scalar write eligibility/coercion"],
        &["transform.positionX/Y and Z only when 3D and supplied", "axis base values or keys"],
        &["active_comp", "selected_layers default", "current time if animated", "default_spatial_linear"]
    ),
    property!(
        "prop.set",
        "selected compatible twins absolute or offset",
        &[
            "source match path/value kind/components",
            "selected layer membership and twins.locked",
            "twin property membership/value kind/components",
            "source and twin sampled values for offset delta",
            "all assigned properties normal branch dependencies"
        ],
        &["selected true and source selected", "locked/incompatible twin skipped", "twin set errors ignored"],
        &["source and matched selected layer property values/keys (offset preserves differences)", "separated dimensions as applicable"],
        &["active_comp", "selected_layers", "time_of(comp)", "default_spatial_linear"]
    ),
    property!(
        "prop.toggleAnimation",
        "enable stopwatch",
        &["property.static_only", "property.keys empty status when toggling", "property.value", "property.hold_only", "layer timing if key created"],
        &["property animatable"],
        &["initial key at CTI if none; no write if already animated", "new spatial-key linear defaults if preference"],
        &["active_comp", "selected_layers", "time_of(comp)", "default_spatial_linear"]
    ),
    property!(
        "prop.toggleAnimation",
        "disable stopwatch",
        &["property.static_only", "sampled animated property curve at CTI", "layer timing"],
        &["property animatable"],
        &["property.value sampled at CTI", "property.keys clear"],
        &["active_comp", "selected_layers", "time_of(comp)"]
    ),
    property!(
        "prop.toggleAnimation",
        "selected compatible twins",
        &["same match-path/kind/component and unlocked eligibility as selected prop.set", "source target on/off", "each target property value/curve at CTI"],
        &["selected true and source selected", "twin failures ignored"],
        &["all matched properties stopwatch state follows source"],
        &["active_comp", "selected_layers", "time_of(comp)", "default_spatial_linear"]
    ),
    property!(
        "prop.addKey",
        "supplied value or sampled default",
        &[
            "destination property sampled value for coercion/default",
            "property.hold_only/spatial",
            "destination key interpolation/ease/label preserved on replacement",
            "layer.start_time/stretch",
            "comp.frame_rate if explicit time"
        ],
        &["value coercible if supplied", "explicit time valid", "no static_only check"],
        &["key at snapped explicit time or CTI", "new spatial key linear defaults if preference"],
        &["active_comp", "selected_layers default", "time_of(comp) when time omitted", "default_spatial_linear"]
    ),
    property!(
        "prop.toggleKey",
        "remove existing key",
        &["key exact CTI membership", "key count", "sole key value if last", "layer timing"],
        &[],
        &["remove exact current-time key", "property.value from key if last"],
        &["active_comp", "selected_layers default", "time_of(comp)"]
    ),
    property!(
        "prop.toggleKey",
        "add absent key",
        &["sampled curve at CTI", "property.hold_only/spatial", "layer timing"],
        &[],
        &["new key at CTI", "new spatial-key linear defaults if preference"],
        &["active_comp", "selected_layers default", "time_of(comp)", "default_spatial_linear"]
    ),
    property!(
        "prop.toggleKey",
        "selected compatible twins",
        &[
            "source and twin match-path/type/dimensions and locks",
            "source toggle result",
            "twin exact current-time key membership and sampled curve/sole-key value"
        ],
        &["source selected and selected true", "twin failures ignored"],
        &["source toggle; twins made to match source key existence", "last-key removal may assign base value"],
        &["active_comp", "selected_layers", "time_of(comp)", "default_spatial_linear"]
    ),
    Row {
        command: "keys.toggleTransform",
        path: "target transform selected layers",
        resolves: &["layers_p", "prop names Transform type, not uid", "rotation expands 3D orientation/X/Y/Z; position expands separated axes"],
        reads: &["layer.is_3d", "transform/positionX membership", "property membership, keys at CTI, sampled curves/hold_only/sole-key value", "layer timing"],
        checks: &["prop recognized"],
        writes: &["transform properties key insert/remove; last-key base value"],
        state: &["active_comp", "selected_layers default", "time_of(comp)"],
        source: "commands/prop.rs:540",
    },
    property!(
        "prop.setExpression",
        "explicit text including empty removal",
        &["assigned expr text/enabled fields", "target membership; source reference construction computed but does not affect explicit text"],
        &["target exists", "syntax check failure disables supplied expression"],
        &["property.expr complete replacement/removal"],
        &["active_comp", "selected_layers default", "selected_props when no path/prop", "expr_check"]
    ),
    property!(
        "prop.setExpression",
        "toggle or enabled-only",
        &["existing property.expr presence/text/enabled", "property node-chain names/match_ids for new default expression"],
        &["at least one explicit or selected leaf target"],
        &["property.expr remove/create/toggle enabled"],
        &["active_comp", "selected_layers default", "selected_props when omitted"]
    ),
    property!(
        "prop.reset",
        "explicit coercible default",
        &["property.value kind/retained components for coercion"],
        &["property exists"],
        &["property.value supplied default", "property.keys clear", "property.expr None"],
        &["active_comp", "selected_layers default"]
    ),
    property!(
        "prop.reset",
        "own effect or Transform default; else preserve base",
        &[
            "property match_id and effect ancestor kind/id",
            "effect registry parameter defaults and effect_bounds source dimensions",
            "Transform eligibility excludes Camera/Light; source_size and comp width/height",
            "property.value if no known/coercible default"
        ],
        &["property exists"],
        &["property.keys clear", "property.expr None", "property.value when default known"],
        &["active_comp", "selected_layers default"]
    ),
    Row {
        command: "prop.select",
        path: "editor selection",
        resolves: &["layer_p", "numeric prop uid accepted directly including group else prop_ref"],
        reads: &["path group kind/path sampled topology for vertex selection", "property key times when selectKeys true"],
        checks: &["layer exists"],
        writes: &["selected_props", "selected_layers", "selected_vertices", "selected_keys"],
        state: &["active_comp", "selected_layers", "selected_props", "selected_keys", "selected_vertices", "current time"],
        source: "commands/prop.rs:682",
    },
    Row {
        command: "prop.deselect",
        path: "editor selection",
        resolves: &["layer_p", "numeric uid accepted directly else prop_ref"],
        reads: &["path-group topology to identify mask selection"],
        checks: &["layer exists"],
        writes: &["selected_props/keys drop property", "selected_vertices drop owner path"],
        state: &["active_comp", "selected_layers", "selected_props", "selected_keys", "selected_vertices", "current time"],
        source: "commands/prop.rs:720",
    },
    property!(
        "prop.convertExpressionToKeyframes",
        "sample every active layer frame",
        &[
            "comp.frame_rate",
            "layer.in_point/out_point/start_time/stretch",
            "sampled property base/keys/spatial/expr",
            "dynamic enabled expression dependency closure (other layers/comps/footage/transforms queried by host)",
            "separated position component properties through EvalCtx.value"
        ],
        &["property exists"],
        &[
            "property.keys replaced with sampled frame keys sorted by layer time",
            "property.expr.enabled false when expression present",
            "spatial defaults for newly timed keys if preference"
        ],
        &["active_comp", "selected_layers default", "expr host/footage provider", "default_spatial_linear"]
    ),
    Row {
        command: "keys.select",
        path: "editor selection",
        resolves: &["active comp keys[].layer refs", "property numeric uid/string path or path", "timeBase comp/layer; nearest stored key"],
        reads: &["layer timing", "key times", "layer.locked"],
        checks: &["valid known params and keys array", "each layer/property/time resolves", "property has keys", "locked keys removed"],
        writes: &["selected_keys", "selected_props/layers if selectProperties"],
        state: &["active_comp", "selected_keys", "selected_props", "selected_layers"],
        source: "commands/prop.rs:755",
    },
    Row {
        command: "keys.move",
        path: "current or repeated merged-gesture keys",
        resolves: &["active comp selected_keys exact layer/property/time"],
        reads: &[
            "frame_rate",
            "layer.start_time/stretch/locked",
            "selected key records and destination collisions",
            "spatial and roving geometry/times",
            "undo base whole project for merge repeat",
        ],
        checks: &["delta supplied", "locked layers skipped"],
        writes: &[
            "keys.time and collisions replaced",
            "roving times/interpolation/flags",
            "selected_keys",
            "whole project reset to undo base on repeated merge",
        ],
        state: &["active_comp", "selected_keys", "history.merge_key/undo", "key_move"],
        source: "commands/prop.rs:911,970",
    },
    Row {
        command: "keys.delete",
        path: "exact selected stored keys",
        resolves: &["active comp selected_keys layer/property/exact stored time"],
        reads: &["key membership/count", "last remaining key.value"],
        checks: &["selected_keys for enabled", "missing refs skipped; no locked check"],
        writes: &["remove selected key records", "base property.value from last key", "selected_keys clear"],
        state: &["active_comp", "selected_keys"],
        source: "commands/prop.rs:975",
    },
    selected_ease!("keys.easyEase", "key in/out interp/ease on requested sides; auto_bezier false"),
    selected_ease!("keys.easyEaseIn", "key in_interp/in_ease; auto_bezier false"),
    selected_ease!("keys.easyEaseOut", "key out_interp/out_ease; auto_bezier false"),
    Row {
        command: "keys.toggleHold",
        path: "selected key outgoing interpolation toggle",
        resolves: &["active comp selected_keys exact stored time"],
        reads: &["key.out_interp/in_interp/auto_bezier/continuous/out_ease", "spatial and roving flags/geometry/times"],
        checks: &["selected keys"],
        writes: &["key.out_interp", "endpoint roving flags and roving times/interpolation", "selected_keys remap"],
        state: &["active_comp", "selected_keys"],
        source: "commands/prop.rs:1105,850",
    },
    Row {
        command: "keys.interpolation",
        path: "selected key temporal/spatial settings",
        resolves: &["active comp selected_keys exact stored time"],
        reads: &[
            "key.value dims",
            "ease lists and lengths for defaults/continuous average",
            "spatial_handles neighbour values/tangents when spatial requested",
            "remaining continuous/auto fields",
            "roving flags and spatial run geometry/times",
        ],
        checks: &["selected keys; unrecognized interpolation strings ignored"],
        writes: &[
            "supplied in/out interpolation/auto_bezier/continuous/roving/spatial fields",
            "continuous side speed average",
            "roving endpoint flags/times/interpolation",
            "selected_keys remap",
        ],
        state: &["active_comp", "selected_keys"],
        source: "commands/prop.rs:1003; keyframe/src/lib.rs:341,431",
    },
    Row {
        command: "keys.velocity",
        path: "selected key temporal speed/influence",
        resolves: &["active comp selected_keys exact stored time"],
        reads: &[
            "key.value dims",
            "side_ease presentation for ease-list resize; neighbours and interpolation/auto slopes",
            "existing ease lists unspecified components",
            "spatial/roving run geometry/times",
        ],
        checks: &["selected keys"],
        writes: &[
            "supplied in/out ease and corresponding Bezier interp",
            "continuous if supplied",
            "auto_bezier false",
            "roving endpoint flags/times/interpolation",
            "selected_keys remap",
        ],
        state: &["active_comp", "selected_keys"],
        source: "commands/prop.rs:1130,850; keyframe/src/lib.rs:392,431",
    },
];
