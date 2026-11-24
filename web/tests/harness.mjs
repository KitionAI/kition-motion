// What the browser scripts share: the development host served by Vite, a browser, the editor's
// frame, PASS/FAIL steps, waiting for the engine's answers, and read-only engine queries (the
// agent's own context: never the person's selection). A script:
//   const h = await start(process.argv[2] ?? 'chromium')
//   try { await h.open(); …; h.step('name', ok, detail) } catch (error) { h.crashed(error) } finally { await h.finish() }
import { deflateSync } from 'node:zlib'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'
import { delayEngineReplies } from './slowEngine.mjs'

/**
 * The browser for `which`. WebKit on Linux (the CI runners: no GPU, Mesa's llvmpipe) paints with
 * Skia on the CPU: its GPU painting threads there died now and then inside WebKit, not in the page
 * (a Skia shader that failed to compile, "trap invalid opcode in libWPEWebKit", an llvmpipe or
 * SkiaGPUWorker segfault), which closed the page under the test. `KIT_WEBKIT_GPU=1` keeps the GPU path.
 */
export function launchBrowser(which) {
  if (which !== 'webkit') return chromium.launch()
  const cpu = process.platform === 'linux' && process.env.KIT_WEBKIT_GPU !== '1'
  return webkit.launch(cpu ? { env: { ...process.env, WEBKIT_SKIA_ENABLE_CPU_RENDERING: '1' } } : {})
}

/** A solid-colour PNG, `width` × `height`, made here (no file from anywhere else). */
export function png(width, height, [r, g, b]) {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    return c >>> 0
  })
  const crc = (bytes) => {
    let c = 0xffffffff
    for (const byte of bytes) c = crcTable[(c ^ byte) & 255] ^ (c >>> 8)
    return (c ^ 0xffffffff) >>> 0
  }
  const chunk = (type, data) => {
    const out = Buffer.alloc(12 + data.length)
    out.writeUInt32BE(data.length, 0)
    out.write(type, 4, 'ascii')
    data.copy(out, 8)
    out.writeUInt32BE(crc(out.subarray(4, 8 + data.length)), 8 + data.length)
    return out
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(width, 0)
  header.writeUInt32BE(height, 4)
  header[8] = 8
  header[9] = 2
  const rows = Buffer.alloc((width * 3 + 1) * height)
  for (let y = 0; y < height; y += 1) for (let x = 0; x < width; x += 1) rows.set([r, g, b], y * (width * 3 + 1) + 1 + x * 3)
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', header), chunk('IDAT', deflateSync(rows)), chunk('IEND', Buffer.alloc(0))])
}

/** A mono 16-bit WAV of a 440 Hz tone, `seconds` long. */
export function toneWav(seconds = 1, rate = 48000) {
  const frames = Math.round(seconds * rate)
  const out = Buffer.alloc(44 + frames * 2)
  out.write('RIFF', 0, 'ascii')
  out.writeUInt32LE(36 + frames * 2, 4)
  out.write('WAVEfmt ', 8, 'ascii')
  out.writeUInt32LE(16, 16)
  out.writeUInt16LE(1, 20)
  out.writeUInt16LE(1, 22)
  out.writeUInt32LE(rate, 24)
  out.writeUInt32LE(rate * 2, 28)
  out.writeUInt16LE(2, 32)
  out.writeUInt16LE(16, 34)
  out.write('data', 36, 'ascii')
  out.writeUInt32LE(frames * 2, 40)
  for (let i = 0; i < frames; i += 1) out.writeInt16LE(Math.round(Math.sin((2 * Math.PI * 440 * i) / rate) * 12000), 44 + i * 2)
  return out
}

/** `HH:MM:SS.mmm`, for the lines that say which step a run had reached. */
export const clock = () => new Date().toISOString().slice(11, 23)

