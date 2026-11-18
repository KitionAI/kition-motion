//! The parameters commands read that their notation does not spell out, added to the schemas the
//! catalogue offers agents. Every command refuses a key its schema does not name (engine rule E7a),
//! so the schema must name every key the command reads: the keys a
//! handler reads beside its listed ones.

use serde_json::{Value, json};

#[derive(Clone, Copy)]
enum Kind {
    Integer,
    Number,
    String,
    Boolean,
    Array,
    IndexOrName,
}

impl Kind {
    fn node(self) -> Value {
        let description = "read by the command beside its listed parameters";
        let kind = match self {
            Kind::IndexOrName => return json!({ "oneOf": [{ "type": "integer" }, { "type": "string" }], "description": description }),
            Kind::Integer => "integer",
            Kind::Number => "number",
            Kind::String => "string",
            Kind::Boolean => "boolean",
            Kind::Array => "array",
        };
        json!({ "type": kind, "description": description })
    }
}

/// Keys a command's handler reads (directly or through a helper it passes its parameters to) that
/// its notation leaves out, found by reading each handler.
const EXTRA: &[(&str, &[(&str, Kind)])] = &[
    ("anim.applyRecentPreset", &[("comp", Kind::Integer)]),
    ("anim.reveal", &[("comp", Kind::Integer)]),
    ("app.keyboardShortcuts", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName), ("comp", Kind::Integer)]),
    ("camera.analyze", &[("comp", Kind::Integer)]),
    ("camera.createFromSolve", &[("time", Kind::Number), ("comp", Kind::Integer)]),
    ("camera.deletePoints", &[("time", Kind::Number), ("comp", Kind::Integer)]),
    ("camera.fromModel", &[("layer", Kind::Integer)]),
    ("camera.linkFocusToPoi", &[("layer", Kind::Integer)]),
    ("camera.move", &[("dx", Kind::Number), ("dy", Kind::Number), ("layer", Kind::Integer), ("tracker", Kind::IndexOrName)]),
    ("camera.orbit", &[("dx", Kind::Number), ("dy", Kind::Number), ("tracker", Kind::IndexOrName), ("comp", Kind::Integer)]),
    ("camera.orbitNull", &[("camera", Kind::Integer)]),
    ("camera.points", &[("comp", Kind::Integer)]),
    ("camera.selectPoints", &[("time", Kind::Number), ("layer", Kind::Integer), ("effect", Kind::String), ("comp", Kind::Integer)]),
    ("camera.setGroundPlane", &[("time", Kind::Number), ("comp", Kind::Integer)]),
    ("camera.solveStatus", &[("comp", Kind::Integer)]),
    ("command.describe", &[("id", Kind::String)]),
    ("comp.addMarker", &[("comp", Kind::Integer)]),
    ("comp.cropToLayerBounds", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("comp.open", &[("item", Kind::Integer)]),
    ("comp.renderer", &[("comp", Kind::Integer)]),
    ("comp.responsiveTime", &[("comp", Kind::Integer)]),
    ("comp.saveFrameAs", &[("comp", Kind::Integer)]),
    ("comp.setPosterTime", &[("comp", Kind::Integer)]),
    ("comp.setSwitch", &[("comp", Kind::Integer)]),
    ("comp.settings", &[("anchor", Kind::Integer)]),
    ("comp.workArea", &[("comp", Kind::Integer), ("merge", Kind::String)]),
    ("edit.clear", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("edit.copy", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("edit.copyWithPropertyLinks", &[("layer", Kind::Integer), ("comp", Kind::Integer), ("enabled", Kind::Boolean)]),
    ("edit.copyWithRelativePropertyLinks", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("edit.cut", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("edit.duplicate", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("edit.extractWorkArea", &[("comp", Kind::Integer), ("layer", Kind::Integer)]),
    ("edit.history.goto", &[("redo", Kind::Integer)]),
    ("edit.label", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("edit.liftWorkArea", &[("comp", Kind::Integer), ("layer", Kind::Integer)]),
    ("edit.paste", &[("comp", Kind::Integer), ("layers", Kind::Array), ("layer", Kind::Integer), ("enabled", Kind::Boolean)]),
    ("edit.pasteReversedKeyframes", &[("comp", Kind::Integer)]),
    ("edit.pasteTextFormattingOnly", &[("created", Kind::Boolean), ("select", Kind::String), ("caret", Kind::Integer), ("comp", Kind::Integer)]),
    ("edit.splitLayer", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("effect.apply", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("effect.applyLast", &[("effect", Kind::String), ("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("effect.copy", &[("comp", Kind::Integer)]),
    ("effect.duplicate", &[("comp", Kind::Integer)]),
    ("effect.editDropdown", &[("comp", Kind::Integer)]),
    ("effect.paste", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("effect.pickColor", &[("comp", Kind::Integer)]),
    ("effect.plugins.list", &[("filter", Kind::String)]),
    ("effect.plugins.load", &[("filter", Kind::String)]),
    ("effect.remove", &[("comp", Kind::Integer), ("effects", Kind::Array)]),
    ("effect.removeAll", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("effect.reorder", &[("comp", Kind::Integer)]),
    ("effect.reset", &[("comp", Kind::Integer)]),
    ("effect.toggle", &[("comp", Kind::Integer)]),
    ("effect.warning", &[("comp", Kind::Integer)]),
    ("engine.batch", &[("comp", Kind::Integer)]),
    ("templateProps.addMedia", &[("comp", Kind::Integer)]),
    ("templateProps.addProperty", &[("control", Kind::Integer), ("text", Kind::String)]),
    ("templateProps.canAdd", &[("comp", Kind::Integer)]),
    ("templateProps.instance", &[("comp", Kind::Integer)]),
    ("templateProps.pushToComp", &[("comp", Kind::Integer)]),
    ("templateProps.rename", &[("text", Kind::String)]),
    ("templateProps.revert", &[("comp", Kind::Integer), ("supported", Kind::Boolean)]),
    ("templateProps.set", &[("merge", Kind::String), ("comp", Kind::Integer)]),
    ("file.applyInterpretation", &[("item", Kind::Integer)]),
    ("file.createProxy", &[("quality", Kind::String), ("layers", Kind::Array), ("layer", Kind::Integer), ("tracker", Kind::IndexOrName)]),
    ("file.findMissing", &[("rescan", Kind::Boolean), ("query", Kind::String)]),
    ("file.importMultiple", &[("comp", Kind::Integer)]),
    ("file.interpretFootage", &[("item", Kind::Integer)]),
    ("file.interpretProxy", &[("guessAlpha", Kind::Boolean)]),
    ("file.reloadFootage", &[("item", Kind::Integer)]),
    ("file.rememberInterpretation", &[("items", Kind::Array)]),
    ("file.replaceWithPlaceholder", &[("items", Kind::Array)]),
    ("file.replaceWithSolid", &[("items", Kind::Array)]),
    ("file.revealInFinder", &[("items", Kind::Array)]),
    ("file.runScript", &[("name", Kind::String), ("steps", Kind::Integer), ("comp", Kind::Integer)]),
    ("file.watchFolder", &[("path", Kind::String)]),
    ("footage.overlayEdit", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName)]),
    ("footage.reloadChanged", &[("items", Kind::Array), ("item", Kind::Integer)]),
    ("footage.rippleInsertEdit", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName)]),
    ("help.docs", &[("enabled", Kind::Boolean), ("layers", Kind::Array), ("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("keys.applyEase", &[("comp", Kind::Integer)]),
    ("keys.easyEaseIn", &[("which", Kind::String)]),
    ("keys.easyEaseOut", &[("which", Kind::String)]),
    ("keys.nudgeBackward", &[("frames", Kind::Number)]),
    ("keys.nudgeBackward10", &[("frames", Kind::Number)]),
    ("keys.nudgeForward", &[("frames", Kind::Number)]),
    ("keys.nudgeForward10", &[("frames", Kind::Number)]),
    ("keys.paste", &[("comp", Kind::Integer), ("layer", Kind::Integer), ("merge", Kind::String)]),
    ("keys.selectAll", &[("layer", Kind::Integer), ("comp", Kind::Integer), ("prop", Kind::String)]),
    ("keys.set", &[("comp", Kind::Integer)]),
    ("keys.setEase", &[("comp", Kind::Integer)]),
    ("keys.setSpatialTangents", &[("comp", Kind::Integer)]),
    ("keys.toggleTransform", &[("layer", Kind::Integer), ("comp", Kind::Integer), ("time", Kind::Number), ("path", Kind::String)]),
    ("keys.velocity", &[("inInfluence", Kind::Number), ("outInfluence", Kind::Number), ("merge", Kind::String)]),
    ("keys.wiggle", &[("tolerance", Kind::Number), ("quality", Kind::String)]),
    ("layer.addItem", &[("comp", Kind::Integer)]),
    ("layer.addMarker", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.addMask", &[("rect", Kind::Array), ("comp", Kind::Integer)]),
    ("layer.addShapeItem", &[("comp", Kind::Integer)]),
    ("layer.addTextAnimator", &[("comp", Kind::Integer), ("enabled", Kind::Boolean), ("toggle", Kind::String)]),
    ("layer.addTextAnimatorProperty", &[("comp", Kind::Integer), ("properties", Kind::Array)]),
    ("layer.addTextSelector", &[("comp", Kind::Integer)]),
    ("layer.align", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.alignVideoToData", &[("comp", Kind::Integer)]),
    ("layer.arrange", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.autoOrient", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.cameraSettings", &[("comp", Kind::Integer)]),
    ("layer.centerAnchor", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.channels", &[("comp", Kind::Integer), ("channel", Kind::String), ("toggle", Kind::String), ("colorized", Kind::Boolean)]),
    ("layer.create", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.deleteAllMarkers", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.distribute", &[("edge", Kind::String), ("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.enablePerChar3D", &[("comp", Kind::Integer)]),
    ("layer.enableTimeRemap", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.environment", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.expressions", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.frameBlending", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.freezeFrame", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.freezeOnLastFrame", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.handheld", &[("quality", Kind::String)]),
    ("layer.hideOtherVideo", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.liftShadow", &[("comp", Kind::Integer)]),
    ("layer.lightSettings", &[("type", Kind::String), ("comp", Kind::Integer)]),
    ("layer.markersLock", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.mask.featherFalloff", &[("tolerance", Kind::Number)]),
    ("layer.mask.lockOthers", &[("layers", Kind::Array), ("comp", Kind::Integer)]),
    ("layer.mask.remove", &[("layers", Kind::Array), ("comp", Kind::Integer)]),
    ("layer.mask.removeAll", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.mask.unlockAll", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.new3dPrimitive", &[("position", Kind::Array), ("comp", Kind::Integer), ("type", Kind::String)]),
    ("layer.newContentAwareFill", &[("comp", Kind::Integer)]),
    ("layer.newModel", &[("comp", Kind::Integer)]),
    ("layer.newNull", &[("comp", Kind::Integer)]),
    ("layer.newPrimitive", &[("position", Kind::Array), ("comp", Kind::Integer), ("type", Kind::String)]),
    ("layer.newShape", &[("comp", Kind::Integer)]),
    ("layer.newSolid", &[("color", Kind::String), ("comp", Kind::Integer)]),
    ("layer.newText", &[("box", Kind::Array), ("comp", Kind::Integer), ("position", Kind::Array)]),
    ("layer.nudge", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.openSource", &[("comp", Kind::Integer)]),
    ("layer.quality", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.rename", &[("comp", Kind::Integer)]),
    ("layer.revealInFinder", &[("comp", Kind::Integer)]),
    ("layer.revealSource", &[("comp", Kind::Integer)]),
    ("layer.sampling", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.sceneEditDetection", &[("comp", Kind::Integer)]),
    ("layer.select", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.sequence", &[("comp", Kind::Integer), ("layer", Kind::Integer)]),
    ("layer.setBlendMode", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.setComment", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.setMask", &[("comp", Kind::Integer)]),
    ("layer.setParent", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.setSwitch", &[("quality", Kind::String), ("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.setText", &[("comp", Kind::Integer), ("merge", Kind::String)]),
    ("layer.setTrackMatte", &[("comp", Kind::Integer)]),
    ("layer.setTransform", &[("layer", Kind::Integer), ("comp", Kind::Integer), ("merge", Kind::String)]),
    (
        "layer.settings",
        &[
            ("comp", Kind::Integer),
            ("type", Kind::String),
            ("preset", Kind::String),
            ("zoom", Kind::Number),
            ("focalLength", Kind::Number),
            ("angleOfView", Kind::Number),
            ("dof", Kind::Boolean),
            ("focusDistance", Kind::Number),
            ("aperture", Kind::Number),
            ("fStop", Kind::Number),
            ("blurLevel", Kind::Number),
            ("lockToZoom", Kind::Boolean),
            ("position", Kind::Array),
            ("poi", Kind::Array),
            ("intensity", Kind::Number),
            ("coneAngle", Kind::Number),
            ("coneFeather", Kind::Number),
            ("castsShadows", Kind::Boolean),
            ("shadowDarkness", Kind::Number),
            ("shadowDiffusion", Kind::Number),
            ("falloff", Kind::IndexOrName),
            ("radius", Kind::Number),
            ("falloffDistance", Kind::Number),
        ],
    ),
    ("layer.shake", &[("tracker", Kind::IndexOrName), ("quality", Kind::String)]),
    ("layer.slip", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.style.add", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.style.globalLight", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName), ("merge", Kind::String)]),
    ("layer.style.list", &[("comp", Kind::Integer)]),
    ("layer.style.remove", &[("comp", Kind::Integer)]),
    ("layer.style.removeAll", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.style.showAll", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.style.toggle", &[("comp", Kind::Integer)]),
    ("layer.timeStretch", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName), ("comp", Kind::Integer)]),
    ("layer.timing", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.trackMatte", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.transform", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("layer.updateMarkersFromSource", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("learn.start", &[("tutorial", Kind::String)]),
    ("learn.state", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName), ("comp", Kind::Integer)]),
    ("learn.step", &[("comp", Kind::Integer)]),
    ("light.controlWithCamera", &[("layer", Kind::Integer)]),
    ("light.environmentBackground", &[("on", Kind::Boolean), ("layers", Kind::Array)]),
    ("light.fromModel", &[("layer", Kind::Integer)]),
    ("light.sheen", &[("comp", Kind::Integer)]),
    ("liquify.clear", &[("comp", Kind::Integer)]),
    ("liquify.stroke", &[("comp", Kind::Integer)]),
    ("markers.convert", &[("comp", Kind::Integer)]),
    ("markers.delete", &[("comp", Kind::Integer)]),
    ("markers.list", &[("comp", Kind::Integer)]),
    ("markers.set", &[("comp", Kind::Integer), ("merge", Kind::String), ("name", Kind::String), ("kind", Kind::String)]),
    ("mask.addVertex", &[("comp", Kind::Integer)]),
    ("mask.convertVertex", &[("tolerance", Kind::Number)]),
    ("mask.deleteVertices", &[("layer", Kind::Integer), ("mask", Kind::Integer), ("index", Kind::Integer), ("merge", Kind::String)]),
    ("mask.featherPoint.list", &[("comp", Kind::Integer)]),
    ("mask.insertVertex", &[("comp", Kind::Integer)]),
    ("mask.interpolate", &[("comp", Kind::Integer), ("layers", Kind::Array)]),
    ("mask.interpolationOptions", &[("layers", Kind::Array), ("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("mask.moveVertices", &[("layer", Kind::Integer), ("mask", Kind::Integer), ("index", Kind::Integer)]),
    ("mask.new", &[("comp", Kind::Integer)]),
    ("mask.remove", &[("comp", Kind::Integer)]),
    ("mask.removeAll", &[("mask", Kind::Integer), ("comp", Kind::Integer)]),
    ("material.duplicateAssign", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("material.reset", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("material.revealSource", &[("comp", Kind::Integer)]),
    ("material.set", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("paint.brushPreset", &[("name", Kind::String)]),
    ("paint.options", &[("name", Kind::String)]),
    ("paint.setCloneSource", &[("comp", Kind::Integer)]),
    ("path.freeTransform", &[("path", Kind::String), ("comp", Kind::Integer), ("layers", Kind::Array)]),
    ("path.groupShapes", &[("comp", Kind::Integer)]),
    ("path.rotoBezier", &[("path", Kind::String), ("comp", Kind::Integer), ("layers", Kind::Array)]),
    ("path.setFirstVertex", &[("comp", Kind::Integer)]),
    ("path.ungroupShapes", &[("comp", Kind::Integer)]),
    ("paths.nullsFollowPoints", &[("pin", Kind::Integer), ("comp", Kind::Integer)]),
    ("paths.pointsFollowNulls", &[("pin", Kind::Integer), ("comp", Kind::Integer)]),
    ("paths.tracePath", &[("comp", Kind::Integer)]),
    ("playback.scrubAudio", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName), ("comp", Kind::Integer)]),
    ("playback.settings.set", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName), ("comp", Kind::Integer)]),
    ("project.move", &[("item", Kind::Integer)]),
    ("project.setComment", &[("item", Kind::Integer)]),
    ("project.setLabel", &[("item", Kind::Integer)]),
    ("project.setProjectComment", &[("merge", Kind::String)]),
    ("project.usage", &[("item", Kind::Integer)]),
    ("prop.addKey", &[("comp", Kind::Integer)]),
    ("prop.convertExpressionToKeyframes", &[("comp", Kind::Integer)]),
    ("prop.deselect", &[("comp", Kind::Integer)]),
    ("prop.duplicateGroup", &[("comp", Kind::Integer)]),
    ("prop.get", &[("comp", Kind::Integer)]),
    ("prop.moveGroup", &[("comp", Kind::Integer)]),
    ("prop.pickWhip", &[("comp", Kind::Integer)]),
    ("prop.removeGroup", &[("comp", Kind::Integer)]),
    ("prop.renameGroup", &[("comp", Kind::Integer)]),
    ("prop.reset", &[("comp", Kind::Integer)]),
    ("prop.select", &[("comp", Kind::Integer)]),
    ("prop.separateDimensions", &[("comp", Kind::Integer)]),
    ("prop.set", &[("comp", Kind::Integer)]),
    ("prop.setExpression", &[("comp", Kind::Integer), ("merge", Kind::String), ("time", Kind::Number)]),
    ("prop.setGroupEnabled", &[("comp", Kind::Integer)]),
    ("prop.toggleAnimation", &[("comp", Kind::Integer)]),
    ("prop.toggleKey", &[("comp", Kind::Integer)]),
    ("puppet.addPin", &[("comp", Kind::Integer)]),
    ("puppet.follow", &[("comp", Kind::Integer), ("pin", Kind::Integer)]),
    ("puppet.info", &[("comp", Kind::Integer)]),
    ("puppet.mesh", &[("comp", Kind::Integer)]),
    ("puppet.recordPin", &[("tracker", Kind::IndexOrName), ("comp", Kind::Integer)]),
    ("puppet.removePin", &[("comp", Kind::Integer)]),
    ("puppet.selectPins", &[("pin", Kind::Integer), ("comp", Kind::Integer)]),
    ("render.backend", &[("renderer", Kind::String)]),
    ("render.saveCurrentPreview", &[("wait", Kind::Boolean)]),
    ("renderQueue.add", &[("item", Kind::Integer), ("layers", Kind::Array), ("layer", Kind::Integer)]),
    ("renderQueue.setNotify", &[("value", Kind::String)]),
    ("renderQueue.setOutput", &[("output", Kind::String)]),
    ("renderQueue.setRender", &[("wait", Kind::Boolean)]),
    ("renderQueue.setRenderSettings", &[("layers", Kind::Array), ("layer", Kind::Integer), ("comp", Kind::Integer), ("tracker", Kind::IndexOrName)]),
    ("roto.clearStrokes", &[("comp", Kind::Integer)]),
    ("roto.freeze", &[("comp", Kind::Integer)]),
    ("roto.propagate", &[("comp", Kind::Integer), ("start", Kind::Number), ("end", Kind::Number)]),
    ("roto.span", &[("comp", Kind::Integer)]),
    ("roto.status", &[("comp", Kind::Integer)]),
    ("roto.stroke", &[("comp", Kind::Integer), ("tracker", Kind::IndexOrName)]),
    ("roto.unfreeze", &[("comp", Kind::Integer)]),
    ("script.settings.save", &[("path", Kind::String)]),
    ("scriptWindow.get", &[("views", Kind::Integer)]),
    ("shape.dashes.add", &[("comp", Kind::Integer)]),
    ("shape.dashes.remove", &[("comp", Kind::Integer)]),
    ("shape.fillStroke", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("shape.newPath", &[("comp", Kind::Integer)]),
    ("shape.newShape", &[("comp", Kind::Integer), ("size", Kind::Integer), ("position", Kind::Array)]),
    ("shape.stroke.taper", &[("comp", Kind::Integer), ("merge", Kind::String)]),
    ("shape.stroke.wave", &[("comp", Kind::Integer), ("merge", Kind::String)]),
    ("shape.toolOptions", &[("strokeWidth", Kind::Number)]),
    ("templates.saveAs", &[("comp", Kind::Integer)]),
    ("text.animatorFontAxes", &[("comp", Kind::Integer)]),
    ("text.delete", &[("comp", Kind::Integer)]),
    ("text.edit", &[("comp", Kind::Integer)]),
    ("text.fontFeatures", &[("comp", Kind::Integer)]),
    ("text.insert", &[("comp", Kind::Integer)]),
    ("text.moveCaret", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("text.presets", &[("id", Kind::String), ("name", Kind::String), ("description", Kind::String)]),
    ("text.removeAllAnimators", &[("layer", Kind::Integer), ("comp", Kind::Integer)]),
    ("text.setSelection", &[("comp", Kind::Integer)]),
    ("time.back10", &[("frames", Kind::Number)]),
    ("time.end", &[("to", Kind::String), ("prop", Kind::String), ("visible", Kind::Array), ("layer", Kind::Integer)]),
    ("time.forward10", &[("frames", Kind::Number)]),
    ("time.go", &[("layer", Kind::Integer)]),
    ("time.layerIn", &[("to", Kind::String), ("prop", Kind::String), ("visible", Kind::Array), ("layer", Kind::Integer)]),
    ("time.layerOut", &[("to", Kind::String), ("prop", Kind::String), ("visible", Kind::Array), ("layer", Kind::Integer)]),
    ("time.nextFrame", &[("frames", Kind::Number)]),
    ("time.nextKey", &[("to", Kind::String), ("prop", Kind::String), ("layer", Kind::Integer)]),
    ("time.previousFrame", &[("frames", Kind::Number)]),
    ("time.previousKey", &[("to", Kind::String), ("prop", Kind::String), ("layer", Kind::Integer)]),
    ("time.start", &[("to", Kind::String), ("prop", Kind::String), ("visible", Kind::Array), ("layer", Kind::Integer)]),
    ("track.analyze", &[("comp", Kind::Integer)]),
    ("track.apply", &[("comp", Kind::Integer)]),
    ("track.camera", &[("angleOfView", Kind::Number), ("comp", Kind::Integer)]),
    ("track.delete", &[("comp", Kind::Integer)]),
    ("track.extractFaceMeasurements", &[("wait", Kind::Boolean), ("comp", Kind::Integer)]),
    ("track.mask", &[("comp", Kind::Integer)]),
    ("track.new", &[("comp", Kind::Integer)]),
    ("track.options", &[("comp", Kind::Integer), ("toggle", Kind::String), ("colorized", Kind::Boolean)]),
    ("track.property", &[("comp", Kind::Integer)]),
    ("track.reset", &[("comp", Kind::Integer)]),
    ("track.select", &[("comp", Kind::Integer)]),
    ("track.setPoint", &[("comp", Kind::Integer), ("merge", Kind::String)]),
    ("track.setTarget", &[("comp", Kind::Integer)]),
    ("track.setType", &[("comp", Kind::Integer)]),
    ("track.status", &[("comp", Kind::Integer)]),
    ("track.warpStabilizer", &[("comp", Kind::Integer)]),
    ("transition.iris", &[("mask", Kind::Integer)]),
    ("view.addGuide", &[("comp", Kind::Integer)]),
    ("view.get3D", &[("layer", Kind::Integer), ("tracker", Kind::IndexOrName)]),
    ("view.importGuides", &[("comp", Kind::Integer)]),
    ("view.lookAtAll", &[("layers", Kind::Array), ("layer", Kind::Integer)]),
    ("view.set3DViewCamera", &[("poi", Kind::Array), ("comp", Kind::Integer)]),
    ("view.snapToGrid", &[("steps", Kind::Integer), ("redo", Kind::Integer)]),
    ("warp.analyze", &[("comp", Kind::Integer)]),
    ("warp.status", &[("comp", Kind::Integer)]),
    ("window.panel", &[("steps", Kind::Integer), ("redo", Kind::Integer)]),
];

/// Adds to `schema` (a command's `ParameterSchemaSpec`) the keys `id` reads that its notation
/// leaves out.
pub fn complete(id: &str, schema: &mut Value) {
    let Some(properties) = schema.as_object_mut() else { return };
    if matches!(id, "path.convertToBezier" | "layer.mask.motionBlur" | "layer.mask.featherFalloff") {
        properties.insert("mask".to_owned(), Kind::IndexOrName.node());
    }
    for (key, kind) in EXTRA.iter().filter(|(command, _)| *command == id).flat_map(|(_, keys)| keys.iter()) {
        properties.entry(*key).or_insert_with(|| kind.node());
    }
    if id == "prop.pickWhip"
        && let Some(target) = properties.get_mut("target").and_then(|target| target.get_mut("properties")).and_then(Value::as_object_mut)
    {
        target.entry("comp").or_insert_with(|| Kind::Integer.node());
    }
    if id == "mask.deleteVertices" {
        let mut layer = layer_id_node();
        layer["required"] = json!(true);
        properties.insert(
            "vertices".to_owned(),
            json!({"type":"array", "items":{"type":"object", "properties":{
                "layer":layer, "mask":{"type":"integer","required":true}, "index":{"type":"integer","required":true}
            }, "additionalProperties":true}}),
        );
    }
}

/// The keys whose value names a layer (or, `comp`, a composition) by its id.
const LAYER_KEYS: [&str; 4] = ["layer", "parent", "above", "toLayer"];

/// One id as the commands take it: the number `motion_inspect` lists (`51`), or the same id as a
/// string, the way receipts and `others` print it (`"51"`); for a layer also its name or `#n`.
fn id_node(description: &str, text: &str) -> Value {
    json!({ "oneOf": [{ "type": "integer", "description": description }, { "type": "string", "description": text }] })
}

fn layer_id_node() -> Value {
    id_node(
        "layer id as motion_inspect lists it (when no layer has that id, its 1-based stack position)",
        "layer id as receipts print it (\"51\"), a layer name, or #n for the n-th from the top",
    )
}

/// Retypes every parameter that names a layer or a comp by an integer id (`layer`, `layers`'
/// items, `parent`, `above`, `toLayer`, `comp`, at any depth of `schema`, a command's parameter
/// map) to take the id as a number or as the string receipts print, so an id read from a receipt
/// works unchanged in the next call (task card C8-motion).
pub fn ids_as_text(schema: &mut Value) {
    ids_in(schema, 0);
}

fn integer(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("integer")
}

fn ids_in(properties: &mut Value, depth: usize) {
    let Some(properties) = properties.as_object_mut() else { return };
    if depth > 8 {
        return;
    }
    for (key, node) in properties.iter_mut() {
        if LAYER_KEYS.contains(&key.as_str()) && integer(node) {
            *node = with_description(layer_id_node(), node);
        } else if key == "comp" && integer(node) {
            *node =
                with_description(id_node("comp's item id as motion_inspect lists it", "the item id as receipts print it (\"1\"), or the comp's name"), node);
        } else if key == "layers"
            && let Some(items) = node.get_mut("items")
            && integer(items)
        {
            *items = layer_id_node();
        }
        if let Some(items) = node.get_mut("items") {
            ids_in(items.get_mut("properties").unwrap_or(&mut Value::Null), depth + 1);
        }
        ids_in(node.get_mut("properties").unwrap_or(&mut Value::Null), depth + 1);
    }
}

fn with_description(mut node: Value, was: &Value) -> Value {
    if let (Some(description), Some(fields)) = (was.get("description"), node.as_object_mut()) {
        fields.insert("description".into(), description.clone());
    }
    node
}
