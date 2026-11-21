# Development

## Repository layout

| Path | Contents |
|---|---|
| `crates/` | The engine (Rust), in groups: `engine/` (the project model, keyframes, paths, effects, expressions, scripting, plug-ins, 3D models, tracking, segmentation, the session builder `host` and the command layer `engine`), `formats/` (export, interchange, Lottie, PDF, PSD, SVG), `media/` (time base, media sources and the encoders), `render/` (raster and the compositor), `support/` (`color`, `geom`, `text`) and `kdm/` (`kdm-model`). The bundled script panel is in `crates/engine/engine/extensions/`, next to the sample scripts in `crates/engine/engine/scripts/` |
| `bindings/` | `wasm`: `kition-motion-kit`, the engine behind Kition's `KitDocument` contract, compiled to WebAssembly for the editor; `headless`: `kition-motion`, the headless kit executable for Kition's agent |
| `web/` | The kit's editor (React, on kition-sdk's `@kition/sdk/editor`); `web/dev.html` is a host for development |
| `kit.json` | The kit manifest (KHP) |
| `agent/` | The agent guide (`guide.md`) and checks |
| `conformance/` | The agent conformance fixtures (`.kmotion` projects) that cases and tests refer to |
| `features.yaml`, `cases/` | The feature catalogue and its cases ([quality.md](quality.md)) |
| `docs/` | These documents: the entry documents, `features/` (one document per feature) and `parity/` (the comparison with After Effects, and the live measurements in `parity/fidelity/`) |
| `assets/` | The bundled Latin fonts (`assets/fonts/`) and the README's pictures (`assets/readme/`), each with its `.attribution` sidecar |
| `examples/` | `plugins/posterize-bands`: a complete WebAssembly effect plug-in ([plugins.md](features/plugins.md)) |
| `tools/` | `tools/xtask` (`cargo xtask layers`, `assets`, `wasm`, `version`, `ci` and `ai`) and `tools/fidelity` (the After Effects comparison harness, [fidelity](parity/fidelity/README.md)) |

## Build and test

```sh
cargo test --workspace                                    # everything
cargo build --release -p kition-motion                    # the headless kit executable
cd web && pnpm install && pnpm engine && pnpm test:flow   # the editor
cargo xtask ci                                            # fmt, clippy -D warnings, tests, layers, assets, wasm
```

