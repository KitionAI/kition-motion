# Settings, keyboard shortcuts, auto-save and crash recovery

The engine keeps settings, keyboard shortcut presets and auto-save state; this page says what each
does and what of it applies in the kit. The kit's editor (`web/`) has no Settings dialog and no
shortcut editor, and reads no settings: settings and presets are read and changed with the commands
below, which Kition's agent reaches through `bindings/headless`.

## Settings

The model is `kition_motion_engine::prefs::Prefs`: serde, versioned (`version`), addressed by dotted
keys (`general.undoLevels`, `autoSave.intervalMinutes`, `labels.3.name`) and grouped into After
Effects 2026's pages (General, Startup & Repair, Project, Composition, Previews, Appearance, Grids
& Guides, Labels, Type, Import, Export, Audio, Disk, Memory & CPU, Video, 3D, Scripting &
Expressions). Older page names still name the right page (`prefs.open {"page": "Auto-Save"}` is
Project, `Media & Disk Cache` is Disk, `Memory & Performance` is Memory & CPU).

The engine stores settings as `prefs.json` through the session's `ConfigStore` when the session has
one; loading migrates older layouts, keeps unknown keys (from newer versions) and falls back to
defaults for values that don't parse. Neither `bindings/wasm` nor `bindings/headless` gives its
session a store, so in the kit a setting changed with `prefs.set` lasts while the document is open
and is not saved.

Commands:

| Command | Parameters |
|---|---|
| `prefs.get` | `{key?}` (omit for everything) |
| `prefs.set` | `{key, value}` or `{values: {key: value}}` |
| `prefs.reset` | `{page?}` (omit for everything) |
| `prefs.open` | `{page?}` asks the frontend to show Settings; the kit's editor has no Settings dialog, so in the kit it changes nothing |
| `prefs.pages` | the page schema: every row, its key, type, range and whether it is wired |

### Settings that change behaviour

- `general.undoLevels`: Levels of Undo
- `general.recentItems`: Recent Projects Shown
- `general.createLayersAtCompStart`: Create Layers at Composition Start Time
- `general.defaultSpatialLinear`: Default Spatial Interpolation to Linear
- `project.useTemplate`: New Project Loads Template
- `project.templatePath`: Template Project
- `previews.fastPreviews`: Fast Previews (Draft 3D, Faster Effects)
- `import.stillFootage`: Still Footage
- `import.stillSeconds`: Still Duration
- `import.sequenceFps`: Sequence Footage
- `memory.layerCacheMb`: Layer Cache (in the browser at most 512 MB)
- `memory.mediaCacheMb`: Footage Frame Cache (in the browser at most 256 MB)
- `threeD.defaultRenderer`: Default 3D Renderer
- `labels.N.name` / `labels.N.color`: the 16 label names and colours, used by the Label menu
- `general.switchesAffectNestedComps`: a precomp layer's Quality (Draft / Wireframe) and Motion
  Blur switches limit the layers of the nested comp (viewer, Render Queue, `render`)
- `general.preserveConstantVertexCount`: adding or deleting mask / shape path vertices at one
  keyframe does the same on every other keyframe (added vertices split the matching segment)
