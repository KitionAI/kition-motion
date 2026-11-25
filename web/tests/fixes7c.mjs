// The phase 7c hand-test fixes, by hand in a real browser through the development host: node tests/fixes7c.mjs [chromium|webkit]
// V after the Pen gives the transform box and Escape ends path editing; a new light keeps a turned
// 3D layer readable; a text layer typed into from the panel is named after its whole text; new
// shape, null and adjustment layers have Chinese names; the Pen's hint fits the status bar; a new
// text animator comes into view; cameras have no Align section; the Loop button has its label; View
// ▸ 3D View shows the keys that work; New Folder takes no key of New Mask's; Shift+8 and the
// full-width ＊ add a marker; the switch columns' heads show short names on request. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const TEXT = 73
const status = async () => ((await kit.getByTestId('motion-tool-status').count()) ? kit.getByTestId('motion-tool-status').textContent() : '')
const editingPath = async () => (await status()).includes('正在编辑路径')
const newest = (before, layers) => layers.find((layer) => !before.some((old) => old.id === layer.id))
/** Layer ▸ New ▸ a row of the menu bar. */
async function newLayer(command) {
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[data-kit-ui-id="menu:Layer/New"]').hover()
  await kit.locator(`[role=menu] [data-testid="motion-cmd-${command}"]`).first().click()
}
const brightness = (rgb) => (rgb[0] + rgb[1] + rgb[2]) / 3

