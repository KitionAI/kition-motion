// The Properties panel by hand, in a real browser through the development host: node tests/properties.mjs [chromium|webkit]
// Every step drives the editor with the pointer and the keyboard (page.mouse, page.keyboard) and
// checks the result in the engine (read-only ai.run queries: prop.get, layer.tree, comp.info).
// Prints PASS or FAIL per step.
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
const near = (a, b, eps = 1e-3) => Math.abs(a - b) <= eps
/** A read-only engine query (the agent's own context: never the person's selection). */
const query = async (command, params) => (await page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])).completed[0].result
const value = async (layer, path) => (await query('prop.get', { layer, path })).value
const info = async (id) => (await query('comp.info', {})).layers.find((layer) => layer.id === id)
const effectsOf = async (layer) => ((await query('layer.tree', { layer, depth: 2 })).properties.children.find((group) => group.match === 'effects')?.children ?? []).map((effect) => ({ name: effect.name, enabled: effect.enabled, uid: effect.uid }))
const kit = page.frameLocator('#kit')
let primary = 'Meta'

const SHAPE = 27
const TEXT = 73
const SOLID = 4
const FILL = 'contents/group/contents/fill/color'
const hex = (rgb) => `#${rgb.slice(0, 3).map((part) => Math.round(part * 255).toString(16).padStart(2, '0')).join('')}`

async function open() {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
  primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
}

/** Clicks a layer's row in the timeline and waits for the panel to show it. */
async function select(id) {
  await kit.getByTestId(`motion-layer-${id}`).click()
  await kit.getByTestId('motion-layer-header').waitFor()
  await until(async () => (await kit.getByTestId('motion-layer-name').inputValue()) === (await info(id)).name)
}

/** Undo with the keyboard, from the editor (not from a field). */
async function undo() {
  await kit.locator('.kui-app').focus()
  await page.keyboard.press(`${primary}+KeyZ`)
}

/** A press on `testId`, dragged `dx` pixels sideways in small steps, and a release. */
async function scrub(testId, dx) {
  const field = await kit.getByTestId(testId).boundingBox()
  const x = field.x + field.width / 2
  const y = field.y + field.height / 2
  await page.mouse.move(x, y)
  await page.mouse.down()
  for (let i = 1; i <= 12; i += 1) {
    await page.mouse.move(x + (dx * i) / 12, y)
    await page.waitForTimeout(16)
  }
  await page.mouse.up()
}

/** The composition as the canvas draws it. */
const picture = async () => kit.locator('[data-testid="motion-workspace"] canvas').first().screenshot()

