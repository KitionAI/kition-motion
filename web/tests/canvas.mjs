// The canvas tools by hand, in a real browser through the development host: node tests/canvas.mjs [chromium|webkit]
// Every step drives the composition with the pointer and the keyboard (page.mouse, page.keyboard)
// and checks the result in the engine (read-only ai.run queries: prop.get, layer.tree, comp.info),
// in the person's selection (the timeline's rows and inspect's `selected`), and in the canvas's
// pixels. `node tests/canvas.mjs chromium shots` also writes the help screenshots.
// Prints PASS or FAIL per step.
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const shots = process.argv[3] === 'shots' ? new URL('../../../kition-plan/kit-docs/motion/images/', import.meta.url).pathname : null
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1', watch: null }, logLevel: 'error' })
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
const near = (a, b, eps = 1e-3) => Math.abs(a - b) <= eps
const nearAll = (a, b, eps = 1e-3) => Array.isArray(a) && b.every((v, i) => near(a[i], v, eps))
/** A read-only engine query (the agent's own context: never the person's selection). */
const query = async (command, params) => (await page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])).completed[0].result
const value = async (layer, path) => (await query('prop.get', { layer, path })).value
const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
const layers = async () => (await details()).info.layers
/** The person's selection: the layers inspect marks selected (the timeline's and the Properties panel's). */
const selection = async () => (await layers()).filter((layer) => layer.selected).map((layer) => layer.id).sort((a, b) => a - b)
const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i])
const kit = page.frameLocator('#kit')
let primary = 'Meta'

const SHAPE = 27
const TEXT = 73
const SOLID = 4

