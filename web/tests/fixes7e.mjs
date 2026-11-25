// The fixes after the round 7e hand test, by hand, in a real browser through the development host: node tests/fixes7e.mjs [chromium|webkit]
// The pick-whip with the expression never clicked replaces the whole text (with a one-value
// property's `[0]` from a several-value one); a broken expression says why in Chinese; opening the
// function menu keeps the half-typed text a draft (no error, still on, one undo step after Enter);
// the field grows with its text; in the default (front) view the edge-on X and Y rings turn the
// layer and the Z dot moves it in depth; Animation ▸ Keyframe Assistant keeps its rows and says
// what they wait for, the Wiggler applies with Enter and closes with Escape, its "per second"
// stays on one line; Convert Expression to Keyframes does nothing to a property without one; the
// multi-select panel lists no 3D-only rows for 2D layers, shows units and has a stopwatch for all;
// Layer ▸ New gives text, null and camera layers their own labels (the text named in Chinese) and
// Layer ▸ 标签 labels the selection; the cache button shows its progress; Export says 完整 and
// 二分之一 as the Render Queue does; F11 is left to the browser; the second view fills and names
// its layers and leaves the viewer most of the width at 1280 × 800. Results are read from the
// engine (`layer.tree`, `prop.get`, `comp.info`, `view.get3D`). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const NIGHT = 4
const ROTATION = 'transform/rotation'
const row = `motion-expr-${SHAPE}-transform.rotation`
const near = (a, b, eps) => Math.abs(a - b) <= eps
function node(tree, path) {
  if (tree?.path === path) return tree
  for (const child of tree?.children ?? []) {
    const found = node(child, path)
    if (found) return found
  }
  return null
}
const rotationNode = async () => node((await h.query('layer.tree', { layer: SHAPE })).properties, ROTATION)
const expression = async () => (await rotationNode())?.expression
const value = async (layer, path) => (await h.query('prop.get', { layer, path })).value
const keysOf = async (layer, path) => ((await h.query('prop.get', { layer, path })).keys ?? []).length
const labelOf = async (id) => (await h.query('comp.info', {})).layers.find((layer) => layer.id === id)?.label
/** Waits until `locator`'s box stops moving (two reads 100 ms apart the same); answers its centre. */
async function settled(locator) {
  await locator.scrollIntoViewIfNeeded()
  let last = null
  for (let i = 0; i < 30; i += 1) {
    const box = await locator.boundingBox()
    if (box && last && Math.abs(box.x - last.x) < 0.5 && Math.abs(box.y - last.y) < 0.5 && Math.abs(box.height - last.height) < 0.5) return { x: box.x + box.width / 2, y: box.y + box.height / 2 }
    last = box
    await page.waitForTimeout(100)
  }
  return h.centre(locator)
}
/** A menu bar menu, then its rows down `path` (submenus hovered), the last one clicked. */
async function menu(name, path, last) {
  await kit.getByTestId(`motion-menu-${name}`).click()
  let at = `menu:${name}`
  for (const sub of path) {
    at = `${at}/${sub}`
    await kit.locator(`[data-kit-ui-id="${at}"]`).hover()
  }
  await kit.locator(`[role=menu] [data-testid="${last}"]`).first().click()
}
const newest = (before, layers) => layers.find((layer) => !before.some((old) => old.id === layer.id))
/** The 3D handles' parts where the engine draws them, once two reads agree. */
async function handles(part) {
  let last = ''
  await h.until(async () => {
    const now = (await kit.getByTestId('motion-workspace').getAttribute('data-gizmo')) || '{}'
    const ok = now === last && JSON.parse(now)[part] != null
    last = now
    return ok
  })
  return JSON.parse((await kit.getByTestId('motion-workspace').getAttribute('data-gizmo')) || '{}')
}
/** A press on a part of the handles, moved by (dx, dy) page pixels in steps, released. */
async function dragPart(part, dx, dy) {
  const spot = (await handles(part))[part]
  const from = await h.at(spot[0], spot[1])
  await h.drag(from, { x: from.x + dx, y: from.y + dy }, 10)
}

