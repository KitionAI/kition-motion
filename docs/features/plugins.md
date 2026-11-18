# Plug-ins and extensions

Kition Motion's core aims at After Effects parity. What After Effects users get from third parties
(effect plug-ins, scripts, script panels) is an extension in Kition Motion too, and Kition Motion
offers the same kinds of extension points as After Effects so that anyone can write their own.

## Extension points compared with After Effects

From After Effects' public documentation (the user guide's scripting pages and the Scripting
Guide):

| After Effects | Kition Motion | |
|---|---|---|
| Effect plug-ins (C/C++ SDK, `.aex` / `.plugin`) | [Effect plug-ins](#effect-plug-ins): WebAssembly modules (plug-in API v1, loaded with `effect.plugins.load`) or Rust | Different ABI: After Effects SDK plug-ins can't run (G8) |
| Scripts folder: its scripts are listed in File ▸ Scripts | `Scripts/` in the session's settings store (`file.installScript` copies a script there), listed by `file.scripts.list` with the bundled samples; the kit's sessions have no settings store, so there only the bundled scripts are listed and installing fails | Gap in the kit |
| File ▸ Scripts ▸ Run Script File… (`.jsx`, `.jsxbin`) | `.jsx` / `.js` | Gap: `.jsxbin` (an undocumented binary encoding) |
| ExtendScript preprocessor directives (`#include`, `#target`, `#targetengine`) | Not understood: a script that starts with one doesn't parse | Gap |
| `Scripts/Startup` and `Scripts/Shutdown`: run in alphabetical order when the app starts and quits | Not run | Gap |
| `Scripts/Script Panels`: listed at the bottom of the Window menu, dockable like the app's own panels, `this` is the panel | `window.scriptPanel {name}` runs a panel script with `this` as the panel and keeps the panel's controls in the engine (`scriptWindow.*`); installed panels go in `Scripts/Script Panels/` of the settings store, which the kit's sessions don't have | Gap in the kit: its editor draws no script windows or panels |
| Keyboard shortcuts for scripts (the Keyboard Shortcuts editor lists the scripts of the Scripts folder) | Shortcuts bind to commands and menu entries; the File ▸ Scripts and Window menu entries of scripts aren't offered | Gap |
| Running a script from the command line (`afterfx -r script.jsx`) | the `script.run` engine command, driven by Kition's agent through `bindings/headless` | Same purpose, other entry points |
| `app.settings` (kept in the preferences between sessions) | Kept in `script_settings.json` in the session's settings store (`script.settings.get` / `script.settings.save`); without a store, as in the kit, they last only while the document is open | Gap in the kit |
| Allow Scripts to Write Files and Access Network | The `scripting.allowScriptsWriteFiles` setting | Same |

The settings store is the engine's `ConfigStore` ([preferences.md](preferences.md)); neither
`bindings/wasm` nor `bindings/headless` gives its sessions one.

## Scripts

Scripts use After Effects' scripting object model (`app`, `app.project`, `CompItem`, layers,
`Property` with keyframes and eases, `app.beginUndoGroup`…), so most scripts written for After
Effects' documented API run unchanged. Every edit a script makes is an engine command: undoable,
journaled and the same as the UI's action. The `kition-motion-script` crate's docs
(`crates/engine/script/src/lib.rs`) list the object model.

* Run one: the `script.run {code, name}` engine command, for example from an agent.
* Install one: `file.installScript` (File ▸ Scripts ▸ Install Script File…) copies it into
  `Scripts/` of the settings store, and it is listed at once (After Effects asks for a restart).
  The kit's sessions have no settings store, so there it fails.
* `file.scripts.list` lists installed and bundled scripts and panels; `file.runScript {name}` runs
  one; `file.uninstallScript {name}` removes an installed one.

## Script panels

A script panel is a script that builds its user interface into the panel it is given as `this`.
`window.scriptPanel {name}` runs a panel script (built with the window API compatible with After Effects' ScriptUI; an installed one, or a bundled one such as
`Layer Tools.jsx`) with `this` as the panel, and the engine keeps the panel and its controls as a
model (`kition_motion_engine::script_windows`). The kit's editor draws no script windows or panels;
in the kit a panel is used through the `scriptWindow.*` commands below.

Run with `file.runScript` instead, the same script gets no panel, so it should make a floating
palette itself. A minimal panel:

```js
// Nudge.jsx: moves the selected layers 10 px right, as a dockable panel.
(function (thisObj) {
  var ui = thisObj instanceof Panel ? thisObj : new Window("palette", "Nudge", undefined, { resizeable: true });
  ui.orientation = "column";
  ui.alignChildren = ["fill", "top"];
  var go = ui.add("button", undefined, "Nudge Right", { name: "nudge" });
  var status = ui.add("statictext", undefined, "Select layers, then click.");
  go.onClick = function () {
    var comp = app.project.activeItem;
    if (!(comp instanceof CompItem)) return;
    var layers = comp.selectedLayers;
    app.beginUndoGroup("Nudge Right");
    for (var i = 0; i < layers.length; i++) {
      var p = layers[i].transform.position;
      var v = p.value;
      if (p.numKeys === 0) p.setValue([v[0] + 10, v[1]]);
    }
    app.endUndoGroup();
    status.text = "Moved " + layers.length + " layer(s).";
  };
  if (ui instanceof Window) ui.show(); else ui.layout.layout(true);
})(this);
```

