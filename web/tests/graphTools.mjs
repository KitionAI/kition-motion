// The graph editor's legend and the keyframe assistants by hand, in a real browser through the development host: node tests/graphTools.mjs [chromium|webkit]
// The legend's eye hides a curve and shows it again, its solo shows one curve alone; Animation ▸
// Keyframe Assistant ▸ Wiggler… adds random keys between two selected keyframes, and the
// keyframe menu's Smoother… takes keys off a straight stretch, each one undo step. Results are
// read from the engine (`prop.get`'s keys). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const POSITION = 'transform/position'
const keyCount = async () => ((await h.query('prop.get', { layer: SHAPE, path: POSITION })).keys ?? []).length
const curve = (path, dim) => kit.getByTestId(`motion-graph-curve-${SHAPE}-${path}-${dim}`)
const mark = (ms) => kit.getByTestId(`motion-key-${SHAPE}-transform.position-${ms}`)

try {
  await h.open()
  // Opacity animated too: two curves in the graph.
  await h.setUp([
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/opacity', value: 100, time: 0 } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/opacity', value: 20, time: 2 } },
  ])
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  await kit.getByTestId('motion-graph-toggle').click()
  await kit.getByTestId(`motion-graph-legend-${SHAPE}-transform.opacity`).waitFor()
  await curve('transform.opacity', 0).waitFor()
  h.step('the graph shows Position and Opacity', (await curve('transform.position', 0).count()) === 1 && (await curve('transform.opacity', 0).count()) === 1)

  // 1. The eye hides Opacity's curve; again shows it.
  const eye = kit.getByTestId(`motion-graph-eye-${SHAPE}-transform.opacity`)
  h.step('the eye says the curve shows', (await eye.getAttribute('aria-pressed')) === 'true' && ((await eye.getAttribute('title')) ?? '').includes('隐藏'), await eye.getAttribute('title'))
  await eye.click()
  await h.until(async () => (await curve('transform.opacity', 0).count()) === 0)
  h.step('a click on the eye hides Opacity’s curve, Position’s stays', (await curve('transform.opacity', 0).count()) === 0 && (await curve('transform.position', 0).count()) === 1 && (await eye.getAttribute('aria-pressed')) === 'false')
  await eye.click()
  await h.until(async () => (await curve('transform.opacity', 0).count()) === 1)
  h.step('another shows it again', (await curve('transform.opacity', 0).count()) === 1)

  // 2. Solo: Position alone, then every curve again.
  const solo = kit.getByTestId(`motion-graph-solo-${SHAPE}-transform.position`)
  await solo.click()
  await h.until(async () => (await curve('transform.opacity', 0).count()) === 0)
  h.step('solo shows Position alone', (await curve('transform.opacity', 0).count()) === 0 && (await curve('transform.position', 0).count()) === 1 && (await solo.getAttribute('aria-pressed')) === 'true')
  h.step('the other curve’s name is dimmed in the legend', (await kit.getByTestId(`motion-graph-legend-${SHAPE}-transform.opacity`).getAttribute('data-hidden')) === 'true')
  await solo.click()
  await h.until(async () => (await curve('transform.opacity', 0).count()) === 1)
  h.step('solo again: every curve', (await curve('transform.opacity', 0).count()) === 1)
  await kit.getByTestId('motion-graph-toggle').click()
  await h.until(async () => (await kit.getByTestId('motion-graph').count()) === 0)

  // 3. Wiggler between the two Position keyframes.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await page.keyboard.press('KeyP')
  await mark(0).waitFor()
  await mark(0).click()
  await mark(2500).click({ modifiers: ['Shift'] })
  await h.until(async () => (await kit.locator('.motion-key[aria-selected="true"]').count()) === 2)
  const before = await keyCount()
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-control-wiggler"]').first().click()
  await kit.getByTestId('motion-wiggler-dialog').waitFor()
  h.step('Wiggler… opens its dialog, in Chinese', ((await kit.getByTestId('motion-wiggler-dialog').textContent()) ?? '').includes('摆动器'))
  await kit.getByTestId('motion-wiggler-magnitude').fill('40')
  await page.keyboard.press('Enter')
  await h.until(async () => (await keyCount()) > before)
  h.step('it adds keys between the two', (await keyCount()) > before + 3, `${before} → ${await keyCount()}`)
  await h.undo()
  await h.until(async () => (await keyCount()) === before)
  h.step('one undo takes them away', (await keyCount()) === before)

  // 4. Smoother from the keyframe menu: keys on a straight stretch go.
  const at = async (time) => (await h.query('prop.get', { layer: SHAPE, path: POSITION, time })).value
  await h.setUp([
    { command: 'prop.set', params: { layer: SHAPE, path: POSITION, value: await at(1), time: 1 } },
    { command: 'prop.set', params: { layer: SHAPE, path: POSITION, value: await at(1.5), time: 1.5 } },
  ])
  await mark(1500).waitFor()
  await mark(0).click()
  for (const ms of [1000, 1500, 2500]) await mark(ms).click({ modifiers: ['Shift'] })
  await h.until(async () => (await kit.locator('.motion-key[aria-selected="true"]').count()) === 4)
  const four = await keyCount()
  await mark(1000).click({ button: 'right' })
  await kit.getByTestId('motion-key-menu-smoother').click()
  await kit.getByTestId('motion-smoother-dialog').waitFor()
  await page.keyboard.press('Enter')
  await h.until(async () => (await keyCount()) < four)
  h.step('Smoother… takes the keys of a straight stretch off', (await keyCount()) < four, `${four} → ${await keyCount()}`)
  await h.undo()
  await h.until(async () => (await keyCount()) === four)
  h.step('one undo puts them back', (await keyCount()) === four)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
