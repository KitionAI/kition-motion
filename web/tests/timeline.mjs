// The timeline by hand, in a real browser through the development host: node tests/timeline.mjs [chromium|webkit]
// Every step drives the editor with the pointer and the keyboard (page.mouse, page.keyboard) and
// checks the result in the engine (ai.inspect, read-only ai.run queries). Prints PASS or FAIL per step.
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
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

  // 2. Rows: reorder, switches, rename, menu, delete, duplicate, deselect.
  await open()
  const top = await box(`motion-layer-${TEXT}`)
  const night = await box(`motion-name-${SOLID}`)
  const line = await drag({ x: night.x + 10, y: night.y + night.height / 2 }, { x: night.x + 10, y: top.y + 4 }, async () => kit.getByTestId('motion-drop-line').isVisible())
  await until(async () => (await layer(SOLID)).index === 1)
  step('dragging a row reorders the layer, with a drop line', (await layer(SOLID)).index === 1 && line === true, `index ${(await layer(SOLID)).index}, line ${line}`)
  await kit.getByTestId(`motion-switch-video-${SHAPE}`).click()
  await kit.getByTestId(`motion-switch-solo-${SHAPE}`).click()
  await kit.getByTestId(`motion-switch-lock-${SHAPE}`).click()
  await until(async () => (await layer(SHAPE)).switches.locked)
  shape = await layer(SHAPE)
  step('the eye, solo and lock switches toggle', !shape.switches.video && shape.switches.solo && shape.switches.locked, JSON.stringify({ video: shape.switches.video, solo: shape.switches.solo, locked: shape.switches.locked }))
  await kit.getByTestId(`motion-switch-lock-${SHAPE}`).click()
  await until(async () => !(await layer(SHAPE)).switches.locked)
  step('lock toggles back', !(await layer(SHAPE)).switches.locked)
  await kit.getByTestId(`motion-name-${TEXT}`).dblclick()
  await kit.getByTestId(`motion-rename-${TEXT}`).waitFor()
  await page.keyboard.press(`${primary}+KeyA`)
  await page.keyboard.type('Title')
  await page.keyboard.press('Enter')
  await until(async () => (await layer(TEXT)).name === 'Title')
  step('double-clicking a name renames the layer (Enter commits)', (await layer(TEXT)).name === 'Title', (await layer(TEXT)).name)
  await kit.getByTestId(`motion-name-${TEXT}`).dblclick()
  await page.keyboard.type('Nope')
  await page.keyboard.press('Escape')
  await page.waitForTimeout(300)
  step('Escape cancels a rename', (await layer(TEXT)).name === 'Title' && (await kit.getByTestId(`motion-rename-${TEXT}`).count()) === 0, (await layer(TEXT)).name)
  const count = (await layers()).length
  await kit.getByTestId(`motion-name-${SHAPE}`).click({ button: 'right' })
  await kit.getByTestId('motion-layer-menu').waitFor()
  step('right-clicking a row opens the layer menu and selects the layer', await kit.getByTestId('motion-layer-menu').isVisible() && (await layer(SHAPE)).selected)
  await kit.getByTestId('motion-layer-menu-duplicate').click()
  await until(async () => (await layers()).length === count + 1)
  step('the menu duplicates the layer', (await layers()).length === count + 1, String((await layers()).length))
  await seek(1)
  await kit.getByTestId(`motion-name-${SOLID}`).click({ button: 'right' })
  await kit.getByTestId('motion-layer-menu-split').click()
  await until(async () => (await layers()).length === count + 2)
  step('the menu splits the layer at the current time', (await layers()).length === count + 2 && near((await layer(SOLID)).out, 1) !== near((await layer(SOLID)).in, 1), JSON.stringify(await layer(SOLID).then((each) => [each.in, each.out])))
  await kit.getByTestId(`motion-name-${SHAPE}`).click()
  await until(async () => (await layers()).filter((each) => each.selected).length === 1)
  await page.keyboard.press('Backspace')
  await until(async () => !(await layer(SHAPE)))
  step('Backspace deletes the selected layer', !(await layer(SHAPE)))
  await page.keyboard.press(`${primary}+KeyZ`)
  await until(async () => !!(await layer(SHAPE)))
  const restored = await layer(SHAPE)
  const shownSelected = (await kit.getByTestId(`motion-layer-${SHAPE}`).getAttribute('data-selected')) === 'true'
  step('undo brings the deleted layer back, and its row shows its selection as the engine reports it', !!restored && shownSelected === restored.selected, `selected ${restored?.selected}, row ${shownSelected}`)
  await kit.getByTestId(`motion-name-${SHAPE}`).click()
  await until(async () => (await layer(SHAPE)).selected)
  const before = (await layers()).length
  await page.keyboard.press(`${primary}+KeyD`)
  await until(async () => (await layers()).length === before + 1)
  step('Cmd+D duplicates the selected layer', (await layers()).length === before + 1)
  const rows = await box('motion-timeline-rows')
  await page.mouse.click(rows.x + rows.width - 40, rows.y + rows.height - 6)
  await until(async () => (await layers()).every((each) => !each.selected))
  step('a click on empty timeline deselects', (await layers()).every((each) => !each.selected))

  // 3. Twirl-down: the property tree, values, add key, U and P.
  await open()
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).waitFor()
  const tree = {
    transform: await kit.getByTestId(`motion-group-${SHAPE}-transform`).count(),
    position: await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count(),
    orientation: await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.orientation`).count(),
    material: await kit.getByTestId(`motion-group-${SHAPE}-materialOptions`).count(),
    keys: await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).locator('.motion-key').count(),
  }
  step('the twirl shows Transform open, no 3D-only rows on a 2D layer, and the keyframes on the property row', tree.transform === 1 && tree.position === 1 && tree.orientation === 0 && tree.material === 0 && tree.keys === 2, JSON.stringify(tree))
  await kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.scale`).click()
  await until(async () => (await times(SHAPE, 'transform/scale')).length === 1)
  step('the diamond on a property row adds a keyframe at the current time', near((await times(SHAPE, 'transform/scale'))[0] ?? -1, 0))
  await kit.getByTestId(`motion-row-value-${SHAPE}-transform.rotation-0`).fill('45')
  await kit.getByTestId(`motion-row-value-${SHAPE}-transform.rotation-0`).press('Enter')
  await until(async () => (await query('prop.get', { layer: SHAPE, path: 'transform/rotation' })).value === 45)
  step('a property row edits its value', (await query('prop.get', { layer: SHAPE, path: 'transform/rotation' })).value === 45)
  await kit.getByTestId(`motion-twirl-${SHAPE}-transform`).click()
  await until(async () => (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count()) === 0)
  step('a group closes', (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count()) === 0)
  await kit.getByTestId(`motion-name-${SHAPE}`).click()
  await until(async () => (await layer(SHAPE)).selected)
  await page.keyboard.press('u')
  await until(async () => (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count()) === 1)
  const animated = {
    position: await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count(),
    scale: await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.scale`).count(),
    anchor: await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.anchor`).count(),
    groups: await kit.getByTestId(`motion-layer-block-${SHAPE}`).locator('[data-testid^="motion-group-"]').count(),
  }
  step('U shows only the animated properties of the selected layer', animated.position === 1 && animated.scale === 1 && animated.anchor === 0 && animated.groups === 0, JSON.stringify(animated))
  await page.keyboard.press('p')
  await until(async () => (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.scale`).count()) === 0)
  step('P shows only Position', (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count()) === 1 && (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.scale`).count()) === 0)
  await page.keyboard.press('p')
  await until(async () => (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count()) === 0)
  step('P again closes it', (await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).count()) === 0)

  // 4. Keyframes: select, add to the selection, drag, delete, copy and paste, marquee, J and K, F9, the collapsed row.
  await open()
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  const first = kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`)
  const last = kit.getByTestId(`motion-key-${SHAPE}-transform.position-2500`)
  await first.waitFor()
  await first.click()
  await until(async () => (await first.getAttribute('aria-selected')) === 'true')
  step('a click selects a keyframe, drawn selected', (await first.getAttribute('aria-selected')) === 'true' && (await last.getAttribute('aria-selected')) === 'false')
  await last.click({ modifiers: ['Shift'] })
  await until(async () => (await last.getAttribute('aria-selected')) === 'true')
  step('Shift+click adds a keyframe to the selection', (await first.getAttribute('aria-selected')) === 'true' && (await last.getAttribute('aria-selected')) === 'true')
  let lane = await track(`motion-prop-row-${SHAPE}-transform.position`)
  await drag({ x: lane.at(2.5), y: lane.cy }, { x: lane.at(2.7), y: lane.cy })
  await until(async () => near((await times(SHAPE, 'transform/position'))[0], 0.2))
  let position = await times(SHAPE, 'transform/position')
  step('dragging a selected keyframe moves the selection together, on frames', near(position[0], 0.2) && near(position[1], 2.7), JSON.stringify(position))
  await page.keyboard.press(`${primary}+KeyZ`)
  await until(async () => near((await times(SHAPE, 'transform/position'))[0], 0))
  step('one undo takes the keyframe drag back', JSON.stringify(await times(SHAPE, 'transform/position')) === '[0,2.5]', JSON.stringify(await times(SHAPE, 'transform/position')))
  await first.click()
  await until(async () => (await last.getAttribute('aria-selected')) === 'false')
  await page.keyboard.press('Delete')
  await until(async () => (await times(SHAPE, 'transform/position')).length === 1)
  step('Delete deletes the selected keyframe, not its layer', (await times(SHAPE, 'transform/position')).length === 1 && !!(await layer(SHAPE)), JSON.stringify(await times(SHAPE, 'transform/position')))
  await page.keyboard.press(`${primary}+KeyZ`)
  await until(async () => (await times(SHAPE, 'transform/position')).length === 2)
  await last.click()
  await until(async () => (await last.getAttribute('aria-selected')) === 'true')
  await page.keyboard.press(`${primary}+KeyC`)
  await seek(1)
  await page.keyboard.press(`${primary}+KeyV`)
  await until(async () => (await times(SHAPE, 'transform/position')).length === 3)
  position = await times(SHAPE, 'transform/position')
  step('Cmd+C and Cmd+V paste the keyframe at the playhead', position.length === 3 && position.some((time) => near(time, 1)), JSON.stringify(position))
  await page.keyboard.press('k')
  await until(async () => near((await details()).time, 2.5, 0.02))
  step('K jumps to the next keyframe', near((await details()).time, 2.5, 0.02), String((await details()).time))
  await page.keyboard.press('j')
  await until(async () => near((await details()).time, 1, 0.02))
  step('J jumps to the previous keyframe', near((await details()).time, 1, 0.02), String((await details()).time))
  // A marquee from the empty Anchor Point track down across Position's keyframes at 0 and 1.
  const anchor = await track(`motion-prop-row-${SHAPE}-transform.anchor`)
  lane = await track(`motion-prop-row-${SHAPE}-transform.position`)
  await drag({ x: anchor.at(0) + 4, y: anchor.cy }, { x: lane.at(1.5), y: lane.cy + 6 })
  const marked = kit.getByTestId(`motion-key-${SHAPE}-transform.position-1000`)
  await until(async () => (await marked.getAttribute('aria-selected')) === 'true')
  const marquee = { at1: await marked.getAttribute('aria-selected'), at25: await last.getAttribute('aria-selected') }
  step('a marquee on empty track selects the keyframes inside it', marquee.at1 === 'true' && marquee.at25 === 'false', JSON.stringify(marquee))
  await page.keyboard.press('F9')
  // prop.get: keys.info with a property path for `prop` answers nothing (it finds keys by uid only).
  const keyAt = async (time) => (await query('prop.get', { layer: SHAPE, path: 'transform/position' })).keys.find((key) => near(key.compTime, time))
  await until(async () => (await keyAt(1))?.in === 'Bezier')
  const eased = await keyAt(1)
  step('F9 eases the selected keyframes', eased?.in === 'Bezier' && eased?.out === 'Bezier' && (await keyAt(2.5))?.in === 'Linear', JSON.stringify({ in: eased?.in, out: eased?.out }))
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  const collapsed = kit.getByTestId(`motion-key-${SHAPE}-2500`)
  await collapsed.waitFor()
  await collapsed.click()
  await until(async () => (await collapsed.getAttribute('aria-selected')) === 'true')
  step("the collapsed layer row shows its keyframes and selects them", (await collapsed.getAttribute('aria-selected')) === 'true')
  const own = await track(`motion-layer-${SHAPE}`)
  await drag({ x: own.at(2.5), y: own.cy }, { x: own.at(2.8), y: own.cy })
  await until(async () => (await times(SHAPE, 'transform/position')).some((time) => near(time, 2.8)))
  step('and drags them', (await times(SHAPE, 'transform/position')).some((time) => near(time, 2.8)) && near((await layer(SHAPE)).in, 0), JSON.stringify(await times(SHAPE, 'transform/position')))

  // 5. Time navigation: zoom with = and - and Cmd+wheel, scroll when zoomed, ruler ticks, the work area.
  await open()
  const timeline = kit.getByTestId('motion-timeline')
  const view = async () => (await timeline.getAttribute('data-view')).split(' ').map(Number)
  const ruler = kit.getByTestId('motion-ruler')
  const fitStep = Number(await ruler.getAttribute('data-step-seconds'))
  step('the whole composition is in view, no scrollbar', (await view())[1] === 3 && (await kit.getByTestId('motion-timeline-scroll').count()) === 0, `${JSON.stringify(await view())}, ticks every ${fitStep} s`)
  await kit.getByTestId(`motion-name-${SHAPE}`).click()
  for (let i = 0; i < 4; i += 1) await page.keyboard.press('Equal')
  await until(async () => (await view())[1] < 1)
  const zoomedStep = Number(await ruler.getAttribute('data-step-seconds'))
  step('= zooms in and the ruler ticks get finer, in frames', (await view())[1] < 1 && zoomedStep < fitStep && (await ruler.getAttribute('data-step')) === 'frames', `${JSON.stringify(await view())}, ticks every ${zoomedStep} s (${fitStep} s at fit)`)
  step('a scrollbar appears while zoomed', (await kit.getByTestId('motion-timeline-scroll').count()) === 1)
  const rowsBox = await box('motion-timeline-rows')
  await page.mouse.move(rowsBox.x + rowsBox.width / 2, rowsBox.y + 60)
  const scrolled = await view()
  await page.mouse.wheel(300, 0)
  await until(async () => (await view())[0] > scrolled[0])
  step('a sideways wheel scrolls the zoomed timeline', (await view())[0] > scrolled[0], `${JSON.stringify(scrolled)} → ${JSON.stringify(await view())}`)
  for (let i = 0; i < 6; i += 1) await page.keyboard.press('Minus')
  await until(async () => (await view())[1] === 3)
  step('- zooms out to the whole composition', (await view())[1] === 3, JSON.stringify(await view()))
  await page.keyboard.down(primary)
  await page.mouse.wheel(0, -200)
  await page.keyboard.up(primary)
  await until(async () => (await view())[1] < 3)
  const wheeled = (await view())[1]
  step('Cmd+wheel over the timeline zooms it, gently', wheeled < 3 && wheeled > 1, JSON.stringify(await view()))
  for (let i = 0; i < 6; i += 1) await page.keyboard.press('Minus')
  await until(async () => (await view())[1] === 3)
  const end = await box('motion-work-end')
  const rulerBox = await box('motion-ruler')
  await drag({ x: end.x + end.width / 2, y: end.y + end.height / 2 }, { x: rulerBox.x + (2 * rulerBox.width) / 3, y: end.y + end.height / 2 })
  await until(async () => near((await details()).info.workArea[1], 2))
  step('dragging the work area end on the ruler sets it', near((await details()).info.workArea[1], 2), JSON.stringify((await details()).info.workArea))
  const workStart = await box('motion-work-start')
  await drag({ x: workStart.x + workStart.width / 2, y: workStart.y + workStart.height / 2 }, { x: rulerBox.x + (0.5 * rulerBox.width) / 3, y: workStart.y + workStart.height / 2 })
  await until(async () => near((await details()).info.workArea[0], 0.5))
  step('and its start', near((await details()).info.workArea[0], 0.5) && near((await details()).info.workArea[1], 2), JSON.stringify((await details()).info.workArea))
} catch (error) {
  failures += 1
  console.log(`FAIL ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
