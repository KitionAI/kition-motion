use super::Row;

macro_rules! text_row {
    ($command:literal, $path:literal, $reads:expr, $checks:expr, $writes:expr, $state:expr, $source:literal) => {
        Row {
            command: $command,
            path: $path,
            resolves: &[
                "layer_p comp/layer id/index/name/default first selected",
                "animator if used: numeric uid first else 1-based index, omitted selected descendant owner else last animator",
            ],
            reads: $reads,
            checks: $checks,
            writes: $writes,
            state: $state,
            source: $source,
        }
    };
}

pub(super) const ROWS: &[Row] = &[
    text_row!(
        "layer.addTextAnimator",
        "normal properties",
        &["perChar3d.value", "animators count when name omitted", "project.next_id"],
        &["selected text layer for enabled", "text/animators group exists", "at least one requested property recognized unless empty property list"],
        &["animators.children append new animator with range selector and property defaults", "project.next_id"],
        &["active_comp default", "selected_layers default"],
        "commands/text_anim.rs:109; project/src/build.rs:163,283"
    ),
    text_row!(
        "layer.addTextAnimator",
        "properties includes perChar3d",
        &["layer.source", "text.perChar3d membership", "all animator property kinds/values and rotation membership"],
        &["target is text layer"],
        &[
            "text.perChar3d true",
            "switches.three_d true",
            "animator shown_dims/value dimensionality/rotation names; missing X/Y rotations inserted",
            "project.next_id"
        ],
        &["active_comp default", "selected_layers default"],
        "commands/text_anim.rs:113,230"
    ),
    text_row!(
        "layer.addTextAnimatorProperty",
        "append missing property types",
        &["animator membership/order and selected property ancestry", "text.perChar3d.value", "existing animator property match_ids", "project.next_id"],
        &["recognized property kinds", "resolved animator exists"],
        &["animator properties append only absent matches", "project.next_id"],
        &["active_comp", "selected_layers default", "selected_props if animator omitted"],
        "commands/text_anim.rs:138,55,81; project/src/build.rs:283"
    ),
    text_row!(
        "layer.addTextSelector",
        "append selector",
        &["animator membership/order and selected property ancestry", "selectors membership and count of same kind", "project.next_id"],
        &["animator exists", "kind range|wiggly|expression"],
        &["selectors group if missing", "selectors.children append default selector", "project.next_id"],
        &["active_comp", "selected_layers default", "selected_props if animator omitted"],
        "commands/text_anim.rs:167; project/src/build.rs:230"
    ),
    text_row!(
        "layer.enablePerChar3D",
        "enable/disable/toggle",
        &["layer.source", "perChar3d value when toggle", "all animator property match_ids/shown_dims/value kinds and rotation membership", "project.next_id"],
        &["text layer"],
        &[
            "perChar3d value or missing property insertion",
            "switches.three_d true only enabling",
            "animator shown_dims, Vec2->Vec3 preserved XY/default Z, rotation names",
            "rotationX/Y inserted on enable with rotation or removed on disable",
            "project.next_id"
        ],
        &["active_comp", "selected_layers default"],
        "commands/text_anim.rs:198,230"
    ),
    text_row!(
        "layer.applyTextPreset",
        "standard built-in JSON preset",
        &[
            "layer.start_time/stretch",
            "text.perChar3d.value",
            "text/animators membership",
            "project.next_id",
            "when frames supplied: sourceText sampled TextDoc.text, comp.frame_rate"
        ],
        &["preset id/name exists", "start finite if supplied", "frames positive if supplied", "text/animators exists"],
        &["append fresh animator/selectors/properties and preset key/value defaults", "project.next_id"],
        &["active_comp", "selected_layers default", "current time when start omitted"],
        "commands/text_anim.rs:302,351; project/src/build.rs:163,230,283"
    ),
    text_row!(
        "layer.applyTextPreset",
        "promo blurSlide",
        &["sampled TextDoc.text for CJK basedOn", "text.perChar3d.value", "layer.start_time/stretch", "comp.frame_rate", "project.next_id"],
        &["text source", "valid start"],
        &["new animator properties/selectors/keys", "optional layer.motion_blur, comp.enable_motion_blur/shutter_angle/motion_blur_samples", "project.next_id"],
        &["active_comp", "selected_layers default", "time_of(comp) when start omitted"],
        "commands/promo_text.rs:174,214; independent handler reading by blind_visual"
    ),
    text_row!(
        "layer.applyTextPreset",
        "promo letterpress",
        &[
            "sampled TextDoc.text",
            "perChar3d.value",
            "anchorGrouping.value",
            "groupingAlignment.value and keys empty check",
            "layer timing",
            "comp.frame_rate",
            "project.next_id"
        ],
        &["text source"],
        &["new animator", "grouping values only for unkeyed defaults", "optional motion blur comp/layer fields", "project.next_id"],
        &["active_comp", "selected_layers default", "time_of(comp) default"],
        "commands/promo_text.rs:273; independent handler reading by blind_visual"
    ),
    text_row!(
        "layer.applyTextPreset",
        "promo leadWordZoom",
        &[
            "full sampled TextDoc and layout glyph/word/line geometry",
            "TextDoc.size",
            "font/layout state",
            "perChar3d",
            "grouping fields/key membership",
            "layer timing",
            "comp.frame_rate",
            "project.next_id"
        ],
        &["text source"],
        &["new animator(s)/keys", "grouping default values", "optional motion blur comp/layer fields", "project.next_id"],
        &["active_comp", "selected_layers default", "time_of(comp) default"],
        "commands/promo_text.rs:308; independent handler reading by blind_visual"
    ),
    text_row!(
        "layer.applyTextPreset",
        "promo typewriterFast",
        &[
            "sampled TextDoc and layout",
            "justify/line count/box/glyph existence determine alignment branch",
            "sourceText base and every TextDoc key when alignment changes",
            "transform.anchor base and key values when alignment changes",
            "layer timing",
            "comp.frame_rate"
        ],
        &["text source"],
        &["sourceText.expr", "conditional all sourceText justification values and anchor base/key X offsets"],
        &["active_comp", "selected_layers default", "time_of(comp) default"],
        "commands/promo_text.rs:383; independent handler reading by blind_visual"
    ),
    text_row!(
        "layer.applyTextPreset",
        "promo countUp",
        &["sampled TextDoc.text determines number/prefix/suffix/default to/default decimals", "layer timing", "comp.frame_rate"],
        &["text source"],
        &["sourceText.value figureWidth tabular", "sourceText.expr"],
        &["active_comp", "selected_layers default", "time_of(comp) default"],
        "commands/promo_text.rs:453; independent handler reading by blind_visual"
    ),
    text_row!(
        "layer.applyTextPreset",
        "promo digitRoll",
        &[
            "full sampled TextDoc/layout/font/size/leading",
            "layer.name/start_time/in_point/out_point/stretch/three_d",
            "comp.width/height/duration/frame_rate and names/insertion gap",
            "perChar3d",
            "project.next_id"
        ],
        &["text source contains digit"],
        &[
            "two new layers per digit with parent and matte refs",
            "sourceText base figureWidth",
            "append original hide animator",
            "optional motion blur strips and comp/shutter",
            "project.next_id"
        ],
        &["active_comp", "selected_layers default", "time_of(comp) default"],
        "commands/promo_text.rs:508; independent handler reading by blind_visual"
    ),
    text_row!(
        "text.animatorFontAxes",
        "axis supplied",
        &[
            "sourceText sampled full TextDoc",
            "layer.start_time/stretch",
            "font/layout first glyph resolved face and variable axes",
            "target animator resolution if supplied",
            "animators count when new",
            "project.next_id"
        ],
        &["text property", "font variable axes nonempty", "requested axis exists", "target properties or text animators group exists"],
        &["append variable-axis property to existing animator or create new animator", "project.next_id"],
        &["active_comp", "selected_layers default", "current time", "font registry"],
        "commands/text_anim.rs:455; project/src/build.rs:163"
    ),
    text_row!(
        "text.animatorFontAxes",
        "axis omitted query branch",
        &["sampled TextDoc/font and layout face axes"],
        &["sourceText has TextDoc"],
        &[],
        &["active_comp", "selected_layers default", "current time", "font registry"],
        "commands/text_anim.rs:455"
    ),
];
