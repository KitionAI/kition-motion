// Expressions and animation presets by hand, in a real browser through the development host:
// node tests/expressions.mjs [chromium|webkit] [shots]
// A property's expression is added from its diamond's right-click menu, Animation ▸ Add Expression
// and Alt+Shift+=, typed in the row under the property, turned off and on, shown failing in
// Chinese, and removed, each one undo step; an animation preset is saved through the host and
// applied to another layer from the host's picker. Every step uses the pointer and the keyboard;
// results are read from the engine (read-only ai.run queries). Prints PASS or FAIL per step.
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
const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
const layer = async (id) => (await details()).info.layers.find((each) => each.id === id)
/** A read-only engine query (the agent's own context: never the person's selection). */
const query = async (command, params) => (await page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])).completed[0].result
const kit = page.frameLocator('#kit')
let primary = 'Meta'

const SHAPE = 27
const SOLID = 4
const ROTATION = 'transform/rotation'

async function open() {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
  primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
}
async function undo() {
  await kit.locator('.kui-app').focus()
  await page.keyboard.press(`${primary}+KeyZ`)
  await page.waitForTimeout(300)
}
/** The node at `path` in `layer.tree`'s answer. */
function node(tree, path) {
  if (tree?.path === path) return tree
  for (const child of tree?.children ?? []) {
    const found = node(child, path)
    if (found) return found
  }
  return null
}
const prop = async (layerId, path) => node((await query('layer.tree', { layer: layerId })).properties, path)

