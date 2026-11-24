// The timeline's switch and mode columns by hand, in a real browser through the development host:
// node tests/switches.mjs [chromium|webkit]
// A click on a layer's switch toggles it, a drag down a switch column sets every layer it passes
// (one undo step), the column headers turn the composition's switches on and off (Hide Shy Layers
// hides the shy layers' rows), the header's menu shows and hides columns (kept after a reload),
// and the blending mode, track matte and parent drop-downs set the layer's. Results are read from
// the engine (read-only queries). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const SHAPE = 27
const SOLID = 4
const cell = (column, layer) => kit.getByTestId(`motion-switch-${column}-${layer}`)
const pressed = async (locator) => (await locator.getAttribute('aria-pressed')) === 'true'
const compSwitches = async () => (await h.query('comp.info', {})).switches

try {
  await h.open()
  const order = (await h.layers()).map((layer) => layer.id)

  // 1. The columns: the A/V switches left of the names by default; the layer switches are shown
  // one by one from the header's Columns menu.
  const avOnly = (await cell('video', SHAPE).count()) === 1 && (await cell('shy', SHAPE).count()) === 0
  step('by default the A/V switches show and the layer switches wait in the Columns menu', avOnly)
  for (const id of ['shy', 'collapse', 'quality', 'effects', 'frameBlend', 'motionBlur', 'adjustment', 'threeD']) {
    if ((await cell(id, SHAPE).count()) === 1) continue
    await kit.getByTestId('motion-columns-button').click()
    await kit.getByTestId(`motion-column-toggle-${id}`).click()
    await until(async () => (await cell(id, SHAPE).count()) === 1, 30)
  }
  const columns = ['video', 'audio', 'solo', 'lock', 'shy', 'collapse', 'quality', 'effects', 'frameBlend', 'motionBlur', 'adjustment', 'threeD']
  const present = []
  for (const column of columns) if ((await cell(column, SHAPE).count()) === 1) present.push(column)
  step('the Columns menu shows all twelve switches', present.length === columns.length, present.join(' '))

  // 2. A click toggles one switch of one layer; undo puts it back.
  const shyBefore = (await h.layer(SHAPE)).switches.shy
  await cell('shy', SHAPE).click()
  await until(async () => (await h.layer(SHAPE)).switches.shy !== shyBefore)
  step('a click on the shy switch makes the layer shy', (await h.layer(SHAPE)).switches.shy === !shyBefore && (await pressed(cell('shy', SHAPE))) === !shyBefore)
  await cell('threeD', SHAPE).click()
  await until(async () => (await h.layer(SHAPE)).switches.three_d === true)
  step('a click on the 3D switch makes it a 3D layer', (await h.layer(SHAPE)).switches.three_d === true)
  await h.undo()
  await until(async () => (await h.layer(SHAPE)).switches.three_d === false)
  step('one undo takes the 3D switch back', (await h.layer(SHAPE)).switches.three_d === false)
  await h.undo()
  await until(async () => (await h.layer(SHAPE)).switches.shy === shyBefore)
  step('and another the shy switch', (await h.layer(SHAPE)).switches.shy === shyBefore)

  // 3. A drag down the motion blur column sets every layer it passes, in one undo step.
  const first = order[0]
  const last = order[order.length - 1]
  const from = await h.centre(cell('motionBlur', first))
  const to = await h.centre(cell('motionBlur', last))
  const blurBefore = (await h.layers()).map((layer) => layer.switches.motion_blur)
  await h.drag(from, to, Math.max(10, order.length * 4))
  await until(async () => (await h.layers()).every((layer) => layer.switches.motion_blur === !blurBefore[0]))
  const blurred = (await h.layers()).map((layer) => layer.switches.motion_blur)
  step(`a drag down the motion blur column sets all ${order.length} layers alike`, blurred.every((value) => value === !blurBefore[0]), JSON.stringify(blurred))
  await h.undo()
  await until(async () => JSON.stringify((await h.layers()).map((layer) => layer.switches.motion_blur)) === JSON.stringify(blurBefore))
  step('one undo takes the whole drag back', JSON.stringify((await h.layers()).map((layer) => layer.switches.motion_blur)) === JSON.stringify(blurBefore))

  // 4. The composition's switches in the column headers: Hide Shy Layers hides the shy rows.
  await cell('shy', SHAPE).click()
  await until(async () => (await h.layer(SHAPE)).switches.shy === true)
  await kit.getByTestId('motion-comp-switch-hideShy').click()
  await until(async () => (await compSwitches())?.hideShy === true)
  await until(async () => (await kit.getByTestId(`motion-layer-${SHAPE}`).count()) === 0)
  step('Hide Shy Layers in the header hides the shy layer’s row', (await compSwitches())?.hideShy === true && (await kit.getByTestId(`motion-layer-${SHAPE}`).count()) === 0 && (await kit.getByTestId(`motion-layer-${SOLID}`).count()) === 1)
  await kit.getByTestId('motion-comp-switch-hideShy').click()
  await until(async () => (await kit.getByTestId(`motion-layer-${SHAPE}`).count()) === 1)
  step('and again shows it', (await compSwitches())?.hideShy === false && (await kit.getByTestId(`motion-layer-${SHAPE}`).count()) === 1)
  const compBlur = (await compSwitches())?.motionBlur
  await kit.getByTestId('motion-comp-switch-motionBlur').click()
  await until(async () => (await compSwitches())?.motionBlur === !compBlur)
  step('the motion blur header turns the composition’s motion blur on or off', (await compSwitches())?.motionBlur === !compBlur && (await pressed(kit.getByTestId('motion-comp-switch-motionBlur'))) === !compBlur)

  // 5. From the keyboard: a switch takes the focus and Enter toggles it.
  const audioBefore = (await h.layer(SOLID)).switches.audio
  await cell('audio', SOLID).focus()
  await page.keyboard.press('Enter')
  await until(async () => (await h.layer(SOLID)).switches.audio === !audioBefore)
  step('Enter on a focused switch toggles it', (await h.layer(SOLID)).switches.audio === !audioBefore)

  // 6. The header's menu shows and hides columns; the choice stays after a reload.
  await kit.getByTestId('motion-column-header').click({ button: 'right', position: { x: 60, y: 10 } })
  await kit.getByTestId('motion-columns-menu').waitFor()
  await kit.getByTestId('motion-column-toggle-mode').click()
  await kit.getByTestId(`motion-cell-mode-${SHAPE}`).waitFor({ timeout: 3000 }).catch(() => null)
  step('the header’s right-click menu shows the Mode column', (await kit.getByTestId(`motion-cell-mode-${SHAPE}`).count()) === 1)
  await kit.getByTestId(`motion-cell-mode-${SHAPE}`).selectOption('Screen')
  await until(async () => (await h.layer(SHAPE)).blendMode === 'Screen')
  step('the Mode column sets the blending mode', (await h.layer(SHAPE)).blendMode === 'Screen', (await h.layer(SHAPE)).blendMode)
  await kit.getByTestId('motion-columns-button').click()
  await kit.getByTestId('motion-column-toggle-matte').click()
  await kit.getByTestId('motion-columns-button').click()
  await kit.getByTestId('motion-column-toggle-parent').click()
  await kit.getByTestId('motion-columns-button').click()
  await kit.getByTestId('motion-column-toggle-audio').click()
  await until(async () => (await cell('audio', SHAPE).count()) === 0)
  step('the Columns button’s menu shows Track Matte and Parent and hides Audio', (await kit.getByTestId(`motion-cell-matte-${SHAPE}`).count()) === 1 && (await kit.getByTestId(`motion-cell-parent-${SHAPE}`).count()) === 1 && (await cell('audio', SHAPE).count()) === 0)
  await kit.getByTestId(`motion-cell-matte-${SHAPE}`).selectOption(`lumaInverted:${SOLID}`)
  await until(async () => (await h.layer(SHAPE)).trackMatte?.layer === SOLID)
  const matte = (await h.layer(SHAPE)).trackMatte
  step('the Track Matte column sets an inverted luma matte', matte?.layer === SOLID && /luma/i.test(matte?.kind ?? '') && /invert/i.test(matte?.kind ?? ''), JSON.stringify(matte))
  await kit.getByTestId(`motion-cell-parent-${SHAPE}`).selectOption(String(SOLID))
  await until(async () => (await h.layer(SHAPE)).parent === SOLID)
  step('the Parent column sets the parent', (await h.layer(SHAPE)).parent === SOLID)
  await h.open()
  step('the columns shown stay after a reload', (await kit.getByTestId(`motion-cell-mode-${SHAPE}`).count()) === 1 && (await cell('audio', SHAPE).count()) === 0)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
