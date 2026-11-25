//! Composition attributes and source sizes a layer write computes its result from (r2 rows;
//! object-class comparison `comp:unheld`, `items:unheld`): the frame grid a time snaps to, the
//! duration that bounds a span, the size a default centre or a fill derives from, and the size of
//! the item a layer shows (solid, footage or precomposition). A write names its layers, so the
//! guard holds the composition for existence only and the source item not at all; these premises
//! hold the attributes read.
//!
//! Only unconditional reads are listed, or reads guarded by a parameter the row names. A read that
//! happens only on a fallback (a zero-sized source) is not held, and remains a held-less path.
use kition_kit::{Premise, Under};
use kition_motion_engine::Session;
use kition_motion_project::{ItemId, LayerId, LayerSource};
use serde_json::{Value, json};

const SIZE: &[&str] = &["comp.attrs.width", "comp.attrs.height"];
const SIZE_DURATION: &[&str] = &["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration"];

const RATE: &[&str] = &["comp.attrs.frame_rate"];
const RATE_DURATION: &[&str] = &["comp.attrs.frame_rate", "comp.attrs.duration"];

/// The composition fields a command reads, with the parameter that must be present for the read
/// (`None`: always). Sources are the r2 family rows (`declarations_independent_r2/<family>.rs`).
fn reads(command: &str, params: &Value) -> Option<&'static [&'static str]> {
    let has = |key: &str| params.get(key).is_some_and(|value| !value.is_null());
    let fields: &'static [&'static str] = match command {
        // anim.rs nudge: the key delta is whole frames of the comp (anim/3-7).
        "keyframe.nudge" | "keyframe.nudgeForward" | "keyframe.nudgeBackward" | "keyframe.nudgeForward10" | "keyframe.nudgeBackward10" => RATE,
        // anim_tools/0-1, layer_time/0-2, puppet/1-2/4, prop/18, roto_cmds/3, promo_moves/2, promo_ease/0-1.
        "keyframe.wiggle"
        | "keyframe.smooth"
        | "layer.timeStretch"
        | "layer.timeReverse"
        | "puppet.movePin"
        | "puppet.setPin"
        | "puppet.mesh"
        | "prop.convertExpressionToKeyframes"
        | "roto.freeze"
        | "layer.hop"
        | "keyframe.applyEase" => RATE,
        // anim_tools/2, layer_time/6, puppet/6, roto_cmds/0-2, track/9, warp_cmds/1.
        "motion.sketch"
        | "layer.freezeOnLastFrame"
        | "puppet.recordPin"
        | "roto.stroke"
        | "roto.propagate"
        | "roto.span"
        | "track.analyze"
        | "warp.analyze"
        | "track.warpStabilizer" => RATE_DURATION,
        // promo_moves.rs:280-344: frames at the comp's rate, the default centre at its middle.
        "layer.gather" if !has("center") => &["comp.attrs.frame_rate", "comp.attrs.width", "comp.attrs.height"],
        "layer.gather" => RATE,
        // promo_moves.rs:152-160: the float lasts to the composition's end unless `end` is given.
        "layer.handheld" if !has("end") => &["comp.attrs.duration"],
        // text_anim/6-10 (text_anim/5 reads it only when `frames` is given; held for it too).
        "layer.applyTextPreset" => RATE,
        // three_d.rs:252-258: a Light Type change rebuilds the light's groups at the frame centre.
        "layer.lightSettings" if (has("kind") || has("type")) && has("frame") => &["comp.attrs.frame_rate", "comp.attrs.width", "comp.attrs.height"],
        "layer.lightSettings" if has("kind") || has("type") => SIZE,
        // three_d/12 (three_d.rs:641-662): the view camera frames the comp; build::layer sizes and
        // spans the new camera.
        "camera.fromView" => SIZE_DURATION,
        // promo_light.rs:85-140: the spotlight's solid is the frame, its radius ramps in frames.
        "light.spotlight" => &["comp.attrs.frame_rate", "comp.attrs.width", "comp.attrs.height", "comp.attrs.pixel_aspect", "comp.attrs.duration"],
        // promo_light.rs:279-285: `to` is reached `frames` after `start`, unless `keys` are given.
        "layer.liftShadow" if has("to") && !has("keys") => RATE,
        // create.rs:90-130: outlines become a new shape layer (build::layer) or a frame-sized solid.
        "layer.add" if params.get("op").and_then(Value::as_str) == Some("shapesFromText") => SIZE_DURATION,
        "layer.add" if params.get("op").and_then(Value::as_str) == Some("masksFromText") => {
            &["comp.attrs.width", "comp.attrs.height", "comp.attrs.pixel_aspect"]
        }
        // path_nulls.rs:173-185: each null is a layer.newNull; tracing keys its progress in time.
        "path.nullsFollowPoints" | "path.pointsFollowNulls" => SIZE_DURATION,
        "path.tracePath" => &["comp.attrs.width", "comp.attrs.height", "comp.attrs.duration", "comp.attrs.frame_rate"],
        // layer.rs:335: the next or previous layer skips shy layers while they are hidden.
        "layer.selectNext" | "layer.selectPrevious" => &["comp.attrs.hide_shy"],
        // three_d/1, three_d/4: a `frame` argument is converted with the comp's rate.
        "layer.cameraSettings" | "layer.lightSettings" if has("frame") => RATE,
        // track/8: an explicit time is snapped to the comp's frames.
        "track.setPoint" if has("time") => RATE,
        // autotrace/1: a work-area span is walked frame by frame.
        "layer.autoTrace" if params.get("timeSpan").and_then(Value::as_str) == Some("workArea") => &["comp.attrs.frame_rate", "comp.attrs.work_area"],
        // layer.rs transform_op: centring/resetting/fitting uses the frame size. Flips keep
        // their existing premise, since no output uses the cloned composition's size.
        "layer.transform" if matches!(params.get("op").and_then(Value::as_str), Some("reset" | "center" | "fit" | "fitWidth" | "fitHeight") | None) => SIZE,
        // build::layer defaults size the transform and bound the new layer's visible span.
        "layer.newSolid"
        | "layer.newAdjustment"
        | "layer.newNull"
        | "layer.newText"
        | "layer.newShape"
        | "layer.addItem"
        | "layer.newCamera"
        | "layer.newLight"
        | "layer.new3dPrimitive"
        | "layer.newPrimitive" => SIZE_DURATION,
        // align.rs placed() uses comp pixel aspect; alignment to the frame also reads its size.
        "layer.align" if params.get("to").and_then(Value::as_str).is_none_or(|to| to == "composition") => {
            &["comp.attrs.width", "comp.attrs.height", "comp.attrs.pixel_aspect"]
        }
        "layer.align" | "layer.distribute" => &["comp.attrs.pixel_aspect"],
        // Work-area edits consume the span even when each affected layer is explicit.
        "edit.liftWorkArea" | "edit.extractWorkArea" => &["comp.attrs.work_area"],
        // responsive_time.rs marks a frame-snapped duration or the current work-area span.
        "comp.responsiveTime" => &["comp.attrs.frame_rate", "comp.attrs.duration", "comp.attrs.work_area", "comp.markers"],
        // promo_camera/0: the move spans the comp and frames its size.
        "camera.move" => &["comp.attrs.frame_rate", "comp.attrs.duration", "comp.attrs.width", "comp.attrs.height"],
        _ => return None,
    };
    Some(fields)
}