`pnpm engine` builds the engine into `web/public/engine/` and copies a fallback font for Chinese
text from a [kition-fonts](https://github.com/KitionAI/kition-fonts) checkout next to this
repository (or the one `KITION_FONTS_DIR` names); without it, Chinese text shows as boxes.
`features.yaml` and its cases under `cases/` are the feature catalogue ([quality.md](quality.md)),
and [writing-format.md](writing-format.md) lists every command.

### What the tests cover

- **Engine commands** (`crates/engine/engine/src/tests*.rs`): every command family, including undo and
  redo, and that every menu entry resolves to a registered command.
- **Rendering** (`crates/render/render`): pixel checks on small compositions, for blend modes, masks,
  mattes, time remapping, 3D projection, lighting and shadows. Fast paths (the layer cache, the
  affine compositor, blurs, motion blur accumulation) are checked against their reference
  implementations.
- **Effects** (`crates/engine/effects`): each effect has a determinism test and at least one behaviour
  test (identity at neutral settings, known pixel results, or simulations giving the same frame
  whether you seek straight to it or play up to it). A registry-wide test checks that effects
  which read the clock are declared time-dependent. `crates/engine/effects/tests/sim_golden.rs` pins
  exact pixel hashes of the simulation effects (CC Rainfall … Card Wipe) at several settings,
  times, bit depths and resolutions, so any change to their output fails (re-pin with `SIM_GOLDEN_PRINT=1` after an intended change; pinned for aarch64 macOS,
  with the hashes another macOS release's libm gives for unchanged code in `LIBM_VARIANTS`).
- **Export** (`crates/formats/export/tests`): every format is encoded and decoded back, checking frame
  count, size and pixels. When `ffmpeg`/`ffprobe` are installed they are used as an outside
  check; they are never linked or shipped.
- **SDK** (`bindings/wasm/tests`): `kition_kit::testing::check_contract` and the
  `KitDocument` round trip against a `.kmotion` fixture.
- **Headless kit** (`bindings/headless`): builds against `kition-headless` on the Kit Protocol.
- **Editor** (`web/tests`): `pnpm test:flow` and `pnpm test:modes` drive the kit's editor through
  the development host in Chromium and WebKit (Playwright).

### The editor's dependencies

By default the editor's dependencies come from npm: `pnpm install` in `web/` installs kition-sdk's
`@kition/sdk` and Kition Tessera's `@kition/tessera` at the versions the committed
`web/pnpm-lock.yaml` pins, so no other checkout is needed. To develop against local checkouts
instead, build the two packages (`pnpm build` in kition-sdk's `packages/sdk` and kition-tessera's
`packages/tessera`: their exports are the built `dist/`), then run `KITION_LINK=1 pnpm install` in
`web/`: `web/.pnpmfile.cjs` then links `@kition/sdk` from a kition-sdk checkout and
`@kition/tessera` from a kition-tessera checkout, by default `../kition-sdk` and `../kition-tessera`
beside this repository (`KITION_SDK` and `KITION_TESSERA` name others). A package whose folder is
not there stays on npm. The linked install rewrites `web/pnpm-lock.yaml`; a lockfile changed that
way must not be committed.

### Benchmarks

`crates/engine/engine/src/perf.rs` (`engine::perf::ops_bench`) measures everyday operations on a large
generated project; see [architecture.md §7](architecture.md).

## Run the headless kit for an agent

`kition-motion` (`bindings/headless`) serves one project at a time over the Kit Protocol;
kition-sdk's `kition-kits-mcp` turns it into MCP tools (`motion_inspect`, `motion_run`,
`motion_check`, ...) on a workspace folder for any MCP client:

```sh
cargo build --release -p kition-motion   # target/release/kition-motion
# in a kition-sdk checkout:
cargo build --release -p kition-kits --bin kition-kits-mcp
target/release/kition-kits-mcp --workspace <folder> --kit <kition-motion>/target/release/kition-motion
```

How an agent uses the tools is in [agent/guide.md](../agent/guide.md); kition-sdk's
[agent guide](https://github.com/KitionAI/kition-sdk/blob/main/docs/sdk/agent.md) covers the server.

## Take the README pictures

[`web/scripts/readme-shots.mjs`](../web/scripts/readme-shots.mjs) takes the README's editor pictures
(`assets/readme/hero.png`, `see-1.png` … `see-4.png`) with kition-sdk's shared runner
(`tools/readme-shots/`). It opens the showcase sample `motion/lumen-studio-logo-reveal` in the
development host (`web/dev.html`) at 1440 × 900, device scale 2, in Classic mode, the light theme
and English, and works the editor as a person would: the time ruler, the timeline's layers, the
ribbon's menus and Render MP4. It needs the engine built in `web/public/` (`pnpm engine`), and
kition-sdk and kition-showcase checkouts beside this repository (or `KITION_SDK` and
`KITION_SHOWCASE` naming them):

```sh
cd web
node scripts/readme-shots.mjs            # every picture
node scripts/readme-shots.mjs see-2      # only the named ones
```

Run it with no HTTP proxy set; `CHROME` may name a Chrome or Chromium executable to use instead of
Playwright's. When `pngquant` is installed the script shrinks each picture with
`pngquant --quality 70-90`; a README picture is at most 400 KB. `made-1.png` … `made-4.png` are
copies of showcase samples' `thumb.png` (ringed-planet-3d-title, neon-sign-flicker,
solar-output-bar-chart, festival-ticket-card-flip). Every picture has its `.attribution` sidecar
and a row in [attribution.md](attribution.md) (`cargo xtask assets` checks both).

## Publish

Kition ships the kit's package, `kition-motion-<version>.tgz` (the `motion-ui/` folder), a release
asset of this repository tagged `package-v<version>`. It is built on GitHub, never on a developer
machine: the Package workflow (`.github/workflows/package.yml`, which calls Kition's reusable
`kit-package.yml`) builds the commit it is started on, with `gh workflow run package.yml -R
KitionAI/kition-motion`, and publishes the Release with the package's lock entry,
`kition-motion-<version>.json`. Kition pins it with `node scripts/pin-kits.mjs motion` (see
`docs/kits.md` there). A version is published once: bump `version` in `kit.json` to publish a
change.
