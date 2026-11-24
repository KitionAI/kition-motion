// Editing several layers at once, by hand, in a real browser through the development host: node tests/multiEdit.mjs [chromium|webkit]
// With two layers selected (Cmd-click in the timeline) the Properties panel shows what they have in
// common: Position empty with a dash where they differ, Opacity's shared value; a typed Opacity goes
// to both (one undo step), a scrubbed Position X moves both by the same amount (one undo step), and
// the blending mode menu sets both. Results are read with `prop.get` and `comp.info`. Prints PASS
// or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const NIGHT = 4
const near = (a, b, eps = 1e-3) => Math.abs(a - b) <= eps
const value = async (layer, path) => (await h.query('prop.get', { layer, path })).value
const both = async (path) => [await value(SHAPE, path), await value(NIGHT, path)]

try {
  await h.open()
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  await page.keyboard.down(h.primary)
  await kit.getByTestId(`motion-layer-${NIGHT}`).locator('.motion-row-name').click()
  await page.keyboard.up(h.primary)
  await h.until(async () => (await h.layer(SHAPE))?.selected && (await h.layer(NIGHT))?.selected)
  await kit.getByTestId('motion-multi-opacity-0').waitFor()
  h.step('two layers selected: the panel says it edits both, in Chinese', ((await kit.getByTestId('motion-properties-many').textContent()) ?? '').includes('2 个图层'), await kit.getByTestId('motion-properties-many').textContent())
  const positions = await both('transform/position')
  const x = kit.getByTestId('motion-multi-position-0')
  h.step('Position X differs: empty, with a dash, marked mixed', (await x.getAttribute('data-mixed')) === 'true' && (await x.inputValue()) === '' && (await x.getAttribute('placeholder')) === '—', JSON.stringify(positions))
  const opacity = kit.getByTestId('motion-multi-opacity-0')
  h.step('Opacity is shared: its value shows', (await opacity.getAttribute('data-mixed')) === null && (await opacity.inputValue()) === '100')
  h.step('nothing took the focus', await page.evaluate(() => !(document.getElementById('kit').contentDocument.activeElement instanceof HTMLInputElement)))

  // 1. A typed Opacity goes to both, one undo step.
  await opacity.click()
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('40')
  await page.keyboard.press('Enter')
  await h.until(async () => (await both('transform/opacity')).every((each) => near(each, 40)))
  h.step('a typed Opacity sets both layers', (await both('transform/opacity')).every((each) => near(each, 40)), JSON.stringify(await both('transform/opacity')))
  await h.undo()
  await h.until(async () => (await both('transform/opacity')).every((each) => near(each, 100)))
  h.step('one undo puts both back', (await both('transform/opacity')).every((each) => near(each, 100)), JSON.stringify(await both('transform/opacity')))

  // 2. Position X scrubbed: each moves by the same amount from where it was.
  await x.scrollIntoViewIfNeeded()
  const box = await x.boundingBox()
  await h.drag({ x: box.x + box.width / 2, y: box.y + box.height / 2 }, { x: box.x + box.width / 2 + 40, y: box.y + box.height / 2 }, 8)
  await h.until(async () => !near((await value(SHAPE, 'transform/position'))[0], positions[0][0]))
  await page.waitForTimeout(300)
  const moved = await both('transform/position')
  const dShape = moved[0][0] - positions[0][0]
  const dNight = moved[1][0] - positions[1][0]
  h.step('scrubbing Position X moves both by the same amount', dShape > 10 && near(dShape, dNight, 1e-6) && near(moved[0][1], positions[0][1]) && near(moved[1][1], positions[1][1]), JSON.stringify({ dShape, dNight }))
  await h.undo()
  await h.until(async () => near((await value(SHAPE, 'transform/position'))[0], positions[0][0]) && near((await value(NIGHT, 'transform/position'))[0], positions[1][0]))
  h.step('one undo puts both back', near((await value(SHAPE, 'transform/position'))[0], positions[0][0]) && near((await value(NIGHT, 'transform/position'))[0], positions[1][0]))

  // 3. A typed Position X sets both to it.
  await x.click()
  await page.keyboard.type('500')
  await page.keyboard.press('Enter')
  await h.until(async () => (await both('transform/position')).every((each) => near(each[0], 500)))
  h.step('a typed Position X sets both to it, keeping their Y', (await both('transform/position')).every((each, at) => near(each[0], 500) && near(each[1], positions[at][1])))
  await h.until(async () => (await x.inputValue()) === '500')
  h.step('the field then shows the shared value', (await x.inputValue()) === '500' && (await x.getAttribute('data-mixed')) === null)
  await h.undo()
  await h.until(async () => near((await value(SHAPE, 'transform/position'))[0], positions[0][0]))

  // 4. The blending mode menu sets both.
  await kit.getByTestId('motion-multi-blend-mode').selectOption('Screen')
  const modes = async () => (await h.layers()).filter((layer) => layer.id === SHAPE || layer.id === NIGHT).map((layer) => layer.blendMode)
  await h.until(async () => (await modes()).every((mode) => mode === 'Screen'))
  h.step('the blending mode menu sets both', (await modes()).every((mode) => mode === 'Screen'), JSON.stringify(await modes()))
  await h.undo()
  await h.until(async () => (await modes()).every((mode) => mode !== 'Screen'))
  h.step('undone', (await modes()).every((mode) => mode !== 'Screen'))
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
