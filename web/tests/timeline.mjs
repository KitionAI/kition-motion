// The timeline by hand, in a real browser through the development host: node tests/timeline.mjs [chromium|webkit]
// Every step drives the editor with the pointer and the keyboard (page.mouse, page.keyboard) and
// checks the result in the engine (ai.inspect, read-only ai.run queries). Prints PASS or FAIL per step.
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
const page = await context.newPage()
const errors = []
let failures = 0
page.on('pageerror', (error) => errors.push(error.message))
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` - ${detail}` : ''}`)
}
const until = async (check, tries = 50) => {
  for (let i = 0; i < tries; i += 1) {
    if (await check().catch(() => false)) return true
    await page.waitForTimeout(100)
  }
  return false
}
const near = (a, b, eps = 1e-6) => Math.abs(a - b) <= eps
const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
const layers = async () => (await details()).info.layers
const layer = async (id) => (await layers()).find((each) => each.id === id)
const times = async (id, path) => (await layer(id))?.animated.find((property) => property.path === path)?.times ?? []
/** A read-only engine query (the agent's own context: never the person's selection). */
const query = async (command, params) => (await page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])).completed[0].result
const kit = page.frameLocator('#kit')
const box = async (testId) => kit.getByTestId(testId).boundingBox()
let primary = 'Meta'

/** Opens the sample afresh: every section starts from the same document. */
async function open() {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
  primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
}

/** A track's box and its pixels per second (the timeline shows the whole composition until zoomed). */
async function track(testId) {
  const area = await kit.getByTestId(testId).locator('.motion-row-track').first().boundingBox()
  const duration = (await details()).comp.duration
  return { ...area, pps: area.width / duration, x: area.x, at: (seconds) => area.x + (seconds * area.width) / duration, cy: area.y + area.height / 2 }
}

/** A press, a drag in small steps, and a release; `during` runs before the release. */
async function drag(from, to, during) {
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  const steps = 10
  for (let i = 1; i <= steps; i += 1) await page.mouse.move(from.x + ((to.x - from.x) * i) / steps, from.y + ((to.y - from.y) * i) / steps)
  const seen = during ? await during() : undefined
  await page.mouse.up()
  return seen
}

/** Seeks by clicking the ruler at `seconds`. */
async function seek(seconds) {
  const ruler = await box('motion-ruler')
  const duration = (await details()).comp.duration
  await page.mouse.click(ruler.x + (seconds * ruler.width) / duration, ruler.y + ruler.height - 4)
  await until(async () => near((await details()).time, seconds, 0.02))
}

const SHAPE = 27
const TEXT = 73
const SOLID = 4

try {
  // 1. Layer bars: move, trim, snap, one undo step.
  await open()
  let row = await track(`motion-layer-${SHAPE}`)
  const readout = await drag({ x: row.at(1.5), y: row.cy }, { x: row.at(2), y: row.cy }, async () => (await kit.getByTestId('motion-bar-readout').textContent().catch(() => null)) ?? '')
  await until(async () => near((await layer(SHAPE)).in, 0.5))
  let shape = await layer(SHAPE)
  step('dragging a bar moves the layer in time', near(shape.in, 0.5) && near(shape.out, 3.5), `${shape.in}–${shape.out}`)
  step('the bar shows its new in and out while dragging', readout.includes('0:00:00:15') && readout.includes('0:00:03:15'), readout)
  await page.keyboard.press(`${primary}+KeyZ`)
  await until(async () => near((await layer(SHAPE)).in, 0))
  step('one undo takes the whole drag back', near((await layer(SHAPE)).in, 0), String((await layer(SHAPE)).in))
  row = await track(`motion-layer-${SHAPE}`)
  await drag({ x: row.at(0) + 2, y: row.cy }, { x: row.at(0.5) + 2, y: row.cy })
  await until(async () => near((await layer(SHAPE)).in, 0.5))
  shape = await layer(SHAPE)
  step('dragging the start of a bar trims the in point', near(shape.in, 0.5) && near(shape.out, 3), `${shape.in}–${shape.out}`)
  await drag({ x: row.at(3) - 2, y: row.cy }, { x: row.at(2.5) - 2, y: row.cy })
  await until(async () => near((await layer(SHAPE)).out, 2.5))
  shape = await layer(SHAPE)
  step('dragging the end of a bar trims the out point', near(shape.in, 0.5) && near(shape.out, 2.5), `${shape.in}–${shape.out}`)
  await seek(1)
  // Between half a frame and the snap distance from the playhead: a frame would round away from it.
  const frame = row.pps / 30
  const offset = (frame / 2 + 8) / 2
  await drag({ x: row.at(0.5) + 2, y: row.cy }, { x: row.at(1) + offset, y: row.cy })
  await until(async () => (await layer(SHAPE)).in > 0.6)
  step('a dragged end snaps to the playhead', frame / 2 < 8 && near((await layer(SHAPE)).in, 1), `${(await layer(SHAPE)).in} (frame ${frame.toFixed(1)} px, offset ${offset.toFixed(1)} px)`)

} catch (error) {
  failures += 1
  console.log(`FAIL ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
