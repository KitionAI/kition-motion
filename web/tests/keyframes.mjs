// Keyframe assistants by hand, in a real browser through the development host: node tests/keyframes.mjs [chromium|webkit]
// A double-click on a keyframe types its value and time in; its right-click menu toggles Hold and
// Rove Across Time (showing when it is on), reverses keyframes in time and selects the previous,
// following and equal ones; Animation ▸ Keyframe Assistant runs Exponential Scale and Convert
// Expression to Keyframes; J and K go to the previous and next keyframe. Each edit is undone.
// Results are read from the engine (`prop.get`, `keys.info`). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const POSITION = 'transform/position'
const near = (a, b, eps = 1e-3) => Math.abs(a - b) <= eps
const prop = (path) => h.query('prop.get', { layer: SHAPE, path })
const keys = async (path) => (await prop(path)).keys ?? []
const times = async (path) => (await keys(path)).map((key) => Math.round((key.compTime ?? key.time) * 1000) / 1000)
const mark = (ms, path = 'transform.position') => kit.getByTestId(`motion-key-${SHAPE}-${path}-${ms}`)
const selectedMarks = () => kit.locator('.motion-key[aria-selected="true"]').count()
const roving = async (time) => {
  const uid = (await prop(POSITION)).uid
  return (await h.query('keys.info', { keys: [{ layer: SHAPE, prop: uid, time }] }))[0]?.roving
}
/** Opens a keyframe's right-click menu and picks a row. */
async function keyMenu(ms, row) {
  await mark(ms).click({ button: 'right' })
  await kit.getByTestId('motion-key-menu').waitFor()
  await kit.getByTestId(`motion-key-menu-${row}`).click()
}
/** Animation ▸ Keyframe Assistant ▸ a row. */
async function assistant(command) {
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
  await kit.locator(`[role=menu] [data-testid="motion-cmd-${command}"]`).first().click()
}