try {
  await h.open()

  // 1. The pick-whip with the expression never clicked: the whole text is replaced (it was the
  // placeholder Alt-click puts there), and a one-value property takes Position's first value.
  await h.setUp([{ command: 'prop.setExpression', params: { layer: SHAPE, path: ROTATION, expression: 'transform.rotation' } }])
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  const field = kit.getByTestId(`${row}-field`)
  await field.waitFor()
  const whip = await settled(kit.getByTestId(`${row}-whip`))
  const position = await settled(kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).locator('.motion-prop-label'))
  await h.drag(whip, position, 12)
  await h.until(async () => ((await expression()) ?? '').includes('position'))
  h.step('a whip with the field never clicked replaces the whole expression, with [0]', (await expression()) === 'transform.position[0]', await expression())
  const x = (await value(SHAPE, 'transform/position'))[0]
  await h.until(async () => near((await h.query('prop.get', { layer: SHAPE, path: ROTATION })).evaluated, x, 1e-6))
  h.step('and the rotation follows Position X', near((await h.query('prop.get', { layer: SHAPE, path: ROTATION })).evaluated, x, 1e-6))

  // 2. A broken expression says why in Chinese (the engine's words for an empty value).
  await field.click()
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('var a; a.b')
  await page.keyboard.press('Enter')
  const error = kit.getByTestId(`${row}-error`)
  await h.until(async () => (await error.count()) === 1)
  const said = (await error.textContent()) ?? ''
  h.step('a broken expression’s error is all Chinese', said.includes('值是空的') && !/[A-Za-z]{4,}/.test(said.replace(/null|undefined/g, '')), said)

  // 3. Σ with half-typed text: no error, still on, the function goes into the draft; one undo
  // after Enter puts back what was there before.
  await field.click()
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('time * 0')
  await page.keyboard.press('Enter')
  await h.until(async () => (await expression()) === 'time * 0')
  await field.click()
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('time + ')
  await kit.getByTestId(`${row}-functions`).click()
  const group = kit.getByTestId(`${row}-fn-random`)
  await group.waitFor()
  // Long enough for a commit to reach the engine and come back, were there one.
  await h.until(async () => (await expression()) !== 'time * 0', 10)
  h.step('opening the function menu does not commit the half-typed text', (await expression()) === 'time * 0' && (await error.count()) === 0 && (await rotationNode())?.expressionEnabled !== false, `${await expression()} ${(await rotationNode())?.expressionEnabled}`)
  await group.hover()
  await kit.getByTestId(`${row}-fn-random-random-0-100-`).click()
  await h.until(async () => (await field.inputValue()) === 'time + random(0, 100)')
  h.step('the function goes into the draft, the cursor after it', (await field.inputValue()) === 'time + random(0, 100)' && (await field.evaluate((element) => element === document.activeElement)))
  await page.keyboard.press('Enter')
  await h.until(async () => (await expression()) === 'time + random(0, 100)')
  h.step('Enter keeps it, turned on', (await expression()) === 'time + random(0, 100)' && (await rotationNode())?.expressionEnabled !== false)
  await h.undo()
  await h.until(async () => (await expression()) !== 'time + random(0, 100)')
  h.step('one undo puts back the expression before it (no half-typed step)', (await expression()) === 'time * 0', await expression())

  // 4. The field grows with its text (up to eight lines) and can be dragged taller.
  await field.click()
  const lineHeight = await field.evaluate((element) => parseFloat(getComputedStyle(element).lineHeight) || 16)
  h.step('the field can be made taller by dragging its corner', (await field.evaluate((element) => getComputedStyle(element).resize)) === 'vertical')
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('var a = 1;')
  for (const text of ['var b = 2;', 'var c = 3;', 'var d = 4;', 'var e = 5;', 'a + b + c + d + e']) {
    await page.keyboard.press('Shift+Enter')
    await page.keyboard.type(text)
  }
  await h.until(async () => (await field.boundingBox()).height >= lineHeight * 6)
  h.step('six lines of text show six lines', (await field.boundingBox()).height >= lineHeight * 6 && (await field.evaluate((element) => element.scrollHeight <= element.clientHeight + 2)), `${(await field.boundingBox()).height}`)
  await page.keyboard.press('Escape')
  await h.until(async () => (await field.inputValue()) === 'time * 0')

  // 5. In the default (front) view the X and Y rings, seen edge-on, turn Night; the Z dot moves it in depth.
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [NIGHT], switch: 'threeD', value: true } }])
  await kit.getByTestId(`motion-layer-${NIGHT}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layer(NIGHT))?.selected && !(await h.layer(SHAPE))?.selected)
  h.step('the default view is the active camera’s', (await h.query('view.get3D', {}))?.view === 'activeCamera')
  const shown = await handles('rx')
  h.step('the X and Y rings and the Z dot are shown', shown.rx != null && shown.ry != null && shown.z != null, Object.keys(shown).join(' '))
  await dragPart('rx', 0, -45)
  await h.until(async () => Math.abs(await value(NIGHT, 'transform/rotationX')) > 1)
  const tilted = await value(NIGHT, 'transform/rotationX')
  h.step('dragging along the edge-on X ring turns X Rotation', Math.abs(tilted) > 10, String(tilted))
  await h.undo()
  await h.until(async () => near(await value(NIGHT, 'transform/rotationX'), 0, 1e-6))
  // The handles drawn where they were before the turn (not a picture of the turned layer).
  await h.until(async () => JSON.stringify(JSON.parse((await kit.getByTestId('motion-workspace').getAttribute('data-gizmo')) || '{}')) === JSON.stringify(shown))
  await dragPart('ry', 45, 0)
  await h.until(async () => Math.abs(await value(NIGHT, 'transform/rotationY')) > 1)
  const turned = await value(NIGHT, 'transform/rotationY')
  h.step('dragging along the edge-on Y ring turns Y Rotation', Math.abs(turned) > 10 && near(await value(NIGHT, 'transform/rotationX'), 0, 1e-6), String(turned))
  await h.undo()
  await h.until(async () => near(await value(NIGHT, 'transform/rotationY'), 0, 1e-6))
  await h.until(async () => JSON.stringify(JSON.parse((await kit.getByTestId('motion-workspace').getAttribute('data-gizmo')) || '{}')) === JSON.stringify(shown))
  const depth = (await value(NIGHT, 'transform/position'))[2] ?? 0
  await dragPart('z', 0, -40)
  await h.until(async () => !near((await value(NIGHT, 'transform/position'))[2] ?? 0, depth, 0.5))
  const deeper = await value(NIGHT, 'transform/position')
  h.step('dragging the Z dot up moves it away (Z grows), X and Y stay', (deeper[2] ?? 0) > depth + 5, JSON.stringify(deeper))
  await h.undo()
  await h.until(async () => near((await value(NIGHT, 'transform/position'))[2] ?? 0, depth, 1e-6))
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [NIGHT], switch: 'threeD', value: false } }])

  // 6. Keyframe Assistant with nothing selected: its rows are there and say what they wait for.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  await h.focusEditor()
  await page.keyboard.press('Escape')
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
  const wigglerRow = kit.locator('[role=menu] [data-testid="motion-control-wiggler"]').first()
  await wigglerRow.waitFor()
  h.step('without keyframes selected Wiggler is listed, saying it needs two', ((await wigglerRow.textContent()) ?? '').includes('至少两个关键帧'), await wigglerRow.textContent())
  const convertRow = kit.locator('[role=menu] [data-testid="motion-control-convert-expression"]').first()
  h.step('and Convert Expression to Keyframes says it needs a property with one', ((await convertRow.textContent()) ?? '').includes('有表达式的属性'), await convertRow.textContent())
  await wigglerRow.click()
  await h.until(async () => (await kit.locator('.kui-toast').count()) > 0, 20)
  h.step('choosing it says so, and opens nothing', (await kit.getByTestId('motion-wiggler-dialog').count()) === 0 && ((await kit.locator('.kui-toast').last().textContent()) ?? '').includes('至少两个关键帧'))

  // 7. Convert Expression to Keyframes on Position (keyframes, no expression): nothing happens.
  await h.setUp([
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 0 } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 2, value: [700, 400] } },
  ])
  await page.keyboard.press('KeyP')
  const keyed = await keysOf(SHAPE, 'transform/position')
  const positionName = kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).locator('.motion-prop-label')
  await positionName.waitFor()
  await positionName.click()
  await menu('Animation', ['Keyframe Assistant'], 'motion-control-convert-expression')
  await page.waitForTimeout(300)
  h.step('Convert Expression to Keyframes leaves a property without one alone', (await keysOf(SHAPE, 'transform/position')) === keyed, `${keyed} → ${await keysOf(SHAPE, 'transform/position')}`)

  // 8. The Wiggler between the two keyframes: Enter applies, Escape cancels; per second on one line.
  const mark = (ms) => kit.getByTestId(`motion-key-${SHAPE}-transform.position-${ms}`)
  await mark(0).waitFor()
  await mark(0).click()
  await mark(2000).click({ modifiers: ['Shift'] })
  await h.until(async () => (await kit.locator('.motion-key[aria-selected="true"]').count()) === 2)
  await menu('Animation', ['Keyframe Assistant'], 'motion-control-wiggler')
  await kit.getByTestId('motion-wiggler-dialog').waitFor()
  const unit = kit.getByTestId('motion-wiggler-dialog').locator('.motion-form-input .kui-muted').first()
  const unitBox = await unit.boundingBox()
  const unitLine = await unit.evaluate((element) => parseFloat(getComputedStyle(element).lineHeight) || 18)
  h.step('“每秒” stays on one line', ((await unit.textContent()) ?? '').includes('每秒') && unitBox.height < unitLine * 1.5, `${unitBox.height} for a line of ${unitLine}`)
  await page.keyboard.press('Escape')
  await h.until(async () => (await kit.getByTestId('motion-wiggler-dialog').count()) === 0)
  h.step('Escape closes it at once and changes nothing', (await kit.getByTestId('motion-wiggler-dialog').count()) === 0 && (await keysOf(SHAPE, 'transform/position')) === keyed)
  await menu('Animation', ['Keyframe Assistant'], 'motion-control-wiggler')
  await kit.getByTestId('motion-wiggler-dialog').waitFor()
  await page.keyboard.press('Enter')
  await h.until(async () => (await keysOf(SHAPE, 'transform/position')) > keyed)
  h.step('Enter applies it at once', (await keysOf(SHAPE, 'transform/position')) > keyed, `${keyed} → ${await keysOf(SHAPE, 'transform/position')}`)

  // 9. Two 2D layers selected: no 3D-only rows, units, and a stopwatch for both (one undo step).
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await page.keyboard.down(h.primary)
  await kit.getByTestId(`motion-layer-${NIGHT}`).locator('.motion-row-name').first().click()
  await page.keyboard.up(h.primary)
  await kit.getByTestId('motion-multi-opacity-0').waitFor()
  h.step('two 2D layers: no Orientation or X/Y Rotation rows', (await kit.getByTestId('motion-multi-orientation').count()) === 0 && (await kit.getByTestId('motion-multi-rotationX').count()) === 0 && (await kit.getByTestId('motion-multi-rotationY').count()) === 0)
  const opacityRow = ((await kit.getByTestId('motion-multi-opacity').textContent()) ?? '') + ((await kit.getByTestId('motion-multi-rotation').textContent()) ?? '')
  h.step('the rows show their units', opacityRow.includes('%') && opacityRow.includes('°'), opacityRow)
  await kit.getByTestId('motion-multi-opacity-key').click()
  await h.until(async () => (await keysOf(SHAPE, 'transform/opacity')) === 1 && (await keysOf(NIGHT, 'transform/opacity')) === 1)
  h.step('the stopwatch animates Opacity on both layers', (await keysOf(SHAPE, 'transform/opacity')) === 1 && (await keysOf(NIGHT, 'transform/opacity')) === 1)
  await h.until(async () => (await kit.getByTestId('motion-multi-opacity-key').getAttribute('data-animated')) === 'true')
  h.step('and shows it is on', (await kit.getByTestId('motion-multi-opacity-key').getAttribute('data-animated')) === 'true')
  await h.undo()
  await h.until(async () => (await keysOf(SHAPE, 'transform/opacity')) === 0 && (await keysOf(NIGHT, 'transform/opacity')) === 0)
  h.step('one undo takes it off both', (await keysOf(SHAPE, 'transform/opacity')) === 0 && (await keysOf(NIGHT, 'transform/opacity')) === 0)

  // 10. Layer ▸ New: a text layer named in Chinese and its own label; a null, a camera theirs.
  let before = await h.layers()
  await menu('Layer', ['New'], 'motion-cmd-layer.newText')
  await h.until(async () => (await h.layers()).length === before.length + 1)
  const text = newest(before, await h.layers())
  h.step('Layer ▸ New ▸ Text is named 文本', text?.name === '文本', text?.name)
  await h.until(async () => (await labelOf(text.id)) === 'Peach')
  h.step('and has the text label (桃色), not red', (await labelOf(text.id)) === 'Peach', await labelOf(text.id))
  for (const [command, label] of [['layer.newNull', 'Brown'], ['layer.newCamera', 'Cyan']]) {
    before = await h.layers()
    await menu('Layer', ['New'], `motion-cmd-${command}`)
    await h.until(async () => (await h.layers()).length === before.length + 1)
    const made = newest(before, await h.layers())
    await h.until(async () => (await labelOf(made.id)) === label)
    h.step(`${command} gets its own label`, (await labelOf(made.id)) === label, await labelOf(made.id))
  }
  // Layer ▸ 标签 ▸ 蓝色 on the selection.
  await kit.getByTestId(`motion-layer-${text.id}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layer(text.id))?.selected)
  await menu('Layer', ['Label'], 'motion-control-label-Blue')
  await h.until(async () => (await labelOf(text.id)) === 'Blue')
  h.step('Layer ▸ 标签 ▸ 蓝色 labels the selected layer', (await labelOf(text.id)) === 'Blue')

  // 11. The cache button shows how far it got.
  await kit.getByTestId('motion-cache-ahead').click()
  await h.until(async () => /\d+\/\d+|缓存已满/.test((await kit.getByTestId('motion-cache-ahead').textContent()) ?? ''))
  h.step('the cache button shows its progress', /后台缓存 \d+\/\d+|缓存已满/.test((await kit.getByTestId('motion-cache-ahead').textContent()) ?? ''), await kit.getByTestId('motion-cache-ahead').textContent())
  await kit.getByTestId('motion-cache-ahead').click()
  await h.until(async () => (await kit.getByTestId('motion-cache-ahead').textContent()) === '后台缓存')

  // 12. Export says 完整 and 二分之一, as the Render Queue does.
  await kit.getByTestId('motion-menu-File').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-file.exportMovie"]').first().click()
  await kit.getByTestId('motion-export-size').waitFor()
  const sizes = await kit.getByTestId('motion-export-size').locator('option').allTextContents()
  const ranges = await kit.getByTestId('motion-export-range').locator('option').allTextContents()
  h.step('Export’s sizes and range use the Render Queue’s words', sizes.some((each) => each.startsWith('完整')) && sizes.some((each) => each.startsWith('二分之一')) && ranges.some((each) => each.startsWith('工作区域')), `${sizes.join(' | ')} / ${ranges.join(' | ')}`)
  await kit.getByTestId('motion-export-dialog-cancel').click()

  // 13. F11 is the browser's: the view stays.
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await h.focusEditor()
  await page.keyboard.press('F11')
  await page.waitForTimeout(300)
  h.step('F11 leaves the 3D view alone', (await h.query('view.get3D', {}))?.view === 'activeCamera', (await h.query('view.get3D', {}))?.view)
  await page.keyboard.press('Alt+Shift+Digit2')
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'custom1')
  h.step('Alt+Shift+2 still switches to Custom View 1', (await h.query('view.get3D', {}))?.view === 'custom1')
  await page.keyboard.press('Alt+Shift+Digit3')
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'activeCamera')

  // 14. The second view at 1280 × 800: the layers filled and named, the viewer most of the width.
  await page.setViewportSize({ width: 1280, height: 800 })
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await menu('View', [], 'motion-control-two-views')
  const second = kit.getByTestId('motion-second-view')
  await second.waitFor()
  await h.until(async () => ((await second.getAttribute('data-layers')) ?? '').includes(`${SHAPE}*`))
  h.step('the second view names the 3D layer and marks it selected', ((await second.getAttribute('data-layers')) ?? '').includes(`${SHAPE}*`), await second.getAttribute('data-layers'))
  const viewer = await kit.locator('.motion-views > .kui-canvas-region').boundingBox()
  const pane = await second.boundingBox()
  // Round 7g widened the second view (about 28 % of the window at 1280 px, its divider drags): the
  // viewer keeps most of the two views' width, still more than half of it.
  h.step('the viewer keeps most of the width beside the second view', viewer.width >= (viewer.width + pane.width) * 0.55, `${viewer.width} beside ${pane.width}`)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