/** The canvas view: zoom and the composition point at the surface's top left, and the surface's place on the page. */
async function view() {
  const [zoom, x, y] = (await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ').map(Number)
  const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
  return { zoom, x, y, left: surface.x, top: surface.y }
}
/** A composition point on the page. */
async function at(cx, cy) {
  const v = await view()
  return { x: v.left + (cx - v.x) * v.zoom, y: v.top + (cy - v.y) * v.zoom }
}
async function click(cx, cy, modifiers = []) {
  const p = await at(cx, cy)
  for (const key of modifiers) await page.keyboard.down(key)
  await page.mouse.click(p.x, p.y)
  for (const key of modifiers) await page.keyboard.up(key)
}
/** A press at composition point `from`, moved in small steps to `to` (composition pixels), released; `held` keys are down for the move only, `pressed` from the press. */
async function drag(from, to, { held = [], pressed = [] } = {}) {
  const a = await at(...from)
  const b = await at(...to)
  for (const key of pressed) await page.keyboard.down(key)
  await page.mouse.move(a.x, a.y)
  await page.mouse.down()
  for (const key of held) await page.keyboard.down(key)
  for (let i = 1; i <= 10; i += 1) {
    await page.mouse.move(a.x + ((b.x - a.x) * i) / 10, a.y + ((b.y - a.y) * i) / 10)
    await page.waitForTimeout(16)
  }
  await page.waitForTimeout(100)
  await page.mouse.up()
  for (const key of [...held, ...pressed].reverse()) await page.keyboard.up(key)
  await page.waitForTimeout(150)
}
const active = async () => (await kit.locator('[data-testid^="motion-tool-"][aria-pressed="true"]').getAttribute('data-testid'))?.replace('motion-tool-', '')
/** Keys go to the editor, not to a field. */
async function focusEditor() {
  await kit.locator('.kui-app').focus()
}
async function undo() {
  await focusEditor()
  await page.keyboard.press(`${primary}+KeyZ`)
}
/** The composition's picture as the canvas draws it, and one of its pixels at a composition point. */
async function pixel(cx, cy) {
  const p = await at(cx, cy)
  const png = await page.screenshot({ clip: { x: Math.round(p.x), y: Math.round(p.y), width: 1, height: 1 } })
  return png
}
const sourceText = async (layer) => {
  const v = await value(layer, 'text/sourceText')
  return typeof v === 'string' ? v : v?.text
}
const newest = async (before) => (await layers()).find((layer) => !before.some((old) => old.id === layer.id))

async function open() {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(800)
  primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
}

try {
  await open()
  // The sample: a 1280 × 720 comp with the Night solid over it all, the blue 300-pixel square of
  // Shape Layer 1 centred at (200, 360), and the Northern Lights text (faded in from nothing at 0 s).

  // 1. The rail and the default tool.
  const rail = await kit.locator('[data-testid^="motion-tool-"]').evaluateAll((els) => els.map((e) => e.getAttribute('data-testid')))
  step('the tool rail lists Selection, Hand, Zoom, Rectangle, Ellipse, Text and Pen', ['selection', 'hand', 'zoom', 'rectangle', 'ellipse', 'text', 'pen'].every((id) => rail.includes(`motion-tool-${id}`)), rail.join(' '))
  step('Selection is the tool the editor opens with', (await active()) === 'selection', await active())

  // 2. Clicks select the topmost layer under the pointer, as the timeline shows.
  await click(200, 360)
  await until(async () => same(await selection(), [SHAPE]))
  step('a click on the square selects Shape Layer 1', same(await selection(), [SHAPE]), JSON.stringify(await selection()))
  step('the timeline row shows it selected', (await kit.getByTestId(`motion-layer-${SHAPE}`).getAttribute('data-selected')) === 'true')
  await click(900, 600, ['Shift'])
  await until(async () => (await selection()).length === 2)
  step('Shift-click adds the solid under the empty area', same(await selection(), [SOLID, SHAPE]), JSON.stringify(await selection()))
  await click(900, 600, ['Shift'])
  await until(async () => (await selection()).length === 1)
  step('Shift-click again takes it out', same(await selection(), [SHAPE]), JSON.stringify(await selection()))
  // Outside the composition nothing is under the pointer.
  await click(-80, 300)
  await until(async () => (await selection()).length === 0)
  step('a click off every layer deselects', (await selection()).length === 0, JSON.stringify(await selection()))
  // A marquee from outside the picture over the square's corner touches the square and the solid.
  await drag([-60, 150], [80, 240])
  step('a marquee selects the layers it touches', same(await selection(), [SOLID, SHAPE]), JSON.stringify(await selection()))
  await click(-80, 300)

  // 3. Drawing shapes: Q, a drag, a layer of that size where it was drawn, selected, Selection again.
  await focusEditor()
  await page.keyboard.press('q')
  step('Q picks the Rectangle tool', (await until(async () => (await active()) === 'rectangle')), await active())
  let before = await layers()
  await drag([700, 100], [1000, 300])
  await until(async () => (await layers()).length === before.length + 1)
  const rect = await newest(before)
  step('a drag draws a rectangle layer', rect?.type === 'Shape' || rect?.name?.startsWith('Shape'), JSON.stringify(rect))
  const rectSize = await value(rect.id, 'contents/group/contents/rect/size')
  step('the rectangle is the size dragged (300 × 200)', nearAll(rectSize, [300, 200], 2), JSON.stringify(rectSize))
  const rectPos = await value(rect.id, 'transform/position')
  step('and centred where it was dragged (850, 200)', nearAll(rectPos, [850, 200], 2), JSON.stringify(rectPos))
  // Where the drag put it, as the engine holds it: WebKit reports whole page pixels for the pointer,
  // so at the canvas's zoom the drawn rectangle sits up to a composition pixel off (850, 200), and
  // the moves below are measured from there.
  const home = rectPos.slice(0, 2)
  const from = (dx, dy) => [home[0] + dx, home[1] + dy]
  step('the new layer is selected', same(await selection(), [rect.id]), JSON.stringify(await selection()))
  step('the tool goes back to Selection', await until(async () => (await active()) === 'selection'), await active())
  if (shots) await page.screenshot({ path: `${shots}phase2-selected.png` })
  await kit.getByTestId('motion-tool-ellipse').click()
  before = await layers()
  await drag([1000, 450], [1100, 520], { held: ['Shift'] })
  await until(async () => (await layers()).length === before.length + 1)
  const ellipse = await newest(before)
  const ellipseSize = ellipse ? await value(ellipse.id, 'contents/group/contents/ellipse/size') : null
  step('Shift draws a circle with the Ellipse tool (100 × 100)', nearAll(ellipseSize, [100, 100], 2), JSON.stringify(ellipseSize))

  // 4. Moving: a drag moves by composition pixels at the canvas's zoom, one undo step.
  await click(850, 200)
  await until(async () => same(await selection(), [rect.id]))
  const v = await view()
  await drag([850, 200], [910, 240], { held: [primary] })
  let moved = await value(rect.id, 'transform/position')
  step(`a drag moves the layer by the composition pixels dragged (zoom ${Math.round(v.zoom * 100)}%)`, nearAll(moved, from(60, 40), 1.5), JSON.stringify(moved))
  await undo()
  await until(async () => nearAll(await value(rect.id, 'transform/position'), home, 1e-3))
  step('one undo puts it back', nearAll(await value(rect.id, 'transform/position'), home, 1e-3), JSON.stringify(await value(rect.id, 'transform/position')))
  await drag([850, 200], [950, 215], { held: ['Shift', primary] })
  moved = await value(rect.id, 'transform/position')
  step('Shift holds the move to one axis', near(moved[1], home[1]) && nearAll(moved, from(100, 0), 1.5), JSON.stringify(moved))
  await undo()
  await until(async () => nearAll(await value(rect.id, 'transform/position'), home))
  // The arrow keys nudge: 1 pixel, 10 with Shift.
  await focusEditor()
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('Shift+ArrowDown')
  await until(async () => nearAll(await value(rect.id, 'transform/position'), from(1, 10)))
  step('arrow keys nudge 1 pixel, Shift 10', nearAll(await value(rect.id, 'transform/position'), from(1, 10)), JSON.stringify(await value(rect.id, 'transform/position')))
  await undo()
  await undo()
  await until(async () => nearAll(await value(rect.id, 'transform/position'), home))
  step('each nudge is one undo step', nearAll(await value(rect.id, 'transform/position'), home), JSON.stringify(await value(rect.id, 'transform/position')))

  // 5. Scaling by a handle: the bottom right corner scales each axis freely, and with Shift keeps
  // the proportions; the top left corner stays.
  await drag([1000, 300], [1150, 300], { held: [primary] })
  const free = await value(rect.id, 'transform/scale')
  step('a corner handle scales each axis freely', near(free[0], 150, 2) && near(free[1], 100, 1), JSON.stringify(free))
  await undo()
  await until(async () => nearAll(await value(rect.id, 'transform/scale'), [100, 100]))
  await drag([1000, 300], [1150, 330], { held: [primary, 'Shift'] })
  const scaled = await value(rect.id, 'transform/scale')
  step('Shift keeps a corner handle proportional', near(scaled[0], scaled[1], 0.5) && scaled[0] > 120 && scaled[0] < 148, JSON.stringify(scaled))
  const corner = await value(rect.id, 'transform/position')
  const factor = scaled[0] / 100
  step('the opposite corner stays where it was', nearAll([corner[0] - factor * 150, corner[1] - factor * 100], [700, 100], 2), JSON.stringify(corner))
  if (shots) {
    await drag([700 + 450, 100 + 300], [1180, 430], { held: [primary] })
    await page.screenshot({ path: `${shots}phase2-scaling.png` })
    await undo()
  }
  await undo()
  await until(async () => nearAll(await value(rect.id, 'transform/scale'), [100, 100]))
  step('one undo puts the scale back', nearAll(await value(rect.id, 'transform/scale'), [100, 100]) && nearAll(await value(rect.id, 'transform/position'), home))
  await drag([1000, 200], [1060, 200], { held: [primary] })
  await until(async () => !nearAll(await value(rect.id, 'transform/scale'), [100, 100]))
  const wide = await value(rect.id, 'transform/scale')
  step('a side handle scales one axis', near(wide[0], 120, 1.5) && near(wide[1], 100), JSON.stringify(wide))
  await undo()
  await until(async () => nearAll(await value(rect.id, 'transform/scale'), [100, 100]))

  // 6. Rotating: just outside a corner, about the anchor point; Shift snaps to 15°.
  await drag([1012, 88], [862, 362], { held: ['Shift'] })
  const turned = (await value(rect.id, 'transform/rotation'))
  step('a drag outside a corner rotates, Shift by 15°', Math.abs(turned) > 1 && near(turned % 15, 0, 1e-6), String(turned))
  await undo()
  await until(async () => near(await value(rect.id, 'transform/rotation'), 0))
  step('one undo puts the rotation back', near(await value(rect.id, 'transform/rotation'), 0))

  // 7. An animated position gets a key at the current time: the rectangle's Position keyed at
  // 0 s from the Properties panel's diamond, then dragged at 1.5 s.
  await click(850, 200)
  await until(async () => same(await selection(), [rect.id]))
  await kit.getByTestId('motion-add-key-position').click()
  await until(async () => (await query('prop.get', { layer: rect.id, path: 'transform/position' })).keys.length === 1)
  const ruler = await kit.getByTestId('motion-ruler').boundingBox()
  await page.mouse.click(ruler.x + ruler.width * 0.5, ruler.y + ruler.height / 2)
  await until(async () => (await details()).time > 1.3)
  const now = (await details()).time
  await drag([850, 200], [890, 220], { held: [primary] })
  const keys = (await query('prop.get', { layer: rect.id, path: 'transform/position' })).keys ?? []
  const atNow = (await query('prop.get', { layer: rect.id, path: 'transform/position', time: now })).value
  step(
    'moving an animated position keys it at the current time',
    keys.length === 2 && nearAll(atNow, [890, 220], 1.5),
    `${keys.length} keys, ${JSON.stringify(atNow)} at ${now}`,
  )
  await undo()
  await focusEditor()
  await page.keyboard.press('Home')
  await until(async () => (await details()).time === 0)

  // 8. Text: T, a click, typing lands in the new layer's Source Text; shortcuts do not fire.
  await click(-80, 300)
  await focusEditor()
  await page.keyboard.press('t')
  step('T picks the Text tool', await until(async () => (await active()) === 'text'), await active())
  before = await layers()
  await click(700, 600)
  await until(async () => (await layers()).length === before.length + 1)
  const typed = await newest(before)
  await page.waitForTimeout(200)
  await page.keyboard.type('Hello vpt')
  await page.keyboard.insertText('你好')
  await until(async () => (await sourceText(typed.id)) === 'Hello vpt你好')
  step('typed text lands in the new text layer (Chinese too)', (await sourceText(typed.id)) === 'Hello vpt你好', JSON.stringify(await sourceText(typed.id)))
  step('V, P and T typed into text pick no tool', (await active()) === 'text', await active())
  step('and Space typed into text does not play', !(await kit.getByTestId('motion-play').getAttribute('aria-label'))?.includes('暂停'))
  await page.keyboard.press('Backspace')
  await page.keyboard.press('Enter')
  await page.keyboard.type('x')
  await until(async () => (await sourceText(typed.id)) === 'Hello vpt你\nx')
  step('Backspace and Enter edit the text', (await sourceText(typed.id)) === 'Hello vpt你\nx', JSON.stringify(await sourceText(typed.id)))
  if (shots) await page.screenshot({ path: `${shots}phase2-text-editing.png` })
  await page.keyboard.press('Escape')
  await page.waitForTimeout(300)
  await page.keyboard.press('v')
  step('Escape ends the edit; V picks Selection again', await until(async () => (await active()) === 'selection'), await active())

  // 9. A double-click on a text layer with the Selection tool edits it, all of it selected.
  const pos = await value(typed.id, 'transform/position')
  const p = await at(pos[0] + 20, pos[1] - 10)
  await page.mouse.dblclick(p.x, p.y)
  await page.waitForTimeout(300)
  await page.keyboard.type('Bye')
  await until(async () => (await sourceText(typed.id)) === 'Bye')
  step('a double-click edits a text layer; typing replaces it', (await sourceText(typed.id)) === 'Bye', JSON.stringify(await sourceText(typed.id)))
  await click(-80, 300)
  await page.waitForTimeout(200)
  step('a click outside ends the edit', (await kit.getByTestId('kit-text-sink').evaluate((el) => document.activeElement !== el)), '')

  // 10. Space: a tap plays and pauses; held with a drag it pans and does not play.
  await focusEditor()
  const t0 = (await details()).time
  await page.keyboard.down('Space')
  const mid = await at(640, 360)
  await page.mouse.move(mid.x, mid.y)
  await page.mouse.down()
  await page.mouse.move(mid.x + 60, mid.y + 30, { steps: 5 })
  await page.mouse.up()
  await page.keyboard.up('Space')
  await page.waitForTimeout(400)
  step('Space with a drag pans and does not play', near((await details()).time, t0), `${t0} → ${(await details()).time}`)
  await page.keyboard.press('Space')
  await page.waitForTimeout(500)
  const playing = (await details()).time
  await page.keyboard.press('Space')
  await page.waitForTimeout(300)
  const paused = (await details()).time
  await page.waitForTimeout(400)
  step('a Space tap plays and a second one pauses', playing > t0 && near((await details()).time, paused), `${t0} → ${playing}, then ${paused} → ${(await details()).time}`)

  // 11. Drawing after Space played and paused (a key held back during playback must not leave the
  // canvas panning), with the rectangle shown while it is dragged.
  {
    await kit.getByTestId('motion-tool-rectangle').click()
    const a = await at(150, 520)
    const b = await at(450, 680)
    await page.mouse.move(a.x, a.y)
    await page.mouse.down()
    await page.mouse.move(b.x, b.y, { steps: 8 })
    await page.waitForTimeout(200)
    if (shots) await page.screenshot({ path: `${shots}phase2-draw-rect.png` })
    const count = (await layers()).length
    await page.mouse.up()
    step('a rectangle draws after playing and pausing with Space', await until(async () => (await layers()).length === count + 1))
  }

  // 12. View ▸ Show Rulers, a guide dragged out of the left ruler, the Selection tool snapping a
  // layer's edge onto it, the grid, and the guide dragged back onto the ruler to remove it.
  {
    const viewRow = async (command) => {
      await kit.getByTestId('motion-menu-View').click()
      await kit.locator(`[role=menu] [data-testid="motion-cmd-${command}"]`).first().click()
      await page.waitForTimeout(200)
    }
    const shown = async () => ((await kit.getByTestId('motion-workspace').getAttribute('data-overlays')) ?? '').split(' ')
    await kit.getByTestId('motion-tool-selection').click()
    await viewRow('view.rulers')
    await kit.getByTestId('motion-ruler-left').waitFor({ timeout: 3000 }).catch(() => null)
    step('View ▸ 显示标尺 shows the rulers', (await kit.getByTestId('motion-ruler-left').count()) === 1 && (await shown()).includes('rulers'))
    const guides = async () => (await query('comp.info', {})).guides ?? []
    // A 100 × 100 solid on top of the others, to move: its right edge is 50 px right of its position.
    await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'layer.newSolid', params: { name: 'Guide test', width: 100, height: 100, color: '#ffcc00' } }], false))
    await until(async () => (await layers()).some((each) => each.name === 'Guide test'))
    const MOVED = (await layers()).find((each) => each.name === 'Guide test').id
    const start = await value(MOVED, 'transform/position')
    const target = Math.round(start[0] + 50 + 203)
    const ruler = await kit.getByTestId('motion-ruler-left').boundingBox()
    const to = await at(target, 300)
    await page.mouse.move(ruler.x + ruler.width / 2, to.y)
    await page.mouse.down()
    for (let i = 1; i <= 10; i += 1) await page.mouse.move(ruler.x + ruler.width / 2 + ((to.x - ruler.x - ruler.width / 2) * i) / 10, to.y)
    await page.mouse.up()
    await until(async () => (await guides()).length === 1)
    const made = (await guides())[0]
    step('a drag out of the left ruler makes a vertical guide where it is let go', made?.orientation === 'vertical' && Math.abs(made.position - target) <= 2, JSON.stringify(made))
    await viewRow('view.grid')
    await until(async () => (await shown()).includes('grid'))
    if (shots) await page.screenshot({ path: `${shots}phase6-guides-grid.png` })
    step('View ▸ 显示网格 shows the grid', (await shown()).includes('grid'))
    await viewRow('view.grid')
    // The solid's right edge (start + 50) dragged 200 px lands 3 px short of the guide: it snaps.
    await drag([start[0], start[1]], [start[0] + 200, start[1]])
    await until(async () => (await value(MOVED, 'transform/position'))[0] > start[0] + 150)
    const moved = await value(MOVED, 'transform/position')
    step('a moved layer’s edge snaps onto the guide', near(moved[0] + 50, made?.position ?? NaN, 1e-6), `${moved[0] + 50} vs ${made?.position}`)
    await undo()
    await until(async () => near((await value(MOVED, 'transform/position'))[0], start[0]))
    // Lock Guides: the guide no longer takes a press; unlocked, dragged onto the ruler it goes.
    await viewRow('view.lockGuides')
    await until(async () => (await kit.getByTestId('motion-guide-0').count()) === 0)
    step('locked guides cannot be grabbed', (await kit.getByTestId('motion-guide-0').count()) === 0)
    await viewRow('view.lockGuides')
    const grab = await kit.getByTestId('motion-guide-0').boundingBox()
    await page.mouse.move(grab.x + grab.width / 2, grab.y + 200)
    await page.mouse.down()
    for (let i = 1; i <= 10; i += 1) await page.mouse.move(grab.x + grab.width / 2 + ((ruler.x + ruler.width / 2 - grab.x - grab.width / 2) * i) / 10, grab.y + 200)
    await page.mouse.up()
    await until(async () => (await guides()).length === 0)
    step('a guide dragged back onto the ruler is removed', (await guides()).length === 0)
    await undo()
    await until(async () => (await guides()).length === 1)
    step('and undo brings it back', (await guides()).length === 1)
    await viewRow('view.clearGuides')
    await until(async () => (await guides()).length === 0)
    step('View ▸ 清除参考线 removes every guide', (await guides()).length === 0)
    await viewRow('view.rulers')
  }

  // 13. Preview resolution: Quarter from the status bar's control draws the picture in blocks,
  // Full from View ▸ Resolution draws it as before.
  {
    const area = async () => {
      const p = await at(560, 300)
      return (await page.screenshot({ clip: { x: Math.round(p.x), y: Math.round(p.y), width: 60, height: 40 } })).toString('base64')
    }
    const full = await area()
    await kit.getByTestId('motion-resolution').selectOption('4')
    let blocky = full
    await until(async () => (blocky = await area()) !== full)
    step('四分之一 from the status bar draws the composition at a quarter of its pixels', blocky !== full)
    await kit.getByTestId('motion-menu-View').click()
    await kit.locator('[data-kit-ui-id="menu:View/Resolution"]').hover()
    const quarterRow = await kit.locator('[role=menu] [data-testid="motion-cmd-view.res.quarter"]').count()
    await kit.locator('[role=menu] [data-testid="motion-cmd-view.res.full"]').first().click()
    let back = blocky
    await until(async () => (back = await area()) === full)
    step('View ▸ 分辨率 ▸ 完整 draws it in full again, and the menu hides the resolution in use', back === full && quarterRow === 0 && (await kit.getByTestId('motion-resolution').inputValue()) === '1')
  }

  // 14. Masks: the Pen (G) draws a closed mask on the selected layer (corners by clicks, a curve
  // by a drag, closed on the first point), the Selection tool drags its vertex, and the Properties
  // panel sets its mode and Inverted; each one undo step.
  {
    await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'layer.newSolid', params: { name: 'Mask test', width: 300, height: 200, color: '#33aa66' } }], false))
    await until(async () => (await layers()).some((each) => each.name === 'Mask test'))
    const MASKED = (await layers()).find((each) => each.name === 'Mask test').id
    await kit.getByTestId(`motion-layer-${MASKED}`).click()
    await until(async () => same(await selection(), [MASKED]))
    const masksOf = async () => {
      const tree = await query('layer.tree', { layer: MASKED })
      const group = tree.properties.children.find((child) => child.match === 'masks')
      return (group?.children ?? []).map((mask) => ({ uid: mask.uid, path: mask.path, mode: mask.mask?.mode, inverted: mask.mask?.inverted, shape: mask.children.find((child) => child.match === 'path')?.value?.v }))
    }
    const rail = await kit.locator('[data-testid^="motion-tool-"]').evaluateAll((els) => els.map((e) => e.getAttribute('data-testid')))
    step('the rail has the Pen', rail.includes('motion-tool-pen'), rail.join(' '))
    await focusEditor()
    await page.keyboard.press('KeyG')
    await until(async () => (await active()) === 'pen')
    step('G picks the Pen', (await active()) === 'pen', await active())
    // The solid spans (490, 260)–(790, 460) around the composition's centre (640, 360).
    await click(540, 300)
    await click(740, 300)
    await drag([740, 420], [770, 440])
    await click(540, 420)
    if (shots) await page.screenshot({ path: `${shots}phase6-mask-pen.png` })
    step('nothing is made before the path is closed', (await masksOf()).length === 0)
    await click(541, 301)
    await until(async () => (await masksOf()).length === 1)
    const made = (await masksOf())[0]
    step('a click on the first point closes the mask, with the dragged point curved', made?.shape?.closed === true && made.shape.vertices.length === 4 && Math.abs(made.shape.out_tangents[2][0]) > 5, JSON.stringify(made?.shape))
    await undo()
    await until(async () => (await masksOf()).length === 0)
    step('one undo takes the mask away', (await masksOf()).length === 0)
    await page.keyboard.press(`${primary}+Shift+KeyZ`)
    await until(async () => (await masksOf()).length === 1)
    // The Selection tool drags the mask's vertex, not the layer.
    await kit.getByTestId('motion-tool-selection').click()
    const position = await value(MASKED, 'transform/position')
    const first = (await masksOf())[0].shape.vertices[1]
    const layerLeft = position[0] - 150
    const layerTop = position[1] - 100
    await drag([layerLeft + first[0], layerTop + first[1]], [layerLeft + first[0] + 30, layerTop + first[1] + 20])
    await until(async () => (await masksOf())[0].shape.vertices[1][0] > first[0] + 10)
    const after = (await masksOf())[0].shape.vertices[1]
    step('the Selection tool drags a mask vertex, the layer stays', after[0] > first[0] + 10 && nearAll(await value(MASKED, 'transform/position'), position), `${JSON.stringify(first)} → ${JSON.stringify(after)}`)
    await undo()
    await until(async () => near((await masksOf())[0].shape.vertices[1][0], first[0]))
    step('one undo puts the vertex back', near((await masksOf())[0].shape.vertices[1][0], first[0]))
    // The Properties panel: the mask's mode and Inverted.
    const id = (await masksOf())[0].path.replaceAll('/', '.')
    const toggle = kit.getByTestId(`motion-group-toggle-${id}`)
    await toggle.waitFor({ timeout: 5000 }).catch(() => null)
    if ((await toggle.count()) && (await toggle.getAttribute('aria-expanded')) === 'false') await toggle.click()
    const modeSelect = kit.getByTestId(`motion-mask-mode-${id}`)
    await modeSelect.waitFor({ timeout: 5000 }).catch(() => null)
    const modes = (await modeSelect.count()) ? await modeSelect.locator('option').allTextContents() : []
    step('the mask’s modes read in Chinese', modes.includes('相减') && modes.includes('相加'), modes.join(' '))
    if (modes.length) await modeSelect.selectOption('Subtract')
    await until(async () => (await masksOf())[0].mode === 'Subtract')
    step('choosing 相减 sets the mask to Subtract', (await masksOf())[0].mode === 'Subtract')
    await kit.getByTestId(`motion-mask-invert-${id}`).click()
    await until(async () => (await masksOf())[0].inverted === true)
    step('反转 inverts it', (await masksOf())[0].inverted === true)
    await undo()
    await undo()
    await until(async () => (await masksOf())[0].mode === 'Add' && (await masksOf())[0].inverted === false)
    step('two undos put mode and Inverted back', (await masksOf())[0].mode === 'Add' && (await masksOf())[0].inverted === false)
  }

  step('no page errors', errors.length === 0, errors.join(' | '))
} catch (error) {
  step('ran to the end', false, error instanceof Error ? error.stack : String(error))
} finally {
  await browser.close()
  await server.close()
}
console.log(failures ? `${failures} failed` : 'all passed')
process.exit(failures ? 1 : 0)
