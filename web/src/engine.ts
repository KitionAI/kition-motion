import type { KitModuleManifest } from '@kition/kit-editor'

/** The engine, from this package's own folder. */
export function engineManifest(): KitModuleManifest {
  return {
    kitId: 'kit.motion',
    base: new URL('./engine/', document.baseURI).href,
    module: 'kition_motion_kit',
    documentClass: 'MotionDocument',
    // The web build embeds no CJK face (build-engine.sh copies this one from kition-fonts).
    fonts: [new URL('./fonts/DroidSansFallbackFull.ttf', document.baseURI).href],
  }
}
