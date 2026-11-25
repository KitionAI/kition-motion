// Colour labels and Collect Files by hand, in a real browser through the development host: node tests/labels.mjs [chromium|webkit]
// A layer's menu sets its colour label (its bar shows it) and Select Label Group selects the
// layers with the same label; a Project item's menu labels it (its row shows it); each label is
// one undo step. File ▸ Dependencies ▸ Collect Files… writes a copy of the project and its footage
// into a folder through the host. Round 7g: the menu bar's Layer ▸ 标签 lists every colour with its
// square, the current one ticked beside its square; the layer menu ticks the current one beside its
// square too. Results are read from the engine (`comp.info`, `project.summary`) and the host's
// files. Prints PASS or FAIL per step.
import { start, png } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const NIGHT = 4
const labelOf = async (id) => (await h.query('comp.info', {})).layers.find((layer) => layer.id === id)?.label
/** A layer row's menu ▸ 标签 ▸ a colour (or 选择标签组). */
async function layerLabel(id, row) {
  await kit.getByTestId(`motion-layer-${id}`).locator('.motion-row-name').click({ button: 'right' })
  await kit.getByTestId('motion-layer-menu').waitFor()
  await kit.getByTestId('motion-layer-menu-label').hover()
  await kit.getByTestId(`motion-layer-menu-label-${row}`).click()
}