- `general.syncTimeRelatedItems`: moving the current time moves it in the comps nested in this
  one and in the comps that nest it (through the precomp layers' timing)
- `general.expressionPickWhipCompact`: off, the pick whip writes match names
  (`thisComp.layer("A")("transform")("opacity")`)
- `general.createSplitLayersAbove`: where Edit ▸ Split Layer puts the new layer
- `previews.displayProfile`: the monitor's colour space for View ▸ Use Display Color Management
- `appearance.cycleMaskColors`: new masks cycle through the mask colours (off: all the first)
- `import.reportMissingFrames`: gaps in an image sequence's numbering are reported on import
  (they show as colour-bar placeholders, as in After Effects; see [footage.md](footage.md))
- `import.autoReloadFootage`: Automatically Reload Footage, `off`, `nonSequence` (the default, as
  in After Effects) or `all` (image sequences too): the engine compares each footage file's size
  and modification time with what they were and reloads what another app saved. It is not an
  undo step and doesn't modify the project; a layer item stays on its layer (found again by name),
  and a file that can't be read yet (still being written) is tried again once it changes again.
  The engine checks only when asked: `footage.reloadChanged` runs the check once, and the kit's
  editor does not check on its own
- `import.unlabeledAlpha`: alpha of TGA / TIFF / movie footage (Guess takes premultiplied for
  movies, straight for stills; Ask leaves it to the frontend's Interpret Footage dialog, which the
  kit's editor does not have)
- `import.dragImportAs`: layered files imported as dropped files (`file.import {"drag": true}`)
  import as footage or a comp
- `export.defaultOutputFolder`: where relative Render Queue outputs go. The kit sets it to `/`, a
  virtual root: renders land in memory and the host saves them
- `export.segmentSequences`, `export.segmentSequenceFiles`: sequences split into numbered
  folders of N files; `export.segmentMovies`, `export.segmentMovieMb`: movies split into
  numbered files of about N MB (cut at the frame count the output's data rate gives)
- `export.appendBitsToName`: `Comp 1.png` → `Comp 1_16bpc.png`
- `disk.diskCacheEnabled`, `disk.diskCacheMaxGb`, `disk.diskCacheFolder`: the persistent disk
  cache of processed layers and frames. The kit never opens one: the host owns every file
  (`Session::host_files_only`)
- `disk.mediaCacheFolder`: audio waveform summaries kept between sessions (`Peaks/`); not used in
  the kit, for the same reason
- `disk.conformedMediaFolder`: decoded (conformed) footage audio, written once per file and
  sample rate and read back instead of decoding again; not used in the kit, for the same reason
- `memory.ramReservedGb`: the cache budgets together leave this much physical memory free
- `memory.reduceCacheWhenLow`: cache budgets halve while the system is low on memory. This and
  `memory.ramReservedGb` need a reading of the system's memory, which the engine takes only when
  the frontend calls `Session::memory_tick`; the kit does not, so neither applies there
- `threeD.extendedViewer`: 3D layers reaching beyond the comp frame are outlined on the
  pasteboard
- `threeD.realtimeShadows`: draft (Fast Previews) renders keep 3D shadows
- `scripting.allowScriptsWriteFiles`: the scripting file / network gate (and `File.execute()`)
- `scripting.warnExecutingFiles`: `File.execute()` asks before opening a file
- `roto.model` (the Rotoscope Brush page): the segmentation model Rotoscope Brush 2.0 / 3.0 use,
  `classical` (built in) or `mobilesam` (once installed). `roto.models` lists every registered
  model with its authors, license, size, source and status; `roto.model.*` downloads, installs and
  removes weights in a `models` folder next to the settings, each with a `.NOTICE.txt` naming its
  authors and license. The kit's sessions have no such folder, so there downloading and installing
  fail and Rotoscope Brush uses the classical engine
- `face.model` (the Face Tracking page): the model face tracking uses, `classical` (built in) or
  `mediapipe-face` (once installed); the same commands (`face.models` / `face.model.*`) and the
  same limit in the kit

### Settings the kit stores but does not use

These are stored, returned by `prefs.get`, and reported as wired by `prefs.pages`, but nothing in
the kit acts on them: they describe an interface the kit does not draw, and the kit's editor reads
no settings.

- General: `general.pathPointSize`, `general.showToolTips`, `general.language` (nothing in the
  engine reads it; the kit's editor has its own English and Chinese strings in
  `web/src/strings.ts`, chosen by the host's locale)
- Startup & Repair: `startup.showHomeOnLaunch`, `startup.showHomeOnOpenProject`,
  `startup.windowGraphics`
- Composition: `composition.showRenderingProgress`, `composition.motionPath`,
  `composition.motionPathSeconds`, `composition.motionPathKeyframes`
- Previews: `previews.adaptiveResolutionLimit`, `previews.cacheFramesWhenIdle`,
  `previews.showInternalWireframes`, `previews.zoomQuality`; Memory & CPU:
  `memory.previewCacheMb`
- Appearance: `appearance.appearanceMode`, `appearance.darkTheme`, `appearance.lightTheme`,
  `appearance.theme`, `appearance.brightness`, `appearance.uiScale`,
  `appearance.useLabelColorForHandles`, `appearance.useLabelColorForTabs`,
  `appearance.useGradients`, `appearance.inWindowMenuBarMac`
- Grids & Guides: `grids.gridColor`, `grids.gridSpacing`, `grids.gridSubdivisions`,
  `grids.guideColor`, `grids.actionSafe`, `grids.titleSafe`, `grids.gridStyle`,
  `grids.guideStyle`, `grids.proportionalHorizontal`, `grids.proportionalVertical`
- Type: `type.textEngine`, `type.fontPreview`, `type.recentFonts`, `type.fontNamesInEnglish`
- Audio: `audio.outputDevice`, `audio.outputLeft`, `audio.outputRight`,
  `audio.previewSampleRate`
- Video: `video.enableOutput`, `video.device`, `video.outputDuringPlayback`,
  `video.mirrorOnMonitor`, `video.disableWhenBackground`
- 3D: `threeD.showReferenceAxes`
- Scripting & Expressions: `scripting.editorFontSize`, `scripting.syntaxHighlighting`,
  `scripting.lineNumbers`, `scripting.autoComplete`, `scripting.bracketMatching`,
  `scripting.wordWrap`, `scripting.errorBanner`

### Display-only settings

These rows are in the schema for After Effects parity and are stored, but have no Kition Motion
behaviour to change. `prefs.pages` reports them with `"live": false`; a test keeps this list in
step with the schema.

- `general.useSystemColorPicker`: the kit's editor has no colour picker.
- `composition.hardwareAcceleratePanels`: rendering always runs on the CPU path; there is no GPU
  compositor ([architecture.md](../architecture.md) §4).
- `scripting.enableJsDebugger`: the JavaScript engine has no step debugger; script errors report
  their line and column (`script.run`'s `error`).

## Keyboard shortcuts

The engine keeps keyboard shortcut presets (`kition_motion_engine::shortcuts`): every command and
menu entry with bound parameters has a binding key, a scope and default keys.

- **Presets.** "Kition Motion Default" is After Effects' default layout and is read-only; editing
  it creates a "Custom" copy. Custom presets store only their differences from the default and
  can be duplicated, renamed, deleted, exported and imported (JSON).
- **Scope.** Menu and engine commands are application-wide; timeline commands work in the
  Timeline and viewer commands in the Composition panel. `shortcut.conflicts` lists keys that
  collide in overlapping scopes.

In the kit, the presets live in the session like the settings (there is no store to save
`shortcuts.json` in), and the command list the kit hands the host carries each command's default
shortcut, not the active preset's.

Commands: `shortcut.list {query?, keys?, assigned?}`, `shortcut.set {command, params?, keys}`,
`shortcut.reset {command?}`, `shortcut.preset {op: select|new|duplicate|delete|rename, name?,
from?, newName?}`, `shortcut.export {preset?, path?}`, `shortcut.import {path? | preset?}`,
`shortcut.conflicts`.

## Auto-save and crash recovery

The engine's auto-save (`kition_motion_engine::autosave`) writes a project with unsaved changes
every *n* minutes (`autoSave.intervalMinutes`) to a `Kition Motion Auto-Save` folder next to it (or
`autoSave.folder`) as `<name> auto-save N.kmotion`, rotating through slots 1…`autoSave.maxVersions`
and overwriting the oldest; writes are atomic (a temporary file, flushed, then renamed over the
target). Its crash recovery keeps a `session.lock` sentinel in the settings store, naming the open
project and its latest auto-save; a frontend that finds the sentinel still there when it starts
can offer that auto-save (`Session::begin_recovery`, `startup.offerCrashRecovery`).

None of this runs in the kit: the host owns and saves the document. `file.autoSave` there fails
("auto-save is unavailable here: the host saves the document"), `file.recoveryInfo` reports no
auto-save folder, the engine keeps no sentinel (its sessions have no settings store), and nothing
calls the auto-save timer. The `autoSave.*` settings, `autoSave.saveOnRenderStart` included, and
`startup.offerCrashRecovery` have no effect there.

Commands: `file.autoSave` (now), `file.recoveryInfo`, `file.openRecent {index? | path?}`,
`file.clearRecent`, `file.revert`, `file.incrementAndSave`.
