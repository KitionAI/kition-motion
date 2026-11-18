# After Effects 2026 replacement acceptance

Target: reproduce observable After Effects 2026 behavior and output closely enough to
replace it in professional motion-design work on Windows, macOS, and Linux. Feature
presence and a polished interface are separate evidence; neither establishes this target on
its own. Pin each reference case to the exact AE build, platform, fonts, color settings, source
files, and project parameters.

## Acceptance gates

| Gate | Evidence required | Current boundary |
| --- | --- | --- |
| Work projects | Representative projects recreated or imported, edited, saved, reopened and delivered without lost state | Native AE project import and third-party AE plug-in execution remain gaps |
| Workflow | Pointer and keyboard cases for all panels, tools, shortcuts, undo, dialogs and selections | The kit's editor has its browser flow tests (`web/tests`) only |
| Animation | AE-sampled values for scalar, spatial, multidimensional and expression-driven properties, timing and interpolation | Eight scalar Rotation/ease cases, 168 samples verified against AE 26.3x87; auto-Bezier spatial tangents (ten keys, three paths) match AE 26.3; other cases remain open |
| Render fidelity | Original test scenes compared at 8/16/32 bpc, specified color spaces and alpha conventions; edges, impulses, gradients and temporal sequences | Most AE-rendered comparisons remain open |
| Media and delivery | Real footage import, seek, frame rates, audio sync, relinking, image sequences, codec/container metadata and exports checked in downstream tools | Actual work fixtures still required |
| Performance | Matched AE/Kition Motion input-to-present and export timings at 1080p/4K, warm/cold caches, memory and long sessions | No claim of beating AE or measured blur response yet |
| Recovery | Save/reopen, interrupted render, invalid input, exhausted resources and cache failures without silent corruption | Not measured for the kit |

## Graphics and technology policy

Keep one shared Rust scene/effect model. Rendering runs on the CPU path
([architecture.md](../../architecture.md) §4). Use hardware capabilities and measured
results to select optimized paths, with a CPU path that preserves output semantics. Newer APIs, SIMD, parallelism, and hardware codecs
must preserve color, precision, timing and file compatibility. Avoid making baseline
features depend on a specific GPU vendor or an optional advanced device capability.

Use current stable dependencies after regression checks; experimental technology belongs
behind an explicit optional path.

## Reference process

Use the official [AE 2026 user guide](https://helpx.adobe.com/after-effects/desktop.html),
public scripting documentation, legally available publisher samples, and original scratch
projects observed through AESync. Do not copy Adobe implementation code or redistribute
Adobe lesson artwork. Record documented expectations separately from live observations.
Maintain per-case pass/fail/untested results, tolerances and repeatable commands; do not
replace them with a global parity percentage.

Next priorities: measure Gaussian Blur interaction and output against AE; expand
animation/render oracles; exercise representative work projects and required plug-ins.
