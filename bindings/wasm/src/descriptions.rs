//! What each command does, for people and models choosing one (kition-sdk `check_commands`).
//! Generated from the commands' doc comments, or from their title, menu and parameter notes where a
//! command has none; edit the text here.

/// The command `id`'s description.
pub fn description(id: &str) -> &'static str {
    match id {
        "anim.addKeyframe" => {
            "Keys every selected property at the current time (CTI): turns on animation for an unkeyed property, else adds a key holding its current value. Needs selected properties; use prop.addKey to key one property by path."
        }
        "anim.applyPreset" => {
            "Applies an animation preset saved by anim.savePreset (`path` to an .ecpreset, or the preset JSON inline as `preset`) to `layers` (default selected): adds its effects and keyed properties from the current time, and records it as recent."
        }
        "anim.applyRecentPreset" => {
            "Re-applies a recently used animation preset: `path`, or `index` into the recent list (default the newest), to `layers` or the selection, exactly as anim.applyPreset with that path."
        }
        "anim.browsePresets" => {
            "Editor window only: opens the animation preset browser. Headless sessions (agents) accept it and change nothing; use anim.applyPreset instead."
        }
        "anim.clearRecentPresets" => "Empties the Recent Animation Presets list in the preferences. Changes no composition.",
        "anim.reveal" => {
            "Reveals (twirls open) properties on the selected layers, or all layers: `kind` keyframes (animated), animation (keys or expressions) or modified (changed from defaults). Returns the revealed property ids; changes no values."
        }
        "anim.savePreset" => {
            "Saves the selected properties (with their keyframes, relative to the earliest key) and selected effects as an .ecpreset file at `path` (`name` optional). Needs a selection; the comp is unchanged."
        }
        "app.about" => "Editor window only: shows the About box. Headless sessions accept it and change nothing.",
        "app.capabilities" => {
            "Returns what this build can do: version, command count, effect counts (total, GPU, float), export formats, whether import, expressions, scripting and GPU are available, and the After Effects parity summary."
        }
        "app.commandPalette" => {
            "Editor window only: opens Quick Apply to search commands and effects by name. Headless sessions accept it and change nothing; agents use <prefix>_commands."
        }
        "app.find" => {
            "Editor window only: focuses the project panel search (`query`). Headless sessions accept it and change nothing; use project queries instead."
        }
        "app.gpuInfo" => {
            "Editor window only: shows the GPU information dialog. Headless sessions accept it and change nothing; app.capabilities reports whether GPU rendering is on."
        }
        "app.hide" => "Editor window only: hides the application (macOS). Headless sessions accept it and change nothing.",
        "app.hideOthers" => "Editor window only: hides other applications (macOS). Headless sessions accept it and change nothing.",
        "app.keyboardShortcuts" => "Editor window only: opens the keyboard shortcut editor. Headless sessions accept it and change nothing.",
        "app.quit" => "Editor window only: quits the application. Headless sessions accept it and change nothing; the host closes documents.",
        "app.settings" => {
            "Editor window only: opens Settings at `page` (general, previews, import, export, audio, disk, memory, 3d, scripting…). Headless sessions accept it and change nothing."
        }
        "app.showAll" => "Editor window only: shows all hidden applications (macOS). Headless sessions accept it and change nothing.",
        "app.templates" => "Editor window only: opens the render settings or output module templates (`kind`). Headless sessions accept it and change nothing.",
        "cache.diskStats" => {
            "Returns the disk cache's state: enabled, folder, entries, cached frames and layers, bytes used and limit, hits, misses, writes and evictions ({enabled:false} when there is none)."
        }
        "camera.analyze" => {
            "Runs (or continues) analysis of a 3D Camera Tracker effect on `layer` (`effect` by uid, name or index); `wait: true` blocks until done. Check progress with camera.solveStatus."
        }
        "camera.cancel" => "Stops the 3D Camera Tracker analysis or solve that is running; returns {stopped}. Enabled only while one runs.",
        "camera.create" => {
            "Creates the 3D Tracker Camera layer from a solved 3D Camera Tracker on `layer` (`effect` picks the tracker), as camera.createFromSolve does for the camera alone. Needs a finished solve."
        }
        "camera.createFromSolve" => {
            "Create Text / Solid / Null / Shadow Catcher (and Light) on the target, plus the 3D Tracker Camera when the tracker has none yet — one undo step."
        }
        "camera.deletePoints" => "Delete points (and, with Auto-delete Points Across Time, the points of other tracks on the same 3D feature), then re-solve.",
        "camera.dolly" => {
            "Moves the active 3D view's camera along its view direction by `amount` comp pixels (positive = forward), keying its position at the current time; `merge` folds repeated calls into one undo step."
        }
        "camera.fromModel" => {
            "Creates a camera layer for every camera inside the 3D model layers (`layers`, default selected) of `comp`, matching their position, orientation and zoom."
        }
        "camera.fromView" => "Layer ▸ Camera ▸ Create Camera from 3D View: a two-node camera matching the current 3D view.",
        "camera.linkFocusToLayer" => {
            "Adds an expression on the camera's Focus Distance that tracks the distance to `layer`, so focus follows it over time; use camera.setFocusToLayer for a one-time value."
        }
        "camera.linkFocusToPoi" => {
            "Adds an expression on the camera's Focus Distance equal to the distance from its position to its point of interest, so focus follows the target."
        }
        "camera.orbit" => {
            "Orbits the active 3D view's camera around its point of interest by `yaw` and `pitch` degrees, keying it at the current time; `merge` makes repeated calls one undo step."
        }
        "camera.move" => {
            "Adds a promo camera move from `start`: pushHover (the view pushes from 0.8x to 2.6x onto `target` in 18 frames while the rig turns 34 degrees Y and 8 X, hovers, then returns), craneRise (3x to 1x over 102 frames after a 1.5% anticipation push, rising), orbit (the rig turns `degrees` with continuous, monotone velocity through its keys) on a new camera parented to a 3D null at the target (target layers made 3D); or parallax: `layers` far to near pan at `speeds` 0.35/0.7/1.4 with blur 5 + 70% desaturation on the far one and blur 7 on the near one. One undo step; answers the camera and rig."
        }
        "transition.iris" => {
            "Iris carry: a circular mask on the incoming `layer` opens from `center` (a comp point, or the middle of a layer it follows) over `frames` (14) until it reaches the farthest corner, so the next scene opens out of a subject; an Iris slider keyed 0 to 1 with `ease` drives the mask path expression; `ring` adds a shape layer stroking the iris edge. One undo step."
        }
        "transition.morphRect" => {
            "Container-transform carry: the `from` layer's on-screen rectangle (with its corner radius) morphs into the `to` layer's rectangle over `frames` (18, smooth): both layers get a rounded-rectangle mask that follows the container and a cover scale and position keyed every frame, so the outgoing content scales and crops into the incoming one, which fades in inside the container over the last third. One undo step."
        }
        "transition.zoomThrough" => {
            "Zoom-through carry: the outgoing `layer` pushes into `target` with its scale keyed every frame in log space (ease-in) until the area of `size` px around the point fills the frame, then crosses over `blend` frames (the upper layer fades, the lower cuts) into `toLayer`, which continues from `overshoot` (1.25x) and settles to its own scale over `settleFrames`; motion blur on. One undo step."
        }
        "layer.shake" => {
            "Adds a deterministic decaying shake from `frame` (comp seconds) to a layer's position and rotation, or a camera's position, point of interest and Z rotation: `amplitude` px and `rotation` degrees falling off by exp(-decay t) at `frequency` Hz with phases from `seed`; zero before `frame`; written as an expression on top of the existing keys or expression. One undo step."
        }
        "layer.handheld" => {
            "Adds a slow handheld float (two sines per axis) to a layer's or camera's position and rotation between `start` and `end`, `amount` px, easing in and out over `ramp` seconds and exactly zero inside every `holds` range; written as an expression on top of the existing keys or expression. One undo step."
        }
        "layer.hop" => {
            "Hops a layer along a 2D ballistic arc from where it is at `start` to `to`, `height` px above the start, over `frames`; stretches along the flight, lands with a squash of `squash` (the bottom stays on the ground) and rings out damped over `ringFrames`. Position and Scale keyed every frame. One undo step."
        }
        "layer.gather" => {
            "Gathers `layers` into a ring of `radius` around `center`: each flies along a spiral arc that sweeps about `arc` degrees into its slot, `stagger` frames after the one before, over `frames` with `ease`; Position keyed every frame, motion blur on. One undo step."
        }
        "camera.orbitNull" => {
            "Create Orbit Null: a 3D null at the camera's point of interest becomes the camera's parent, so rotating the null orbits the camera around what it looks at."
        }
        "camera.pan" => {
            "Moves the active 3D view's camera sideways and up by `dx`, `dy` comp pixels (position and point of interest together), keyed at the current time."
        }
        "camera.points" => {
            "Returns the solved 3D Camera Tracker points visible at `time` (comp seconds, default now): id, comp and layer position, depth, world position, error, colour, selected. Fails before a solve."
        }
        "camera.selectPoints" => {
            "Selects 3D Camera Tracker points by id (`points`); `add` adds to the selection, `toggle` flips each. Use before camera.setGroundPlane or camera.createFromSolve."
        }
        "camera.setFocusToLayer" => {
            "Sets the camera's Focus Distance, keyed at the current time, to `layer`'s current distance from it. Fails if the layer is behind the camera; camera.linkFocusToLayer keeps following it."
        }
        "camera.setGroundPlane" => {
            "Sets the solved 3D Camera Tracker's ground plane and origin from three or more track points (`points`, default the selected ones) at the current frame; created layers then sit on that plane. Fails if the points don't define a plane."
        }
        "camera.solveStatus" => {
            "Returns the 3D Camera Tracker state: running, progress and banner text, and for `layer`/`effect` whether it is pending, analyzed and solved. Read it after camera.analyze before creating layers."
        }
        "camera.stereoRig" => {
            "Builds a stereo 3D rig in `comp`: a master camera (the active one or new), left and right eye cameras and comps, a Stereo 3D Controls layer and a 3D Glasses output. `configuration`, `sceneDepth` (% of width) and `convergence` set the rig."
        }
        "command.describe" => {
            "Returns one command in full: id, label, menu path, shortcut, parameter notes, the JSON Schema of its parameters, whether it is undoable, and whether it is enabled now (with the reason when not)."
        }
        "command.list" => {
            "Lists the engine's commands with id, label and menu path; `filter` matches ids and labels, `enabledOnly` drops disabled ones, `schemas: true` adds each parameter JSON Schema. Agents usually use <prefix>_commands instead."
        }
        "comp.addMarker" => {
            "Adds a marker at `time` (comp seconds, default the current time) with `comment` to the selected layers, replacing any marker at that time; layers with locked markers are skipped. For comp-level markers use the markers commands."
        }
        "comp.close" => "Closes a composition's viewer tab (`comp`, default the active one). The comp stays in the project.",
        "comp.cropToLayerBounds" => {
            "Resizes the composition to the visible bounds of `layers` (default selected) at the current time and shifts every layer so nothing moves on screen. Fails when the layers draw nothing."
        }
        "comp.cropToRegionOfInterest" => {
            "Resizes `comp` to the viewer's region of interest and shifts its layers so they keep their place. Needs a region of interest drawn in the viewer."
        }
        "comp.flowchart" => {
            "Editor window only: opens the Composition Flowchart panel. Headless sessions accept it and change nothing; read nesting with project queries."
        }
        "comp.info" => {
            "Returns a composition's settings and layers: id, name, size, frame rate, duration (seconds), work area, background, preserve flags, and each layer's index, name, type, in/out points and switches. `comp` defaults to the active one."
        }
        "comp.miniFlowchart" => "Editor window only: shows the mini-flowchart of nested compositions. Headless sessions accept it and change nothing.",
        "comp.new" => {
            "Creates a composition (default 1920×1080, 29.97 fps, 10 s) and makes it active: set `name`, `width`, `height`, `frameRate`, `duration` (s), `background` and `renderer`. Returns its id; add layers to it afterwards."
        }
        "comp.open" => "Opens the composition `comp` (id or name) in a viewer tab and makes it the active comp.",
        "comp.openInGraphicsTemplates" => "Composition ▸ Open in Graphics Templates.",
        "comp.renderer" => {
            "Reads (no arguments) or sets the comp's 3D renderer: `renderer` advanced3d (draws 3D model layers, extrusions) or classic3d. Returns {renderer}."
        }
        "comp.responsiveTime" => "Responsive Design — Time: protected comp markers for an intro, an outro or the work area.",
        "comp.revealInProject" => "Selects `comp` in the project panel and shows the panel. Returns {item}; changes no content.",
        "comp.saveFrameAs" => {
            "Writes the frame at `time` (default now) as a PNG to `path`, at `scale`; `queue: true` adds a Render Queue item instead. Use comp.saveFrameAsPsd or comp.saveFrameAsExr for layered stills, file.exportMovie for video."
        }
        "comp.saveFrameAsExr" => {
            "Writes the frame at `time` as a multi-layer OpenEXR to `path` (.exr): the composite plus R,G,B,A per visible layer, linear and premultiplied. Returns path, channels, width, height and time."
        }
        "comp.saveFrameAsPsd" => {
            "Writes the frame at `time` as a layered Photoshop file to `path` (.psd): one layer per visible comp layer with its blend mode and opacity, plus the merged frame. Returns path, layer count and size."
        }
        "comp.setPosterTime" => "Sets the composition's poster frame (its thumbnail in the project) to the current time. Changes no layers.",
        "comp.setSwitch" => {
            "Sets or toggles a composition switch: `switch` hideShy, motionBlur, frameBlending or draft3d; `value` true/false, omitted to toggle. Returns the new value. The layer switches themselves are set per layer."
        }
        "comp.settings" => {
            "Changes a composition's settings: `name`, `width`/`height` (with `anchor` 0–8 for where content stays), `frameRate`, `duration`, `startTime`, `background`, shutter angle and phase, motion blur samples, renderer. `comp` defaults to the active one."
        }
        "comp.trimToWorkArea" => {
            "Shortens the composition to its work area: time before the work area start is cut, layers shift earlier so their content keeps its place, and the duration becomes the work area length."
        }
        "comp.vr.createEnvironment" => "Composition ▸ VR ▸ Create VR Environment: renders the active comp's 3D scene in every direction.",
        "comp.vr.environments" => {
            "Lists the VR environments in the project: output comp, name, cube map, each face's comp and camera, and the current view orientation [x, y, z] degrees."
        }
        "comp.vr.extractCubemap" => {
            "Composition ▸ VR ▸ Extract Cubemap: a 3:2 cube-map comp of an equirectangular comp (VR Converter, equirectangular → cube map), for editing the faces. One undo step."
        }
        "comp.vr.setView" => {
            "Turns all six face cameras of a VR environment together: `orientation` [x, y, z] degrees, or `pan` (Y), `tilt` (X), `roll` (Z). `comp` names the environment's output, cube map or a face comp."
        }
        "comp.workArea" => {
            "Sets the work area: `start`/`end` in comp seconds, or `set: begin|end` at the current time; the area stays at least one frame and inside the comp. It bounds previews, renders and comp.trimToWorkArea."
        }
        "contentFill.generate" => {
            "Generates a Content-Aware Fill layer for `layer` from its masks, using the Content-Aware Fill panel settings (parameters override them): fills the masked area from surrounding frames. Needs a mask on the layer."
        }
        "contentFill.set" => {
            "Sets the Content-Aware Fill options used by layer.newContentAwareFill: `method` (object, surface, edgeBlend), `range` (workArea, entire), `alphaExpansion` (px), `lightingCorrection` (none, subtle, moderate, strong), `referenceLayer`, `referenceTime` (s)."
        }
        "edit.clear" => {
            "Deletes `layers`: the command that deletes layers. Without `layers` it deletes the selection (keyframes, effects, masks or shape items when selected, else the selected layers), or, while text is being edited, the character before the caret."
        }
        "edit.copy" => "Edit ▸ Copy: selected properties copy their keyframes and values, selected keyframes go to the keyframe clipboard, else layers.",
        "edit.copyExpressionOnly" => {
            "Copies only the expressions of the selected properties (not their values or keys) to the clipboard; edit.paste then applies them to matching properties. Fails if none of them has an expression."
        }
        "edit.copyWithPropertyLinks" => {
            "Copies the selected properties, or whole `layers`, so that pasting creates expressions linking each pasted property to its original (absolute comp references). Use for driving copies from one master."
        }
        "edit.copyWithRelativePropertyLinks" => {
            "Like edit.copyWithPropertyLinks, but the pasted expressions use thisComp, so they link to same-named layers in whatever comp the copy is pasted into."
        }
        "edit.cut" => "Edit ▸ Cut while editing text.",
        "edit.deselectAll" => "Clears the selection of layers, properties, keyframes and mask vertices. Changes no content.",
        "edit.duplicate" => {
            "Duplicates the selected layers (or `layers`); when effects or shape items are selected, duplicates those instead. Answers the new ids."
        }
        "edit.editOriginal" => {
            "Opens the source file of the selected footage item or layer in its default application (emits an open-URL request). Returns {url}; changes nothing in the project."
        }
        "edit.extractWorkArea" => {
            "Removes the work area's time span from `layers` (default selected) and closes the gap: layers are split and later parts shift earlier. Use edit.liftWorkArea to remove it and leave a gap."
        }
        "edit.history" => {
            "Edit ▸ History: `{steps: n}` undoes the last n steps (the entry n−1 of the list), `{redo: n}` redoes n; without either, lists the steps (newest first)."
        }
        "edit.history.goto" => "jump to a state by `index` (from `edit.history.list`), `id`, or `steps` along the working line (negative = back, like Undo).",
        "edit.history.list" => {
            "Returns the branching undo history: every state with index, id, label, parent, depth, whether it is current or in the future, plus the current index and the number of branches."
        }
        "edit.label" => "Sets the label colour (`label`: Red, Yellow, Aqua, …) of the selected or given `layers`, or of the selected keyframes (`keys`).",
        "edit.liftWorkArea" => {
            "Removes the work area's time span from `layers` (default selected) and leaves a gap: layers are split at the work area edges and the part inside is deleted, later parts don't move. Use edit.extractWorkArea to close the gap."
        }
        "edit.paste" => {
            "Edit ▸ Paste while editing text: the copied text with its formatting, or `text` (from the system clipboard) in the caret's style when it's something else."
        }
        "edit.pasteReversedKeyframes" => {
            "Edit ▸ Paste Reversed Keyframes: the copied keys mirrored in time over their whole span (the last key first, incoming and outgoing eases swapped), pasted like Edit ▸ Paste (`keys.paste`: at the current time, into the selected or the same properties)."
        }
        "edit.pasteTextFormattingOnly" => {
            "Edit ▸ Paste Text Formatting Only: the copied text's character style (its first character) and paragraph settings on the selected text, or on the whole of the selected text layers."
        }
        "edit.pasteTextMatchFormatting" => "Edit ▸ Paste Text and Match Formatting: the clipboard's plain text in the style at the caret.",
        "edit.purge" => {
            "Frees caches: `what` all (default), memoryAndDisk, memory, disk, image, 3d or snapshot. Clears rendered layer and footage caches, the disk cache and the viewer snapshot; returns what was freed. The project is unchanged."
        }
        "edit.purgeUndo" => "Clears the whole undo history (Edit › Purge › Undo); earlier states can no longer be restored. Changes no content.",
        "edit.redo" => {
            "Redoes the last undone step in the caller's scope (`scope` person or agent; an agent's redo never redoes the person's step). Returns the label of the redone step, or null when there is nothing to redo."
        }
        "edit.selectAll" => {
            "Selects all layers of the active comp that can be selected (not locked; not shy while shy layers are hidden). While editing text, selects all the text; with a puppet pin selected, every pin of that kind."
        }
        "edit.selectLabelGroup" => {
            "Selects every layer of the active comp whose label colour matches a selected layer's; with no layer selected, every project item whose label matches a selected item's."
        }
        "edit.splitLayer" => {
            "Splits `layers` (default selected) at the current time into two layers: the original ends there and a copy starts there (placed above or below per the preference). Layers the current time doesn't cross are skipped; returns the new layers."
        }
        "edit.undo" => {
            "Undoes the last step. A step the replica holds is undone on the replica too, and the project becomes what the replica holds then (any collaborator's edits that arrived since kept); see [`kdm_bridge`]."
        }
        "editor.state" => {
            "Returns the editor's session state: active comp, current time, selected layers, properties, keyframes and mask vertices, open panels and viewer settings. Use it to see what selection-based commands will act on."
        }
        "effect.apply" => {
            "Applies an effect (`effect`: id or name, e.g. \"Gaussian Blur\", \"Gradient Ramp\", \"Glow\") to `layers` (default selected). `params` sets its settings in the same step, by name or match id (`{\"Blurriness\": 30}`, `{\"Start Color\": \"#1d4ed8\"}`); a wrong name changes nothing and the error lists the right ones. Settings start at After Effects' defaults (blurs, Sharpen and Noise at 0, where they change nothing). Answers each instance's path (`effects/#2`) and the parameters with their paths and values, to change or animate with prop.set and prop.addKey."
        }
        "effect.applyLast" => {
            "Applies the most recently applied effect again, to `layers` or the selection, with the same parameters as effect.apply. Fails when no effect has been applied in this session."
        }
        "effect.copy" => "Edit ▸ Copy with effects selected: puts the effect instances on the effect clipboard.",
        "effect.duplicate" => "Duplicates the effect `effect` (index from 1, uid or name) on `layer`, below the original, with its settings and keyframes.",
        "effect.editDropdown" => "Dropdown Menu Control ▸ Edit: the menu items of a dropdown (popup) property.",
        "effect.list" => {
            "Lists the effects effect.apply accepts (`filter` matches names or ids) with each parameter's id, name, type, default, popup options, ranges and decimals; popup values are zero-based. Point defaults are in pixels of a comp-sized layer of the active comp (`unit` px, as prop.set takes them), or fractions of the layer size when no comp is open (`unit` layer). Read before effect.apply or setting effect properties."
        }
        "effect.manage" => {
            "Editor window only: opens the Manage Effects dialog. Headless sessions accept it and change nothing; effect.plugins.list reports plug-ins."
        }
        "effect.paste" => {
            "Edit ▸ Paste with effects on the clipboard: adds copies to every target layer, after the layer's selected effect when it has one, else at the end of its stack."
        }
        "effect.pickColor" => {
            "A colour parameter's eyedropper on a keyer (Effect Controls): the colour of the effect's input (the layer's pixels before this effect, so a keyed screen can still be picked) at a layer-space point, set on the parameter. `average` takes the 5 × 5 pixels around it."
        }
        "effect.plugins.list" => {
            "Lists the registered plug-in effects (WebAssembly or Rust): id, name, category, version, author, description, source and parameter ids, plus the plug-in API version and whether WebAssembly plug-ins can load."
        }
        "effect.plugins.load" => {
            "load WebAssembly effect plug-ins (a file, or every `.wasm` in a folder) through the host's loader ([`Session::plugin_loader`])."
        }
        "effect.remove" => "Removes the effect `effect` (index from 1, uid or name) from `layer` (default the selected layer).",
        "effect.removeAll" => {
            "Deletes every effect from `layers` (default selected) in one undo step. To remove one effect use effect.remove; to turn effects off without deleting, toggle the layer's effects switch."
        }
        "effect.reorder" => {
            "Moves one effect (`effect` by uid, name or index) on `layer` to position `index` (1-based) in the effect stack; effects render top to bottom, so order changes the result."
        }
        "effect.reset" => {
            "Effect Controls' Reset: every parameter back to its default. Animated parameters get a keyframe at the current time with the default value (as in After Effects); static ones change their value."
        }
        "effect.toggle" => "Switches the effect `effect` on `layer` on or off (`value`; without it, toggles). A switched-off effect does not render.",
        "effect.warning" => {
            "Returns what Effect Controls warns about one effect (`effect` by index, uid or name, default the selected one) at `time`: e.g. an unreadable OCIO config or transforms it passes through; `warning` is null when it renders as asked."
        }
        "engine.batch" => {
            "Runs several commands in one call and one undo step: `steps` [{command, params}]; a string \"$N\" or \"$N.key\" in params is step N's result (1-based), e.g. {\"layer\": \"$1.layer\"}. `atomic` (default true) rolls all back if a step fails."
        }
        "templateProps.addComment" => {
            "Adds a comment entry with `text` to the Graphics Templates panel of `comp`, optionally inside group `group`. Returns {control}. Comments only label the panel; they drive nothing."
        }
        "templateProps.addGroup" => {
            "Adds an empty group (`name`) to the Graphics Templates panel of `comp` at `index`; move controls into it with templateProps.move. Groups can't nest. Returns {control}."
        }
        "templateProps.addMedia" => "Media Replacement for a footage layer.",
        "templateProps.addMirror" => "Add a mirror of a property control (the same property shown again, e.g. in another group).",
        "templateProps.addProperty" => "Add Property to Graphics Templates: `{layer, path|prop}` or the selected properties.",
        "templateProps.canAdd" => {
            "Checks whether a property (`layer` plus `path` or `prop`) can be added to Graphics Templates, optionally `as` a given control type. Returns {ok, type, reason}; changes nothing."
        }
        "templateProps.exportTemplate" => {
            "Writes `comp` and everything it uses, with its Graphics Templates controls, as a template file at `path` (.ectemplate); footage is packed beside it. Fails if the comp has no Graphics Templates controls."
        }
        "templateProps.importTemplate" => {
            "Imports an .ectemplate from `path`: its compositions and media enter the project; `addToComp: true` also places the template's comp as a layer in `comp`. Returns the imported items."
        }
        "templateProps.instance" => {
            "Returns the Template Properties of a precomp `layer`: each control of the source comp with its kind, current value at the current time and whether this instance overrides it."
        }
        "templateProps.linkProperty" => "Make a property control drive another property too (same kind of value).",
        "templateProps.list" => {
            "Returns the Graphics Templates of `comp`: its name, the primary comp, the solo state and every control (id, name, kind, group, linked properties); `supported: true` also lists the properties that could be added."
        }
        "templateProps.move" => {
            "Moves a Graphics Templates control (`control` id) into group `group` (null = top level) at `index`. Groups can't nest or contain themselves."
        }
        "templateProps.pushToComp" => {
            "Push to Comp: write the instance's override values into the source comp's properties (and replaced media into the source layer), then drop the overrides."
        }
        "templateProps.remove" => "Removes the control `control` (id) from the Template Properties of `comp` (default the active comp).",
        "templateProps.rename" => {
            "Renames a Graphics Templates control (`control` id) in `comp` to `name`. The name is what instances of the comp show for that property."
        }
        "templateProps.revert" => {
            "Removes a precomp instance's overrides so its Template Properties follow the source comp again: `control` one, or all overridden ones by default, on `layer`. Clears their keys and expressions."
        }
        "templateProps.set" => "Set an instance's value for a control (`value`, or `item` for Media Replacement).",
        "templateProps.setName" => "Sets the name the Graphics Templates panel and templates use for `comp`. Returns {comp, name}.",
        "templateProps.setPrimary" => {
            "Makes `comp` the primary composition whose Graphics Templates controls the panel and templateProps.* commands edit by default. Changes no content."
        }
        "templateProps.soloSupported" => {
            "Shows only the properties that can be added to Graphics Templates in the timeline (`on`, default toggles). Returns {on}; changes no content."
        }
        "templateProps.templateInfo" => {
            "Reads an .ectemplate file's manifest at `path` without importing it: its name, the template comp and the controls it exposes."
        }
        "templateProps.unlinkProperty" => {
            "Disconnects one layer property (`layer` plus `path`/`prop`, default the selected property) from Graphics Templates control `control`; the control keeps its other links. Fails if that property isn't linked to it."
        }
        "expr.errors" => {
            "Lists the expression errors in `comp` at `time` (seconds, default now): each failing property with layer, path and message, plus text lines like the error bar shows. Use after setting expressions with prop.setExpression."
        }
        "expr.languageMenu" => {
            "Returns the expression language menu: categories with each entry's label and the snippet it inserts. Use it to look up expression functions; changes nothing."
        }
        "face.model.download" => {
            "Downloads the official face tracking model `id` and verifies it (desktop only, uses the system curl); `wait` blocks until done. track.mask's faceOutline and faceDetailed methods need an installed model."
        }
        "face.model.install" => {
            "Installs a face tracking model from the model file at `path`, verified against the registry's SHA-256 (`id` names it); `wait` blocks until installed. Use face.model.download to fetch the official one instead."
        }
        "face.model.remove" => {
            "Deletes the installed face tracking model `id` from disk. Face tracking falls back to the classical tracker until another model is selected."
        }
        "face.model.select" => {
            "Chooses the face tracking model face tracking uses: `id` classical or mediapipe-face (must be installed); `wait` returns only once it is loaded."
        }
        "face.models" => {
            "Lists the face tracking models: id, name, whether each is installed, its size and which one is selected. Read before face.model.select."
        }
        "file.applyInterpretation" => {
            "Applies the remembered footage interpretation (alpha, frame rate, fields, colour; remembered with Remember Interpretation) to `items` (default the selected footage). Fails if none is remembered."
        }
        "file.autoSave" => {
            "Saves an auto-save copy of the project now, to the auto-save folder; the open project's own file is not overwritten. Returns {path}."
        }
        "file.clearRecent" => "Empties the Open Recent projects list in the preferences. Changes no project.",
        "file.clearRecentFootage" => "Empties the Import Recent Footage list in the preferences. Changes no project.",
        "file.close" => "File ▸ Close: closes the active composition's tab (or `comp`). The comp stays in the project.",
        "file.closeProject" => {
            "Closes the project and starts an empty one in its place, discarding unsaved changes; save first with the host's save. Not undoable."
        }
        "file.collectFiles" => {
            "Copies the project and every footage file it uses into `folder` (footage goes into `(Footage)`), renaming clashes, and points the copy at the collected files. Returns copied count and errors. The open project is unchanged."
        }
        "file.consolidateFootage" => {
            "Merges footage items that use the same file with the same interpretation into one item and points every layer at it; returns how many duplicates were removed."
        }
        "file.createProxy" => {
            "File ▸ Create Proxy ▸ Still / Movie: queue a half-resolution render of the composition whose post-render action sets the result as its proxy."
        }
        "file.cycleBitDepth" => {
            "Steps the project colour depth 8 → 16 → 32 bits per channel and back. Returns the new label; set a specific depth with file.projectSettings `bitDepth`."
        }
        "file.executeFile" => {
            "`File.execute()` from scripts (and agents): open a file with its default application. Needs Allow Scripts to Write Files and Access Network; with Warn User When Executing Files on, the frontend asks first (`confirmed: true` skips the question)."
        }
        "file.exportLottie" => {
            "Writes `comp` as Lottie to `path` (.json, or .lottie for dotLottie); `includeExpressions` keeps expressions, `textAsShapes` converts text to shapes. Returns path, bytes and warnings for features Lottie can't hold."
        }
        "file.exportMovie" => {
            "Renders the active composition to H.264 MP4 through a scratch Render Queue item and returns it (base64 `data`, `mime`, `bytes`); only `format: mp4`. Writes no file and leaves the project's queue as it is: use <prefix>_export to save."
        }
        "file.exportTimeline" => {
            "Exports `comp` as an editing timeline to `path`: Final Cut Pro XML for Premiere Pro (default), .fcpxml, .otio, .edl, .aaf or .omf (`format`). `prerender` renders layers the format can't express; `precomps` nest or prerender."
        }
        "file.findMissing" => {
            "Finds what the project lacks: `what` footage (missing files, also selected in the project), effects (unknown effect ids per layer) or fonts. Returns the list; changes nothing."
        }
        "file.import" => {
            "File ▸ Import; with `addToComp` (files dropped on the Timeline or the Composition viewer) they also become layers of the active comp ([`add_to_comp`])."
        }
        "file.importLottie" => {
            "Imports a Lottie animation from `path` (.json or .lottie) as a new composition of shape, text and image layers, and opens it; embedded images are written beside the file. Returns the comp and warnings."
        }
        "file.importMultiple" => {
            "Imports several files at once (`paths`), exactly as file.import with that list: footage, image sequences or projects become project items."
        }
        "file.importPlaceholder" => {
            "Adds a placeholder footage item (`name`, `width`, `height`, `frameRate`, `duration` seconds) that stands in for media you will replace later. Returns {item}."
        }
        "file.importRecent" => "Imports a recently imported footage file again: `path`, or `index` into the recent list (default the newest), as file.import.",
        "file.importSolid" => {
            "Adds a solid footage item (`name`, `color`, `width`, `height`) to the project's Solids folder without placing it in a comp; use layer.newSolid to add a solid layer to a comp. Returns {item}."
        }
        "file.importTimeline" => {
            "Imports an editing timeline from `path` (Final Cut Pro XML exported from Premiere Pro, .fcpxml, .otio, .edl, .aaf, .omf) as compositions with its clips; `edlFrameRate` for EDLs. Native .prproj is refused."
        }
        "file.importVanishingPoint" => "Import an exported preset (renamed if the name is taken) and activate it.",
        "file.incrementAndSave" => {
            "Saves the project as a new numbered file next to the current one (Intro.aep → Intro 2.aep) and continues in it. Fails if the project was never saved."
        }
        "file.installScript" => {
            "Copies the script at `path` (.jsx or .js) into the Scripts folder so it appears in File › Scripts; it does not run it. Run scripts with file.runScript."
        }
        "file.installScriptPanel" => {
            "Copies the script panel script at `path` (.jsx or .js) into the Script Panels folder so it appears in the Window menu; it does not run it."
        }
        "file.interpretFootage" => {
            "Sets how footage `items` (default selected) is read: `frameRate`, `alpha` (straight, premultiplied, ignore, guess), `invertAlpha`, `loop`, `pixelAspect`, `fields`, `colorProfile`, `linearLight`, start timecode. Layers using it change accordingly."
        }
        "file.interpretProxy" => "File ▸ Interpret Footage ▸ Proxy: Interpret Footage settings for the proxy file.",
        "file.newCompFromSelection" => "File ▸ New Comp from Selection.",
        "file.newFromTemplate" => {
            "Editor window only: opens the Home screen's Templates tab to start a project from a template. Headless sessions accept it and change nothing."
        }
        "file.newProject" => {
            "Replaces the open project with a new empty one (or the template set in Settings › Project), discarding unsaved changes. Not undoable; save first."
        }
        "file.open" => {
            "Opens the project file at `path` (.kmotion, or an XML .ecprojx), replacing the open project; returns `savedBy`, the version that saved it. A project from a newer version opens with a warning; a file that is not a project fails."
        }
        "file.openDemoProject" => {
            "Replaces the open project with the built-in demo project and opens its main composition at 2.5 s. Discards unsaved changes; returns {comp}."
        }
        "file.openRecent" => {
            "Opens a recent project, replacing the current one: `path`, or `index` into the recent list (default the newest). A project that no longer exists is dropped from the list and the call fails."
        }
        "file.projectSettings" => {
            "Changes project settings: `bitDepth` 8|16|32, colour engine and `workingSpace`, `linearize`, `blendLinear`, HDR handling, `outputSpace`, renderer gpu|software and `timeDisplay` (timecode or frames). Affects every comp's rendering."
        }
        "file.recoveryInfo" => {
            "Returns auto-save status: the last auto-save path, the newest auto-save on disk, the auto-save folder and the recent projects list. Changes nothing."
        }
        "file.reduceProject" => {
            "Deletes every project item the selected compositions don't use (directly or through nested comps and footage). Select the comps to keep in the project first; returns {removed}."
        }
        "file.relinkFootage" => "Run the recorded jobs in a worker session (sharing `footage`).",
        "file.reloadFootage" => {
            "Reload items `ids`. `undoable` makes it an edit; otherwise it records the disk without modifying the project, and an item whose file can't be read now (another app is still writing it) is left as it was. Returns the number reloaded and the files that couldn't be read (only without `undoable`)."
        }
        "file.rememberInterpretation" => {
            "Remembers the interpretation (alpha, frame rate, fields, colour) of footage `item` (default the selected one) so file.applyInterpretation can copy it to other footage. Changes nothing itself."
        }
        "file.removeUnusedFootage" => {
            "Deletes footage and solid items that no layer in any composition uses. Returns {removed}; compositions and folders stay."
        }
        "file.replaceFootage" => {
            "Points footage `item` (default selected) at the file at `path`, keeping every layer that uses it; a numbered still brings its image sequence unless `sequence: false`. Use for swapping media without rebuilding layers."
        }
        "file.replaceWithLayeredComp" => {
            "File ▸ Replace Footage ▸ With Layered Comp: a layered (Photoshop) footage item becomes a composition of its layers; every layer using the footage now uses the comp, and the footage item goes. One undo step."
        }
        "file.replaceWithPlaceholder" => {
            "Turns footage `item` into a placeholder with its size, frame rate and duration (override with `width`, `height`, `frameRate`, `duration`), keeping the layers that use it."
        }
        "file.replaceWithSolid" => "Turns footage `item` into a solid of `color` (size defaults to the footage's), keeping every layer that uses it.",
        "file.revealInFinder" => {
            "Opens the folder holding footage `item`'s file in the system file browser. Returns {url}; fails for items that are not files."
        }
        "file.revert" => "Reloads the project from its saved file, discarding every change since the last save. Fails if the project was never saved.",
        "file.runScript" => {
            "File ▸ Scripts ▸ Run Script File…: `.jsx` / `.js` files run as JavaScript against the After Effects-style object model ([`Session::script`]); anything else is a command script: a JSON array or JSON lines of `{\"command\": id, \"params\": {…}}` (also accepts the control channel's `{\"method\": \"engine.execute\", \"params\": {\"id\", \"params\"}}`)."
        }
        "file.save" => {
            "Saves the project to its file, or to `path` when given (the project then continues in that file). Fails when the project has no path yet: use file.saveAs."
        }
        "file.saveAs" => "Saves the project to `path` and continues editing that file. Use file.saveCopy to write a copy and keep working in the original.",
        "file.saveCopy" => "Writes a copy of the project to `path` and keeps working in the current file. Returns {path, bytes}.",
        "file.saveCopyAsXml" => {
            "Writes a copy of the project as XML (.ecprojx) to `path`, for tools that read XML; keeps working in the current file. Returns {path, bytes}."
        }
        "file.scripts.list" => {
            "Lists the scripts and script panels available: name, whether it is a panel, and source (installed, sample or extension). Run one with file.runScript."
        }
        "file.setProxy" => {
            "Sets the file at `path` as the proxy for footage or compositions `items` (default selected); with proxies on, layers render the proxy instead. Returns the items and the proxy's size."
        }
        "file.setProxyNone" => "Removes the proxy from `items` (default selected), so layers render the original again. Returns the items.",
        "file.timelineFormats" => "The formats, for agents and the export dialog.",
        "file.uninstallScript" => {
            "Deletes the installed script or script panel `name` (file name with .jsx/.js) from the Scripts and Script Panels folders. Fails if it isn't installed."
        }
        "file.useProxy" => "The Project panel's proxy switch: use the proxy (filled box) or the full-resolution source.",
        "file.watchFolder" => {
            "Watches `folder` for .kmotion projects with queued renders, renders them and writes `<project>.status.json` beside each; renders what is there now and returns it. `stop: true` stops watching."
        }
        "file.watchFolder.poll" => {
            "Checks the watched folder (or `folder`) now and renders newly arrived projects; returns each project's state (done or failed) and its render items' status and outputs."
        }
        "footage.check" => {
            "Looks for the file of every footage item (or of `items`) and flags the ones that are missing; runs in the background unless `wait`."
        }
        "footage.clearInOut" => "Clears the In and Out points set in the Footage panel for the open footage item. Changes no composition.",
        "footage.info" => {
            "Returns the Footage panel state: the open item, its name, the panel time (seconds, frame, timecode), the In and Out points, duration and frame rate; null when no footage is open."
        }
        "footage.open" => "Opens the footage item `item` (id or name) in the Footage panel, at its first frame.",
        "footage.overlayEdit" => {
            "Places footage `item` (its In–Out range from the Footage panel or `in`/`out` seconds) as a new layer in `comp` at `at` (default now), on top, overlapping what's there. Use footage.rippleInsertEdit to push later layers instead."
        }
        "footage.reloadChanged" => {
            "Reloads the footage whose file changed on disk since it was last seen (size or modification time); files seen for the first time are only remembered. Answers {files, reloaded}."
        }
        "footage.rippleInsertEdit" => {
            "Inserts footage `item`'s In–Out range (or `in`/`out` seconds) into `comp` at `at` (default now) and splits and pushes every later layer later by its length. Use footage.overlayEdit to overlap instead."
        }
        "footage.setIn" => {
            "Sets the Footage panel's In point for the open footage at `time` (source seconds) or `frame`, default the panel time; the range overlay and insert edits use."
        }
        "footage.setOut" => {
            "Sets the Footage panel's Out point for the open footage at `time` (source seconds) or `frame`, default the panel time; the range overlay and insert edits use."
        }
        "footage.setTime" => {
            "Moves the Footage panel's time to `time` (source seconds) or `frame`, clamped to the footage. Returns the time in seconds; changes no composition."
        }
        "help.docs" => "Help ▸ Kition Motion Help / Expression Reference / Effect Reference.",
        "help.enableLogging" => "Turns diagnostic logging to a file on or off (`on`, default toggles). Returns {enabled, path}.",
        "help.github" => "Opens the project's GitHub page in the browser (emits an open-URL request). Changes nothing.",
        "help.inAppTutorials" => "Editor window only: opens the Home screen's Learn tab. Headless sessions accept it and change nothing.",
        "help.reportIssue" => "Opens the issue tracker page in the browser to give feedback. Changes nothing.",
        "help.revealLogFile" => "Opens the folder holding the log file in the system file browser. Returns {path, url}.",
        "help.systemInfo" => {
            "Returns the system report: app name, version and build, OS, CPU architecture and core count, GPU, whether expressions, media import and the disk cache are available, and the plug-in count. Changes nothing."
        }
        "help.systemReport" => {
            "Answers a system compatibility report (build, processor, memory, GPU, fonts) and shows it in the editor; `quiet` skips the window."
        }
        "item.metadata" => {
            "Returns metadata for the project (name, path, item and comp counts, bit depth, working space, time display) and, when `item` (id or name) is given, for that item (type, size, label, comment and its own fields)."
        }
        "jobs.cancel" => {
            "Cancels running background work: `job` render, track, maskTrack, warp, camera, roto, task:<n> or all (default). Returns what was cancelled; finished results stay."
        }
        "jobs.list" => {
            "Returns the background jobs (renders, tracking, warp stabilizing, camera solves, roto) that are running with their progress, and the log of finished ones. Use with jobs.wait."
        }
        "jobs.wait" => {
            "Blocks until every background job (renders, tracking, solves) has finished, then returns the finished jobs log. Call after starting work you need the result of."
        }
        "keys.audioToKeyframes" => {
            "Animation ▸ Keyframe Assistant ▸ Convert Audio to Keyframes: an \"Audio Amplitude\" null with Left Channel, Right Channel and Both Channels sliders keyed on every frame of the work area with the amplitude of the comp's audible layers. One undo step."
        }
        "keys.copy" => "Copy the selected keyframes (Edit ▸ Copy with keys selected).",
        "keys.delete" => {
            "Deletes the selected keyframes; a property left with no keys keeps the deleted key's value as its static value. Select keys with keys.select first. Returns how many were deleted."
        }
        "keys.easyEase" => {
            "Applies Easy Ease to the selected keyframes: Bezier interpolation with 33% influence and zero speed on both sides (`which` in or out for one side), so motion slows into and out of each key."
        }
        "keys.easyEaseIn" => {
            "Eases only the incoming side of the selected keyframes (Bezier, zero speed, 33% influence), so motion slows into each key; the outgoing side is unchanged."
        }
        "keys.easyEaseOut" => {
            "Eases only the outgoing side of the selected keyframes (Bezier, zero speed, 33% influence), so motion starts slowly from each key; the incoming side is unchanged."
        }
        "keys.exponentialScale" => {
            "Replaces the motion between two selected Scale keyframes with exponential scaling (keys every frame), so a zoom looks constant-speed. Needs two Scale keys selected on one layer."
        }
        "keys.info" => {
            "Returns details of the selected keyframes, or `keys` [{layer, prop, time}]: value, interpolation in and out, ease speed and influence per side and dimension, spatial tangents and roving, with units."
        }
        "keys.interpolation" => {
            "Sets the selected keyframes' interpolation: `interpolation` (linear, bezier, continuousBezier, autoBezier, hold) or `in`/`out` per side, `spatial` for paths, and roving. Changes how values move between keys, not the key values."
        }
        "keys.move" => {
            "Shifts the selected keyframes in time by `delta` seconds (negative = earlier), keeping their spacing; `merge` makes repeated calls one undo step. Use keys.nudgeForward/Backward to move by frames."
        }
        "keys.nudge" => "Move the selected keys by whole frames (Alt+→ / Alt+←, Shift for 10 frames).",
        "keys.nudgeBackward" => "Moves the selected keyframes one frame earlier. keys.move shifts by any number of seconds.",
        "keys.nudgeBackward10" => "Moves the selected keyframes ten frames earlier.",
        "keys.nudgeForward" => "Moves the selected keyframes one frame later. keys.move shifts by any number of seconds.",
        "keys.nudgeForward10" => "Moves the selected keyframes ten frames later.",
        "keys.paste" => "Paste copied keyframes at the current time into the target layers' properties.",
        "keys.rpfCameraImport" => {
            "Animation ▸ Keyframe Assistant ▸ RPF Camera Import: a camera layer keyed from camera data. RPF/RLA files carry their camera in an undocumented block, so Kition Motion reads the same data from JSON or CSV exports (see `docs/features/preferences.md` ▸ RPF Camera Import). One undo step."
        }
        "keys.select" => {
            "Selects keyframes: `keys` [{layer, prop uid or path such as transform/position, time}], times in layer seconds unless `timeBase: comp`; `add`/`toggle` extend the selection. Fails on keys that don't exist. Most keys.* commands act on this selection."
        }
        "keys.selectAll" => "Select every keyframe of the selected properties, or of the given / selected layers.",
        "keys.selectEqual" => "Adds to the selection every keyframe, on the same properties, whose value equals a selected keyframe's value.",
        "keys.selectFollowing" => "Adds to the selection every keyframe after the last selected one on the same properties.",
        "keys.selectLabelGroup" => {
            "Select every keyframe that shares a label with a selected keyframe, on the selected or all layers. `visible` (property uids revealed in the Timeline) limits the \"Visible Keyframes\" scopes; without it every property counts as visible."
        }
        "keys.selectPrevious" => "Adds to the selection every keyframe before the first selected one on the same properties.",
        "keys.set" => "Edit one key directly: move it in time and/or set its value (Graph Editor drags).",
        "keys.applyEase" => {
            "Eases every segment between consecutive keyframes of one property (`path` or `prop`; `from`/`to` in layer seconds limit the keys) with a named curve: snappy, out, smooth, swift, linear, or the overshoot landings settle, pop, reseat, fly (`bezier: [x1,y1,x2,y2]` for any CSS cubic-bezier). Curves become exact AE temporal ease (speed and influence); on a motion path an overshoot inserts a key at each turning point. Spring presets (crisp, lively, noBounce, cardFlip, or spring with damping/stiffness/mass) attach an expression that springs the motion into the last key. One undo step."
        }
        "keys.easePresets" => {
            "Lists the ease presets keys.applyEase takes: curves (id, CSS bezier, when to use) and springs (id, damping, stiffness, mass, when to use). Read-only."
        }
        "keys.setEase" => "Set one side's temporal ease of a key (Graph Editor handle drags, scripting).",
        "keys.setLabel" => {
            "Sets the label colour of the selected keyframes: `label` a colour name, 0–16, or none. Returns the label and how many keys changed; it only marks keys, motion is unchanged."
        }
        "keys.setSpatialTangents" => {
            "Spatial tangents of one key of a spatial property (motion path handles), relative to the key value. Setting one side only mirrors it to the other unless `break` is true; the key stops being auto Bezier."
        }
        "keys.smooth" => {
            "Smoother: removes keyframes from the selected run (three or more keys on a property) where the curve stays within `tolerance` of the original (property units, default 1), simplifying hand-drawn or tracked motion."
        }
        "keys.timeReverse" => "Keyframe Assistant ▸ Time-Reverse Keyframes: mirror the selected keys of each property.",
        "keys.toggleHold" => {
            "Toggles Hold interpolation on the selected keyframes' outgoing side: the value jumps at the next key instead of moving; toggling off restores Bezier (eased keys) or Linear."
        }
        "keys.toggleTransform" => {
            "Alt+Shift+A / P / S / R / T: add or remove a keyframe at the current time on that Transform property of every selected layer, in one undo step. Position covers separated X / Y / Z; Rotation on a 3D layer covers Orientation and X / Y / Z Rotation."
        }
        "keys.transform" => {
            "Transform the selected keys: times scale about `timeAnchor` (comp seconds, default the first selected key) by `timeScale` and move by `timeOffset`; values scale about `valueAnchor` by `valueScale` and move by `valueOffset` (every dimension, or only `dim`). Times land on frames."
        }
        "keys.velocity" => {
            "Sets the speed and influence of the selected keyframes: `inSpeed`, `inInfluence` %, `outSpeed`, `outInfluence` % (numbers, or arrays per dimension), `continuous` to tie both sides. Use for custom eases; keys.easyEase for the standard one."
        }
        "keys.wiggle" => {
            "Wiggler: adds random keyframes between two selected keys on a property: `frequency` keys per second (5), `magnitude` in property units, `noise` smooth or jagged, `dimensions` one/same/independent, `seed` for repeatability."
        }
        "layer.addItem" => {
            "Places a project item (`item` id or name: footage, comp or solid) in the active comp as a new layer: starting at `time` (seconds), at stack `index` (1-based, default above the selected layer), at `position` [x, y]; `duration` for stills."
        }
        "layer.addMarker" => {
            "Adds a marker with `comment` at `time` (comp seconds, default now) to `layers` (default selected), replacing a marker at that time; layers with locked markers are skipped. Returns `time` and `markers`: each added marker's `layer`, `index` (as markers.list shows it) and `time`."
        }
        "layer.addMask" => {
            "Adds a mask to `layer`: `shape` rect, ellipse, rounded, polygon or star; `rect` [x, y, w, h] in layer pixels (default the whole layer); `mode` add, subtract and so on. Use mask commands to edit its path, feather and opacity."
        }
        "layer.addShapeItem" => {
            "Adds an item to a shape layer's contents: `kind` group, rect, ellipse, star, polygon, path, fill, stroke, gradient fill/stroke, trim, repeater, round corners, offset, pucker, twist, zig zag, wiggle or merge; `group` to nest; `index` (1-based) to place it in the group (by default last: a paint added above the existing paints shows in front of them). Returns {uid, path}."
        }
        "layer.addTextAnimator" => {
            "Adds a text animator with the given `properties` (position, scale, rotation, opacity, fillColor, tracking, blur, characterOffset…) and a range selector to a text layer; animate the selector to reveal per character. `perChar3d` enables 3D."
        }
        "layer.addTextAnimatorProperty" => {
            "Adds `property` (any layer.addTextAnimator property) to an existing text animator (`animator` uid or index) on `layer`, so it is driven by that animator's selectors."
        }
        "layer.addTextSelector" => "Animation ▸ Add Text Selector ▸ Range / Wiggly / Expression (see `layer.addTextSelector`).",
        "layer.align" => {
            "Aligns `layers` (default selected) by `edge` (left, hcenter, right, top, vcenter, bottom) to the composition (default) or to the selection's bounds, moving their positions. Use layer.distribute to space them evenly."
        }
        "layer.alignVideoToData" => {
            "Lines a video layer up in time with a data footage item (`data`, with `key` its time field): the video's start `videoStart` (default the file's creation time) is matched against the data's samples, whose first sample sits at `dataStart` (comp seconds, default 0), and the layer is moved so both share one clock."
        }
        "layer.applyTextPreset" => {
            "Applies a built-in text animation to a text `layer`: `preset` typewriter, fadeUpCharacters, bounceInWords, trackingIn, scramble, blurIn, jitter or dropInLines, keyed from the current time; or a promo preset from `start` (comp s, default the current time) with timing in frames: blurSlide (per word rise + blur + fade, `variant: subtitle`), letterpress (per character scale + blur), leadWordZoom (first word zoomed and centred, then settles into the line), typewriterFast (0.7 frames per character with a cursor), digitRoll (odometer digits: strip and window layers parented to the text) or countUp (`from`/`to`/`duration`, expo-out, thousands separators). `frames`, `stagger`, `ease`, `by`, `dy`, `blur`, `scale` override the tuned defaults; CJK text goes per character. One undo step; the answer has the animators or layers made and `end`. Fails on non-text layers."
        }
        "layer.liftShadow" => {
            "Gives a `layer` a lift shadow driven by an Elevation slider (px): a tight contact Drop Shadow (opacity 12%/(1+e/24)) and a wide ambient one (opacity 8%+22%*min(e,64)/64), distance and softness growing with elevation. `elevation` static, or `to` reached `frames` after `start`, or `keys` [[comp s, elevation]]. One undo step; answers the effects added."
        }
        "layer.arrange" => {
            "Changes the stacking order of `layers` (default selected): `to` front, forward, backward or back, or an exact `index` (1-based), or `above` another layer. Upper layers draw over lower ones; locked layers are refused."
        }
        "layer.autoOrient" => {
            "Layer ▸ Transform ▸ Auto-Orient: off | alongPath | towardsCamera (3D layers) | towardsPointOfInterest (cameras and spot/parallel lights)."
        }
        "layer.autoTrace" => {
            "Traces the edges of `layer` (its `channel`: alpha, red, green, blue or luminance) into masks, at the current frame or over the work area (`timeSpan`): `threshold`, `blur`, `tolerance`, `minimumArea`, `cornerRoundness`, `invert`; `applyToNewLayer` puts the masks on a new solid."
        }
        "layer.cameraSettings" => {
            "Changes a camera layer: `name`, `type` (one- or two-node), `preset` or `zoom`/`focalLength`/`angleOfView`, depth of field (`dof`, `focusDistance`, `aperture`/`fStop`, `blurLevel`), `position` and point of interest `poi`, keyed at the current time."
        }
        "layer.centerAnchor" => {
            "Moves the anchor point of `layers` (default selected) to the centre of their visible content and compensates position, so nothing moves on screen; rotation and scale then pivot around the content's centre."
        }
        "layer.channels" => {
            "Returns the layers and channels of a multi-layer OpenEXR footage `layer` and what each EXR layer maps to red, green, blue and alpha; these are the names EXtractoR's channel parameters take. Empty for other layers."
        }
        "layer.create" => {
            "Converts the selected layer by `op`: editableText (Photoshop type layer to a text layer), shapesFromText (a shape layer of the glyph outlines), masksFromText (masks on a solid) or shapesFromVector (SVG/vector footage to a shape layer)."
        }
        "layer.deleteAllMarkers" => "Deletes every marker on `layers` (default selected); layers whose markers are locked are skipped.",
        "layer.distribute" => {
            "Spaces three or more `layers` (default selected) evenly between the outermost two by `mode`: left, hcenter, right (horizontal) or top, vcenter, bottom (vertical) edges or centres. Use layer.align to line them up."
        }
        "layer.enablePerChar3D" => {
            "Turns per-character 3D on a text layer on (default), off with `enabled: false`, or `toggle`; text animators can then move, rotate and scale each character in 3D. Fails on non-text layers."
        }
        "layer.enableTimeRemap" => {
            "Turns time remapping on or off (`value`, default toggles) for footage or comp `layers`: adds a Time Remap property keyed at the layer's start and end, which you then key to speed up, slow down, reverse or hold."
        }
        "layer.environment" => {
            "Makes footage, comp or solid `layers` (default selected) environment layers (`on`, default toggles): a 360° image that lights and reflects in Advanced 3D. Turns their 3D switch on; model and light layers are refused."
        }
        "layer.expressions" => {
            "Enables or disables (`enabled`) every expression on `layers` (default selected) without deleting them; disabled expressions keep their text and the property uses its own value. Returns how many changed."
        }
        "layer.frameBlending" => {
            "Sets frame blending on `layers` (default selected): `mode` off, frameMix (cross-fades frames) or pixelMotion (motion-interpolated); smooths slowed or time-remapped footage. The comp's frame blending switch must be on to render it."
        }
        "layer.freezeFrame" => {
            "Freezes footage or comp `layers` (default selected) on the frame they show at the current time: adds time remapping held at that frame for the whole layer."
        }
        "layer.freezeOnLastFrame" => {
            "Holds footage or comp `layers` on their last source frame from the current time on: adds time remapping that plays normally until now, then freezes."
        }
        "layer.hideOtherVideo" => {
            "Turns off the video switch of every audio/visual layer in the comp except `layers` (default selected), to see those alone. Undoable; turn layers back on with their video switch."
        }
        "layer.lightSettings" => {
            "Changes a light layer: `name`, `kind` (Parallel, Spot, Point, Ambient, Environment; changing it rebuilds the light), `color`, `intensity`, cone angle and feather, falloff, `castsShadows`, shadow darkness and diffusion, `position`, `poi`."
        }
        "layer.markersLock" => {
            "Locks or unlocks the markers of `layers` (`value`, default toggles): locked markers can't be added, moved or deleted. Returns the new state."
        }
        "layer.mask.featherFalloff" => "Sets how a mask's feather fades: `mode` smooth or linear, for `mask` (default selected masks) on `layer`.",
        "layer.mask.hideLocked" => "Hides or shows locked masks in the viewer (`value`, default toggles). Display only; masks still apply.",
        "layer.mask.invert" => "Inverts or un-inverts the target masks (`value`, default toggles): an inverted mask keeps the outside instead of the inside.",
        "layer.mask.lock" => "Locks or unlocks `mask` on `layer` (`value`, default toggles); locked masks can't be selected or edited in the viewer.",
        "layer.mask.lockOthers" => "Locks every mask on the layer except the target ones (`mask`, default selected), so only those can be edited.",
        "layer.mask.mode" => {
            "Sets how a mask combines with the masks above it: `mode` None, Add, Subtract, Intersect, Lighten, Darken or Difference, for `mask` (default selected) on `layer`."
        }
        "layer.mask.motionBlur" => "Sets a mask's motion blur: `mode` sameAsLayer (follows the layer's switch), on or off.",
        "layer.mask.remove" => "Deletes `mask` from `layer`; without `mask`, the selected masks, else the first one. layer.mask.removeAll deletes them all.",
        "layer.mask.removeAll" => "Deletes every mask on `layers` (default selected).",
        "layer.mask.reset" => {
            "Resets the target masks to a full-layer rectangle with no feather, 100% opacity, no expansion, mode Add and not inverted, removing their keyframes and expressions."
        }
        "layer.mask.set" => {
            "Sets a mask property at the current time: `field` feather (pixels, one number or [x, y]), opacity (%) or expansion (pixels), `value`. Keys it if the property is animated."
        }
        "layer.mask.shape" => {
            "Sets a mask's path to a `shape` rect or ellipse filling `rect` [x, y, w, h] in layer pixels, for `mask` (default selected) on `layer`."
        }
        "layer.mask.unlockAll" => "Unlocks every mask on `layers` (default selected).",
        "layer.new3dPrimitive" => {
            "Adds a 3D primitive layer: `kind` cube, sphere, plane, torus, cone or cylinder with size options, `position` [x, y, z] and material (`baseColor`, `metallic` and `roughness` as percents 0 to 100, `emissive`, shadows). Switches the comp to Advanced 3D."
        }
        "layer.newAdjustment" => {
            "Adds an adjustment layer the size of the comp: effects on it apply to every layer below it. Same options as layer.newSolid (`name`, `width`, `height`)."
        }
        "layer.newCamera" => {
            "Adds a camera layer: `type` oneNode or twoNode (default, with a point of interest), lens `preset` (15–200mm) or `zoom`/`focalLength`/`angleOfView`, depth of field options, `position` and `poi` [x, y, z]. Only 3D layers see it."
        }
        "layer.newContentAwareFill" => {
            "Content-Aware Fill: renders a fill layer that removes the masked area of `layer` over the work area or the whole comp, with the options of contentFill.set (or given here); writes the frames to `outputDir`."
        }
        "layer.newLight" => {
            "Adds a light layer: `kind` Parallel, Spot (default), Point, Ambient or Environment, with color, intensity, cone, falloff and shadow options. Lights affect 3D layers; Environment lights from an equirectangular image layer."
        }
        "layer.newModel" => "New 3D model layer from a model footage item (`shape`) or a file (`path`, imported first).",
        "layer.newNull" => "A new null at `at` (comp space) named `name`, made unique in the comp (expressions find layers by name). Returns (layer id, name).",
        "layer.newPrimitive" => {
            "Adds a 3D primitive layer: `kind` cube, sphere, plane, torus, cone or cylinder with size options, `position` [x, y, z] and material (`baseColor`, `metallic` and `roughness` as percents 0 to 100, `emissive`, shadows). Switches the comp to Advanced 3D."
        }
        "layer.newShape" => {
            "Adds a new shape layer with one shape: `kind` rect, rounded, ellipse, polygon or star (none for an empty layer) of `size` [w, h] (default 300 × 300), centred at `position`. Paint: `fill` (default blue) or `fillGradient` (two or more colours, `fillAngle` in degrees, `fillType: radial` for a radial one), and `stroke` with `strokeWidth`; the Tools bar's paint is not used. Returns the layer."
        }
        "layer.newSolid" => {
            "Adds a solid colour layer (`color` #hex or [r, g, b], default comp-sized `width`/`height`) at the top of the comp. Use for backgrounds or as a base for effects; use a shape layer for vector shapes. Returns the layer."
        }
        "layer.newText" => {
            "Adds a text layer with `text` (point text at `position`, or paragraph text in `box` [x, y, w, h]) and character options: `font`, `size`, `fill`, `stroke`, `tracking`, `leading`, `justify`. Edit it later with layer.setText. Returns the layer."
        }
        "layer.nudge" => {
            "The arrow keys in the Composition panel: move the selected (unlocked) layers by `x`, `y` comp pixels; After Effects nudges 1 pixel at the panel's magnification, 10 with Shift. Each call is one undo step."
        }
        "layer.openLayer" => "Editor window only: opens `layer` in the Layer panel. Headless sessions accept it and change nothing.",
        "layer.openSource" => {
            "Opens a layer's source: a precomp layer's composition becomes the active comp (returns {comp}); footage opens in the Footage panel in the editor."
        }
        "layer.precompose" => {
            "Moves `layers` (default selected) into a new composition `name` and replaces them with one layer of it (`mode: move`), or keeps a single layer's attributes outside (`mode: leave`); `adjustDuration` fits it to the layers, `open` opens it."
        }
        "layer.quality" => {
            "Sets render quality of `layers` (default selected): `quality` best (default), draft (faster, lower quality) or wireframe (outlines only)."
        }
        "layer.rename" => "Renames `layer` (default the selected one) to `name`. Expressions that refer to the layer by its old name are not updated.",
        "layer.revealExpressionErrors" => {
            "Finds the properties whose expressions fail in `comp` at the current time, reveals them in the timeline and returns them with their errors."
        }
        "layer.revealInFinder" => {
            "Opens the folder holding `layer`'s source file in the system file browser. Returns {url}; fails when the source is not a file."
        }
        "layer.revealSource" => "Selects `layer`'s source item in the project panel and shows the panel. Returns {item}.",
        "layer.sampling" => {
            "Sets how `layers` (default selected) are resampled when transformed: `sampling` bilinear (default) or bicubic (sharper when scaled)."
        }
        "layer.sceneEditDetection" => {
            "Finds the cuts in a video layer and marks them (`mode` markers), splits the layer at them (split) or splits and precomposes the parts (splitPrecompose); `threshold` 0…1 (default 0.25)."
        }
        "layer.select" => {
            "Selects layers in the active comp by id, name or #index (`layers`); `add` adds to the selection, `toggle` flips each. Commands without explicit `layers` act on this selection."
        }
        "layer.selectNext" => "Selects the layer below the selected one (or the top layer when none is selected), replacing the selection.",
        "layer.selectPrevious" => "Selects the layer above the selected one (or the bottom layer when none is selected), replacing the selection.",
        "layer.sequence" => {
            "Places two or more `layers` (in selection order) one after another in time; `overlap` with `duration` seconds overlaps them, `transition` dissolveFront or crossDissolve adds opacity fades."
        }
        "layer.setBlendMode" => {
            "Sets the blending mode of `layers` (default selected): `mode` Normal, Multiply, Screen, Overlay, Add and so on, or `step` ±1 to cycle. Controls how a layer combines with the layers below."
        }
        "layer.setComment" => "Sets the Comment column text of `layers` (default selected). It is a note only; rendering is unchanged.",
        "layer.setMask" => {
            "Sets a mask's `mode` (Add, Subtract, Intersect…) and `inverted` on `layer`, picking the mask by 1-based index, uid or name (`mask`). Fails when the layer has no masks; add one with layer.addMask."
        }
        "layer.setParent" => {
            "Parents `layers` (default selected) to `parent` (a layer, or null to unparent) so they follow its transform. By default their transforms are compensated so nothing moves now (`compensate: false` to keep raw values). Cycles are refused."
        }
        "layer.setSwitch" => {
            "Sets or toggles a layer switch on `layers`: `switch` video, audio, solo, lock, shy, collapse, quality, fx, frameBlend, motionBlur, adjustment, threeD, guide or preserveTransparency; `value` true/false, omitted to toggle."
        }
        "layer.setText" => {
            "Edits a text layer: replaces `text` (whole or a character `range`) and sets character and paragraph formatting (`font`, `size`, `fill`, `stroke`, `tracking`, `leading`, `justify`, OpenType features, `box`). Keys the source text when animated."
        }
        "layer.setTrackMatte" => {
            "Uses `matte` (a layer, or null to remove) as `layer`'s track matte: `kind` alpha (default), alphaInverted, luma or lumaInverted. The matte layer's video is hidden and it shapes the layer's visibility."
        }
        "layer.setTransform" => {
            "Sets a transform property of `layers` at the current time: `prop` anchor, position, scale, orientation, rotation or opacity, `value` a number or [x, y(, z)]. Keys it if animated. For any property path use prop.set."
        }
        "layer.settings" => {
            "Layer ▸ Layer Settings: cameras and lights open their own settings; solids (and adjustment layers, which are solids) edit their Solid Settings."
        }
        "layer.showAllVideo" => "Turns the video switch on for every layer in `comp` except track mattes, undoing layer.hideOtherVideo.",
        "layer.slip" => {
            "Slip edit: move a layer's source (footage or precomp) under its fixed in and out points, like dragging the source-duration bar in After Effects. `delta` in seconds or `frames`; positive moves the source later. Clamped so the source still covers the in–out span."
        }
        "layer.slipBack" => {
            "Slip edit: moves the layers' source one frame earlier under their fixed In and Out points, so the same span shows different content. Footage and comp layers only; layers with time remapping are skipped."
        }
        "layer.slipForward" => {
            "Slip edit: moves the layers' source one frame later under their fixed In and Out points, so the same span shows different content. Footage and comp layers only; layers with time remapping are skipped."
        }
        "layer.style.add" => {
            "Adds a layer style to `layers` (default selected): `style` dropShadow, innerShadow, outerGlow, innerGlow, bevelEmboss, satin, colorOverlay, gradientOverlay or stroke (display names work too). `params` sets its settings in the same step (`{\"size\": 36, \"opacity\": 60, \"color\": \"#a5b4fc\"}`); a wrong name changes nothing and the error lists the right ones. At 1080p a glow or shadow needs a size of about 30, not the default 5."
        }
        "layer.style.bevelEmboss" => {
            "Adds a Bevel and Emboss layer style to `layers` (default selected), giving them raised edges with highlight and shadow. Same as layer.style.add with bevelEmboss."
        }
        "layer.style.colorOverlay" => {
            "Adds a Color Overlay layer style to `layers` (default selected), filling their visible pixels with one colour. Same as layer.style.add with colorOverlay."
        }
        "layer.style.convertToEditable" => {
            "Converts the Photoshop layer styles of layers imported from PSD files into editable styles. Layers made here have none to convert."
        }
        "layer.style.dropShadow" => {
            "Adds a Drop Shadow layer style to `layers` (default selected): a shadow offset behind their visible pixels, using the comp's global light angle by default. Same as layer.style.add with dropShadow."
        }
        "layer.style.globalLight" => {
            "Reads (no values) or sets the comp's global light used by layer styles: `angle` and `altitude` degrees (0–90), keyed at `time` when animated. Returns {angle, altitude}."
        }
        "layer.style.gradientOverlay" => {
            "Adds a Gradient Overlay layer style to `layers` (default selected), filling their visible pixels with a gradient. Same as layer.style.add with gradientOverlay."
        }
        "layer.style.innerGlow" => "Adds an Inner Glow layer style to `layers` (default selected): a glow along the inside edges of their visible pixels.",
        "layer.style.innerShadow" => {
            "Adds an Inner Shadow layer style to `layers` (default selected): a shadow cast inside their visible edges, as if cut out."
        }
        "layer.style.list" => {
            "Returns `layer`'s layer styles: the Layer Styles group uid and whether it is on, each style (id, name, uid, enabled), and the styles that can be added."
        }
        "layer.style.options" => {
            "Editor window only: opens the Layer Style options dialog. Headless sessions accept it and change nothing; set style properties with prop.set."
        }
        "layer.style.outerGlow" => "Adds an Outer Glow layer style to `layers` (default selected): a glow around the outside of their visible pixels.",
        "layer.style.remove" => {
            "Removes one layer style from `layer` (default the selected layer): `style` by id (dropShadow, outerGlow…), name or uid, or `layerStyles` for the whole group."
        }
        "layer.style.removeAll" => "Deletes every layer style from `layers` (default selected). Returns how many layers had styles.",
        "layer.style.satin" => "Adds a Satin layer style to `layers` (default selected): interior shading that gives a glossy, silky look.",
        "layer.style.showAll" => "Show All: every style is added (new ones switched off) so they can be revealed and edited.",
        "layer.style.stroke" => {
            "Adds a Stroke layer style to `layers` (default selected): an outline around their visible pixels. For brush strokes on a layer use paint.stroke instead."
        }
        "layer.style.toggle" => "Eye switch of a style (or of the whole Layer Styles group: `style: \"layerStyles\"`).",
        "layer.timeReverse" => {
            "Plays footage or comp `layers` (default selected) backwards over the same In and Out points (time stretch −100%). Run again to restore."
        }
        "layer.timeStretch" => {
            "Changes the playback speed of `layers` by `percent` (200 = half speed) or to a new `duration` in seconds, keeping the In point, current time or Out point fixed (`hold` in, current, out); `op: reverse` reverses."
        }
        "layer.timing" => {
            "Moves or trims `layers` in time: `op` moveInToTime, moveOutToTime, trimInToTime, trimOutToTime (at the current time), or set `start`, `in`, `out` (seconds), or shift by `delta`. Times snap to frames."
        }
        "layer.trackMatte" => {
            "Sets `layers`' track matte in one step: `op` alpha, alphaInverted, luma or lumaInverted uses the layer above as the matte; above or below picks that layer; none removes it."
        }
        "layer.transform" => {
            "Applies a transform shortcut to `layers` at the current time: `op` reset (defaults), center (centre in the comp), fit, fitWidth, fitHeight (scale to the comp), flipH or flipV."
        }
        "layer.tree" => {
            "Returns `layer`'s property tree at `time` (comp seconds): every group and property with its path (as prop.set takes, e.g. transform/position), value, keyframes and expression, `depth` levels deep. Read before setting properties."
        }
        "layer.unlockAll" => "Unlocks every layer in `comp`.",
        "layer.updateMarkersFromSource" => {
            "Layer ▸ Markers ▸ Update Markers From Source: copy a precomp's composition markers onto the layer (layer time is the nested comp's time), skipping ones already there."
        }
        "learn.list" => "Lists the built-in tutorials: id, title, summary, minutes and step titles. Start one with learn.start.",
        "learn.start" => "Start `req`; replies go to `inbox` (ending with [`WorkerReply::Done`]).",
        "learn.state" => {
            "Returns the running tutorial's state: id, current step, step count, completed steps and the current step's title, text, target and accepted commands; null when none is running."
        }
        "learn.step" => {
            "Moves through the running tutorial: `action` next, back, goto (`index`) or showMe (runs the step's commands for you). Fails when no tutorial is running."
        }
        "learn.stop" => "Closes the running tutorial. Changes no project content.",
        "light.sheen" => {
            "Sweeps one specular band across a `layer` once: two CC Light Sweep effects (soft 0.14 and core 0.5 intensity, warm white, 115 degree CSS angle) clipped to the layer's alpha, driven by a Sheen Progress slider keyed 0 to 1 over `frames` (32) from `start` with the swift curve. One undo step."
        }
        "light.spotlight" => {
            "Darkens the comp (or `layers` and what is under them) except a soft hole that follows `target`: a black solid at `dim` (0.92) opacity whose Circle effects cut a core hole (softness 22 px) and a 15% spill (60 px); the radius goes from 400 to 300 px over `frames` (24) from `start` and then breathes +6% (`pulse`, `period`). One undo step; answers the new layer."
        }
        "light.controlWithCamera" => {
            "Control Light with Camera: the light rides on the camera (parented at the camera's eye, shining where it looks). Again (or `on: false`) releases it where it is."
        }
        "light.environmentBackground" => {
            "Create Environment Light Background Layer: a layer showing the environment light's image as the scene's backdrop (an equirectangular sky drawn through the camera)."
        }
        "light.fromModel" => {
            "Creates a light layer for every light inside the 3D model layers (`layers`, default selected) of `comp`, matching their type, colour, intensity and placement."
        }
        "liquify.clear" => "Clears the mesh of a Liquify effect (`effect` uid, default the first) on `layer`, removing all its distortion.",
        "liquify.stroke" => {
            "Paints one Liquify stroke on a layer's Liquify effect: `tool` warp, twirl, pucker, bloat, turbulence, reconstruction and others, `points` [[x, y]] in effect space, `size`, `pressure`, `jitter`, at `time`. Add the Liquify effect first."
        }
        "markers.convert" => "Layer marker → comp marker (when `layer` is given), or comp marker → layer marker on `toLayer` (default: the selected layers).",
        "markers.delete" => "Deletes a marker: by `index` (from 0) or the one at `at` (comp seconds), of `layer`, or of the comp when no `layer` is given.",
        "markers.list" => {
            "Lists markers: the comp's markers, or `layer`'s (layer markers): index, time, duration, comment, chapter, URL, cue point and label, with times in comp seconds."
        }
        "markers.nested" => {
            "Lists the markers of a precomp `layer`'s nested composition at the times they fall in this comp, as the layer bar shows them: time, nested time, duration, comment and label."
        }
        "markers.set" => "Edit (or with `new: true`, add) a marker. Returns its new index.",
        "mask.addVertex" => {
            "Adds a vertex at `point` [x, y] (layer pixels) to `mask` on `layer`, at `index` (default the end), with optional tangents `in`/`out`. Returns the new vertex index and selects it."
        }
        "mask.convertVertex" => {
            "Convert Vertex tool: a corner vertex becomes smooth (tangents along its neighbours, a sixth of their distance each way), a smooth one becomes a corner (no tangents). `smooth` forces one."
        }
        "mask.deleteVertices" => {
            "Deletes the selected mask vertices, or `vertices`; a path left with fewer than three vertices becomes open. Returns how many were deleted."
        }
        "mask.featherPoint.add" => {
            "Mask Feather tool: add a feather point on the path, by `segment`/`t` or at the path position nearest `point` (layer space). Without `radius`, a `point` off the path sets it from its distance (outside the mask = outer feather, inside = inner)."
        }
        "mask.featherPoint.list" => {
            "Lists the variable-width feather points on `mask` at the current time: segment, position along it (t), radius and tension."
        }
        "mask.featherPoint.remove" => "Deletes a variable-width feather point (`index`) from `mask`, or all of them with `all: true`. Returns how many remain.",
        "mask.featherPoint.set" => {
            "Drag a feather point: move it along the path (`segment`/`t`, or nearest `point`) and/or set its `radius` (signed: negative = inner) and `tension` (%)."
        }
        "mask.insertVertex" => "Add Vertex tool: insert a vertex on segment `segment` at parameter `t` (default 0.5).",
        "mask.interpolate" => {
            "Fills the time between two Mask Path keyframes (`times` [from, to] seconds, default the selected keys) with in-between keys that morph the shape smoothly; options set the key rate, vertex matching and bending resistance."
        }
        "mask.interpolationOptions" => {
            "Sets the default options mask.interpolate uses (keyframe rate, linear vertex paths, bending resistance, quality, added vertices, matching method). Returns the options; changes no masks."
        }
        "mask.moveVertices" => {
            "Moves the selected mask vertices, or `vertices` [{layer, mask, index}], by `delta` [dx, dy] in layer pixels at the current time (keyed when the path is animated); `merge` makes repeated calls one undo step."
        }
        "mask.new" => "Pen tool: a new (open by default) mask from points.",
        "mask.remove" => "Deletes `mask` (uid, name or index) from `layer` (default the selected layer). layer.mask.remove works on the selected masks.",
        "mask.removeAll" => "Deletes every mask on `layer` (default the selected layer).",
        "mask.selectVertices" => {
            "Selects mask vertices `vertices` [{layer, mask, index}]; `add` adds them, with `toggle` flipping those already selected. Returns the selection count; mask.moveVertices and mask.deleteVertices act on it."
        }
        "mask.setClosed" => {
            "Closes (default) or opens (`closed: false`) `mask`'s path on `layer`; an open mask path doesn't cut out the layer but still works for strokes and effects."
        }
        "mask.setVertex" => {
            "Moves one vertex (`index`) of `mask` to `point` [x, y] and/or sets its `in`/`out` tangents (layer pixels, relative to the vertex), at the current time. Fails on an index the path doesn't have."
        }
        "material.duplicateAssign" => {
            "Duplicate and Assign Material: the first layer's Material Options are copied onto the other selected layers of the same kind."
        }
        "material.reset" => {
            "Resets the material of `layers` (default selected) to its defaults (for model layers, the model file's own materials). Returns the layers."
        }
        "material.revealSource" => {
            "Selects the 3D model file a model `layer`'s material comes from in the project panel. Returns {item}; fails when the material is the layer's own."
        }
        "material.set" => {
            "Sets material options on 3D `layers` at `time`: `castsShadows` and `acceptsShadows` (off, on, only), `acceptsLights`, reflections, `baseColor`, `metallic` and `roughness` (percents 0 to 100, a primitive's), `emissive`, and a card's specular and light transmission. Fails when no given layer has an option named."
        }
        "mediaBrowser.action" => "A browser-specific toolbar action (web: Open Folder…, Add Files…).",
        "mediaBrowser.addFavorite" => "Adds the folder `path` (default the browser's current folder) to the Media Browser favorites. Changes no project.",
        "mediaBrowser.fileInfo" => {
            "Returns a file's info without importing it: kind, size, created and modified dates and, for video, image, audio or 3D files, its footage metadata (size, duration, frame rate, codec)."
        }
        "mediaBrowser.go" => {
            "Opens folder `path` (or \"..\" for the parent, default home) in the Media Browser and returns its listing, as mediaBrowser.list; `importableOnly` hides files that can't be imported."
        }
        "mediaBrowser.import" => {
            "Imports files from the Media Browser (`paths`); a numbered still brings its image sequence unless `sequence: false`. `addToComp` with `time`, `index`, `position` also places them in the active comp as layer.addItem does."
        }
        "mediaBrowser.list" => {
            "Lists a folder (`path`, default the browser's current one): entries with name, kind and size, the parent folder, favorites, places and available actions; `importableOnly` hides other files."
        }
        "mediaBrowser.removeFavorite" => "Removes the folder `path` (default the current folder) from the Media Browser favorites.",
        "motion.sketch" => {
            "Motion Sketch: `points` are `[t, x, y]` (t = capture seconds from the start) or `[x, y]` (one per frame); recorded from `start` (default the current time)."
        }
        "paint.brushPreset" => {
            "Chooses a brush preset for later paint strokes by index or name (`preset`); without `preset` returns the preset names. Paint tool state only; the project is unchanged."
        }
        "paint.options" => {
            "Sets the paint tool options later paint.stroke calls use: colour, opacity, flow, mode, channels, duration mode, brush tip (diameter, angle, roundness, hardness, spacing), pressure, clone source and offset; `reset` restores defaults. Returns them."
        }
        "paint.presets" => "Lists the brush presets with index, name, diameter, angle, roundness, hardness and spacing.",
        "paint.removeStroke" => "Find a stroke group by uid or name across the layer's Paint effects.",
        "paint.setCloneSource" => {
            "Sets the clone stamp source to `point` [x, y] (effect space) on `layer` for later paint.stroke calls with kind clone. Paint tool state only."
        }
        "paint.stroke" => {
            "Paints one stroke on a layer (adds a Paint effect if needed): `kind` brush, clone or eraser, `points` [[x, y, pressure]] in effect space, at `time`, with colour, size and opacity options; `durationMode` writeOn animates it."
        }
        "path.convertToBezier" => {
            "Turns RotoBezier masks back into ordinary Bezier masks with editable tangents (for `mask` or the selected ones on `layer`). The shape stays the same. On a shape layer, `shape` (the uid of a Rectangle, Ellipse or Polystar Path; without `shape` and `mask`, every one of a layer with no mask to act on) becomes a Path item with the same outline at the current time, whose vertices mask.setVertex and the like then edit (keyframes and expressions of the shape's size, position and radii are dropped); answers {paths: [uid]}."
        }
        "path.freeTransform" => {
            "Free Transform Points: scale / rotate / move the selected vertices (or whole masks) about an anchor (default: the centre of their bounding box), at the current time."
        }
        "path.groupShapes" => {
            "Puts the selected shape items (in one group of a shape layer's contents) into a new group, so they share a transform. Fails when they are in different groups."
        }
        "path.rotoBezier" => {
            "Turns RotoBezier on or off for `mask` (or the selected masks): `value`, default toggles. With it on, tangents are computed automatically from the vertices for smooth curves."
        }
        "path.setFirstVertex" => {
            "Makes one vertex (the single selected vertex, or `layer`, `mask` uid and `index`) the first vertex of its mask path; this changes how mask interpolation and stroke start points line up."
        }
        "path.ungroupShapes" => {
            "Dissolves the selected shape groups, moving their items into the parent group; a static group transform is baked into the items."
        }
        "paths.nullsFollowPoints" => {
            "Creates a null for every vertex of a path (`layer` plus `path`/`prop`), or for `pins` of a puppet, with expressions so each null follows its point. Use to attach layers to path points. Returns the nulls."
        }
        "paths.pointsFollowNulls" => {
            "Creates a null for every vertex of a path (or Position/Advanced puppet `pins`) and adds expressions so the path follows the nulls; move the nulls to animate the shape. Returns the nulls."
        }
        "paths.tracePath" => {
            "Creates a null that moves along a path (`layer` plus `path`/`prop`) driven by a Progress slider keyed 0→100% over the path's span; `loop` repeats it. Returns the null; parent layers to it to follow the path."
        }
        "playback.audio" => "Editor window only: turns preview audio on or off. Headless sessions accept it and change nothing.",
        "playback.cacheWhenIdle" => "Editor window only: turns caching frames while idle on or off. Headless sessions accept it and change nothing.",
        "playback.scrubAudio" => "Editor window only: turns audio while scrubbing on or off. Headless sessions accept it and change nothing.",
        "playback.settings.get" => {
            "Returns the preview settings for a preview `shortcut` (spacebar by default): frame rate, skip, resolution, range, loop, audio, and the planned start, end and step for the active comp."
        }
        "playback.settings.set" => {
            "Changes the Preview panel settings of a shortcut (`shortcut`, or `current`): range, frame rate, resolution, loop, whether video, audio and overlays play, pre- and post-roll, frames skipped; `reset: true` restores the defaults."
        }
        "playback.toggle" => {
            "Editor window only: starts or stops the preview of the active comp. Headless sessions accept it and change nothing; render a still or movie to see results."
        }
        "prefs.get" => "Returns one setting by `key` (e.g. general.undoLevels), or all settings when omitted. List the pages and keys with prefs.pages.",
        "prefs.open" => "Editor window only: opens Settings at `page`. Headless sessions accept it and change nothing; use prefs.get and prefs.set.",
        "prefs.pages" => "Lists the settings pages and the setting keys on each, for prefs.get and prefs.set.",
        "prefs.reset" => {
            "Resets the settings on `page` (general, labels, project…) or all settings to their defaults; script permissions stay as the user set them when a script calls it."
        }
        "prefs.set" => {
            "Changes settings: `key` and `value` (e.g. general.undoLevels), or several at once as `values` {key: value}. Saved to the preferences; scripts can't change scripting permissions. Affects the app, not the project file."
        }
        "project.delete" => {
            "Delete project items (folders with their contents). Layers that use a deleted item and Render Queue items of deleted comps go too, like After Effects' Edit ▸ Clear in the Project panel."
        }
        "project.deleteWithoutConfirmation" => {
            "Editor window only: deletes the selected project items without the confirmation dialog. Headless sessions accept it and change nothing; check project.usage first."
        }
        "project.duplicate" => {
            "Duplicate project items (Edit ▸ Duplicate in the Project panel): comps get fresh layer ids and a numbered name; the copies are selected."
        }
        "project.move" => "Move items into `folder` (id/name; `null` = the project root).",
        "project.newFolder" => {
            "Creates a folder in the project panel (`name`, inside folder `parent`) and selects it. Returns {item}; move items into it with project item commands."
        }
        "project.rename" => "Renames a project item (`item` id or name, default the selected one) to `name`. Layer names using it are unchanged.",
        "project.select" => {
            "Selects project items by id or name (`items`); `add` adds to the selection. Returns the selected ids; item commands without `items` act on it."
        }
        "project.setComment" => "Sets the Comment column text of project `items` (default selected). A note only.",
        "project.setLabel" => "Sets the label colour of project `items` (default selected): `label` a colour name or index 0–16.",
        "project.setProjectComment" => "Sets the project's own comment (Project panel). A note stored in the project file.",
        "project.summary" => {
            "Returns every project item: id, name, type (composition, footage, solid, folder), label, parent folder, size, duration, and for comps their layer count and frame rate. Start here to find comp and item ids."
        }
        "project.usage" => {
            "Returns what deleting `items` (default selected) would remove: the item count including folder contents, and the layers and compositions that use them. Changes nothing."
        }
        "prop.addKey" => {
            "Adds a keyframe to one property (`layer` plus `path` such as transform/position, or `prop` uid) at `time` (layer seconds, or comp seconds with `timeBase: comp`; default now), with `value` or the current value. Turns animation on if needed."
        }
        "prop.convertExpressionToKeyframes" => {
            "Bakes a property's expression into a keyframe on every frame of the layer and disables the expression, so the result no longer depends on it. For `layer` plus `path`/`prop`."
        }
        "prop.deselect" => {
            "Deselect only the named property, preserving unrelated property, layer, key and mask-vertex selections. Selected keys also imply selectedProperties in the script API."
        }
        "prop.duplicateGroup" => {
            "Duplicates the property group `prop` (uid) on `layer` (a mask, effect, shape group or text animator) next to the original; answers the new group's uid."
        }
        "prop.get" => {
            "Reads one property (`layer` plus `path` like transform/position or effects/Glow/radius, or `prop` uid): its value at `time` (comp seconds, default now), type, keyframes with times and interpolation, and expression."
        }
        "prop.moveGroup" => {
            "Moves a property group (`prop` uid: an effect, mask, shape item or text animator) to position `index` (1-based) among its siblings, e.g. to change effect or shape stacking order."
        }
        "prop.pickWhip" => {
            "Links one property to another with an expression, as the pick whip does: `target` {layer, path or prop, dimension} sets e.g. thisComp.layer(\"A\").transform.position[0]; `expression` with `range` inserts the reference into existing text."
        }
        "prop.removeGroup" => {
            "Deletes the property group `prop` (uid) from `layer`, with everything under it: a mask, an effect, a shape group, a text animator, one of its selectors or a layer style; or one property a text animator animates (its uid under the animator's Properties)."
        }
        "prop.renameGroup" => {
            "Renames a property group (`prop` uid: an effect, mask, shape group or text animator) on `layer` to `name`. Expressions referring to the old name aren't updated."
        }
        "prop.reset" => {
            "Resets a property (`path` or `prop`) of `layer`: removes its keyframes and expression; the value becomes `default`, else the property's own default."
        }
        "prop.select" => {
            "Selects a property (`layer` plus `path` or `prop`); `add` keeps the current selection, `selectKeys` also selects its keyframes. Selecting a mask path selects its points."
        }
        "prop.separateDimensions" => {
            "Splits a layer's Position into separate X, Y (and Z) properties so each can be keyed on its own (`value` true), or joins them back (false); default toggles. Keyframes are split or merged accordingly."
        }
        "prop.set" => {
            "Sets a property's value: `path` (e.g. `transform/opacity`, `effects/#1/blurriness`) or `prop` (uid) of `layer`, to `value` (a number, an array, or a colour as \"#rrggbb\"). On an animated property it adds or changes the keyframe at `time` (layer seconds, or comp seconds with `timeBase: comp`; default the current time). With `selected`, also sets the other selected layers: the same value, or with `offset` the same change from each layer's own value. One undo step (`merge` folds a continuing scrub into one)."
        }
        "prop.setExpression" => {
            "`{layer?, path|prop, ...}` for one property, or (Animation > Add Expression) no `path`/`prop` for the properties selected in the timeline, all in one undo step."
        }
        "prop.setGroupEnabled" => {
            "Switches the property group `prop` (uid) on `layer` on or off (`value`): a mask, effect, shape group, text animator or layer style."
        }
        "prop.toggleAnimation" => {
            "Turns animation (the stopwatch) of a property (`path` or `prop` of `layer`) on, keying its current value, or off, removing its keyframes. With `selected`, the other selected layers' same property follows."
        }
        "prop.toggleKey" => {
            "Adds a keyframe at the current time to a property (`path` or `prop` of `layer`), or removes the one there. With `selected`, the other selected layers' same property gets (or loses) its keyframe too."
        }
        "puppet.addPin" => {
            "Adds a puppet pin to `layer` at `position` [x, y] (effect space): `kind` position (default), advanced, bend, starch or overlap; creates a mesh (Puppet effect) where none covers that point. Returns the pin."
        }
        "puppet.follow" => {
            "Follow-Through (hair, cloth, tails, overlapping action): each follower pin trails the leader pin's motion by a delay, through an expression on its Position. The followers nearest the leader trail least; with `cascade` the k-th nearest trails k × `delay`, otherwise all trail by `delay`."
        }
        "puppet.info" => {
            "Returns a layer's puppet meshes at `time`: each mesh's outline, triangle and vertex counts, rest and deformed mesh, and its pins with kind, rest and current position. Empty without a Puppet effect."
        }
        "puppet.mesh" => {
            "Sets mesh options (`density`, `expansion`, `triangles`) for `mesh` on `layer`, or the defaults for new meshes when no layer is given; `showMesh` toggles the mesh overlay. With no options returns the current ones."
        }
        "puppet.movePin" => {
            "Moves a puppet `pin` (uid or name) to `position` [x, y] at `time`, keying it, which deforms the mesh; starch and overlap pins move their region instead. Bend pins have no position (use puppet.setPin)."
        }
        "puppet.recordOptions" => {
            "Record Options (the Puppet tool's Record Options… dialog): `speed`, `smoothing`, `useDraftDeformation`, `showMesh`. Returns the options."
        }
        "puppet.recordPin" => {
            "Puppet tool Record mode (⌘/Ctrl-drag a pin): the pin's drag, sampled in real time, becomes Position keyframes at the comp frame rate from the current time on, replacing the keys in the recorded span."
        }
        "puppet.removePin" => "Deletes puppet pins: `pin` or `pins` by uid or name, default the selected pins. Returns how many were removed.",
        "puppet.selectPins" => {
            "Select pins (the viewer's click / Shift-click on a pin). Plain: only these pins; `add`: add them; `toggle`: flip each. An empty `pins` deselects every pin."
        }
        "puppet.setPin" => {
            "Sets a puppet pin's values at `time`: `position`, `scale` %, `rotation` °, `amount` %, `extent` px, `inFront`, keyed when animated. Which apply depends on the pin kind."
        }
        "render.addOutputModule" => "Composition ▸ Add Output Module: another output module on a render item (the same frames are encoded once more).",
        "render.backend" => {
            "Video Rendering and Effects: report or set the renderer (Mercury GPU Acceleration when a GPU adapter exists, else Mercury Software Only)."
        }
        "render.preRender" => {
            "Composition ▸ Pre-render…: queue the comp with a lossless-with-alpha module whose post-render action imports the result and replaces the comp's uses."
        }
        "render.saveCurrentPreview" => "Composition ▸ Save Current Preview…: render the work area of the comp straight to a movie file.",
        "renderQueue.add" => {
            "Adds `comp` to the Render Queue with output `format` (h264, hevc, av1, prores, webm, png/jpeg/tiff/exr sequences, gif, wav, aiff), `output` path, `template`, `resolution` and `quality`. Start it with renderQueue.render."
        }
        "renderQueue.applyTemplate" => "Apply templates to an item: `renderSettings` and/or `outputModule` (to `module`).",
        "renderQueue.deleteTemplate" => "Deletes a render settings or output module template (`kind`, `name`) from the project's templates.",
        "renderQueue.duplicate" => "Duplicates a render queue item (`item` id or `index`) with its render settings and output module.",
        "renderQueue.formats" => {
            "Lists the output formats with id, label, extension and whether each is a sequence, supports alpha and audio, and is available in this build, plus the video codec profiles and levels."
        }
        "renderQueue.list" => {
            "Returns the Render Queue: each item's index, comp, status, render checkbox, settings, output modules and paths, plus whether rendering is running and its progress."
        }
        "renderQueue.move" => "Moves a Render Queue item (`item` or `index`) to position `to` (1-based); items render in queue order.",
        "renderQueue.remove" => "Removes an item from the render queue: `item` (id) or `index`.",
        "renderQueue.render" => "Render a comp frame with the session's footage + expression hosts.",
        "renderQueue.saveTemplate" => {
            "Save a template: from an item's settings (`item`/`index`, `module`), or the slot default, adjusted by `params` (the same keys as setRenderSettings / setOutputModule)."
        }
        "renderQueue.setLog" => "Sets a Render Queue item's log level: `log` errorsOnly, plusSettings or plusPerFrameInfo. Returns the item.",
        "renderQueue.setNotify" => "Turns the notification when the queue finishes on or off (`notify`, default toggles).",
        "renderQueue.setOutput" => {
            "Sets where a Render Queue item's output module (`module`, default the first) writes: `path` a file path or a template such as [compName].[fileExtension]. Re-queues a finished item. Returns the item."
        }
        "renderQueue.setOutputModule" => {
            "Changes a Render Queue item's output module: `format` (h264, hevc, av1, prores, webm, image sequences, gif, audio), channels and alpha, quality or bitrate, codec profile, crop, resize, audio settings and post-render action, or a `template`."
        }
        "renderQueue.setOverflowFolders" => "Sets the folders used in order when the output volume is full (`folders`, an empty list turns overflow off).",
        "renderQueue.setRender" => {
            "Ticks or unticks an item's Render checkbox (`render`, default toggles): only ticked (queued) items render. Returns {render}."
        }
        "renderQueue.setRenderSettings" => {
            "Changes a Render Queue item's render settings: `quality`, `resolution`, proxies, effects and solo overrides, `colorDepth`, frame blending, motion blur, `timeSpan` (workArea, comp or custom with `start`/`end` seconds), `frameRate`, or a `template`."
        }
        "renderQueue.setTemplateDefault" => {
            "Makes template `name` the default for a `slot` (movie, still, preRender, movieProxy, stillProxy) of `kind` renderSettings or outputModule; new queue items use it."
        }
        "renderQueue.stop" => "Stops the render in progress; finished items keep their output. Returns {stopped}.",
        "renderQueue.templates" => "Lists the render settings and output module templates with their settings and which ones are the defaults per slot.",
        "roto.cancel" => "Stops the Rotoscope Brush propagation running in the background; frames done so far are kept.",
        "roto.clearStrokes" => {
            "Removes Rotoscope Brush strokes from `layer`'s Rotoscope Brush effect: all, only those on layer `frame`, or only one `kind`. Fails while the Rotoscope Brush is frozen."
        }
        "roto.freeze" => "Freeze Frame: time remapping holding the current source frame for the whole layer.",
        "roto.model.download" => {
            "Downloads the official Rotoscope Brush model weights `id` and verifies them (desktop only, uses the system curl); `wait` blocks until done."
        }
        "roto.model.install" => {
            "Installs Rotoscope Brush model weights from the file at `path`, verified against the registry's SHA-256 (`id` names them); `wait` blocks until installed."
        }
        "roto.model.remove" => "Deletes the installed Rotoscope Brush model `id` from disk; segmentation falls back to the classical model.",
        "roto.model.select" => {
            "Chooses the segmentation model the Rotoscope Brush uses: `id` classical or mobilesam (must be installed); `wait` returns once it is loaded."
        }
        "roto.models" => "Lists the Rotoscope Brush models: id, name, whether each is installed, its size and which is selected.",
        "roto.options" => {
            "Sets Rotoscope Brush tool options: brush `diameter` and `refineDiameter` (px), the viewer `view` (alphaBoundary, alpha, alphaOverlay, none), overlay and boundary colours and opacity, and `autoPropagate`. Returns them; the project is unchanged."
        }
        "roto.propagate" => {
            "Propagates the Rotoscope Brush segmentation from its stroked base frame `direction` forward, backward or both (default) across the span, extended to layer frame `to` if given; runs in the background, `wait` blocks until done."
        }
        "roto.span" => {
            "Sets the frames the Rotoscope Brush segmentation covers: `start` and `end` in layer frames, kept around the stroked frames. Fails when there are no strokes or the Rotoscope Brush is frozen."
        }
        "roto.status" => {
            "Returns the Rotoscope Brush state for `layer`: running and progress, base frame, span, stroke count and frames, frozen, frame limits; for `frame` also its matte coverage, with `matte` the RLE matte and `compareTo` an IoU against another matte."
        }
        "roto.stroke" => {
            "Paints a Rotoscope Brush stroke on `layer` at layer `frame` (default now): `kind` fg (foreground, default), bg, refine or refineErase, `points` [[x, y]] in layer pixels, `radius`. Adds the Rotoscope Brush effect if needed, then segments the frame."
        }
        "roto.unfreeze" => {
            "Unfreezes a layer's Rotoscope Brush so its strokes and span can be edited again (frozen mattes are cached and not recomputed). Returns {frozen: false}."
        }
        "scopes.analyze" => {
            "Analyses the comp's frame at `time`: `scope` waveform (RGB, luma, YC), vectorscope, histogram or parade, with `standard` rec601/709/2020; returns the scope data plus mean RGB and min, max and mean luma. Use to check exposure and colour."
        }
        "script.run" => {
            "run JavaScript (the After Effects scripting object model: `app.project`, comps, layers, properties…) and report `{ok, result, output, error}`; script errors are reported in `error` (with line and column), not as a command failure."
        }
        "script.settings.get" => {
            "Reads a script setting (`section`, `key`) from the settings store, as the scripting API's app.settings.getSetting() does. Returns {have, value}."
        }
        "script.settings.save" => {
            "Saves a script setting (`section`, `key`, text `value`) in the settings store, as the scripting API's app.settings.saveSetting() does; it persists across sessions."
        }
        "scriptWindow.click" => "press a button / toggle a checkbox / pick a radio button.",
        "scriptWindow.close" => "close a window (a dialog's `show()` returns `result`, default 2 = Cancel).",
        "scriptWindow.get" => {
            "Returns a script window's controls (`window` id or title): its tree of controls with type, name, text, value, items, selection, bounds and enabled state."
        }
        "scriptWindow.list" => "Lists the open script windows and panels: window id, title, kind (dialog, palette, window, panel), script, modal and size.",
        "scriptWindow.set" => {
            "Sets a control of a script window: `widget` (id or name) to `value` (text, number, bool, or a list item's index or text); `changing: true` sends a live update, as while dragging a slider."
        }
        "shape.dashes.add" => {
            "Adds a dash/gap pair to a shape stroke (`prop` = the stroke's uid): the first call turns dashes on (Dash 10), later ones add Dash 2/Gap 2 and Dash 3/Gap 3, up to three pairs."
        }
        "shape.dashes.remove" => "Removes the last dash/gap pair from a shape stroke (`prop` = the stroke's uid); removing the last one turns dashes off.",
        "shape.fillStroke" => {
            "The toolbar's Fill and Stroke options with shape layers selected (as in After Effects, they change the layers' fills and strokes): `fill` sets every Fill's colour, `stroke` every Stroke's, `strokeWidth` every stroke's width. A stroke colour or a width above 0 gives the paths that have no stroke one."
        }
        "shape.newPath" => {
            "Pen tool on a shape layer: a new Shape group (\"Shape n\" with Path 1 and the Tools bar's Stroke and Fill unless given) on the given / selected shape layer, or on a new shape layer when none is given. Vertices are in the layer's space, or comp space (`space: \"comp\"`, the default for a new layer)."
        }
        "shape.newShape" => {
            "Draws a shape as the shape tools do: `kind` of `size` at `position` goes into the selected shape layer's contents (or `layer`'s) as a new group, else into a new shape layer, painted with the Tools bar's Fill and Stroke (shape.toolOptions) unless given here, gradients included (`fillGradient`, `fillAngle`). Returns {layer, group}."
        }
        "shape.stroke.taper" => {
            "Sets a shape stroke's taper (`prop` = stroke uid): `startLength`/`endLength` (in `units` pixels or percent), `startWidth`/`endWidth` %, and `startEase`/`endEase` %, so the line thins toward its ends."
        }
        "shape.stroke.wave" => {
            "Makes a shape stroke wavy (`prop` = stroke uid): `amount` %, `wavelength` (pixels) or `cycles` per `units`, and `phase` degrees."
        }
        "shape.toolOptions" => {
            "The Tools bar's options for the shape tools and the Pen: Tool Creates Shape / Tool Creates Mask (`createsMask`) and the Fill and Stroke new shapes get (the keys of [`paint_p`]); `reset` restores the defaults first. Reports the options."
        }
        "shortcuts.conflicts" => "Lists keyboard shortcut conflicts in the active preset: each command, its keys, and the command and scope it collides with.",
        "shortcuts.export" => "Exports a keyboard shortcut preset (`preset`, default the active one) as JSON, written to `path` when given, else returned.",
        "shortcuts.import" => "Imports a keyboard shortcut preset from `path` or an exported document in `preset`, under `name`. Returns the preset name.",
        "shortcuts.list" => {
            "Lists commands with their keyboard shortcuts; `query` filters by words in label, command or keys, `keys` finds what a key combination does, `assigned: true` only bound commands."
        }
        "shortcuts.preset" => "Manages keyboard shortcut presets: `op` select (`name`), new, duplicate (`from`), delete or rename (`newName`).",
        "shortcuts.reset" => "Resets one `command`'s shortcut to its default, or the whole active preset when no command is given.",
        "shortcuts.set" => {
            "Binds keys to a `command` (with `params` when the binding passes some): `keys` \"Cmd+Shift+K\", a list, or null to unbind. Editing the default preset creates a custom copy. Returns keys and conflicts."
        }
        "storage.clear" => {
            "Delete stored data: `diskCache` (cached frames), `media` (imported footage), `projects` (saved projects), `autoSaves`, or `all` of them (settings stay). Returns what was removed: `{cleared, entries, bytes}`."
        }
        "storage.info" => {
            "Returns the browser storage the web app uses: availability, backend, usage and quota, whether storage is persistent, stored files by kind and the disk cache statistics. {available: false} on desktop."
        }
        "storage.persist" => "Asks the browser to make the web app's storage persistent so it isn't evicted. Returns {requested, persisted}.",
        "templates.create" => {
            "Replaces the open project with an untitled copy of a project template (`id` from templates.list, or `path` to an .ectemplate) and opens its main comp; embedded footage is extracted beside `projectPath` or into `footageDir`."
        }
        "templates.delete" => "Deletes a saved user template (`id`, `user/<file>`). Built-in templates cannot be deleted.",
        "templates.list" => {
            "Lists the project templates: id, name, description, category, built-in or user, size, frame rate, duration and controls; `thumbnails: true` adds a 256×144 thumbnail."
        }
        "templates.saveAs" => {
            "Saves the current project as a user project template `name` (with `description`, `category`); the active comp becomes its main comp. Footage is embedded up to `embedLimitMB` (256), larger files stay linked."
        }
        "templates.thumbnail" => "Returns a template's thumbnail (`id`): width, height and RGB pixels as hex.",
        "text.addSelector" => "Animation ▸ Add Text Selector ▸ Range / Wiggly / Expression (see `layer.addTextSelector`).",
        "text.animatorFontAxes" => {
            "Animation ▸ Animate Text ▸ Variable Font Axes: the axes of the layer's font, or (with `axis`) a new animator — or a property on `animator` — that offsets that axis (user units, weighted by the selectors, added to the axis default)."
        }
        "text.animatorProperties" => {
            "Lists the properties a text animator can drive (position, scale, rotation, opacity, fill and stroke colour, tracking, blur and others) with their labels, for layer.addTextAnimator."
        }
        "text.delete" => "Delete the selection, or one character / word before (backward) or after the caret.",
        "text.edit" => {
            "Starts editing a text layer (the editor's text cursor): selects all text by default, or `select` [anchor, caret], `caret`, or none. Returns {layer, anchor, caret}; typing commands then act there. To set text directly use layer.setText."
        }
        "text.endEdit" => "stop editing (a Type-tool layer left empty is deleted).",
        "text.fontFeatures" => {
            "The OpenType features of a font (`font` / `style`, else the target text layer's style at the caret or its first character), and which Character panel options it sets with its own glyphs (the others are synthesized or have no effect)."
        }
        "text.fonts" => {
            "The installed font families (bundled and system), with their styles, where each comes from and its name in the font's own language. `query` keeps the families whose English or native name contains it (case-insensitive); `rescan` first looks for fonts installed since the app started."
        }
        "text.insert" => "Replace the selection (or `range`) with `text`; the caret goes after it.",
        "text.moveCaret" => {
            "Moves the text cursor in the text being edited: `to` left, right, wordLeft/Right, lineStart/End, up, down, paraStart/End, start or end; `extend` grows the selection. Needs text.edit first."
        }
        "text.presets" => "Lists the built-in text animation presets (id, name, description) layer.applyTextPreset accepts, the promo presets included.",
        "text.removeAllAnimators" => "Deletes every text animator from text `layers` (default selected), returning the text to its unanimated look.",
        "text.setSelection" => {
            "Sets the selected character range in the text being edited: `start` and `end` (character indices), or `anchor`/`caret`, or `select: \"all\"`. Formatting from layer.setText then applies to it."
        }
        "time.back10" => "Moves the current time 10 frames earlier.",
        "time.end" => "Moves the current time to the last frame of the composition.",
        "time.forward10" => "Moves the current time 10 frames later.",
        "time.go" => {
            "Moves the current time `to` start, end, workStart, workEnd, layerIn/layerOut (the selected layer), or nextKey/prevKey (keyframes of `prop` or the visible properties, markers and the work area)."
        }
        "time.layerIn" => "Moves the current time to the selected layer's In point.",
        "time.layerOut" => "Moves the current time to the selected layer's last frame (its Out point).",
        "time.nextFrame" => "Moves the current time one frame later.",
        "time.nextKey" => "Moves the current time to the next keyframe or marker visible in the timeline.",
        "time.previousFrame" => "Moves the current time one frame earlier.",
        "time.previousKey" => "Moves the current time to the previous keyframe or marker visible in the timeline.",
        "time.set" => {
            "Moves the current time (CTI) of the active comp to `time` (seconds), `frame` or `timecode`. Commands that act at the current time, such as keying and previews, use it."
        }
        "time.start" => "Moves the current time to the first frame of the composition.",
        "time.step" => {
            "Moves the current time by `frames` (negative = earlier). Returns the new time; most commands without an explicit time act at the current time."
        }
        "track.analyze" => {
            "Tracks the current tracker's track points through the footage: `direction` forward (default), backward or one frame either way, between `start` and `end` seconds; runs in the background, `wait` blocks. Then track.apply writes the result."
        }
        "track.apply" => {
            "Applies a tracker's motion (`tracker` on `layer`, default the active one) to its target layer or effect point; `dimensions` xy, x or y."
        }
        "track.camera" => {
            "Animation ▸ Track Camera / the Tracker panel button: apply the 3D Camera Tracker to the layer (reusing one it already has), set the given options and start the analysis."
        }
        "track.delete" => "Deletes a tracker (`tracker`, default the active one) from `layer`.",
        "track.editTargetDialog" => "Editor window only: opens the Motion Target dialog. Headless sessions accept it and change nothing; use track.setTarget.",
        "track.extractFaceMeasurements" => {
            "Extract & Copy Face Measurements: measure the Face Track Points at every keyed time, key them into the layer's Face Measurements effect and put those keys on the keyframe clipboard."
        }
        "track.mask" => {
            "Tracks a mask (`mask`, default selected) on `layer` so its path follows the footage: `method` position, positionScale, positionScaleRotation, …Skew, perspective, faceOutline or faceDetailed; `direction`, `start`/`end`; `wait` blocks."
        }
        "track.maskMethod" => {
            "Sets the default mask tracking method track.mask uses: position, positionScale, positionScaleRotation, positionScaleRotationSkew, perspective, faceOutline or faceDetailed."
        }
        "track.motion" => {
            "Adds a transform motion tracker (one track point) to the footage `layer` (default selected) and makes it current; place its point with track.setPoint, then track.analyze and track.apply to a target layer."
        }
        "track.new" => {
            "Adds a tracker to `layer`: `kind` transform, stabilize, affine, perspective or raw, tracking `position`, `rotation` and `scale` as set, applied to `target`. Returns the tracker."
        }
        "track.options" => {
            "Sets the current tracker's options: `channel` rgb, luminance or saturation, `blur`, `enhance`, `subpixel`, `adaptEveryFrame`, confidence `threshold` % and the low-confidence `action` (continue, stop, extrapolate, adapt)."
        }
        "track.optionsDialog" => {
            "Editor window only: opens the Motion Tracker Options dialog. Headless sessions accept it and change nothing; use track.options."
        }
        "track.property" => "Track this Property: track another layer and drive the selected property's layer.",
        "track.reset" => {
            "Resets a tracker (`tracker`, default the active one) on `layer`: its track points go back to their default places and their tracked data is cleared."
        }
        "track.select" => "Makes a tracker current (`tracker` uid, name or #n on `layer`); track.* commands without `tracker` act on it.",
        "track.setPoint" => {
            "Moves or resizes track point `point` (1-based) at `time`: feature `center` or `move` [dx, dy], `featureSize`, `searchOffset`, `searchSize` and `attachOffset`, in layer pixels. Place points on high-contrast features before analyzing."
        }
        "track.setTarget" => "Sets the layer the tracker's result is applied to (`target`, or null); a stabilize tracker can only apply to the tracked layer.",
        "track.setType" => {
            "Changes the current tracker's type (`kind` transform, stabilize, affine, perspective, raw) and which of `position`, `rotation`, `scale` it tracks, adding or removing track points as needed."
        }
        "track.stabilize" => {
            "Adds a stabilize tracker to the footage `layer` (default selected): after analysis, applying it moves the layer itself to cancel camera shake."
        }
        "track.status" => {
            "Returns tracking state: whether tracking, mask tracking or warp stabilizing runs and its progress, and for the current tracker its track points (center, feature and search sizes, key counts) and the layer's trackers."
        }
        "track.stop" => "Stops the motion tracking, mask tracking or warp stabilizer analysis that is running; results so far are kept. Returns {stopped}.",
        "track.warpStabilizer" => "Animation ▸ Warp Stabilizer VFX / the Tracker panel button: apply the effect to the selected layer and start analysing it.",
        "view.3d.activeCamera" => "Switches the comp viewer's 3D view to the Active Camera (what renders). View state only; the project is unchanged.",
        "view.3d.back" => "Switches the comp viewer's 3D view to Back (orthographic). View state only.",
        "view.3d.bottom" => "Switches the comp viewer's 3D view to Bottom (orthographic). View state only.",
        "view.3d.custom1" => "Switches the comp viewer's 3D view to Custom View 1 (a perspective working view). View state only.",
        "view.3d.custom2" => "Switches the comp viewer's 3D view to Custom View 2. View state only.",
        "view.3d.custom3" => "Switches the comp viewer's 3D view to Custom View 3. View state only.",
        "view.3d.default" => "Switches the comp viewer's 3D view to the Default camera view. View state only.",
        "view.3d.front" => "Switches the comp viewer's 3D view to Front (orthographic). View state only.",
        "view.3d.last" => "Switches the comp viewer back to the previously used 3D view. View state only.",
        "view.3d.left" => "Switches the comp viewer's 3D view to Left (orthographic). View state only.",
        "view.3d.right" => "Switches the comp viewer's 3D view to Right (orthographic). View state only.",
        "view.3d.top" => "Switches the comp viewer's 3D view to Top (orthographic). View state only.",
        "view.addGuide" => {
            "Adds a guide to `comp`: `orientation` vertical (default) or horizontal at `position` in comp pixels (default the centre). Guides are saved with the comp but don't render."
        }
        "view.assign3dShortcut" => "View ▸ Assign Shortcut to \"<current 3D view>\": F10, F11 or F12 switches to the view.",
        "view.channel" => {
            "Shows one channel in the viewer: `channel` rgb, red, green, blue, alpha or rgbStraight, `colorized` tints single channels, `toggle` returns to RGB when it is already shown. View state only."
        }
        "view.clearGuides" => "Removes every guide from `comp`.",
        "view.closeLockedViewer" => "Closes the locked composition viewer, if one is open. Returns {closed}; changes no project content.",
        "view.customRgb" => {
            "View ▸ Simulate Output ▸ My Custom RGB…: define the custom output device (kept in Settings) and, unless `apply` is false, simulate it."
        }
        "view.displayColor" => {
            "Reports the viewer's colour pipeline: displayColorManagement, output simulation {profile, preserveRgb}, customRgb, display profile, project workingSpace, whether display colour is active, and the locked viewer. Read-only."
        }
        "view.displayColorManagement" => "View ▸ Use Display Color Management (toggle, or `value`).",
        "view.exportGuides" => {
            "Writes the comp's ruler guides, with its width and height, to a JSON file at `path`; returns {path, guides: count}. Load them into another comp with view.importGuides."
        }
        "view.exposure" => {
            "Sets the viewer's preview exposure in stops, either absolute (`stops`) or relative (`delta`), clamped to ±40. Affects only the viewer, not renders; view.resetExposure returns it to 0. Returns the new value."
        }
        "view.extendedViewer" => {
            "Settings ▸ 3D ▸ Extended Viewer: 3D views (custom views, or Draft 3D) show the pasteboard around the comp frame with the layers that reach onto it."
        }
        "view.fastPreviewMode" => {
            "Sets the viewer's Fast Previews mode: off, adaptive, draft, fastDraft or wireframe. Viewer state only; renders are unaffected. Returns the mode id."
        }
        "view.fullScreen" => "Frontend-only: asks the editor to show the viewer full screen. Headless sessions accept it and do nothing.",
        "view.get3D" => {
            "Reports a comp's 3D view state: current view id and label, all view ids, the camera it looks through {eye, forward, zoom, ortho}, the active camera layer {id, name, twoNode} and the light layers. Read-only."
        }
        "view.grid" => "Frontend-only: toggles the viewer's grid overlay. Headless sessions accept it and do nothing.",
        "view.guides" => {
            "Frontend-only: toggles the display of ruler guides in the viewer. Guides themselves are edited with view.moveGuide and view.removeGuide."
        }
        "view.importGuides" => {
            "Reads a guides JSON file written by view.exportGuides at `path` and appends its guides to the comp (undoable). Existing guides stay. Returns {imported: count}."
        }
        "view.layerControls" => "Frontend-only: toggles layer handles, masks and motion paths in the viewer. Headless sessions accept it and do nothing.",
        "view.layout" => "Sets how many viewer panes are shown: `views` 1, 2 or 4. Viewer state only. Returns {views}.",
        "view.lockGuides" => "Frontend-only: toggles whether guides can be dragged in the viewer. Headless sessions accept it and do nothing.",
        "view.lookAtAll" => {
            "Frames the current 3D view so every active non-camera, non-light layer is visible; with the Active Camera view this moves the camera layer (undoable). Use view.lookAtSelected for a subset."
        }
        "view.lookAtSelected" => {
            "Frames the current 3D view on the given `layers` (or the selection); with the Active Camera view this moves the camera layer (undoable). Use view.lookAtAll to fit everything."
        }
        "view.moveGuide" => {
            "Moves the guide at `index` (from the comp's guides list) to `position` in comp pixels along its axis. Undoable; `merge` folds repeated moves into one undo step. Returns the position."
        }
        "view.newViewer" => "Frontend-only: opens another viewer panel. Headless sessions accept it and do nothing.",
        "view.options" => "Frontend-only: opens the View Options dialog. Headless sessions accept it and do nothing.",
        "view.panelBackground" => {
            "Frontend-only: changes the viewer panel's background colour (not the comp background; use comp.settings for that). Headless sessions do nothing."
        }
        "view.removeGuide" => "Deletes the guide at `index` from the comp's guides list. Undoable. Indices of later guides shift down by one.",
        "view.res.custom" => "Frontend-only: opens the custom viewer-resolution dialog. Render resolution is set on the render queue item, not here.",
        "view.res.full" => "Frontend-only: previews the viewer at full resolution. Render resolution is set on the render queue item, not here.",
        "view.res.half" => "Frontend-only: previews the viewer at half resolution. Render resolution is set on the render queue item, not here.",
        "view.res.quarter" => "Frontend-only: previews the viewer at quarter resolution. Render resolution is set on the render queue item, not here.",
        "view.res.third" => "Frontend-only: previews the viewer at third resolution. Render resolution is set on the render queue item, not here.",
        "view.reset3DView" => {
            "Resets the current 3D view. For a custom or orthographic view it drops the stored view camera; for Active Camera it moves the camera layer back to default framing of the comp centre (undoable; fails without a camera layer)."
        }
        "view.resetExposure" => "Sets the viewer's preview exposure back to 0 stops. Returns 0.",
        "view.rulers" => "Frontend-only: toggles the viewer's rulers. Headless sessions accept it and do nothing.",
        "view.set3DView" => {
            "Switches which view a comp's viewer looks through: activeCamera, default, front, left, top, back, right, bottom or custom1-3. Viewer state only; does not move camera layers."
        }
        "view.set3DViewCamera" => {
            "Sets the eye [x,y,z], poi [x,y,z] and zoom of a non-camera 3D view (default, orthographic or custom1-3). For the Active Camera view, edit the camera layer's transform instead."
        }
        "view.setRegionOfInterest" => {
            "Limits viewer previews to `rect` [x, y, w, h] in comp pixels (width and height positive); pass null or omit it to clear. Renders are unaffected. Returns the rect or null."
        }
        "view.shareViewOptions" => "Sets (`value`) or toggles whether all viewers share the same view options. Viewer state only. Returns the new value.",
        "view.showSnapshot" => {
            "Sets (`value`) or toggles showing the last viewer snapshot instead of the live frame. Viewer state only. Returns the new value."
        }
        "view.simulateOutput" => "View ▸ Simulate Output ▸ (profile). `custom` uses (and with `space` sets) the Custom profile.",
        "view.snapToGrid" => {
            "Frontend-only: toggles snapping layer drags to the viewer grid. Headless sessions accept it and do nothing; use view.snapping for the session's snapping flag."
        }
        "view.snapToGuides" => "Frontend-only: toggles snapping layer drags to ruler guides. Headless sessions accept it and do nothing.",
        "view.snapping" => "Sets (`value`) or toggles layer snapping in the viewer. Session state only; the document is unchanged. Returns the new value.",
        "view.snappingOptions" => "Tools bar ▸ Snapping options: sets the options passed (`toggle` flips one by key).",
        "view.splitLockedViewer" => {
            "View ▸ Split with New Locked Viewer: lock a second viewer to the comp (default the active one) and its current 3D view (or `view`)."
        }
        "view.takeSnapshot" => {
            "Renders the active comp at its current time (with `scale` 0.05-1, default 1) and keeps it as the viewer snapshot for view.showSnapshot. Returns {width, height, time in seconds}."
        }
        "view.zoomIn" => "Frontend-only: zooms the viewer in one step. Headless sessions accept it and do nothing.",
        "view.zoomOut" => "Frontend-only: zooms the viewer out one step. Headless sessions accept it and do nothing.",
        "warp.analyze" => {
            "Starts Warp Stabilizer analysis for a layer's Warp Stabilizer effect (`layer`, `effect` uid, name or index; default: first found). `wait` blocks until done and includes warp.status. Returns {layer, effect, frames, running, progress}."
        }
        "warp.cancel" => {
            "Stops the running Warp Stabilizer analysis; only enabled while one runs. Returns {stopped}. Analysis already stored on the effect stays."
        }
        "warp.status" => {
            "Reports Warp Stabilizer progress {running, progress, banner} and, for a layer's Warp Stabilizer effect, whether it is analyzed or pending, frame count, size and the stabilization plan at the current frame. Read-only."
        }
        "window.assignWorkspaceShortcut" => {
            "Window ▸ Assign Shortcut to \"<workspace>\" Workspace: Shift+F10, Shift+F11 or Shift+F12. The frontend fills in the current workspace."
        }
        "window.editWorkspaces" => "Frontend-only: opens the workspace editor. Headless sessions accept it and do nothing.",
        "window.panel" => "Frontend-only: shows or focuses an editor panel. Headless sessions accept it and do nothing.",
        "window.resetWorkspace" => "Frontend-only: restores the current workspace's saved panel layout. Headless sessions accept it and do nothing.",
        "window.restoreScriptPanels" => {
            "open the script panels that were open when Kition Motion last quit (frontends call it once at launch). Panels whose script is gone or fails are reported and forgotten; the others still open."
        }
        "window.saveWorkspace" => "Frontend-only: saves the current panel layout into the current workspace. Headless sessions accept it and do nothing.",
        "window.saveWorkspaceAs" => "Frontend-only: saves the current panel layout as a new workspace. Headless sessions accept it and do nothing.",
        "window.scriptPanel" => "Window ▸ <script panel>: run the panel script (or bring its panel forward).",
        "window.workspace" => "Frontend-only: switches the editor to a named workspace layout. Headless sessions accept it and do nothing.",
        _ => "",
    }
}

/// Reading commands that answer nothing, with why (kition-sdk `check_commands`' `no_output`).
pub const NO_OUTPUT: &[(&str, &str)] = &[
    ("comp.close", "answers nothing (null): it changes view or session state only"),
    ("edit.deselectAll", "answers nothing (null): it changes view or session state only"),
    ("view.setRegionOfInterest", "answers nothing (null): it changes view or session state only"),
];