try {
  await h.open()
  // A third Position keyframe at 1 s, so the middle one may rove.
  await h.setUp([{ command: 'prop.set', params: { layer: SHAPE, path: POSITION, value: [500, 300], time: 1 } }])
  await h.until(async () => (await times(POSITION)).includes(1))
  await kit.getByTestId(`motion-layer-${SHAPE}`).click()
  await page.keyboard.press('KeyP')
  await mark(1000).waitFor()

  // 1. A double-click on a keyframe: its value and time typed in, one undo step.
  await mark(1000).dblclick()
  await kit.getByTestId('motion-key-dialog').waitFor()
  h.step('a double-click on a keyframe opens its value, in Chinese', (await kit.getByTestId('motion-key-dialog').textContent()).includes('关键帧'))
  h.step('with its value and time', (await kit.getByTestId('motion-key-value-0').inputValue()) === '500' && /0?0:00:01:00$/.test(await kit.getByTestId('motion-key-time').inputValue()), await kit.getByTestId('motion-key-time').inputValue())
  await kit.getByTestId('motion-key-value-0').fill('420')
  await kit.getByTestId('motion-key-time').fill('0:00:01:15')
  await page.keyboard.press('Enter')
  await h.until(async () => (await times(POSITION)).includes(1.5))
  const moved = (await keys(POSITION)).find((key) => near(key.compTime ?? key.time, 1.5))
  h.step('the keyframe moves to 1.5 s with x 420', moved && near(moved.value[0], 420) && near(moved.value[1], 300), JSON.stringify(moved?.value))
  await h.undo()
  await h.until(async () => (await times(POSITION)).includes(1))
  const back = (await keys(POSITION)).find((key) => near(key.compTime ?? key.time, 1))
  h.step('undo puts it back at 1 s with x 500', back && near(back.value[0], 500), JSON.stringify(back?.value))

  // 2. Rove Across Time from the keyframe's menu: on, shown on, and undone.
  await mark(1000).click()
  await keyMenu(1000, 'rove')
  await h.until(async () => (await roving(1)) === true)
  h.step('Rove Across Time makes the middle keyframe rove', (await roving(1)) === true)
  // A roving keyframe's time follows the path: its mark is the selected one now.
  await kit.locator('.motion-key[aria-selected="true"]').first().click({ button: 'right' })
  await kit.getByTestId('motion-key-menu-rove').waitFor()
  await h.until(async () => (await kit.getByTestId('motion-key-menu-rove').textContent()).includes('已开'))
  h.step('its menu shows it is on', (await kit.getByTestId('motion-key-menu-rove').textContent()).includes('已开'), await kit.getByTestId('motion-key-menu-rove').textContent())
  await page.keyboard.press('Escape')
  await h.undo()
  await h.until(async () => (await roving(1)) === false)
  h.step('undo stops it roving', (await roving(1)) === false)

  // 3. Toggle Hold from the menu.
  await mark(0).click()
  await keyMenu(0, 'toggle-hold')
  await h.until(async () => (await keys(POSITION))[0]?.out === 'Hold')
  h.step('Toggle Hold Keyframe holds it', (await keys(POSITION))[0]?.out === 'Hold')
  await h.undo()
  await h.until(async () => (await keys(POSITION))[0]?.out !== 'Hold')
  h.step('undo takes the hold away', (await keys(POSITION))[0]?.out !== 'Hold')

  // 4. Select Following and Previous Keyframes.
  await mark(1000).click()
  await h.until(async () => (await selectedMarks()) === 1)
  await keyMenu(1000, 'select-following')
  await h.until(async () => (await selectedMarks()) === 2)
  h.step('Select Following Keyframes adds the one after', (await selectedMarks()) === 2 && (await mark(2500).getAttribute('aria-selected')) === 'true')
  await mark(1000).click()
  await h.until(async () => (await selectedMarks()) === 1)
  await keyMenu(1000, 'select-previous')
  await h.until(async () => (await selectedMarks()) === 2)
  h.step('Select Previous Keyframes adds the one before', (await selectedMarks()) === 2 && (await mark(0).getAttribute('aria-selected')) === 'true')

  // 5. Time-Reverse Keyframes: the values run backwards.
  const values = (await keys(POSITION)).map((key) => key.value[0])
  await mark(0).click()
  await mark(2500).click({ modifiers: ['Shift'] })
  await mark(1000).click({ modifiers: ['Shift'] })
  await h.until(async () => (await selectedMarks()) === 3)
  await keyMenu(1000, 'reverse')
  await h.until(async () => near((await keys(POSITION))[0].value[0], values[2]))
  h.step('Time-Reverse Keyframes reverses their values', (await keys(POSITION)).map((key) => key.value[0]).join() === [...values].reverse().join(), (await keys(POSITION)).map((key) => key.value[0]).join())
  await h.undo()
  await h.until(async () => near((await keys(POSITION))[0].value[0], values[0]))
  h.step('undo puts them back', (await keys(POSITION)).map((key) => key.value[0]).join() === values.join())

  // 6. J and K: the previous and next keyframe of the selected layer.
  await h.setUp([{ command: 'time.set', params: { time: 0 } }])
  await h.until(async () => near((await h.details()).time, 0))
  await h.focusEditor()
  await page.keyboard.press('KeyK')
  await h.until(async () => near((await h.details()).time, 1, 0.02))
  h.step('K goes to the next keyframe', near((await h.details()).time, 1, 0.02), String((await h.details()).time))
  await page.keyboard.press('KeyK')
  await h.until(async () => (await h.details()).time > 1.1)
  await page.keyboard.press('KeyJ')
  await h.until(async () => near((await h.details()).time, 1, 0.02))
  h.step('J goes back to the previous one', near((await h.details()).time, 1, 0.02), String((await h.details()).time))

  // 7. Exponential Scale: two Scale keyframes become one per frame between them.
  await h.setUp([
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/scale', value: [100, 100], time: 0 } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/scale', value: [400, 400], time: 0.5 } },
  ])
  await h.until(async () => (await keys('transform/scale')).length === 2)
  await kit.getByTestId(`motion-layer-${SHAPE}`).click()
  await page.keyboard.press('KeyS')
  await mark(0, 'transform.scale').waitFor()
  await mark(0, 'transform.scale').click()
  await mark(500, 'transform.scale').click({ modifiers: ['Shift'] })
  await h.until(async () => (await selectedMarks()) === 2)
  await assistant('keys.exponentialScale')
  await h.until(async () => (await keys('transform/scale')).length > 2)
  const scaled = await keys('transform/scale')
  const mid = scaled.find((key) => near(key.compTime ?? key.time, 0.25, 0.02))
  h.step('Animation ▸ 关键帧辅助 ▸ 指数比例 keys every frame', scaled.length >= 15, String(scaled.length))
  const midTime = mid ? (mid.compTime ?? mid.time) : 0
  h.step('growing by the same factor each frame (100 × 4^(t / 0.5))', mid && near(mid.value[0], 100 * 4 ** (midTime / 0.5), 0.5), JSON.stringify([midTime, mid?.value]))
  await h.undo()
  await h.until(async () => (await keys('transform/scale')).length === 2)
  h.step('undo leaves the two keyframes', (await keys('transform/scale')).length === 2)

  // 8. Convert Expression to Keyframes: the property selected (a click on its name) gets keyframes.
  await h.setUp([{ command: 'prop.setExpression', params: { layer: SHAPE, path: 'transform/rotation', expression: 'time * 90' } }])
  await kit.getByTestId(`motion-layer-${SHAPE}`).click()
  await page.keyboard.press('KeyR')
  const name = kit.getByTestId(`motion-prop-name-${SHAPE}-transform.rotation`)
  await name.waitFor()
  await name.click()
  await h.until(async () => (await h.layer(SHAPE))?.selectedProps?.includes('transform/rotation'))
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-control-convert-expression"]').first().click()
  await h.until(async () => (await keys('transform/rotation')).length > 2)
  const rotation = await prop('transform/rotation')
  const at1 = rotation.keys.find((key) => near(key.compTime ?? key.time, 1, 0.02))
  h.step('Convert Expression to Keyframes keys what the expression gave', rotation.keys.length > 2 && at1 && near(at1.value, 90, 0.5), JSON.stringify(at1))
  await h.undo()
  await h.until(async () => (await keys('transform/rotation')).length === 0)
  h.step('undo gives the expression back', (await keys('transform/rotation')).length === 0 && (await prop('transform/rotation')).expression === 'time * 90')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