try {
  await h.open()

  // 1. The Pen draws a new shape layer; V gives the plain Selection tool (the transform box), and a
  // double-click on the path edits it until Escape.
  await kit.getByTestId('motion-tool-pen').click()
  let before = await h.layers()
  for (const [x, y] of [[200, 520], [400, 520], [300, 660]]) {
    const p = await h.at(x, y)
    await page.mouse.click(p.x, p.y)
    await page.waitForTimeout(120)
  }
  const close = await h.at(200, 520)
  await page.mouse.click(close.x, close.y)
  await h.until(async () => (await h.layers()).length === before.length + 1)
  const drawn = newest(before, await h.layers())
  h.step('the Pen makes a shape layer named in Chinese', /^形状图层 \d+$/.test(drawn?.name ?? ''), drawn?.name)
  await h.focusEditor()
  await page.keyboard.press('KeyV')
  await h.until(async () => (await kit.getByTestId('motion-tool-selection').getAttribute('aria-pressed')) === 'true')
  await page.waitForTimeout(400)
  h.step('V after the Pen is the plain Selection tool, not path editing', !(await editingPath()), await status())
  const edge = await h.at(300, 520)
  await page.mouse.dblclick(edge.x, edge.y)
  await h.until(editingPath)
  h.step('a double-click on the path edits it', await editingPath(), await status())
  await page.keyboard.press('Escape')
  await h.until(async () => !(await editingPath()))
  h.step('Escape ends path editing, as the status bar says', !(await editingPath()), await status())
  await h.undo()
  await h.until(async () => (await h.layers()).length === before.length)

  // 2. A new light keeps a 3D layer turned 40° about Y readable.
  await h.setUp([
    { command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } },
    { command: 'prop.set', params: { layer: SHAPE, path: 'transform/rotationY', value: 40 } },
  ])
  const at = (await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).value
  await page.waitForTimeout(600)
  const unlit = brightness(await h.rgb(at[0], at[1]))
  before = await h.layers()
  await newLayer('layer.newLight')
  await h.until(async () => (await h.layers()).length === before.length + 1)
  await page.waitForTimeout(800)
  const lit = brightness(await h.rgb(at[0], at[1]))
  h.step('a new light keeps a layer turned 40° readable (at least half as bright)', lit >= unlit * 0.5, `${Math.round(unlit)} → ${Math.round(lit)}`)
  h.step('the light is named in Chinese', newest(before, await h.layers())?.name === '灯光 1')
  // A camera's panel has no Align section.
  before = await h.layers()
  await newLayer('layer.newCamera')
  await h.until(async () => (await h.layers()).length === before.length + 1)
  const camera = newest(before, await h.layers())
  await h.until(async () => (await h.layer(camera.id))?.selected)
  await kit.getByTestId('motion-properties-panel').waitFor()
  await page.waitForTimeout(400)
  h.step('a camera’s Properties panel has no Align section', (await kit.getByTestId('motion-section-align').count()) === 0)
  // On the active camera, Reset 3D View shows only when it would change the camera.
  await kit.getByTestId('motion-menu-View').click()
  await kit.locator('[data-kit-ui-id="menu:View/3D View"]').hover()
  const front = kit.locator('[role=menu] [data-testid="motion-cmd-view.threeD.front"]').first()
  h.step('View ▸ 3D 视图 shows Alt+Shift+1 for Front', (await front.textContent()).includes('Alt+Shift+1'), await front.textContent())
  h.step('Reset 3D View is hidden on an untouched active camera', (await kit.locator('[role=menu] [data-testid="motion-cmd-view.reset3DView"]').count()) === 0)
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await h.undo()
  await h.until(async () => (await h.layers()).length === before.length)
  await h.undo()
  await h.until(async () => (await h.layers()).length === before.length - 1)

  // 3. A new text layer typed into from the panel is named after the whole text.
  before = await h.layers()
  await newLayer('layer.newText')
  await h.until(async () => (await h.layers()).length === before.length + 1)
  const text = newest(before, await h.layers())
  const content = kit.getByTestId('motion-text-content')
  await content.waitFor()
  await h.until(async () => content.evaluate((element) => document.activeElement === element))
  await page.keyboard.type('Hello world', { delay: 30 })
  await h.until(async () => (await h.layer(text.id))?.name === 'Hello world')
  h.step('typed into from the panel, the text layer is named after the whole text', (await h.layer(text.id))?.name === 'Hello world', (await h.layer(text.id))?.name)
  await h.undo()
  await h.until(async () => (await h.layer(text.id))?.name !== 'Hello world')
  await h.undo()
  await h.until(async () => (await h.layers()).length === before.length)

  // 4. New shape, null and adjustment layers have Chinese names.
  for (const [command, pattern, name] of [['layer.newShape', /^形状图层 \d+$/, '形状图层'], ['layer.newNull', /^空对象 \d+$/, '空对象'], ['layer.newAdjustment', /^调整图层 \d+$/, '调整图层']]) {
    before = await h.layers()
    await newLayer(command)
    await h.until(async () => (await h.layers()).length === before.length + 1)
    const made = newest(before, await h.layers())
    h.step(`Layer ▸ New names it ${name} n`, pattern.test(made?.name ?? ''), made?.name)
    await h.undo()
    await h.until(async () => (await h.layers()).length === before.length)
  }

  // 6. The Pen's hint on a shape layer fits the status bar, Draw mask included.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  await kit.getByTestId('motion-tool-pen').click()
  const hint = kit.getByTestId('motion-tool-status')
  await hint.waitFor()
  const fits = await hint.evaluate((element) => element.scrollWidth <= element.clientWidth + 1)
  h.step('the Pen’s hint fits the status bar at 1440 px', fits && (await hint.textContent()).includes('画蒙版'), await hint.textContent())
  await kit.getByTestId('motion-tool-selection').click()

  // 7. A new text animator comes into view in the panel.
  await kit.getByTestId(`motion-layer-${TEXT}`).locator('.motion-row-name').click()
  await kit.getByTestId('motion-section-text').waitFor()
  await kit.getByTestId('motion-text-animate').click()
  await kit.getByTestId('motion-text-animate-position').click()
  await kit.getByTestId('motion-animator-1').waitFor()
  const inView = async () => {
    const panel = await kit.getByTestId('motion-properties-body').boundingBox()
    const block = await kit.getByTestId('motion-animator-1').boundingBox()
    return panel && block && block.y >= panel.y - 1 && Math.min(block.y + block.height, panel.y + panel.height) - block.y >= Math.min(block.height, panel.height) - 2
  }
  h.step('a new text animator is scrolled into view', await h.until(inView, 30))
  await h.undo()

  // 8. The Loop button reads 循环.
  h.step('the Loop button has its label', (await kit.getByTestId('motion-loop').textContent()).includes('循环') && (await kit.getByTestId('motion-loop').getAttribute('title')) !== null)

  // 9. New Folder shows no key (Cmd+Alt+Shift+N is New Mask's).
  await kit.getByTestId('motion-menu-File').click()
  await kit.locator('[data-kit-ui-id="menu:File/New"]').hover()
  const folderRow = kit.locator('[role=menu] [data-testid="motion-cmd-project.newFolder"]').first()
  h.step('File ▸ 新建 ▸ 新建文件夹 shows no key', (await folderRow.locator('kbd').count()) === 0, await folderRow.textContent())
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')

  // 10. Shift+8 and the full-width ＊ add a marker.
  const markers = async () => (await h.query('marker.list', {})).length
  // With no layer selected (a click on the timeline's empty space), * marks the composition.
  const rows = await kit.getByTestId('motion-timeline-rows').boundingBox()
  await page.mouse.click(rows.x + rows.width * 0.6, rows.y + rows.height - 12)
  await h.until(async () => (await h.layers()).every((layer) => !layer.selected))
  const was = await markers()
  await h.focusEditor()
  await page.keyboard.press('Shift+Digit8')
  await h.until(async () => (await markers()) === was + 1)
  h.step('Shift+8 adds a marker', (await markers()) === was + 1)
  // An input method cannot be driven by the test: its key event (the full-width ＊, Shift held on 8) is sent as it would send it.
  await kit.locator('.kui-app').evaluate((app) => app.dispatchEvent(new KeyboardEvent('keydown', { key: '＊', code: 'Digit8', shiftKey: true, bubbles: true, cancelable: true })))
  await h.until(async () => (await markers()) === was + 2)
  h.step('the full-width ＊ of a Chinese input method adds one too', (await markers()) === was + 2)

  // Leftover: the switch columns' heads as short names (the Columns menu's last row), and back to icons.
  await kit.getByTestId('motion-column-header').click({ button: 'right' })
  await kit.getByTestId('motion-column-text').click()
  await h.until(async () => (await kit.getByTestId('motion-column-head-video').textContent()) === '视')
  h.step('Column names as text shows the switch columns’ short names', (await kit.getByTestId('motion-column-head-video').textContent()) === '视' && (await kit.getByTestId('motion-column-head-solo').textContent()) === '独')
  await kit.getByTestId('motion-column-header').click({ button: 'right' })
  h.step('its row shows it is on', (await kit.getByTestId('motion-column-text').getAttribute('aria-checked')) === 'true')
  await kit.getByTestId('motion-column-text').click()
  await h.until(async () => (await kit.getByTestId('motion-column-head-video').textContent()) === '')
  h.step('and off again: the icons', (await kit.getByTestId('motion-column-head-video').locator('svg').count()) === 1)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