try {
  await open()
  // 1. Add from the diamond's right-click menu: the expression row shows under the property.
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  const diamond = kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.rotation`)
  await diamond.waitFor()
  await diamond.click({ button: 'right' })
  await kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.rotation-menu-expression`).click()
  const row = `motion-expr-${SHAPE}-transform.rotation`
  await until(async () => (await kit.getByTestId(row).count()) === 1)
  step('添加表达式 in the diamond’s menu adds an expression, shown under the property', (await kit.getByTestId(row).count()) === 1 && typeof (await prop(SHAPE, ROTATION))?.expression === 'string', JSON.stringify(await prop(SHAPE, ROTATION)))

  // 2. Typed and committed with Enter: the property follows it; one undo goes back.
  const field = kit.getByTestId(`${row}-field`)
  const before = (await prop(SHAPE, ROTATION)).expression
  await field.click()
  await field.fill('time * 90')
  await field.press('Enter')
  await until(async () => (await prop(SHAPE, ROTATION))?.expression === 'time * 90')
  /** The person's current time, set by a click on the ruler. */
  const seek = async (seconds) => {
    const ruler = await kit.getByTestId('motion-ruler').boundingBox()
    const duration = (await details()).comp.duration
    await page.mouse.click(ruler.x + (seconds * ruler.width) / duration, ruler.y + ruler.height - 4)
    await until(async () => Math.abs((await details()).time - seconds) < 0.02)
  }
  await kit.locator('.kui-app').focus()
  step('the typed expression is the property’s, committed with Enter', (await prop(SHAPE, ROTATION))?.expression === 'time * 90')
  await seek(1)
  const result = kit.getByTestId(`${row}-result`)
  await until(async () => ((await result.textContent()) ?? '').includes('90'))
  step('the row shows what the expression makes of the property (90 at 1 s)', ((await result.textContent()) ?? '').includes('90'), `${(await result.textContent()) ?? ''} at ${(await details()).time} s, evaluated ${JSON.stringify((await query('prop.get', { layer: SHAPE, path: ROTATION, time: 1 })).evaluated)}`)
  if (shots) {
    await kit.getByTestId(row).scrollIntoViewIfNeeded()
    await page.screenshot({ path: `${shots}phase6-expression.png` })
  }
  await undo()
  await until(async () => (await prop(SHAPE, ROTATION))?.expression === before)
  step('one undo puts the expression back', (await prop(SHAPE, ROTATION))?.expression === before)

  // 3. A failing expression says why in Chinese.
  await field.click()
  await field.fill('nothingHere * 2')
  await field.press('Enter')
  await until(async () => (await kit.getByTestId(`${row}-error`).count()) === 1)
  const said = (await kit.getByTestId(`${row}-error`).textContent().catch(() => '')) ?? ''
  step('a failing expression shows its error in Chinese', said.includes('表达式出错') && said.includes('nothingHere'), said)
  step('the error is reworded (not defined) and keeps its line number', said.includes('没有定义') && said.includes('第 1 行') && !said.includes('Error at line'), said)

  // 4. The switch turns it off and on.
  await kit.getByTestId(`${row}-switch`).click()
  await until(async () => (await prop(SHAPE, ROTATION))?.expressionEnabled === false)
  step('the = switch turns the expression off', (await prop(SHAPE, ROTATION))?.expressionEnabled === false && (await kit.getByTestId(`${row}-switch`).getAttribute('aria-pressed')) === 'false')
  await kit.getByTestId(`${row}-switch`).click()
  await until(async () => (await prop(SHAPE, ROTATION))?.expressionEnabled === true)
  step('and on again', (await prop(SHAPE, ROTATION))?.expressionEnabled === true)

  // 5. Remove, and undo.
  await kit.getByTestId(`${row}-remove`).click()
  await until(async () => (await prop(SHAPE, ROTATION))?.expression === undefined)
  step('移除 removes the expression and its row', (await prop(SHAPE, ROTATION))?.expression === undefined && (await kit.getByTestId(row).count()) === 0)
  await undo()
  await until(async () => typeof (await prop(SHAPE, ROTATION))?.expression === 'string')
  step('undo brings it back', typeof (await prop(SHAPE, ROTATION))?.expression === 'string' && (await kit.getByTestId(row).count()) === 1)

  // 6. Animation ▸ Add Expression and Alt+Shift+= act on the selected property (a click on its keyframe selects it).
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).click()
  await until(async () => (await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).getAttribute('aria-selected')) === 'true')
  await kit.getByTestId('motion-menu-Animation').click()
  const addRow = kit.locator('[role=menu] [data-testid="motion-cmd-prop.setExpression"]').first()
  await addRow.waitFor({ timeout: 3000 }).catch(() => null)
  const listed = (await addRow.count()) === 1
  if (listed) await addRow.click()
  else await page.keyboard.press('Escape')
  await until(async () => typeof (await prop(SHAPE, 'transform/position'))?.expression === 'string')
  step('Animation ▸ 添加表达式 adds one to the selected property', listed && typeof (await prop(SHAPE, 'transform/position'))?.expression === 'string')
  await undo()
  await until(async () => (await prop(SHAPE, 'transform/position'))?.expression === undefined)
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).click()
  await kit.locator('.kui-app').focus()
  await page.keyboard.press('Alt+Shift+Equal')
  await until(async () => typeof (await prop(SHAPE, 'transform/position'))?.expression === 'string')
  step('Alt+Shift+= does too', typeof (await prop(SHAPE, 'transform/position'))?.expression === 'string')
  await undo()

  // 7. Animation presets: the rotation's expression and Position's keys saved as a preset through
  // the host, then applied to the solid from the host's picker.
  await open()
  await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'prop.setExpression', params: { layer: 27, path: 'transform/rotation', expression: 'time * 45' } }], false))
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).click()
  await until(async () => (await kit.getByTestId(`motion-key-${SHAPE}-transform.position-0`).getAttribute('aria-selected')) === 'true')
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-anim.savePreset"]').first().click()
  await kit.getByTestId('motion-preset-dialog').waitFor()
  await kit.getByTestId('motion-preset-name').fill('Slide in')
  await kit.getByTestId('motion-preset-dialog-ok').click()
  await until(async () => page.evaluate(() => window.devHost.files.has('Slide in.ecpreset')))
  step('保存动画预设 writes the preset beside the project through the host', await page.evaluate(() => window.devHost.files.has('Slide in.ecpreset')))
  await kit.getByTestId(`motion-layer-${SOLID}`).click()
  await until(async () => (await layer(SOLID))?.selected)
  const solidKeys = async () => (await query('prop.get', { layer: SOLID, path: 'transform/position' })).keys?.length ?? 0
  const keysBefore = await solidKeys()
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-anim.applyPreset"]').first().click()
  await until(async () => (await solidKeys()) > keysBefore)
  step('应用动画预设 applies the picked preset to the selected layer', (await solidKeys()) > keysBefore, `${keysBefore} → ${await solidKeys()}`)
  await undo()
  await until(async () => (await solidKeys()) === keysBefore)
  step('one undo takes it off', (await solidKeys()) === keysBefore)

  // The help docs' picture of the interface: a layer selected, twirled open with its expression row.
  if (shots) {
    await open()
    await kit.getByTestId(`motion-layer-${SHAPE}`).click()
    await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
    await kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).waitFor()
    await page.waitForTimeout(500)
    await page.screenshot({ path: `${shots}phase6-interface.png` })
  }

  step('no page errors', errors.length === 0, errors.join(' | '))
} catch (error) {
  step('ran to the end', false, String(error?.stack ?? error).split('\n').slice(0, 3).join(' '))
} finally {
  await browser.close()
  await server.close()
}
console.log(failures ? `${failures} failed (${which})` : `all passed (${which})`)
process.exit(failures ? 1 : 0)
