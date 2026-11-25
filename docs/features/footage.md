# Footage: importing files and image sequences

How to bring footage into a project and tell Kition Motion how to read it. Everything here works
the way it does in After Effects, and every step is an engine command (shown in `code`): the kit's
editor runs it from its menus, and Kition's agent through `bindings/headless`. Files come from the
host, which lends them to the engine.

## Importing files

- **File ▸ Import ▸ File…** (`file.import`) imports one or more files. Photoshop and PDF /
  Illustrator files import as footage or as a composition (`importAs`).

Agents: `file.import {"paths": ["/shots/plate.mov", "/shots/logo.psd"]}`.

## Image sequences

VFX plates, 3D renders and hand-drawn animation usually come as numbered stills:
`shot_0001.png`, `shot_0002.png`, … Kition Motion imports such a run as **one footage item** that
plays like a movie. Its name shows the range, for example `shot_[0001-0250].png`, and its type is
*Image Sequence*.

`file.import` takes these options for numbered files:

| Option | What it does |
|---|---|
| **Sequence** (`sequence`) | `true`: numbered stills of one run import as one sequence; one file brings its whole run, several files of the run bring that range. `false`: the files import as stills, and a folder's files one by one. |
| **Force alphabetical order** (`alphabetical`) | Imports every image of that type in the folder in name order, whatever the numbering, and plays the files one after another (gaps in the numbering don't count). Use it for files that aren't numbered consistently. |
| **Frame rate** (`frameRate`, fps) | The sequence's frame rate. The default comes from the `import.sequenceFps` setting (30 fps). |

Without `sequence`, a numbered file brings its whole run and a folder's numbered runs import as
sequences, except dropped files (`"drag": true`), which import as stills, one item each, as in
After Effects.

Stills of any format Kition Motion reads can form a sequence: PNG, JPEG, TIFF, OpenEXR, BMP and
WebP. Layered and vector documents (PSD, SVG, PDF) import one by one.

**Missing frames.** Where the numbering skips a number (`shot_0003.png` is missing between
`0002` and `0004`), the sequence shows a placeholder of colour bars for that frame, as After
Effects does, so every other frame stays at its number. Render or copy the missing files into the
folder and choose File ▸ Reload Footage (`file.reloadFootage`) to fill the gaps.

Agents: `file.import {"paths": ["/shots/shot_0001.png"], "sequence": true, "frameRate": 24}`.

## Interpret Footage

**File ▸ Interpret Footage ▸ Main…** (`file.interpretFootage`, Ctrl+Alt+G / Cmd+Opt+G) changes
how a footage item is read. A change applies everywhere the footage is
used, and layers that ran to the end of the footage keep doing so.

| Setting | What it does |
|---|---|
| **Alpha** | *Straight - Unmatted*, *Premultiplied - Matted With Color* (with the matte colour), *Ignore* (opaque), or *Guess* (Kition Motion looks at the first frame). *Invert Alpha* flips it. |
| **Assume this frame rate** | The rate the frames play at. A 250-frame sequence at 25 fps lasts 10 seconds; at 24 fps, 10.4 seconds. |
| **Start Timecode** | *Use Source File Timecode* (0:00:00:00), or *Override Start*: the timecode the footage's first frame shows. It relabels source time only; the frames stay the same. For a 1001–1250 render at 24 fps, Override Start 0:00:41:17 (frame 1001) makes source time count from the render's own frame numbers. |
| Fields, pixel aspect, loop | As in After Effects. |
| **Assign Profile**, **Interpret As Linear Light** | The colour profile the file's values are in (the working space converts from it), and whether they are linear light. |
| **Preserve RGB** | The file's values enter the composition unconverted: no colour profile, linear light or working-space conversion, and float files (OpenEXR, float TIFF) keep their linear values instead of being sRGB-encoded. Use it to convert chosen layers yourself with an OCIO effect, or to read data passes (depth, mist, normals) as numbers. |

The `import.reportMissingFrames` setting (Report Missing Frames, on by default) lists the missing
frame numbers when a sequence imports.

**File ▸ Interpret Footage ▸ Remember Interpretation / Apply Interpretation** copy the alpha,
fields, pixel aspect, loop and colour settings (Preserve RGB too) to other footage.

Agents: `file.interpretFootage {"items": [id], "frameRate": 24, "alpha": "premultiplied",
"startTimecode": 1001, "preserveRgb": true}` (timecode or a frame number; `"overrideStart":
false` goes back to Use Source File Timecode).

## Multi-layer OpenEXR

Float OpenEXR renders hold linear light; Kition Motion sRGB-encodes them as they are read, like
After Effects with Interpret As Linear Light for 32 bpc, unless the item preserves RGB (above).
A multi-layer file shows its main RGBA channels, as in After Effects. Renders that keep every
pass in named layers (Blender's `ViewLayer.Combined`, its compositor's File Output `Image`,
Nuke's `beauty`) show the beauty pass: the layer named Combined, Beauty, Image, RGBA, RGB or
Color, else the first colour layer that isn't a data pass. Cryptomatte, depth, mist, normals,
vectors, positions and IDs are never the picture. EXtractoR reads every other layer and channel.

## Replacing and reloading

- **File ▸ Replace Footage ▸ File…** (Ctrl+H) points the item at another file. Pick a numbered
  file to replace it with that file's whole sequence (`file.replaceFootage {"path": …,
  "sequence": false}` for one frame).
- **File ▸ Reload Footage** (Ctrl+Alt+L) reads the files again. A sequence picks up frames
  added to its folder (a render still in progress) and keeps its interpretation.

**File ▸ Dependencies ▸ Relink Missing Footage…** (`file.relinkFootage`) searches a chosen folder
and its subfolders for missing source files. It reads folders by path, so the kit refuses it: there
the host owns every file. It relinks unique exact filenames whose decoded type,
dimensions, codec and audio/video flags agree with the saved footage (movies and audio also
require matching native frame rate and frame count). The result includes each candidate's high or low confidence and reason. Confidence is a metadata
heuristic, not proof of identical contents. Duplicate filenames remain unresolved even if only
one decodes successfully; use Replace Footage to choose them manually.

Relinking preserves item names, interpretation, layer timing and the saved sequence range, and
the entire operation is one undo step. Sequences require every saved frame in the new folder;
extra frames do not extend the range. Existing files are untouched. Layer-specific/page-specific
footage, models and embedded data are left unresolved. Symlinks are skipped, unreadable folders
fail the search without edits, and searches are limited to 100,000 entries.

Agents: `file.relinkFootage {"folder": "/moved/shots", "dryRun": true, "wait": true}` returns
a report without edits. Omit `dryRun` to apply unique high-confidence matches. Without `wait`,
the command returns a cancellable job; `job.wait` / `job.list` expose its report.
