# Effects parity

> **Last reviewed:** 2026-11-17 · **Last updated:** 2026-11-17 · **Target:** Adobe After Effects 2026

Effects are the deepest single area of After Effects. This page separates the questions
that a single "306 of 306" hides: does each effect exist, does it look like After Effects', and
does the preset library exist. The per-effect catalogue is the generated
[effects.md](../features/effects.md); the overall numbers are in [target-app-parity.md](target-app-parity.md).

## Summary

| Question | Value | Kind |
|---|---|---|
| Effects present | **306** registered, 3,014 parameters; all of After Effects' effects by name (298 at the M9.11 count) plus Obsolete and our own | Measured: registry, [effects.md](../features/effects.md). After Effects 26.5's `Plug-ins/Effects` holds 306 `.plugin` bundles in 231 entries (one bundle can hold several effects, so this is a cross-check, not a match) |
| Parameter names, order, popups, units, defaults | aligned in M9.12 | Agent-checked against After Effects' documentation, not against a live dump |
| 32-bit float, no clamping | 306 | Measured: registry flags |
| **Rendered output matches After Effects** | **unmeasured** | No effect is compared with an After Effects render yet. `kition_motion_effects::fidelity::compare_frames` exists for the corpus ([fidelity/README.md](fidelity/README.md)); simulation effects are pinned by golden hashes of our own output (M13.27) |
| Animation presets | **8** original text-animator presets against After Effects' **621** `.ffx` presets | Measured: `crates/engine/engine/presets/text_animators.json`; the After Effects `Presets` folder |
| 26.x additions | Curl Noise (26.3) present (#463 open); effect colour labels and the rebuilt Effect Controls (26.5) missing | Release notes |

**Breadth ≈ 85%** (presets weigh in; the effects themselves are ≈ 100%). **Ready ≈ 40%**: until a
corpus compares renders, each effect is assumed to differ somewhere, and users keep confirming it.

## By category (from [effects.md](../features/effects.md))

| Category | Effects | Known differences or bugs |
|---|---:|---|
| 3D Channel | 8 | Cryptomatte selection (#473); multi-layer EXRs decoded twice (#474) |
| Audio | 13 | Applied in the mixdown |
| Blur & Sharpen | 16 | Gaussian Blur interaction and output to be measured first ([fidelity/ae-2026-acceptance.md](fidelity/ae-2026-acceptance.md)) |
| Channel | 13 | — |
| Color Correction | 40 | Apply Color LUT (#514); 32-bpc blending differences affect looks (#494); ACES 2.0 / OCIO 2.5 (26.5) |
| Distort | 39 | Mesh Warp (#303); CC Power Pin Unstretch (#534, community fix pending); Liquify has no viewer brush |
| Expression Controls | 9 | — |
| Generate | 26 | Fractal's Reset doesn't work (#526); Write-on drops marks when animated (#536, fix pending) |
| Immersive Video | 12 | — |
| Keying | 12 | Key Cleaner approximated; wire-removal Frame Offset (#533, fix pending) |
| Matte | 6 | Mocha shape reads our documented JSON, not Mocha's format |
| Noise & Grain | 13 | Curl Noise (#463) |
| Obsolete | 9 | — |
| Paint | 1 | Tablet pressure not read |
| Perspective | 10 | Chromatic Aberrations pan/tilt crash (#580) |
| Simulation | 18 | Particle Playground lacks Particle Exploder, text particles, ephemeral mapper; CC Particle World camera fixed (#397) |
| Stylize | 25 | Motion Tile output size (#369) |
| Text | 2 | — |
| Time | 8 | Pixel Motion Blur averages frames, ignores Vector Detail (#535, fix pending) |
| Transition | 17 | — |
| Utility | 9 | — |

Third-party bundles in After Effects (Cineware, Mocha AE) are out of scope; Keylight is
implemented under a generic name ("Key Light").

## Estimate

**≈ 70–130 h**: a fidelity corpus case per effect at default and one non-default setting, compared
with After Effects renders, and the fixes they turn up (40–80 h, most of it parallel, but every
reference render needs After Effects on a maintainer's Mac); an original preset library covering
After Effects' preset categories (Behaviors, Backgrounds, Shapes, Synthetics, Text, Transitions,
Image effects) 20–40 h; the open effect bugs 10–20 h.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-17 | major | First public version |
