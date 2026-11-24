/**
 * The development host (dev.html): opens `public/sample/Sample.kmotion`, or a file chosen in the
 * page. Its in-memory workspace is lent to the engine as Kition lends a project's folder: every
 * file put in it (a test's footage, a file dropped on the editor and written through the host) is
 * also served at `__workspace/` (vite.config.ts), where the engine reads footage. File ▸ Import
 * picks the first picture, movie or sound in the workspace, Apply Animation Preset the first preset.
 */
import { startDevHost } from '@kition/sdk/dev-host'

startDevHost({ sample: './sample/Sample.kmotion', lend: { root: './__workspace/', folder: '' }, pickable: /\.(png|jpe?g|gif|webp|svg|mp4|mov|m4v|webm|mp3|wav|ecpreset)$/i })

const workspace = window.devHost.files
const keep = workspace.set.bind(workspace)
workspace.set = (path, file) => {
  // Synchronous, so the file is there before the editor can name it to the engine.
  const request = new XMLHttpRequest()
  request.open('PUT', new URL(`./__workspace/${path.split('/').map(encodeURIComponent).join('/')}`, location.href).href, false)
  try {
    request.send(file.bytes)
  } catch {
    // The server is gone: the editor still works, the engine just cannot read this file.
  }
  return keep(path, file)
}