export async function start(which) {
  const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1', watch: null }, logLevel: 'error' })
  await server.listen()
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`
  const browser = await launchBrowser(which)
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
  await delayEngineReplies(context)
  const page = await context.newPage()
  const errors = []
  let failures = 0
  page.on('pageerror', (error) => errors.push(error.message))
  const kit = page.frameLocator('#kit')
  const h = {
    which,
    page,
    kit,
    primary: 'Meta',
    step(name, ok, detail = '') {
      if (!ok) failures += 1
      console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` - ${detail}` : ''}`)
    },
    /** Waits (up to `tries` × 100 ms) until `check` answers true; answers whether it did. */
    async until(check, tries = 50) {
      for (let i = 0; i < tries; i += 1) {
        if (await check().catch(() => false)) return true
        await page.waitForTimeout(100)
      }
      return false
    },
    async details() {
      return (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
    },
    async layers() {
      return (await h.details()).info.layers
    },
    async layer(id) {
      return (await h.layers()).find((each) => each.id === id)
    },
    async query(command, params) {
      return (await page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])).completed[0].result
    },
    /** Runs commands as the agent would (to set a scene up quickly; never what a step checks). */
    async setUp(steps) {
      return page.evaluate(async (list) => window.devHost.connection().ai.run(list, false), steps)
    },
    /** Puts a file in the host's workspace (where the engine reads footage and the picker finds it). */
    async putFile(name, bytes) {
      await page.evaluate(([path, data]) => window.devHost.files.set(path, { bytes: new Uint8Array(data).buffer, version: 1 }), [name, [...bytes]])
    },
    /** The project's items (`project.summary`). */
    async items() {
      return (await h.query('project.summary', {})).items
    },
    /** Opens the sample afresh. */
    async open() {
      await page.goto(`${origin}/dev.html?locale=zh-CN`)
      await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
      await kit.getByTestId('motion-timeline').waitFor()
      await page.waitForTimeout(500)
      h.primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
    },
    async focusEditor() {
      await kit.locator('.kui-app').focus()
    },
    async undo() {
      await h.focusEditor()
      await page.keyboard.press(`${h.primary}+KeyZ`)
    },
    /** A press at `from`, moved in small steps to `to` (page points), released. */
    async drag(from, to, steps = 10) {
      await page.mouse.move(from.x, from.y)
      await page.mouse.down()
      for (let i = 1; i <= steps; i += 1) {
        await page.mouse.move(from.x + ((to.x - from.x) * i) / steps, from.y + ((to.y - from.y) * i) / steps)
        await page.waitForTimeout(16)
      }
      await page.mouse.up()
    },
    /** The centre of an element on the page, or null when it is not there. */
    async centre(locator) {
      const box = await locator.boundingBox().catch(() => null)
      return box ? { x: box.x + box.width / 2, y: box.y + box.height / 2 } : null
    },
    /** A composition point on the page, from the canvas's view. */
    async at(cx, cy) {
      const [zoom, x, y] = (await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ').map(Number)
      const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
      return { x: surface.x + (cx - x) * zoom, y: surface.y + (cy - y) * zoom }
    },
    /** The colour the canvas shows at a composition point, [r, g, b]. */
    async rgb(cx, cy) {
      const p = await h.at(cx, cy)
      const shot = await page.screenshot({ clip: { x: Math.round(p.x), y: Math.round(p.y), width: 1, height: 1 } })
      return page.evaluate(async (data) => {
        const image = await createImageBitmap(await (await fetch(`data:image/png;base64,${data}`)).blob())
        const canvas = new OffscreenCanvas(1, 1)
        const context = canvas.getContext('2d')
        context.drawImage(image, 0, 0)
        return [...context.getImageData(0, 0, 1, 1).data.slice(0, 3)]
      }, shot.toString('base64'))
    },
    /** What the engine's frame cache reported last (the workspace's `data-preview`): frames kept, shown, rendered, found in the cache, and render milliseconds. */
    async cache() {
      const [cached, shown, rendered, hits, renderMs] = ((await kit.getByTestId('motion-workspace').getAttribute('data-preview')) ?? '').split(' ').map(Number)
      return { cached, shown, rendered, hits, renderMs }
    },
    crashed(error) {
      failures += 1
      console.log(`FAIL ran to the end - ${error instanceof Error ? (error.stack ?? error.message).split('\n').slice(0, 3).join(' ') : String(error)}`)
    },
    async finish() {
      h.step('no page errors', errors.length === 0, errors.slice(0, 8).join(' | '))
      await browser.close()
      await server.close()
      console.log(failures ? `${failures} failed (${which})` : `all passed (${which})`)
      process.exit(failures ? 1 : 0)
    },
  }
  return h
}