/// Whether `command` sizes its result from its layers' source items (r2 rows: transform reset/fit, anchor centring, mask reset, paint/liquify/puppet/roto/track
/// reset/warp bounds, light and shadow sizing, auto-trace, camera analysis, content fill).
fn reads_source_size(command: &str, params: &Value) -> bool {
    match command {
        // Effect point defaults keep their own declaration, which holds the source only for an
        // effect whose defaults derive from it (headless independent_layer).
        "layer.centerAnchor"
        | "layer.mask.reset"
        | "liquify.stroke"
        | "paint.stroke"
        | "light.sheen"
        | "layer.liftShadow"
        | "puppet.addPin"
        | "track.reset"
        | "roto.propagate"
        | "roto.freeze"
        | "warp.analyze"
        | "track.warpStabilizer"
        | "layer.autoTrace"
        | "camera.analyze"
        | "contentFill.generate"
        | "layer.align"
        | "layer.distribute" => true,
        // prop/15: an own effect or Transform default; prop/14 (an explicit `default`) reads none.
        "prop.reset" => params.get("default").is_none_or(Value::is_null),
        // layer/37, layer/39: reset and fit size the layer from its source.
        "layer.transform" => matches!(params.get("op").and_then(Value::as_str), Some("reset" | "fit")),
        // layer/32: a mask with no `rect` covers the source.
        "layer.addMask" => params.get("rect").is_none_or(Value::is_null),
        _ => false,
    }
}

/// The source item premise of one layer: the stored size of the solid or footage item, or of the
/// precomposition, that the layer shows.
fn source_premise(session: &Session, comp: u64, layer: u64) -> Option<Premise> {
    let layer = session.project.comp(ItemId(comp))?.layer(LayerId(layer))?;
    let (item, fields): (ItemId, &[&str]) = match layer.source {
        LayerSource::Solid { item } | LayerSource::Footage { item } => (item, &["data.width", "data.height", "data.pixel_aspect"]),
        LayerSource::Comp { item } => (item, &["comp.attrs.width", "comp.attrs.height", "comp.attrs.pixel_aspect"]),
        _ => return None,
    };
    Some(Premise::on(json!(item.0)).fields(fields.iter().copied()).positions(Vec::<String>::new()))
}

