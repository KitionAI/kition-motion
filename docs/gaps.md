# Gaps

> **Last updated:** 2026-11-17 · **Source:** `features.yaml` (updated 2026-11-17) and the engine's command list

Every known shortfall of Kition Motion, from the feature catalogue: features that are not yet `has`,
then the differences recorded on features that work. Update the feature in `features.yaml` when you
land work, then this list. The comparison with the target application is in
[parity/target-app-parity.md](parity/target-app-parity.md); the order of work is in [roadmap.md](roadmap.md).

## Features not done

- `motion.agent.places` (P0, partial): A write by stack position ("#3") or by time has as premise what that place named, and a layer's startTime, inPoint and outPoint are its place, not its content.
- `motion.agent.premises` (P0, partial): A write with basis is refused only for others' changes to what its command reads of the layer, by id, by name or by place.
- `motion.model.objects` (P0, partial): The object model is reached from its root: query follows fields, batch edits them, replicas converge. Raw assignments, key-list edits, item creation/deletion, owned property creation and layer reordering have guarded contracts. Remaining structural classes and private outside-created receipt fields remain under review. Raw object ids are not stable across a reopen: the kit builds its replica from the plain-JSON file on every open, so an id held from one opening is refused not-found in the next, and an agent re-observes it (tool ids survive); ids stable across reopen are after the launch.
- `motion.camera.lights` (P1, partial): Cameras, lights and 3D materials. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.comp.settings` (P1, partial): Compositions: create, settings, trim, poster time and VR. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.edit.history` (P1, partial): Undo, redo, cut, copy, paste, duplicate and other Edit menu commands. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.effect.apply` (P1, partial): Apply, remove and edit effects. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.file.projects` (P1, partial): Projects: new, open, save, import, export and recovery. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.graphics.templates` (P1, partial): Graphics templates and template files. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.keys.properties` (P1, partial): Properties, keyframes, easing, animation presets and expressions. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.layer.create` (P1, partial): Create, arrange, switch, precompose and transform layers. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.layer.masks` (P1, partial): Layer masks and mask vertices. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.layer.styles` (P1, partial): Layer styles. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.markers.manage` (P1, partial): Markers. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.paint.distort` (P1, partial): Paint, puppet pins, liquify and content-aware fill. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.project.items` (P1, partial): Project panel items, folders, footage and metadata. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.render.queue` (P1, partial): Render queue, output modules and previews. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.shape.paths` (P1, partial): Shape layers and paths. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.text.animate` (P1, partial): Text editing and text animators. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.time.navigate` (P1, partial): Time navigation and time remapping. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.track.motion` (P1, partial): Motion tracking, stabilization and Warp Stabilizer. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.view.3d` (P1, partial): Views and 3D views. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.ai.models` (P2, partial): Rotoscope Brush and face tracking models. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.app.scripting` (P2, partial): Scripts, script windows and the batch engine. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `motion.app.workspace` (P2, partial): Workspaces, panels, settings, shortcuts, background jobs, storage and help. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.

## Catalogue

The catalogue has 43 features: 18 `has`, 25 not yet.
