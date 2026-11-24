// Keyframe interpolation, the graph editor and the stopwatch by hand, in a real browser through the
// development host: node tests/curves.mjs [chromium|webkit]
// Every step drives the editor with the pointer and the keyboard and checks the result in the
// engine (ai.inspect, read-only ai.run queries). Prints PASS or FAIL per step.
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'
import { delayEngineReplies } from './slowEngine.mjs'

const which = process.argv[2] ?? 'chromium'
const shots = process.argv[3] === 'shots' ? new URL('../../../kition-plan/kit-docs/motion/images/', import.meta.url).pathname : null
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
const layer = async (id) => (await details()).info.layers.find((each) => each.id === id)
const times = async (id, path) => (await layer(id))?.animated.find((property) => property.path === path)?.times ?? []
/** A read-only engine query (the agent's own context: never the person's selection). */
const query = async (command, params) => (await page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])).completed[0].result
/** An agent's edit, to set a scene up. */
const agent = (command, params) => page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])
const kit = page.frameLocator('#kit')
const box = async (testId) => kit.getByTestId(testId).boundingBox()
let primary = 'Meta'

const SHAPE = 27
const POSITION = 'transform/position'
const OPACITY = 'transform/opacity'

async function open() {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
  primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
}
/** The sides of a property's keyframes, from the engine: `[[in, out], …]` in time order. */
const sides = async (path) => ((await query('prop.get', { layer: SHAPE, path })).keys ?? []).map((key) => [key.in, key.out])
/** What a property row's keyframe mark shows: its `data-in` and `data-out`. */
const shape = async (ms) => {
  const mark = kit.getByTestId(`motion-key-${SHAPE}-transform.position-${ms}`)
  return [await mark.getAttribute('data-in'), await mark.getAttribute('data-out')]
}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)
async function undo() {
  await kit.locator('.kui-app').focus()
  await page.keyboard.press(`${primary}+KeyZ`)
  await page.waitForTimeout(400)
}
async function seek(seconds) {
  const ruler = await box('motion-ruler')
  const duration = (await details()).comp.duration
  await page.mouse.click(ruler.x + (seconds * ruler.width) / duration, ruler.y + ruler.height - 4)
  await until(async () => near((await details()).time, seconds, 0.02))
}
async function drag(from, to) {
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  for (let i = 1; i <= 10; i += 1) await page.mouse.move(from.x + ((to.x - from.x) * i) / 10, from.y + ((to.y - from.y) * i) / 10)
  await page.mouse.up()
  await page.waitForTimeout(300)
}
const centre = async (locator) => {
  const area = await locator.boundingBox()
  return { x: area.x + area.width / 2, y: area.y + area.height / 2 }
}
/** Both Position keyframes selected by clicks on their marks (the layer twirled to Position with P). */
async function selectBoth() {
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).click()
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-2500`).click({ modifiers: ['Shift'] })
  await until(async () => (await kit.locator('.motion-key[aria-selected="true"]').count()) === 2)
}
/** Opens Animation ▸ Keyframe Interpolation and clicks the row `testId`. */
async function interpolationRow(testId) {
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Interpolation"]').hover()
  await kit.locator(`[role=menu] [data-testid="${testId}"]`).first().click()
  await page.waitForTimeout(400)
}

try {
  await open()
  // 1. Interpolation: the marks' shapes follow the keys' sides.
  await kit.getByTestId(`motion-layer-${SHAPE}`).click()
  await page.keyboard.press('KeyP')
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).waitFor()
  // The marks are drawn before their keys' interpolation has come back from the engine.
  await until(async () => (await shape(0))[0] !== null && (await shape(2500))[0] !== null)
  step('linear keyframes show as diamonds (both halves linear)', same(await shape(0), ['linear', 'linear']) && same(await shape(2500), ['linear', 'linear']), JSON.stringify([await shape(0), await shape(2500)]))
  await selectBoth()
  await page.keyboard.press('F9')
  await until(async () => same(await sides(POSITION), [['Bezier', 'Bezier'], ['Bezier', 'Bezier']]))
  step('F9 eases the selected keyframes', same(await sides(POSITION), [['Bezier', 'Bezier'], ['Bezier', 'Bezier']]), JSON.stringify(await sides(POSITION)))
  await until(async () => same(await shape(0), ['bezier', 'bezier']))
  step('an eased keyframe shows as an hourglass', same(await shape(0), ['bezier', 'bezier']) && same(await shape(2500), ['bezier', 'bezier']), JSON.stringify([await shape(0), await shape(2500)]))
  await undo()
  await until(async () => same(await sides(POSITION), [['Linear', 'Linear'], ['Linear', 'Linear']]))
  step('one undo takes the ease away', same(await sides(POSITION), [['Linear', 'Linear'], ['Linear', 'Linear']]), JSON.stringify(await sides(POSITION)))
  await selectBoth()
  await page.keyboard.press('Shift+F9')
  await until(async () => (await sides(POSITION))[1]?.[0] === 'Bezier')
  step('Shift+F9 eases only the incoming sides', same(await sides(POSITION), [['Bezier', 'Linear'], ['Bezier', 'Linear']]), JSON.stringify(await sides(POSITION)))
  await undo()
  await selectBoth()
  await page.keyboard.press(`${primary}+Shift+F9`)
  await until(async () => (await sides(POSITION))[0]?.[1] === 'Bezier')
  step(`${primary}+Shift+F9 eases only the outgoing sides`, same(await sides(POSITION), [['Linear', 'Bezier'], ['Linear', 'Bezier']]), JSON.stringify(await sides(POSITION)))
  await undo()
  await selectBoth()
  await interpolationRow('motion-control-interp-hold')
  await until(async () => (await sides(POSITION))[0]?.[1] === 'Hold')
  step('Animation ▸ Keyframe Interpolation ▸ Hold holds the keyframes', (await sides(POSITION)).every((side) => side[1] === 'Hold'), JSON.stringify(await sides(POSITION)))
  await until(async () => (await shape(0))[1] === 'hold')
  step('a hold keyframe shows a square half', (await shape(0))[1] === 'hold', JSON.stringify(await shape(0)))
  await undo()
  // The keyframe's right-click menu, on the clicked keyframe alone.
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).click()
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).click({ button: 'right' })
  await kit.getByTestId('motion-key-menu').waitFor()
  await kit.getByTestId('motion-key-menu-bezier').click()
  await until(async () => (await sides(POSITION))[0]?.[1] === 'Bezier')
  step('the keyframe menu sets Bezier on the clicked keyframe', same(await sides(POSITION), [['Bezier', 'Bezier'], ['Linear', 'Linear']]), JSON.stringify(await sides(POSITION)))
  await undo()
  // Keyframe Interpolation… (Cmd+Alt+K): the temporal interpolation from a dialog.
  await selectBoth()
  await page.keyboard.press(`${primary}+Alt+KeyK`)
  await kit.getByTestId('motion-interp-dialog').waitFor({ timeout: 3000 }).catch(() => null)
  step(`${primary}+Alt+K opens Keyframe Interpolation`, (await kit.getByTestId('motion-interp-dialog').count()) === 1)
  await kit.getByTestId('motion-interp-temporal').selectOption('hold')
  await kit.getByTestId('motion-interp-dialog-ok').click()
  await until(async () => (await sides(POSITION))[0]?.[1] === 'Hold')
  step('its Hold holds the selected keyframes', (await sides(POSITION)).every((side) => side[1] === 'Hold'), JSON.stringify(await sides(POSITION)))
  await undo()
  // Keyframe Velocity… from the keyframe menu: influence typed in, one undo step.
  const uid = (await query('prop.get', { layer: SHAPE, path: POSITION })).uid
  const ease = async () => (await query('keys.info', { keys: [{ layer: SHAPE, prop: uid, time: 0 }] }))[0]?.outEase?.[0]
  await selectBoth()
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).click({ button: 'right' })
  await kit.getByTestId('motion-key-menu-velocity').click()
  await kit.getByTestId('motion-velocity-dialog').waitFor()
  await kit.getByTestId('motion-velocity-out-influence').fill('75')
  await kit.getByTestId('motion-velocity-out-speed').fill('0')
  await kit.getByTestId('motion-velocity-dialog-ok').click()
  await until(async () => near((await ease())?.influence ?? 0, 75, 1e-3))
  step('Keyframe Velocity sets the outgoing influence', near((await ease())?.influence ?? 0, 75, 1e-3), JSON.stringify(await ease()))
  await undo()
  await until(async () => !near((await ease())?.influence ?? 0, 75, 1e-3))
  step('one undo puts the velocity back', !near((await ease())?.influence ?? 0, 75, 1e-3), JSON.stringify(await ease()))

  // 2. The graph editor: its button and Shift+F3, the curves, dragging a key and a handle.
  await kit.getByTestId('motion-graph-toggle').click()
  await kit.getByTestId('motion-graph').waitFor()
  const curve = kit.getByTestId(`motion-graph-curve-${SHAPE}-transform.position-0`)
  await until(async () => (await curve.getAttribute('points'))?.split(' ').length > 10)
  step('the Graph button shows the selected layer’s value curves', (await curve.getAttribute('points'))?.split(' ').length > 10 && (await kit.getByTestId(`motion-graph-curve-${SHAPE}-transform.position-1`).count()) === 1)
  await kit.locator('.kui-app').focus()
  await page.keyboard.press('Shift+F3')
  await until(async () => (await kit.getByTestId('motion-graph').count()) === 0)
  step('Shift+F3 goes back to the layers', (await kit.getByTestId('motion-graph').count()) === 0 && (await kit.getByTestId(`motion-layer-${SHAPE}`).count()) === 1)
  await page.keyboard.press('Shift+F3')
  await kit.getByTestId('motion-graph').waitFor()
  step('and Shift+F3 shows the graph again', (await kit.getByTestId('motion-graph').count()) === 1)
  const last = kit.getByTestId(`motion-graph-key-${SHAPE}-transform.position-0-2500`)
  await last.waitFor()
  const from = await centre(last)
  await drag(from, { x: from.x - 120, y: from.y + 30 })
  const keys = async () => (await query('prop.get', { layer: SHAPE, path: POSITION })).keys
  await until(async () => (await keys())[1].compTime < 2.4)
  const moved = (await keys())[1]
  step('a key drags earlier in time, on a frame', moved.compTime < 2.4 && near(moved.compTime * 30, Math.round(moved.compTime * 30), 1e-6), String(moved.compTime))
  step('and to another value', moved.value[0] < 1079 && near(moved.value[1], 360, 1e-6), JSON.stringify(moved.value))
  await undo()
  await until(async () => near((await keys())[1].compTime, 2.5, 1e-6))
  step('one undo puts the key back in time and value', near((await keys())[1].compTime, 2.5, 1e-6) && near((await keys())[1].value[0], 1080, 1e-6), JSON.stringify((await keys())[1]))
  // A handle: Opacity eases in time alone, so its eased keys show handles.
  await agent('prop.addKey', { layer: SHAPE, path: OPACITY, time: 0, value: 100 })
  await agent('prop.addKey', { layer: SHAPE, path: OPACITY, time: 2, value: 0 })
  const opacityKey = kit.getByTestId(`motion-graph-key-${SHAPE}-transform.opacity-0-0`)
  await opacityKey.waitFor()
  await opacityKey.click()
  await page.keyboard.press('F9')
  const handle = kit.getByTestId(`motion-graph-handle-${SHAPE}-transform.opacity-0-0-out`)
  await handle.waitFor({ timeout: 4000 }).catch(() => null)
  step('a selected eased key shows its Bezier handle', (await handle.count()) === 1)
  const opacityUid = (await query('prop.get', { layer: SHAPE, path: OPACITY })).uid
  const opacityEase = async () => (await query('keys.info', { keys: [{ layer: SHAPE, prop: opacityUid, time: 0 }] }))[0]?.outEase?.[0]
  const before = await opacityEase()
  if (await handle.count()) {
    const at = await centre(handle)
    await drag(at, { x: at.x + 80, y: at.y + 25 })
  }
  await until(async () => !near((await opacityEase())?.influence ?? 0, before?.influence ?? 0, 1e-3))
  const after = await opacityEase()
  step('dragging the handle changes the influence and the speed', after && !near(after.influence, before.influence, 1) && !near(after.speed, before.speed, 1e-3), `${JSON.stringify(before)} → ${JSON.stringify(after)}`)
  await undo()
  await until(async () => near((await opacityEase())?.influence ?? 0, before?.influence ?? 0, 1e-3))
  step('one undo puts the handle back', near((await opacityEase())?.influence ?? 0, before.influence, 1e-3), JSON.stringify(await opacityEase()))

  // Each property on its own scale: Opacity (0 to 100) fills the height as Position (hundreds of pixels) does.
  const spanOf = async (testId) => {
    const ys = ((await kit.getByTestId(testId).getAttribute('points')) ?? '').split(' ').map((point) => Number(point.split(',')[1])).filter(Number.isFinite)
    return ys.length ? Math.max(...ys) - Math.min(...ys) : 0
  }
  const plot = await box('motion-graph-svg')
  await until(async () => (await spanOf(`motion-graph-curve-${SHAPE}-transform.opacity-0`)) > plot.height * 0.6)
  step('Opacity fills the graph’s height on its own scale beside Position', (await spanOf(`motion-graph-curve-${SHAPE}-transform.opacity-0`)) > plot.height * 0.6 && (await spanOf(`motion-graph-curve-${SHAPE}-transform.position-0`)) > plot.height * 0.6, `${await spanOf(`motion-graph-curve-${SHAPE}-transform.opacity-0`)} of ${plot.height}`)
  await kit.getByTestId(`motion-graph-legend-${SHAPE}-transform.opacity`).locator('button').click()
  await until(async () => (await kit.getByTestId('motion-graph').getAttribute('data-scale')) === `${SHAPE}-transform.opacity`)
  step('a click on a property’s name puts its values on the scale', (await kit.getByTestId('motion-graph').getAttribute('data-scale')) === `${SHAPE}-transform.opacity`)

  // Fit all: zoomed in so the last key is out of view, F (and the button) brings every key back.
  const lastKey = kit.getByTestId(`motion-graph-key-${SHAPE}-transform.position-0-2500`)
  const inView = async (locator) => {
    const x = Number(await locator.getAttribute('x'))
    return x >= -4 && x <= plot.width
  }
  await kit.locator('.kui-app').focus()
  for (let i = 0; i < 4; i += 1) await page.keyboard.press('Equal')
  await until(async () => !(await inView(lastKey)))
  const hidden = !(await inView(lastKey))
  await page.keyboard.press('KeyF')
  await until(async () => (await inView(lastKey)) && (await inView(kit.getByTestId(`motion-graph-key-${SHAPE}-transform.opacity-0-0`))))
  step('F fits every key into the graph', hidden && (await inView(lastKey)) && (await inView(kit.getByTestId(`motion-graph-key-${SHAPE}-transform.opacity-0-0`))))
  for (let i = 0; i < 4; i += 1) await page.keyboard.press('Equal')
  await until(async () => !(await inView(lastKey)))
  await kit.getByTestId('motion-graph-fit').click()
  await until(async () => inView(lastKey))
  step('and so does the 适合全部 button', await inView(lastKey))

  // A box drawn on empty space selects the keys inside it.
  const area = await box('motion-graph-svg')
  // From the top right to past the left edge: keys at 0 s sit on the graph's left edge. The press must
  // land on empty space (on a key it would pick that key instead of starting a box), and the last key
  // of a curve that reaches the top sits in that corner, nearer or farther with the page's fonts; so
  // the press goes to the right of every key, at the first height along the right edge clear of them.
  const start = await kit.getByTestId('motion-graph-svg').evaluate((svg) => {
    const frame = svg.getBoundingClientRect()
    const marks = [...svg.querySelectorAll('.motion-graph-key, .motion-graph-handle')].map((mark) => mark.getBoundingClientRect())
    const x = frame.width - 1
    for (let y = 1; y < frame.height / 2; y += 2) {
      const [px, py] = [frame.left + x, frame.top + y]
      if (marks.every((mark) => px < mark.left - 6 || px > mark.right + 6 || py < mark.top - 6 || py > mark.bottom + 6)) return { x, y }
    }
    return null
  })
  step('the graph has empty space beside its last keys to start a box from', start !== null, JSON.stringify(start))
  // What the page saw of the drag, printed with the result: a pointer that is cancelled (a native
  // drag or text selection beginning) drops the box.
  await kit.locator('body').evaluate((body) => {
    const seen = []
    window.__boxEvents = seen
    for (const type of ['pointerdown', 'pointerup', 'pointercancel', 'lostpointercapture', 'selectstart', 'dragstart'])
      body.ownerDocument.addEventListener(type, (event) => seen.push(`${type}@${event.target?.tagName ?? '?'}`), true)
  })
  await drag({ x: area.x + (start?.x ?? area.width - 3), y: area.y + (start?.y ?? 3) }, { x: area.x - 12, y: area.y + area.height - 3 })
  await until(async () => (await kit.locator('.motion-graph-key[aria-selected="true"]').count()) === (await kit.locator('.motion-graph-key').count()))
  step('a box over the graph selects every key in it', (await kit.locator('.motion-graph-key[aria-selected="true"]').count()) === (await kit.locator('.motion-graph-key').count()) && (await kit.locator('.motion-graph-key').count()) >= 6, `${await kit.locator('.motion-graph-key[aria-selected="true"]').count()} of ${await kit.locator('.motion-graph-key').count()}, saw ${await kit.locator('body').evaluate(() => window.__boxEvents.join(' '))}`)
  await page.mouse.click(area.x + area.width - 3, area.y + 3)
  await until(async () => (await kit.locator('.motion-graph-key[aria-selected="true"]').count()) === 0)
  step('a click on empty space clears them', (await kit.locator('.motion-graph-key[aria-selected="true"]').count()) === 0)

  // A busy page takes a drag's moves and its release in one go, before the moves are drawn (the
  // WebKit run on Linux selected 1 of 6): the box still reaches where the pointer was let go. The
  // mouse's own pointer (id 1) sends them in one task, after the press has been drawn.
  await kit.getByTestId('motion-graph-svg').evaluate(async (svg) => {
    const r = svg.getBoundingClientRect()
    const [from, to] = [{ x: r.right - 3, y: r.top + 3 }, { x: r.left - 12, y: r.bottom - 3 }]
    const send = (type, x, y) =>
      svg.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'mouse', isPrimary: true, button: 0, buttons: type === 'pointerup' ? 0 : 1, clientX: x, clientY: y }))
    send('pointerdown', from.x, from.y)
    await new Promise((done) => setTimeout(done, 50))
    for (let i = 1; i <= 10; i += 1) send('pointermove', from.x + ((to.x - from.x) * i) / 10, from.y + ((to.y - from.y) * i) / 10)
    send('pointerup', to.x, to.y)
  })
  await until(async () => (await kit.locator('.motion-graph-key[aria-selected="true"]').count()) === (await kit.locator('.motion-graph-key').count()))
  step('a box whose moves and release come at once selects every key in it too', (await kit.locator('.motion-graph-key[aria-selected="true"]').count()) === (await kit.locator('.motion-graph-key').count()), `${await kit.locator('.motion-graph-key[aria-selected="true"]').count()} of ${await kit.locator('.motion-graph-key').count()}`)
  await page.mouse.click(area.x + area.width - 3, area.y + 3)
  await until(async () => (await kit.locator('.motion-graph-key[aria-selected="true"]').count()) === 0)

  // A key dragged past its property's range stops at it: Opacity cannot pass 100.
  const opacityLast = kit.getByTestId(`motion-graph-key-${SHAPE}-transform.opacity-0-2000`)
  const fromLast = await centre(opacityLast)
  await drag(fromLast, { x: fromLast.x, y: area.y - 60 })
  const opacityKeys = async () => (await query('prop.get', { layer: SHAPE, path: OPACITY })).keys
  await until(async () => (await opacityKeys())[1].value > 50)
  step('Opacity dragged above the graph stops at 100', near((await opacityKeys())[1].value, 100, 1e-6), String((await opacityKeys())[1].value))
  await undo()
  await until(async () => near((await opacityKeys())[1].value, 0, 1e-6))

  // The speed graph: Position draws as one speed along its path, and its eased key's handles drag.
  await kit.getByTestId('motion-graph-mode-speed').click()
  await until(async () => (await kit.getByTestId('motion-graph').getAttribute('data-mode')) === 'speed' && (await kit.getByTestId(`motion-graph-curve-${SHAPE}-transform.position-1`).count()) === 0)
  if (shots) await page.screenshot({ path: `${shots}phase6-graph-speed.png` })
  step('the speed graph draws Position as one speed along its path', (await kit.getByTestId(`motion-graph-curve-${SHAPE}-transform.position-0`).count()) === 1 && (await kit.getByTestId(`motion-graph-curve-${SHAPE}-transform.position-1`).count()) === 0)
  const positionKey = kit.getByTestId(`motion-graph-key-${SHAPE}-transform.position-0-2500`)
  await positionKey.click()
  await until(async () => (await positionKey.getAttribute('aria-selected')) === 'true')
  await page.keyboard.press('F9')
  const positionHandle = kit.getByTestId(`motion-graph-handle-${SHAPE}-transform.position-0-2500-in`)
  await positionHandle.waitFor({ timeout: 4000 }).catch(() => null)
  step('an eased Position key shows its handle in the speed graph', (await positionHandle.count()) === 1)
  const positionEase = async () => (await query('keys.info', { keys: [{ layer: SHAPE, prop: uid, time: 2.5 }] }))[0]?.inEase?.[0]
  const easeBefore = await positionEase()
  if (await positionHandle.count()) {
    const at = await centre(positionHandle)
    await drag(at, { x: at.x - 60, y: at.y - 30 })
  }
  await until(async () => !near((await positionEase())?.influence ?? 0, easeBefore?.influence ?? 0, 1e-3))
  const easeAfter = await positionEase()
  step('dragging it changes Position’s influence and speed along its path', easeAfter && easeBefore && !near(easeAfter.influence, easeBefore.influence, 1) && !near(easeAfter.speed, easeBefore.speed, 1e-3), `${JSON.stringify(easeBefore)} → ${JSON.stringify(easeAfter)}`)
  await undo()
  await until(async () => near((await positionEase())?.influence ?? 0, easeBefore?.influence ?? 0, 1e-3))
  step('one undo puts it back', near((await positionEase())?.influence ?? 0, easeBefore?.influence ?? 0, 1e-3))
  await undo()
  await kit.getByTestId('motion-graph-mode-value').click()
  await kit.getByTestId('motion-graph-toggle').click()

  // 3. The stopwatch: start animating, add between keys, remove at a key, clear by Alt-click and by its menu.
  await open()
  await kit.getByTestId(`motion-layer-${SHAPE}`).click()
  await kit.getByTestId('motion-add-key-rotation').waitFor()
  await seek(1)
  await kit.getByTestId('motion-add-key-rotation').click()
  await until(async () => (await times(SHAPE, 'transform/rotation')).length === 1)
  step('the diamond of a still property starts animating it at the current time', (await times(SHAPE, 'transform/rotation')).length === 1 && near((await times(SHAPE, 'transform/rotation'))[0], 1, 0.02), JSON.stringify(await times(SHAPE, 'transform/rotation')))
  await kit.getByTestId('motion-add-key-position').click()
  await until(async () => (await times(SHAPE, POSITION)).length === 3)
  step('between two keyframes it adds one', (await times(SHAPE, POSITION)).length === 3, JSON.stringify(await times(SHAPE, POSITION)))
  await kit.getByTestId('motion-add-key-position').click()
  await until(async () => (await times(SHAPE, POSITION)).length === 2)
  step('at a keyframe it removes it', (await times(SHAPE, POSITION)).length === 2, JSON.stringify(await times(SHAPE, POSITION)))
  await kit.getByTestId('motion-add-key-position').click({ modifiers: ['Alt'] })
  await until(async () => (await times(SHAPE, POSITION)).length === 0)
  step('Alt-click clears the animation', (await times(SHAPE, POSITION)).length === 0, JSON.stringify(await times(SHAPE, POSITION)))
  await undo()
  await until(async () => (await times(SHAPE, POSITION)).length === 2)
  step('one undo brings the keyframes back', (await times(SHAPE, POSITION)).length === 2)
  await kit.getByTestId('motion-add-key-rotation').click({ button: 'right' })
  await kit.getByTestId('motion-add-key-rotation-menu-clear').click()
  await until(async () => (await times(SHAPE, 'transform/rotation')).length === 0)
  step('清除动画 in the diamond’s menu clears it too', (await times(SHAPE, 'transform/rotation')).length === 0)
  // Home ▸ 添加关键帧: the selected layer's Position, here at 1 s.
  await kit.getByTestId('ribbon-tab-home').click()
  const add = kit.getByTestId('motion-home-add-key')
  await add.waitFor()
  step('Home’s 添加关键帧 is enabled with a layer selected', await add.isEnabled())
  await add.click()
  await until(async () => (await times(SHAPE, POSITION)).length === 3)
  step('and keys its Position at the current time', (await times(SHAPE, POSITION)).some((time) => near(time, 1, 0.02)), JSON.stringify(await times(SHAPE, POSITION)))
  step('no page errors', errors.length === 0, errors.join('; '))
} catch (error) {
  step('the run finished', false, String(error?.message ?? error).split('\n')[0])
} finally {
  await browser.close()
  await server.close()
}
console.log(failures ? `${failures} failed` : 'all passed')
process.exit(failures ? 1 : 0)
