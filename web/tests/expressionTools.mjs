// Expressions by hand, in a real browser through the development host: node tests/expressionTools.mjs [chromium|webkit]
// The pick-whip beside an expression is dragged onto another property's row (the engine writes the
// reference where the cursor was, one undo step) and onto a layer's row (thisComp.layer("name"));
// the function menu inserts a function from its group (Random numbers ▸ wiggle-like entries) at the
// cursor; a failing expression lights the line its error is on. Results are read from the engine
// (`layer.tree`). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const NIGHT = 4
const ROTATION = 'transform/rotation'
const row = `motion-expr-${SHAPE}-transform.rotation`
function node(tree, path) {
  if (tree?.path === path) return tree
  for (const child of tree?.children ?? []) {
    const found = node(child, path)
    if (found) return found
  }
  return null
}
const expression = async () => node((await h.query('layer.tree', { layer: SHAPE })).properties, ROTATION)?.expression
/** Waits until `locator`'s box stops moving (two reads 100 ms apart the same); answers its centre. */
async function settled(locator) {
  await locator.scrollIntoViewIfNeeded()
  let last = null
  for (let i = 0; i < 30; i += 1) {
    const box = await locator.boundingBox()
    if (box && last && Math.abs(box.x - last.x) < 0.5 && Math.abs(box.y - last.y) < 0.5) return { x: box.x + box.width / 2, y: box.y + box.height / 2 }
    last = box
    await page.waitForTimeout(100)
  }
  return h.centre(locator)
}
/** The pick-whip dragged from its handle to `target`'s middle and let go. */
async function whipTo(target) {
  const from = await settled(kit.getByTestId(`${row}-whip`))
  const to = await settled(target)
  await h.drag(from, to, 12)
}

try {
  await h.open()
  // An expression on Rotation, shown under it; Opacity's row open too (a property to link to).
  await h.setUp([{ command: 'prop.setExpression', params: { layer: SHAPE, path: ROTATION, expression: '0' } }])
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  await kit.getByTestId(`${row}-field`).waitFor()

  // 1. The pick-whip onto Opacity's row: the reference goes where the cursor was.
  const field = kit.getByTestId(`${row}-field`)
  await field.click()
  await field.fill('10 + ')
  await page.keyboard.press('End')
  const opacityRow = kit.getByTestId(`motion-prop-row-${SHAPE}-transform.opacity`)
  await whipTo(opacityRow.locator('.motion-prop-label'))
  await h.until(async () => ((await expression()) ?? '').includes('opacity'))
  const linked = await expression()
  h.step('the pick-whip onto Opacity writes its reference at the cursor', /^10 \+ .*opacity/.test(linked ?? ''), linked)
  const evaluated = (await h.query('prop.get', { layer: SHAPE, path: ROTATION })).evaluated
  const opacity = (await h.query('prop.get', { layer: SHAPE, path: 'transform/opacity' })).value
  h.step('and the property follows it', Math.abs(evaluated - (10 + opacity)) < 1e-6, `${evaluated} vs 10 + ${opacity}`)
  await h.undo()
  await h.until(async () => (await expression()) !== linked)
  h.step('one undo takes the link away', (await expression()) !== linked, await expression())

  // 2. Onto a layer's row: thisComp.layer("name").
  await field.click()
  await field.fill('')
  await field.press('Enter')
  await h.until(async () => (await expression()) !== undefined)
  await field.click()
  await whipTo(kit.getByTestId(`motion-layer-${NIGHT}`).locator('.motion-row-name').first())
  await h.until(async () => ((await expression()) ?? '').includes('thisComp.layer('))
  h.step('the pick-whip onto a layer writes thisComp.layer("Night")', ((await expression()) ?? '').includes('thisComp.layer("Night")'), await expression())

  // 3. The function menu: Random numbers ▸ random(0, 100), inserted where the cursor is.
  await field.click()
  await field.fill('time + ')
  await page.keyboard.press('End')
  await kit.getByTestId(`${row}-functions`).click()
  const group = kit.getByTestId(`${row}-fn-random`)
  await group.waitFor()
  const groups = await kit.locator('[role=menu]').last().locator('[role=menuitem]').allTextContents()
  h.step('the function menu has its groups in Chinese', ['全局', '向量数学', '随机数', '插值', '图层', '属性', '关键帧', '时间转换'].every((name) => groups.some((text) => text.includes(name))), groups.join(' | '))
  await group.hover()
  await kit.getByTestId(`${row}-fn-random-random-0-100-`).click()
  await h.until(async () => (await field.inputValue()) === 'time + random(0, 100)')
  h.step('choosing random(0, 100) inserts it at the cursor', (await field.inputValue()) === 'time + random(0, 100)', await field.inputValue())
  h.step('and the cursor is back in the field after it', await field.evaluate((element) => element === document.activeElement && element.selectionStart === element.value.length))
  await field.press('Enter')
  await h.until(async () => (await expression()) === 'time + random(0, 100)')
  h.step('Enter keeps it', (await expression()) === 'time + random(0, 100)')

  // 4. An error on line 2 lights that line.
  await field.click()
  await field.fill('x = 1;')
  await page.keyboard.press('Shift+Enter')
  await page.keyboard.type('nothingHere + x')
  await page.keyboard.press('Enter')
  await h.until(async () => (await kit.getByTestId(`${row}-error-line`).count()) === 1)
  const lit = kit.getByTestId(`${row}-error-line`)
  h.step('the error’s line is lit in the field', (await lit.getAttribute('data-line')) === '2', await lit.getAttribute('data-line'))
  const fieldBox = await field.boundingBox()
  const lineBox = await lit.boundingBox()
  h.step('over the second line of the text', lineBox && lineBox.y > fieldBox.y + lineBox.height * 0.6 && lineBox.y < fieldBox.y + fieldBox.height, JSON.stringify({ fieldBox, lineBox }))
  await field.click()
  await field.fill('x = 1;\nx * 2')
  await field.press('Enter')
  await h.until(async () => (await kit.getByTestId(`${row}-error-line`).count()) === 0)
  h.step('fixed, the line is no longer lit', (await kit.getByTestId(`${row}-error-line`).count()) === 0)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