Supported controls: `panel`, `group`, `button`, `iconbutton`, `statictext`, `edittext`,
`checkbox`, `radiobutton`, `slider`, `scrollbar`, `progressbar`, `dropdownlist`, `listbox`,
`tabbedpanel` / `tab` and `image`, with automatic layout compatible with ScriptUI's (`orientation`,
`alignChildren`, `alignment`, `margins`, `spacing`, `preferredSize`), resource strings, event
handlers (`onClick`, `onChange`, `onChanging`, `onClose`, `addEventListener`) and `onDraw` with
ScriptUIGraphics. Mouse events (`mousedown`, `mousemove`…) aren't sent to scripts yet. Agents use
a panel like a user: `scriptWindow.list`, `scriptWindow.get` (its controls), `scriptWindow.click`,
`scriptWindow.set` and `scriptWindow.close`, naming a control by `window` and `widget` (its id, `#id`,
`properties.name` or text). The bundled sample `Layer Tools.jsx`
(`crates/engine/engine/scripts/Script Panels/`) and the Ease Presets extension below are longer
examples.

## Bundled extensions

Optional tools that ship with Kition Motion but are not part of its core: they are scripts on the
public scripting API, like the third-party scripts After Effects users install, and are listed
with the bundled samples (`file.scripts.list` reports them as `extension`). A script of the same
name among the installed scripts takes their place.

### Ease Presets (`crates/engine/engine/extensions/script-panels/Ease Presets.jsx`)

A script panel (`window.scriptPanel {"name": "Ease Presets.jsx"}`): easing curves kept by name and applied to pairs of neighbouring
selected keyframes (#254; it was a core panel in v0.6.0). A curve is the out side of a segment's
first key and the in side of its second, in the Keyframe Velocity dialog's terms: influence in
percent, and speed relative to the segment's average speed (1 = as fast as a straight line, 0 = at
rest), so one curve fits any duration and change of value.

* Click a preset to apply it to every pair of neighbouring selected keyframes of each selected
  property, in one undo step (per dimension, and along the motion path for spatial properties;
  the keys' other sides keep playing as they did). The keys stay selected, so you can try another.
* The drawing shows the working curve; type its numbers (Out % and Speed, In % and Speed) and
  click Apply, or click From Keys to read the curve between the first selected pair.
* Save keeps the working curve under the typed name (replacing a preset of that name); Rename
  and Delete act on the selected preset of your own. The twelve built-in presets can't change.
* User presets are kept with `app.settings` (section `Ease Presets`, key `userPresets`); in the
  kit they last while the document is open (see the table above).

It uses only After Effects' documented scripting API (`comp.selectedProperties`,
`Property.selectedKeys`, `keyInTemporalEase` / `keyOutTemporalEase`, `setTemporalEaseAtKey` with
`KeyframeEase`, `setInterpolationTypeAtKey`, `app.settings`, ScriptUI), so it is also a worked
example of a keyframe tool.

## Effect plug-ins

Kition Motion loads third-party effects through a small, versioned plug-in API (**API version 1**).
A plug-in effect behaves exactly like a built-in one: it has a category like the built-in effects,
is applied by id (`effect.apply {"effect": "org.example.posterize"}`) or display name, its
parameters are ordinary properties (keyframes, expressions, `prop.set`), and it is saved in
projects by id.

There are two ways to write one:

| | WebAssembly plug-in | Rust plug-in |
|---|---|---|
| What | a `.wasm` (or `.wat`) module | a type implementing `kition_motion_effects::plugin::EffectPlugin` |
| Loaded by | `effect.plugins.load {path \| folder}` (`folder` is refused where the host owns every file, as in the kit) | `register_plugin(Arc::new(MyEffect))` in an app that links Kition Motion's crates |
| Sandbox | yes: no imports (no files, clock or network), fuel-limited, at most 1 GiB of memory, deterministic floats | none (it is your code) |
| Platforms | native builds (the wasmi interpreter, crate feature `kition-motion-plugin/wasm`, which `kition-motion-host` turns on outside wasm32); not the editor's WebAssembly engine. The headless `kition-motion` does not offer `effect.plugins.load` to agents, because it names a file | everywhere |

`effect.plugins.list` lists what is loaded: `{api, wasm, plugins: [{id, name, category, version,
author, source, params}]}`.

### The contract

