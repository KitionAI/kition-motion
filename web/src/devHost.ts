/**
 * The development host (dev.html): opens `public/sample/Sample.kmotion`, or a file chosen in the
 * page. Its in-memory workspace is lent to the engine as Kition lends a project's folder
 * (`lend: true`, served by `devHostWorkspace()` in vite.config.ts): every file put in it (a test's
 * footage, a file dropped on the editor and written through the host) is where the engine reads
 * footage. File ▸ Import picks the first picture, movie or sound in the workspace, Apply Animation
 * Preset the first preset.
 */
import { startDevHost } from '@kition/sdk/dev-host'

startDevHost({ sample: './sample/Sample.kmotion', lend: true, pickable: /\.(png|jpe?g|gif|webp|svg|mp4|mov|m4v|webm|mp3|wav|ecpreset)$/i })
