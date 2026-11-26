# Target-app parity

> **Last updated:** 2026-11-17 · **Source:** `features.yaml` (updated 2026-11-17) and the engine's command list

Each feature in `features.yaml` records its status against what the feature promises and, where it
has one, the target application it is compared with (After Effects 2026 (26.5)).
The difference column is the feature's recorded gap.

| Feature | Priority | Status | Target | Difference |
| --- | --- | --- | --- | --- |
| `motion.agent.act` An agent animates properties with keyframes | P0 | has | — | — |
| `motion.agent.context` An agent's call names the comp, time and layers it acts on, and leaves the editor's open comp, current time and selection as they were | P0 | has | — | — |
| `motion.agent.ids` One id space: tools, receipts and others name items and layers by the ids inspect lists and commands take, and the premise guard checks a write against them | P0 | has | — | — |
| `motion.agent.observe` An agent reads a composition's layers and acts on what it finds | P0 | has | — | — |
| `motion.agent.places` A write by stack position ("#3") or by time has as premise what that place named, and a layer's startTime, inPoint and outPoint are its place, not its content | P0 | partial | — | — |
| `motion.agent.premises` A write with basis is refused only for others' changes to what its command reads of the layer, by id, by name or by place | P0 | partial | — | — |
| `motion.agent.recover` An agent fixes what the check reports | P0 | has | — | — |
| `motion.comp.basics` Set a composition's size, frame rate, duration and background, and mark it; read it back | P0 | has | After Effects 2026 (26.5) | — |
| `motion.effect.basics` Apply effects, set their parameters and reorder the stack | P0 | has | After Effects 2026 (26.5) | — |
| `motion.keys.basics` Keyframe and set properties, Easy Ease the keys, add expressions; read the values back | P0 | has | After Effects 2026 (26.5) | — |
| `motion.layer.basics` Make solid, text, shape and null layers, rename, blend, parent and trim them; text edits keep CJK and emoji | P0 | has | After Effects 2026 (26.5) | — |
| `motion.mask.basics` Draw a mask from vertices, set its mode, close it and feather it | P0 | has | After Effects 2026 (26.5) | — |
| `motion.model.objects` The object model is reached from its root: query follows fields, batch edits them, a draft's edits merge | P0 | partial | — | Raw assignments, key-list edits, item creation/deletion, owned property creation and layer reordering have guarded contracts. Remaining structural classes and private outside-created receipt fields remain under review. |
| `motion.agent.design` An agent is told what a designer would change, with the commands that change it | P1 | has | — | — |
| `motion.agent.draft` An agent works in a draft over a conversation | P1 | has | — | — |
| `motion.agent.preview` An agent previews a composition at a time, or a contact sheet over a range, whatever the editor shows | P1 | has | — | — |
| `motion.camera.lights` Cameras, lights and 3D materials | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.comp.settings` Compositions: create, settings, trim, poster time and VR | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.edit.history` Undo, redo, cut, copy, paste, duplicate and other Edit menu commands | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.editor.display-modes` The editor in each display mode (Menus, Single line, Classic, Focus); Focus shows the composition alone | P1 | has | — | — |
| `motion.effect.apply` Apply, remove and edit effects | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.effects.list` List the effects and their parameters with defaults in the units prop.set takes | P1 | has | — | — |
| `motion.file.projects` Projects: new, open, save, import, export and recovery | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.graphics.templates` Graphics templates and template files | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.keys.properties` Properties, keyframes, easing, animation presets and expressions | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.keys.select` Select keyframes by layer, property and time | P1 | has | — | — |
| `motion.layer.create` Create, arrange, switch, precompose and transform layers | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.layer.masks` Layer masks and mask vertices | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.layer.styles` Layer styles | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.markers.layer` Add markers to layers | P1 | has | — | — |
| `motion.markers.manage` Markers | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.paint.distort` Paint, puppet pins, liquify and content-aware fill | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.platform.document-text` document.text: every composition's text layers as plain text for a host's search, cut at a limit | P1 | has | — | — |
| `motion.project.items` Project panel items, folders, footage and metadata | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.render.queue` Render queue, output modules and previews | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.shape.paths` Shape layers and paths | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.text.animate` Text editing and text animators | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.time.navigate` Time navigation and time remapping | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.track.motion` Motion tracking, stabilization and Warp Stabilizer | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.view.3d` Views and 3D views | P1 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.ai.models` Rotoscope Brush and face tracking models | P2 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.app.scripting` Scripts, script windows and the batch engine | P2 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |
| `motion.app.workspace` Workspaces, panels, settings, shortcuts, background jobs, storage and help | P2 | partial | After Effects 2026 (26.5) | The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen. |

| Priority | has | partial | missing |
| --- | ---: | ---: | ---: |
| P0 | 10 | 3 | 0 |
| P1 | 8 | 19 | 0 |
| P2 | 0 | 3 | 0 |
