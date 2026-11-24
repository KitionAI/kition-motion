// What the browser scripts share: the development host served by Vite, a browser, the editor's
// frame, PASS/FAIL steps, waiting for the engine's answers, and read-only engine queries (the
// agent's own context: never the person's selection). A script:
//   const h = await start(process.argv[2] ?? 'chromium')
//   try { await h.open(); …; h.step('name', ok, detail) } catch (error) { h.crashed(error) } finally { await h.finish() }
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'
import { delayEngineReplies } from './slowEngine.mjs'

export async function start(which) {
  const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1', watch: null }, logLevel: 'error' })
  await server.listen()
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`
  const browser = await (which === 'webkit' ? webkit : chromium).launch()
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
