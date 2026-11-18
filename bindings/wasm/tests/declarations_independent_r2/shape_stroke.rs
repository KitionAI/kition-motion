use super::Row;

macro_rules! stroke {
    ($command:literal,$path:literal,$reads:expr,$checks:expr,$writes:expr) => {
        Row {
            command: $command,
            path: $path,
            resolves: &[
                "comp id/name/default active",
                "layer id/index/name/default selected",
                "prop numeric stroke/gstroke uid; omitted selected stroke/descendant else first stroke in contents tree order",
            ],
            reads: $reads,
            checks: $checks,
            writes: $writes,
            state: &["active_comp", "selected_layers", "selected_props when prop omitted", "current time for set_group/fillStroke animated writes"],
            source: "commands/shape_stroke.rs:22,71,107,132,235,266; project/src/props.rs:133",
        }
    };
}

pub(super) const ROWS: &[Row] = &[
    stroke!(
        "shape.dashes.add",
        "enable first dash",
        &["stroke kind and dashes membership", "dash base value <=0"],
        &["stroke exists", "dashes exists"],
        &["dash.value =10"]
    ),
    stroke!(
        "shape.dashes.add",
        "add second/third dash pair",
        &[
            "dash base positive",
            "dash2/dash3 membership",
            "dash/gap base values determine new defaults",
            "offset position names insertion gap",
            "project.next_id"
        ],
        &["fewer than three dash pairs"],
        &["dashes children insert new dash/gap before offset", "project.next_id"]
    ),
    stroke!(
        "shape.dashes.remove",
        "remove last extra pair",
        &["dash3/dash2 membership in reverse order"],
        &["stroke and dashes exist"],
        &["last present dash/gap child pair removed"]
    ),
    stroke!(
        "shape.dashes.remove",
        "disable first dash",
        &["no extra pair exists", "dash membership"],
        &["stroke and dashes exist"],
        &["dash.value =0", "dash.keys cleared"]
    ),
    stroke!(
        "shape.stroke.taper",
        "set supplied taper fields at active CTI",
        &[
            "taper group membership; builder defaults if absent",
            "supplied subproperty UI clamp, keys membership, hold_only and destination key interpolation",
            "layer start_time/stretch",
            "project.next_id if group absent"
        ],
        &["stroke exists", "supplied enum/numeric type valid"],
        &["missing taper group inserted", "supplied units/startLength/endLength/startWidth/endWidth/startEase/endEase base or current key", "project.next_id"]
    ),
    stroke!(
        "shape.stroke.wave",
        "set supplied wave fields at active CTI",
        &[
            "wave group membership; builder defaults if absent",
            "supplied subproperty UI clamp, keys membership, hold_only and destination key interpolation",
            "layer start_time/stretch",
            "project.next_id if group absent"
        ],
        &["stroke exists", "supplied enum/numeric type valid"],
        &["missing wave group inserted", "supplied amount/units/wavelength/cycles/phase base or current key", "project.next_id"]
    ),
    Row {
        command: "shape.fillStroke",
        path: "paint all target shape contents recursively to depth64",
        resolves: &["comp id/name/default active", "layers explicit id/index/name list else selected", "targets filtered to Shape"],
        reads: &[
            "layer source eligibility",
            "contents group topology and fill/stroke/gstroke/path membership",
            "first fill position names insertion gap if adding stroke",
            "supplied property keys membership/hold_only/destination interpolation",
            "layer start_time/stretch",
            "project.next_id",
            "first target first paint sampled for return",
        ],
        checks: &["at least one Shape target", "invalid/nonfinite width ignored", "no locked eligibility check"],
        writes: &[
            "all fill color/stroke color/stroke or gstroke width supplied base/current key",
            "new stroke in path groups without stroke when requested",
            "project.next_id",
        ],
        state: &["active_comp", "selected_layers default", "current time"],
        source: "commands/shape_stroke.rs:235,266",
    },
    Row {
        command: "shape.fillStroke",
        path: "no supplied valid paint: query only",
        resolves: &["same target filtering"],
        reads: &["first Shape target first fill/stroke color/width at CTI"],
        checks: &["Shape target exists"],
        writes: &[],
        state: &["active_comp", "selected_layers", "current time"],
        source: "commands/shape_stroke.rs:266",
    },
];
