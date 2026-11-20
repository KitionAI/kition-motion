# Changelog

Changes to Kition Motion, newest first. A version is the kit package version in `kit.json`;
each package is published with a `package-v<version>` tag.

## 0.1.2

- `kit.json` declares the engine's editor state as `engine.editorState`, the same list as the
  engine's `EDITOR_STATE`, so a host that does not run the engine reads `inspect()` without it.
- The editor announces itself to the host with the kit's version (it still said 0.1.0), and
  `web/package.json` carries the same version.
- The README connects an agent with `npm install -g @kition/cli` and `kition agent add`.

## 0.1.1

- The media, export and interchange crates are built on Kition Film's public `main`: its
  pure-Rust codecs, containers and project model are pinned to a revision on that branch.
  In the Kition Film project model, a keyframe whose value cannot interpolate (text, a choice,
  a checkbox) now defaults to Hold instead of Linear; Motion's import and export keep each
  keyframe's own interpolation.
- The KHP link points to `github.com/kitharness/kitharness`.

## 0.1.0

First public release of the source, under the Apache License 2.0. It contains:

- The Rust engine: compositions of text, shape, solid, null, footage, camera and light layers;
  keyframes with eases and expressions; 306 effects; masks; 3D layers; text animators; footage
  and audio; and rendering to MP4 (a pure-Rust encoder, no ffmpeg).
- The kit engine (`kition-motion-kit`), native and WebAssembly: the project is a KDM replica,
  replicas merge concurrent edits, and undo is per person and per agent. A check catalogue
  reports what a composition cannot carry, with design advice and the commands that fix it.
- The headless kit (`kition-motion`) for agents over the Kit Protocol: it reads a composition's
  layers and keyframes, animates them with the editor's own commands, previews a frame or a
  contact sheet, and works in drafts. A write names the revision it read and is refused when
  someone else changed what it rests on. It reads nothing outside its workspace.
- The kit's editor (`web/`): the composition viewer, the transport, the layer timeline and the
  Transform properties, in kit-editor's frame.
- NOTICE and docs/attribution.md list the upstream work, the fonts and every asset, and Loro
  (MIT), bundled by the editor and linked by the engine.
