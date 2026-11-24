// The Character and Paragraph settings of a text layer by hand, in a real browser through the
// development host: node tests/text.mjs [chromium|webkit]
// With a text layer selected, the Properties panel's Text section reads back the whole text's
// style (a setting that differs across the characters shows as mixed) and a change applies to the
// whole text; while the text is edited on the canvas it reads and changes the selected characters
// only. Each change is one undo step. Results are read from the engine (`layer.tree`'s `text`, a
// read-only query). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const TEXT = 73
const text = async (layer = TEXT) => (await h.query('layer.tree', { layer, depth: 1 })).text
const near = (a, b, eps = 1e-3) => typeof a === 'number' && Math.abs(a - b) <= eps

/** The canvas view and a composition point on the page (as tests/canvas.mjs). */
async function at(cx, cy) {
  const [zoom, x, y] = (await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ').map(Number)
  const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
  return { x: surface.x + (cx - x) * zoom, y: surface.y + (cy - y) * zoom }
}
async function typeNumber(testId, value) {
  const field = kit.getByTestId(testId)
  await field.click()
  await field.fill(String(value))
  await field.press('Enter')
}

try {
  await h.open()
  await kit.getByTestId(`motion-layer-${TEXT}`).locator('.motion-row-name').click()
  await until(async () => (await h.layer(TEXT))?.selected)
  await kit.getByTestId('motion-text-controls').waitFor()
  const start = await text()
  step('the Text section reads the whole text’s settings back', (await kit.getByTestId('motion-text-size').inputValue()) === String(Math.round(start.character.size * 10) / 10) && start.range === null, JSON.stringify(start.character).slice(0, 160))

  // The font search matches the start of a word of the family's name: “Ser” finds Noto Serif alone.
  await kit.getByTestId('motion-text-font').locator('[role=combobox]').click()
  await page.keyboard.type('Ser')
  await page.waitForTimeout(300)
  const found = await kit.locator('.kui-combobox-panel [cmdk-item], .kui-combobox-panel [role=option]').allTextContents()
  step('the font search “Ser” lists the families with a word starting so', found.length >= 1 && found.every((name) => /(^|[\s-])ser/i.test(name)), JSON.stringify(found))
  await page.keyboard.press('Escape')

  // 1. Character settings on the whole text.
  await kit.getByTestId('motion-text-toggles-fauxBold').click()
  await until(async () => (await text()).character.fauxBold === true)
  step('Faux Bold applies to the whole text', (await text()).character.fauxBold === true && (await kit.getByTestId('motion-text-toggles-fauxBold').getAttribute('aria-pressed')) === 'true')
  await h.undo()
  await until(async () => (await text()).character.fauxBold === false)
  step('one undo takes it back', (await text()).character.fauxBold === false)
  await kit.getByTestId('motion-text-toggles-allCaps').click()
  await until(async () => (await text()).character.allCaps === true)
  step('All Caps', (await text()).character.allCaps === true)
  await kit.getByTestId('motion-text-toggles-superscript').click()
  await until(async () => (await text()).character.baseline === 'superscript')
  step('Superscript', (await text()).character.baseline === 'superscript')
  await typeNumber('motion-text-baselineShift', 12)
  await until(async () => near((await text()).character.baselineShift, 12))
  step('Baseline shift takes a typed value', near((await text()).character.baselineShift, 12))
  await typeNumber('motion-text-tracking', 80)
  await until(async () => near((await text()).character.tracking, 80))
  step('Tracking takes a typed value', near((await text()).character.tracking, 80))
  await typeNumber('motion-text-hScale', 150)
  await until(async () => near((await text()).character.hScale, 150))
  step('Horizontal scale takes a typed value', near((await text()).character.hScale, 150))
  await kit.getByTestId('motion-text-leading-auto').click()
  await until(async () => (await text()).character.leading !== 'auto')
  step('the Auto leading switch sets a leading of its own', typeof (await text()).character.leading === 'number', JSON.stringify((await text()).character.leading))
  await kit.getByTestId('motion-text-kerning').selectOption('optical')
  await until(async () => (await text()).character.kerning === 'optical')
  step('Kerning: optical', (await text()).character.kerning === 'optical')

  // 2. Paragraph settings.
  await kit.getByTestId('motion-text-align-justifyAll').click()
  await until(async () => (await text()).paragraph.justify === 'justifyAll')
  step('the seven alignments: Justify All', (await text()).paragraph.justify === 'justifyAll' && (await kit.getByTestId('motion-text-align-justifyAll').getAttribute('aria-pressed')) === 'true')
  await typeNumber('motion-text-indentFirst', 24)
  await typeNumber('motion-text-spaceAfter', 6)
  await until(async () => near((await text()).paragraph.indentFirst, 24) && near((await text()).paragraph.spaceAfter, 6))
  step('first-line indent and space after take typed values', near((await text()).paragraph.indentFirst, 24) && near((await text()).paragraph.spaceAfter, 6), JSON.stringify((await text()).paragraph))

  // 3. Mixed: two sizes in the text show as mixed; a typed size sets them all.
  await h.setUp([{ command: 'layer.setText', params: { layer: TEXT, range: [0, 3], size: 30 } }])
  await until(async () => (await text()).mixed.includes('size'))
  await until(async () => (await kit.getByTestId('motion-text-size').getAttribute('data-mixed')) === 'true')
  step('a size that differs across the text shows as mixed (empty, with a dash)', (await kit.getByTestId('motion-text-size').inputValue()) === '' && (await kit.getByTestId('motion-text-size').getAttribute('placeholder')) === '—')
  await typeNumber('motion-text-size', 64)
  await until(async () => near((await text()).character.size, 64))
  step('typing a size there sets every character’s', near((await text()).character.size, 64) && !(await text()).mixed.includes('size'))

  // 4. While editing on the canvas, the panel reads and changes the selected characters only.
  await kit.getByTestId('motion-tool-text').click()
  const before = (await h.layers()).map((layer) => layer.id)
  const spot = await at(640, 650)
  await page.mouse.click(spot.x, spot.y)
  await until(async () => (await h.layers()).length === before.length + 1)
  const made = (await h.layers()).find((layer) => !before.includes(layer.id))
  await page.waitForTimeout(200)
  await page.keyboard.type('Hello World')
  await until(async () => (await text(made.id))?.length === 11)
  for (let i = 0; i < 5; i += 1) await page.keyboard.press('Shift+ArrowLeft')
  await until(async () => JSON.stringify((await text(made.id))?.range) === '[6,11]')
  await kit.getByTestId('motion-text-scope').waitFor({ timeout: 3000 }).catch(() => null)
  step('editing with “World” selected, the panel says it acts on the selection', JSON.stringify((await text(made.id))?.range) === '[6,11]' && (await kit.getByTestId('motion-text-scope').count()) === 1, JSON.stringify((await text(made.id))?.range))
  await kit.getByTestId('motion-text-toggles-fauxItalic').click()
  await until(async () => (await text(made.id))?.character.fauxItalic === true)
  step('Faux Italic applies to the selected characters', (await text(made.id))?.character.fauxItalic === true)
  // The fill swatch opens its picker and it stays open while the text is edited.
  await kit.getByTestId('motion-text-fill-swatch').click()
  await page.waitForTimeout(600)
  step('while editing, the fill swatch’s colour picker stays open', await kit.getByTestId('motion-text-fill-popover').isVisible().catch(() => false))
  const hex = kit.getByTestId('motion-text-fill-picker-hex')
  await hex.fill('#ff3300')
  await hex.press('Enter')
  const coloured = async () => {
    const fill = (await text(made.id))?.character.fill ?? []
    return [1, 0.2, 0].every((part, at) => Math.abs((fill[at] ?? -1) - part) < 0.01)
  }
  await until(coloured)
  step('and colours the selected characters', await coloured(), JSON.stringify((await text(made.id))?.character.fill))
  await page.keyboard.press('Escape')
  await kit.getByTestId('motion-tool-selection').click()
  await until(async () => (await text(made.id))?.editing === false)
  const whole = await text(made.id)
  step('the whole text then reads Faux Italic as mixed', whole?.mixed.includes('fauxItalic') && whole?.character.fauxItalic === null, JSON.stringify(whole?.mixed))
  await kit.getByTestId('motion-text-toggles-fauxItalic').waitFor()
  step('and its switch shows half-pressed', (await kit.getByTestId('motion-text-toggles-fauxItalic').getAttribute('aria-pressed')) === 'mixed')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