try {
  await open()
  step('with nothing selected the panel says what to do', await kit.getByTestId('motion-properties-empty').isVisible())

  // 1. Text: content, size, fill.
  await select(TEXT)
  step('a text layer shows its Text section', await kit.getByTestId('motion-text-content').isVisible())
  const content = kit.getByTestId('motion-text-content')
  await content.click()
  await page.keyboard.press(`${primary}+KeyA`)
  await page.keyboard.type('Aurora')
  await page.keyboard.press(`${primary}+Enter`)
  await until(async () => (await value(TEXT, 'text/sourceText')) === 'Aurora')
  step('typing in the box changes the source text', (await value(TEXT, 'text/sourceText')) === 'Aurora', await value(TEXT, 'text/sourceText'))
  await undo()
  await until(async () => (await value(TEXT, 'text/sourceText')) === 'Northern Lights')
  step('one undo restores the text', (await value(TEXT, 'text/sourceText')) === 'Northern Lights', await value(TEXT, 'text/sourceText'))
  const size = kit.getByTestId('motion-text-size')
  await size.click()
  await size.fill('96')
  await size.press('Enter')
  // The text's size, read through the scripting model (no engine command answers it).
  const fontSize = async () => Number((await query('script.run', { code: `app.project.activeItem.layer(1).property("ADBE Text Properties").property("ADBE Text Document").value.fontSize` })).result)
  await until(async () => near(await fontSize(), 96))
  step('the font size changes', near(await fontSize(), 96), String(await fontSize()))
  await kit.getByTestId('motion-text-fill-swatch').click()
  const fillHex = kit.getByTestId('motion-text-fill-picker-hex')
  await fillHex.fill('#ff3300')
  await fillHex.press('Enter')
  await page.keyboard.press('Escape')
  await until(async () => (await kit.getByTestId('motion-text-fill-swatch').getAttribute('aria-label'))?.includes('#ff3300'))
  step('the fill colour changes through its popover', (await kit.getByTestId('motion-text-fill-swatch').getAttribute('aria-label'))?.includes('#ff3300'), await kit.getByTestId('motion-text-fill-swatch').getAttribute('aria-label'))
  await undo()
  await until(async () => !(await kit.getByTestId('motion-text-fill-swatch').getAttribute('aria-label'))?.includes('#ff3300'))
  step('undo restores the fill', !(await kit.getByTestId('motion-text-fill-swatch').getAttribute('aria-label'))?.includes('#ff3300'))

  // 2. Scrubbing a number by dragging is one undo step.
  await select(SHAPE)
  await scrub('motion-value-rotation-0', 60)
  await until(async () => (await value(SHAPE, 'transform/rotation')) > 10)
  const rotated = await value(SHAPE, 'transform/rotation')
  step('dragging a number scrubs it', rotated > 10, String(rotated))
  await undo()
  await until(async () => near(await value(SHAPE, 'transform/rotation'), 0))
  step('one undo takes the whole scrub back', near(await value(SHAPE, 'transform/rotation'), 0), String(await value(SHAPE, 'transform/rotation')))

  // 3. Shape: fill colour through the popover, roundness.
  await kit.getByTestId(`motion-value-${FILL.replaceAll('/', '.')}-swatch`).click()
  const shapeHex = kit.getByTestId(`motion-value-${FILL.replaceAll('/', '.')}-picker-hex`)
  await shapeHex.waitFor()
  step('the colour picker opens in a popover', await kit.getByTestId(`motion-value-${FILL.replaceAll('/', '.')}-popover`).isVisible())
  await shapeHex.fill('#22cc44')
  await shapeHex.press('Enter')
  await until(async () => hex(await value(SHAPE, FILL)) === '#22cc44')
  step("the shape's fill colour changes", hex(await value(SHAPE, FILL)) === '#22cc44', hex(await value(SHAPE, FILL)))
  await page.keyboard.press('Escape')
  const roundness = kit.getByTestId('motion-value-contents.group.contents.rect.roundness-0')
  await roundness.click()
  await roundness.fill('40')
  await roundness.press('Enter')
  await until(async () => near(await value(SHAPE, 'contents/group/contents/rect/roundness'), 40))
  step('roundness is set', near(await value(SHAPE, 'contents/group/contents/rect/roundness'), 40))
  await undo()
  await until(async () => near(await value(SHAPE, 'contents/group/contents/rect/roundness'), 0))
  await undo()
  await until(async () => hex(await value(SHAPE, FILL)) !== '#22cc44')
  step('undo restores roundness and fill', near(await value(SHAPE, 'contents/group/contents/rect/roundness'), 0) && hex(await value(SHAPE, FILL)) !== '#22cc44')
  await kit.getByTestId('motion-shape-add-stroke').click()
  const stroked = async () => (await query('prop.get', { layer: SHAPE, path: 'contents/group/contents/stroke/width' }).catch(() => null))?.value
  await until(async () => (await stroked()) > 0)
  step('Add stroke gives the shape a stroke', (await stroked()) > 0 && (await kit.getByTestId('motion-value-contents.group.contents.stroke.width-0').count()) === 1, String(await stroked()))
  await undo()
  await until(async () => (await kit.getByTestId('motion-shape-add-stroke').count()) === 1)

  // 4. Effects: add, expanded, change, see it, toggle, reorder, remove, key.
  await kit.getByTestId('motion-effect-add').click()
  await kit.getByTestId('motion-effect-search').fill('Gaussian Blur')
  await page.keyboard.press('Enter')
  await until(async () => (await effectsOf(SHAPE)).length === 1)
  await kit.getByTestId('motion-effect-card-1').waitFor()
  step('Add effect applies Gaussian Blur and its card opens', (await kit.getByTestId('motion-effect-toggle-1').getAttribute('aria-expanded')) === 'true')
  const before = await picture()
  const blur = kit.getByTestId('motion-value-effects.#1.blurriness-0')
  await blur.click()
  await blur.fill('30')
  await blur.press('Enter')
  await until(async () => near(await value(SHAPE, 'effects/#1/blurriness'), 30))
  step('Blurriness changes', near(await value(SHAPE, 'effects/#1/blurriness'), 30))
  await page.waitForTimeout(600)
  const blurred = await picture()
  step('the composition is drawn blurred', !before.equals(blurred))
  await kit.getByTestId('motion-effect-enabled-1').click()
  await until(async () => (await effectsOf(SHAPE))[0]?.enabled === false)
  step('the switch turns the effect off', (await effectsOf(SHAPE))[0]?.enabled === false)
  await kit.getByTestId('motion-effect-enabled-1').click()
  await until(async () => (await effectsOf(SHAPE))[0]?.enabled === true)
  step('and on again', (await effectsOf(SHAPE))[0]?.enabled === true)
  await kit.getByTestId('motion-effect-add').click()
  await kit.getByTestId('motion-effect-search').fill('Glow')
  await kit.getByTestId('motion-effect-option-ec.stylize.glow').click()
  await until(async () => (await effectsOf(SHAPE)).length === 2)
  step('a second effect opens below the first', (await kit.getByTestId('motion-effect-toggle-2').getAttribute('aria-expanded')) === 'true')
  await kit.getByTestId('motion-effect-toggle-1').click()
  await kit.getByTestId('motion-effect-toggle-2').click()
  const grip = await kit.getByTestId('motion-effect-grip-2').boundingBox()
  const first = await kit.getByTestId('motion-effect-card-1').boundingBox()
  await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2)
  await page.mouse.down()
  for (let i = 1; i <= 10; i += 1) await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2 + ((first.y + 4 - grip.y - grip.height / 2) * i) / 10)
  await page.mouse.up()
  await until(async () => (await effectsOf(SHAPE))[0]?.name === 'Glow')
  step('dragging a card reorders the effects', (await effectsOf(SHAPE)).map((effect) => effect.name).join(',') === 'Glow,Gaussian Blur', (await effectsOf(SHAPE)).map((effect) => effect.name).join(','))
  await undo()
  await until(async () => (await effectsOf(SHAPE))[0]?.name === 'Gaussian Blur')
  step('undo restores the order', (await effectsOf(SHAPE))[0]?.name === 'Gaussian Blur')
  await kit.getByTestId('motion-effect-toggle-1').click()
  await kit.getByTestId('motion-add-key-effects.#1.blurriness').click()
  await until(async () => (await value(SHAPE, 'effects/#1/blurriness')) !== undefined && (await query('prop.get', { layer: SHAPE, path: 'effects/#1/blurriness' })).keys.length === 1)
  step('the diamond keys an effect parameter', (await query('prop.get', { layer: SHAPE, path: 'effects/#1/blurriness' })).keys.length === 1)
  const keyed = () => kit.locator(`[data-testid^="motion-key-${SHAPE}-"][data-keys*="effects/#1/blurriness"]`).count()
  await until(async () => (await keyed()) > 0)
  step("and the key shows on the layer's row in the timeline", (await keyed()) > 0)
  await kit.getByTestId('motion-effect-toggle-2').click()
  await page.keyboard.press('Delete')
  await until(async () => (await effectsOf(SHAPE)).length === 1)
  step('Delete on a selected card removes that effect, not the layer', (await effectsOf(SHAPE)).length === 1 && !!(await info(SHAPE)), JSON.stringify(await effectsOf(SHAPE)))
  await kit.getByTestId('motion-effect-menu-1').click()
  await kit.getByTestId('motion-effect-remove-1').click()
  await until(async () => (await effectsOf(SHAPE)).length === 0)
  step('Remove in the card menu removes it', (await effectsOf(SHAPE)).length === 0)
  await undo()
  await until(async () => (await effectsOf(SHAPE)).length === 1)
  step('undo brings it back', (await effectsOf(SHAPE)).length === 1)

  // 5. Blending mode and parent.
  await kit.getByTestId('motion-blend-mode').selectOption('Screen')
  await until(async () => (await info(SHAPE)).blendMode === 'Screen')
  step('the blending mode changes', (await info(SHAPE)).blendMode === 'Screen', (await info(SHAPE)).blendMode)
  await kit.getByTestId('motion-parent').selectOption(String(SOLID))
  await until(async () => (await info(SHAPE)).parent === SOLID)
  step('the parent changes', (await info(SHAPE)).parent === SOLID, String((await info(SHAPE)).parent))
  await undo()
  await until(async () => (await info(SHAPE)).parent === null)
  await undo()
  await until(async () => (await info(SHAPE)).blendMode === 'Normal')
  step('undo restores parent and blending mode', (await info(SHAPE)).parent === null && (await info(SHAPE)).blendMode === 'Normal')

  // 6. Values follow the current time; sections stay as they were left, per layer type.
  const ruler = await kit.getByTestId('motion-ruler').boundingBox()
  await page.mouse.click(ruler.x + ruler.width * (2 / 3), ruler.y + ruler.height - 4)
  const position = async () => (await query('prop.get', { layer: SHAPE, path: 'transform/position', time: 2 })).value[0]
  await until(async () => near(Number(await kit.getByTestId('motion-value-position-0').inputValue()), await position(), 0.01))
  step('values follow the current time', near(Number(await kit.getByTestId('motion-value-position-0').inputValue()), await position(), 0.01), `${await kit.getByTestId('motion-value-position-0').inputValue()} vs ${await position()}`)
  await kit.getByTestId('motion-section-transform-toggle').click()
  await select(SOLID)
  step("another type's Transform stays open", (await kit.getByTestId('motion-section-transform-toggle').getAttribute('aria-expanded')) === 'true')
  await select(SHAPE)
  step("the shape's Transform stays closed", (await kit.getByTestId('motion-section-transform-toggle').getAttribute('aria-expanded')) === 'false')
  await kit.getByTestId('motion-section-transform-toggle').click()

  // 7. The panel resizes from its left edge.
  const panel = await kit.getByTestId('motion-properties-panel').boundingBox()
  const edge = await kit.getByTestId('motion-properties-edge').boundingBox()
  await page.mouse.move(edge.x + 2, edge.y + 200)
  await page.mouse.down()
  for (let i = 1; i <= 8; i += 1) await page.mouse.move(edge.x + 2 - (80 * i) / 8, edge.y + 200)
  await page.mouse.up()
  const wider = await kit.getByTestId('motion-properties-panel').boundingBox()
  step('dragging the left edge widens the panel', wider.width > panel.width + 60, `${panel.width} → ${wider.width}`)

  // 8. Several layers: the first is shown, and the panel says so.
  await kit.getByTestId(`motion-layer-${TEXT}`).click({ modifiers: ['Shift'] })
  await until(async () => (await kit.getByTestId('motion-properties-many').count()) === 1)
  step('with several layers selected the panel says it shows the first', (await kit.getByTestId('motion-properties-many').textContent())?.includes('2'))
} catch (error) {
  failures += 1
  console.log(`FAIL ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
