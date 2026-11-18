// Help ▸ Enable Logging and Reveal Logging File in the page's engine: the web build has no file
// system, so they must fail with a message, not panic and take the engine down with them.
// Runs against the built engine: `node --test web/tests/engine-logging.test.mjs` after
// `pnpm engine` (or set KITION_MOTION_ENGINE to another wasm-pack --target web output folder).
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath, pathToFileURL } from 'node:url'
import path from 'node:path'

const dir = process.env.KITION_MOTION_ENGINE ?? path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'public', 'engine')
const engine = await import(pathToFileURL(path.join(dir, 'kition_motion_kit.js')).href)
await engine.default({ module_or_path: readFileSync(path.join(dir, 'kition_motion_kit_bg.wasm')) })

for (const command of ['help.enableLogging', 'help.revealLogFile']) {
  test(`${command} fails with a message in the page and the engine keeps working`, () => {
    const doc = engine.MotionDocument.create('a.kmotion', 64, 36)
    let error
    try {
      doc.execute(command, '{}')
    } catch (caught) {
      error = caught
    }
    assert.ok(error, `${command} should fail without a file system`)
    assert.ok(!(error instanceof WebAssembly.RuntimeError), `${command} panicked: ${error}`)
    doc.execute('layer.newSolid', JSON.stringify({ name: 'Red', color: '#ff0000' }))
  })
}