* **Pixels** are premultiplied RGBA `f32` (0–1; values may exceed 1 in 32 bpc projects), row
  major, processed **in place**. The buffer is the layer's (after masks and the effects above),
  possibly at preview resolution (`scale` = buffer pixels per layer pixel) and padded.
* **Parameters** arrive flattened to `f64`s in the order they are declared: slider, angle,
  checkbox (0 / 1) and popup (0-based index) take one value, point two (already in **buffer
  pixels**), colour four (RGBA 0–1).
* **Time** is layer time in seconds.
* Rendering must be **deterministic** and may run on several threads at once (a WebAssembly
  plug-in gets one instance per thread).
* A failed render (an error code, a trap, running out of fuel) leaves the frame unchanged.

### The manifest

Every plug-in describes itself with a JSON manifest:

```json
{
  "api": 1,
  "id": "org.kitionmotion.example.posterize-bands",
  "name": "Posterize Bands",
  "category": "Stylize",
  "version": "1.0.0",
  "author": "Kition Motion contributors",
  "description": "Quantizes luminance into tinted bands.",
  "params": [
    {"id": "levels", "name": "Levels", "type": "slider", "default": 4, "min": 2, "max": 64, "sliderMax": 16, "decimals": 0},
    {"id": "mix", "name": "Tint Amount", "type": "slider", "default": 50, "min": 0, "max": 100},
    {"id": "tint", "name": "Tint", "type": "color", "default": [1.0, 0.55, 0.2, 1.0]},
    {"id": "invert", "name": "Invert Bands", "type": "checkbox", "default": false}
  ]
}
```

* `id`: stable, reverse-domain; letters, digits, `.`, `_`, `-`. `ec.` is reserved for built-ins.
  Ids must be unique (a second plug-in with a taken id is refused).
* `category`: one of the Effects & Presets categories (`Blur & Sharpen`, `Stylize`, …) or a new
  one, which appears after the built-in categories.
* Parameter types: `slider` (`default`, `min`, `max`, optional `sliderMin`, `sliderMax`,
  `decimals`), `angle` (`default` degrees), `checkbox` (`default` bool), `popup` (`options`,
  `default` index), `point` (`default` as fractions of the layer size, `[0.5, 0.5]` = centre),
  `color` (`default` RGBA 0–1). Parameter ids are letters, digits and `_`, unique; at most 256
  parameters; a slider needs `min ≤ default ≤ max` (and `sliderMin ≤ sliderMax`); the manifest
  is at most 1 MiB.

### WebAssembly ABI (v1)

A module with **no imports** that exports:

| export | signature | |
|---|---|---|
| `memory` | memory | linear memory shared with the host |
| `ec_api_version` | `() -> i32` | must return `1` |
| `ec_manifest_ptr`, `ec_manifest_len` | `() -> i32` | the UTF-8 JSON manifest in memory |
| `ec_alloc` | `(bytes: i32) -> i32` | a buffer of at least `bytes` bytes, 8-byte aligned; it may reuse the previous one |
| `ec_render` | `(pixels: i32, width: i32, height: i32, params: i32, nparams: i32, time: f64, scale: f64) -> i32` | process the pixels in place; return 0 on success |

For each frame the host calls `ec_alloc` once for a buffer holding the parameters (`nparams`
little-endian `f64`s) followed by the pixels (`width × height × 4` little-endian `f32`s), fills it,
calls `ec_render`, and reads the pixels back. Each call gets a fuel budget proportional to the
frame size; a module that loops forever stops with an error instead of hanging the engine.

#### Example

[`examples/plugins/posterize-bands`](../../examples/plugins/posterize-bands) is a complete plug-in in
plain Rust (no dependencies):

```sh
cd examples/plugins/posterize-bands
cargo build --release --target wasm32-unknown-unknown
# then: the engine command effect.plugins.load {"path": "target/wasm32-unknown-unknown/release/posterize_bands.wasm"}
```

Any language that compiles to WebAssembly works (C, Zig, AssemblyScript…) as long as the module
has no imports. `crates/engine/plugin/src/tests.rs` has a plug-in written directly in WebAssembly text.

### Rust plug-ins

```rust
use std::sync::Arc;
use kition_motion_effects::plugin::*;

struct Swap(PluginManifest);

impl EffectPlugin for Swap {
    fn manifest(&self) -> &PluginManifest { &self.0 }
    fn render(&self, frame: &mut PluginFrame, params: &PluginParams, _time: f64) -> Result<(), String> {
        if params.b("on") {
            for p in frame.pixels.iter_mut() { p.swap(0, 2); }
        }
        Ok(())
    }
}

register_plugin(Arc::new(Swap(manifest)))?; // before the UI builds its menus
```

`PluginParams` reads values by parameter id (`f`, `b`, `point`, `color`). At most 64 plug-in
effects can be registered per process.

### Compatibility

The API version is checked when a plug-in loads: a manifest or module for another version is
refused with a message. Within a version the contract above does not change; additions (new
parameter types, new optional exports) bump the version.