try {
  await h.open()
  const startShape = await labelOf(SHAPE)
  // 1. Blue on Shape Layer 1: its bar shows it.
  await layerLabel(SHAPE, 'Blue')
  await h.until(async () => (await labelOf(SHAPE)) === 'Blue')
  h.step('the layer menu’s 标签 ▸ 蓝色 labels the layer', (await labelOf(SHAPE)) === 'Blue', await labelOf(SHAPE))
  const bar = kit.getByTestId(`motion-bar-${SHAPE}`)
  await h.until(async () => (await bar.getAttribute('data-label')) === 'Blue')
  const edge = await bar.evaluate((element) => getComputedStyle(element).boxShadow)
  h.step('its bar shows the label’s colour on its left edge', edge.includes('rgb(103, 125, 224)') && edge.includes('inset'), edge)
  await h.undo()
  await h.until(async () => (await labelOf(SHAPE)) === startShape)
  h.step('one undo puts the label back', (await labelOf(SHAPE)) === startShape)

  // 1b. Round 7g: the menu bar's Layer ▸ 标签 lists all 17 rows (Shape is Blue), each with its
  // colour's square, Blue ticked beside its square; the layer menu ticks it beside its square.
  await layerLabel(SHAPE, 'Blue')
  await h.until(async () => (await labelOf(SHAPE)) === 'Blue')
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[data-kit-ui-id="menu:Layer/Label"]').hover()
  const rows = kit.locator('[role=menu] [data-kit-ui-id^="control:label-"]')
  await h.until(async () => (await rows.count()) === 17)
  const shown = await rows.evaluateAll((items) =>
    items.map((item) => ({ id: item.getAttribute('data-kit-ui-id'), checked: item.getAttribute('aria-checked'), tick: item.querySelector('svg') != null, square: getComputedStyle(item.querySelector('span:last-of-type'), '::before').backgroundColor })),
  )
  const blue = shown.find((row) => row.id === 'control:label-Blue')
  h.step('Layer ▸ 标签 lists all 17 colours, the current one too', shown.length === 17 && blue != null, shown.map((row) => row.id.slice(14)).join(' '))
  h.step('each colour shows its square', shown.every((row) => row.square && row.square !== 'rgba(0, 0, 0, 0)') && blue?.square === 'rgb(103, 125, 224)' && new Set(shown.map((row) => row.square)).size === 17, shown.map((row) => row.square).join(' '))
  h.step('the current one (蓝色) is ticked beside its square, the others not', blue?.checked === 'true' && blue.tick && shown.filter((row) => row.checked === 'true').length === 1, JSON.stringify(blue))
  await page.keyboard.press('Escape')
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click({ button: 'right' })
  await kit.getByTestId('motion-layer-menu').waitFor()
  await kit.getByTestId('motion-layer-menu-label').hover()
  const current = kit.getByTestId('motion-layer-menu-label-Blue')
  await current.waitFor()
  const inMenu = await current.evaluate((item) => ({ tick: item.querySelector('.motion-label-icon[data-current] .motion-label-tick')?.textContent, square: getComputedStyle(item.querySelector('.motion-label-swatch')).backgroundColor }))
  h.step('the layer menu ticks 蓝色 beside its square (not instead of it)', inMenu.tick === '✓' && inMenu.square === 'rgb(103, 125, 224)', JSON.stringify(inMenu))
  await page.keyboard.press('Escape')
  await h.undo()
  await h.until(async () => (await labelOf(SHAPE)) === startShape)

  // 2. Select Label Group: Night and Shape Layer 1 both Blue; from Shape alone both get selected.
  await layerLabel(SHAPE, 'Blue')
  await layerLabel(NIGHT, 'Blue')
  await h.until(async () => (await labelOf(NIGHT)) === 'Blue' && (await labelOf(SHAPE)) === 'Blue')
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected && !(await h.layer(NIGHT))?.selected)
  await layerLabel(SHAPE, 'group')
  await h.until(async () => (await h.layer(NIGHT))?.selected && (await h.layer(SHAPE))?.selected)
  h.step('选择标签组 selects every layer with the same label', (await h.layer(NIGHT))?.selected && (await h.layer(SHAPE))?.selected && !(await h.layers()).find((layer) => layer.id !== SHAPE && layer.id !== NIGHT)?.selected)

  // 3. A Project item's label.
  await kit.getByTestId('motion-project-toggle').click()
  const comp = (await h.items()).find((item) => item.type === 'Composition')
  await kit.getByTestId(`motion-item-${comp.id}`).click({ button: 'right' })
  await kit.getByTestId('motion-item-menu-label').hover()
  await kit.getByTestId('motion-item-menu-label-Yellow').click()
  await h.until(async () => (await h.items()).find((item) => item.id === comp.id)?.label === 'Yellow')
  h.step('a Project item’s menu labels it', (await h.items()).find((item) => item.id === comp.id)?.label === 'Yellow')
  await h.until(async () => (await kit.getByTestId(`motion-item-label-${comp.id}`).getAttribute('data-label')) === 'Yellow')
  h.step('its row shows the colour', (await kit.getByTestId(`motion-item-label-${comp.id}`).evaluate((element) => getComputedStyle(element).backgroundColor)) === 'rgb(228, 216, 76)')
  await h.undo()
  await h.until(async () => (await h.items()).find((item) => item.id === comp.id)?.label !== 'Yellow')
  h.step('undone', (await h.items()).find((item) => item.id === comp.id)?.label !== 'Yellow')

  // 4. Collect Files: the project and its footage, copied into a folder.
  await h.putFile('drop.png', png(16, 16, [0, 200, 0]))
  await h.setUp([{ command: 'file.import', params: { paths: ['drop.png'] } }])
  await kit.getByTestId('motion-menu-File').click()
  await kit.locator('[data-kit-ui-id="menu:File/Dependencies"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-control-collect-files"]').first().click()
  await kit.getByTestId('motion-collect-dialog').waitFor()
  h.step('Collect Files… asks for a folder, in Chinese', ((await kit.getByTestId('motion-collect-dialog').textContent()) ?? '').includes('收集文件'))
  await kit.getByTestId('motion-collect-folder').fill('收集')
  await page.keyboard.press('Enter')
  const file = (path) => page.evaluate((name) => {
    const entry = window.devHost.files.get(name)
    return entry ? new TextDecoder().decode(new Uint8Array(entry.bytes).slice(0, 200000)) : null
  }, path)
  await h.until(async () => (await file('收集/Sample.kmotion')) != null, 100)
  const copy = (await file('收集/Sample.kmotion')) ?? ''
  h.step('the project’s copy is in the folder', copy.length > 0)
  h.step('pointing at its own footage', copy.includes('(Footage)/drop.png'))
  h.step('and the footage is beside it', (await file('收集/(Footage)/drop.png')) != null)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
