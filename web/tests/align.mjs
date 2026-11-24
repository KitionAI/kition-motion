// The Align panel by hand, in a real browser through the development host:
// node tests/align.mjs [chromium|webkit]
// With layers selected in the timeline, the Properties panel's Align section aligns them to the
// composition or to the selection (the choice stays after a reload, and Layer ▸ Align follows
// it), distributes three or more, and disables what would change nothing; each is one undo step.
// Results are read from the engine (read-only queries). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const SHAPE = 27
const SOLID = 4
const TEXT = 73
const position = async (layer) => (await h.query('prop.get', { layer, path: 'transform/position' })).value
const near = (a, b, eps = 0.5) => Math.abs(a - b) <= eps
const disabled = async (id) => (await kit.getByTestId(`motion-align-${id}`).getAttribute('disabled')) !== null

try {
  await h.open()
  // The sample: Shape Layer 1's 300-pixel square centred at (200, 360) in a 1280 × 720 comp; the
  // Night solid fills the comp.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await until(async () => (await h.layer(SHAPE))?.selected)
  await kit.getByTestId('motion-section-align').waitFor()
  if ((await kit.getByTestId('motion-align-panel').count()) === 0) await kit.getByTestId('motion-section-align-toggle').click()
  step('with a layer selected the Properties panel has the Align section', (await kit.getByTestId('motion-align-panel').count()) === 1)
  // Near the top of the panel (reached without scrolling), its labels inside the panel's padding.
  const body = await kit.getByTestId('motion-properties-body').boundingBox()
  const section = await kit.getByTestId('motion-section-align').boundingBox()
  const label = await kit.getByTestId('motion-align-panel').locator('.kui-muted').first().boundingBox()
  step('the Align section shows without scrolling, its labels clear of the panel’s edge', section.y + 20 < body.y + body.height && label.x >= body.x + 8, JSON.stringify({ body, section, label }))

  // 1. To the composition: the square's left edge to the comp's.
  const before = await position(SHAPE)
  await kit.getByTestId('motion-align-to-composition').click()
  await until(async () => (await kit.getByTestId('motion-workspace').getAttribute('data-align-to')) === 'composition')
  await until(async () => !(await disabled('align-right')))
  await kit.getByTestId('motion-align-align-right').click()
  await until(async () => near((await position(SHAPE))[0], 1280 - 150))
  step('Align Right to the composition puts the square’s right edge on the comp’s', near((await position(SHAPE))[0], 1130), JSON.stringify(await position(SHAPE)))
  await h.undo()
  await until(async () => near((await position(SHAPE))[0], before[0]))
  step('one undo puts it back', near((await position(SHAPE))[0], before[0]))
  step('distributing one layer is off', await disabled('distribute-left'))

  // 2. To the selection: with one layer it is off; with the solid too, the square goes to its edge.
  await kit.getByTestId('motion-align-to-selection').click()
  await until(async () => (await kit.getByTestId('motion-workspace').getAttribute('data-align-to')) === 'selection')
  step('aligning one layer to the selection is off', await until(async () => disabled('align-top'), 20))
  await kit.getByTestId(`motion-layer-${SOLID}`).locator('.motion-row-name').click({ modifiers: ['Shift'] })
  await until(async () => (await h.layer(SOLID))?.selected && (await h.layer(SHAPE))?.selected)
  await until(async () => !(await disabled('align-top')))
  await kit.getByTestId('motion-align-align-top').click()
  await until(async () => near((await position(SHAPE))[1], 150))
  step('Align Top to the selection puts the square’s top on the solid’s', near((await position(SHAPE))[1], 150) && near((await position(SOLID))[1], 360), JSON.stringify([await position(SHAPE), await position(SOLID)]))
  await h.undo()
  await until(async () => near((await position(SHAPE))[1], before[1]))

  // 3. Three layers distribute.
  await kit.getByTestId(`motion-layer-${TEXT}`).locator('.motion-row-name').click({ modifiers: ['Shift'] })
  await until(async () => (await h.layers()).filter((layer) => layer.selected).length === 3)
  await until(async () => !(await disabled('distribute-hcenter')))
  step('with three layers selected, distributing is on', !(await disabled('distribute-hcenter')))
  await kit.getByTestId('motion-align-distribute-hcenter').click()
  // Evenly spaced, distributing them again would change nothing: the engine says so and the button turns off.
  await until(async () => disabled('distribute-hcenter'))
  step('Distribute Horizontal Centers spaces three layers evenly (doing it again would change nothing)', await disabled('distribute-hcenter'))
  await h.undo()
  await until(async () => !(await disabled('distribute-hcenter')))
  step('one undo takes the distribution back', !(await disabled('distribute-hcenter')))

  // 4. The choice stays after a reload, and Layer ▸ Align offers the other one.
  await h.open()
  step('the target stays “selection” after a reload', (await kit.getByTestId('motion-workspace').getAttribute('data-align-to')) === 'selection')
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('.kui-popover[role=menu] .kui-submenu > [role=menuitem]').filter({ hasText: '对齐' }).first().hover()
  const row = kit.locator('[role=menu] [data-testid="motion-control-align-to"]').first()
  await row.waitFor({ timeout: 3000 }).catch(() => null)
  step('Layer ▸ Align offers to align to the composition instead', ((await row.textContent().catch(() => '')) ?? '').includes('合成'))
  await page.keyboard.press('Escape')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