/// Adds the source-size premises of `command` for the layers it names by id.
pub(crate) fn extend_sources(session: &Session, command: &str, params: &Value, under: &mut Option<Under>) {
    if !reads_source_size(command, params) {
        return;
    }
    let Some(comp) = comp_id(session, params) else { return };
    let Some(under) = under.as_mut() else { return };
    let Some(composition) = session.project.comp(ItemId(comp)) else { return };
    let layers = match params.get("layers").or_else(|| params.get("layer")) {
        Some(Value::Array(values)) => values.iter().filter_map(|value| kition_motion_engine::commands::resolve_layer(composition, value)).collect::<Vec<_>>(),
        Some(value) => kition_motion_engine::commands::resolve_layer(composition, value).into_iter().collect(),
        None => session.state.selected_layers.clone(),
    };
    for layer in layers {
        under.premises.extend(source_premise(session, comp, layer.0));
    }
}

/// Adds the composition premise of `command` to its answer. A command answered `None` keeps its
/// general declaration: an answer with no objects and no window would also say that the step's
/// time names nothing, which this premise does not establish.
pub(crate) fn extend(session: &Session, command: &str, params: &Value, under: &mut Option<Under>) {
    let camera_settings = command == "layer.cameraSettings"
        || command == "layer.settings"
            && comp_id(session, params)
                .and_then(|id| session.project.comp(ItemId(id)))
                .and_then(|comp| {
                    params
                        .get("layer")
                        .and_then(|value| kition_motion_engine::commands::resolve_layer(comp, value))
                        .or_else(|| session.state.selected_layers.first().copied())
                        .and_then(|id| comp.layer(id))
                })
                .is_some_and(|layer| layer.is_camera());
    if camera_settings
        && (params.get("fStop").and_then(Value::as_f64).is_some()
            || params.get("zoom").and_then(Value::as_f64).is_none()
                && ["preset", "focalLength", "angleOfView"].iter().any(|name| params.get(name).is_some_and(|value| !value.is_null())))
        && let Some(comp) = comp_id(session, params)
    {
        under.get_or_insert_with(Under::default).premises.push(Premise::on(json!(comp)).fields(["comp.attrs.width"]).positions(Vec::<String>::new()));
    }
    if assigns_motion_blur(command, params)
        && let Some(comp) = comp_id(session, params)
    {
        under.get_or_insert_with(Under::default).premises.push(Premise::on(json!(comp)).fields(MOTION_BLUR.iter().copied()).positions(Vec::<String>::new()));
    }
    let Some(fields) = reads(command, params) else { return };
    let Some(comp) = comp_id(session, params) else { return };
    let Some(under) = under.as_mut() else { return };
    under.premises.push(Premise::on(json!(comp)).fields(fields.iter().copied()).positions(Vec::<String>::new()));
}

/// The composition switches the promo presets assign when `motionBlur` is on (promo_ease.rs:355-362).
const MOTION_BLUR: &[&str] = &["comp.attrs.enable_motion_blur", "comp.attrs.shutter_angle", "comp.attrs.motion_blur_samples"];

/// Whether `command` turns the composition's motion blur on: the promo moves, carries and text
/// presets that call `promo_ease::motion_blur` unless `motionBlur` is false.
fn assigns_motion_blur(command: &str, params: &Value) -> bool {
    if params.get("motionBlur").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    match command {
        "camera.move" | "layer.hop" | "layer.gather" | "transition.iris" | "transition.morphRect" => true,
        // promo_text.rs: the presets that call motion_blur (text_anim.rs:354 dispatches by id or name).
        "layer.applyTextPreset" => params.get("preset").and_then(Value::as_str).is_some_and(|preset| {
            ["blurSlide", "Blur Slide", "letterpress", "leadWordZoom", "Lead Word Zoom", "digitRoll", "Digit Roll"]
                .iter()
                .any(|name| name.eq_ignore_ascii_case(preset))
        }),
        _ => false,
    }
}

/// The composition a layer command acts in: `comp` by id, else the active composition.
fn comp_id(session: &Session, params: &Value) -> Option<u64> {
    match params.get("comp").filter(|value| !value.is_null()) {
        Some(value) => {
            value.as_u64().or_else(|| value.as_str()?.parse().ok()).filter(|id| session.project.comp(kition_motion_project::ItemId(*id)).is_some()).or_else(
                || session.project.items.values().find(|item| value.as_str() == Some(item.name.as_str()) && item.as_comp().is_some()).map(|item| item.id.0),
            )
        }
        None => session.active_comp_id().map(|id| id.0),
    }
}
