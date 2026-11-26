// The fixes after the round 7q hand test, by hand, in a real browser through the development host: node tests/fixes7q.mjs [chromium|webkit]
// 1. Layer ▸ Layer Styles and Layer ▸ Mask ▸ New Mask are unavailable (greyed, a click does
//    nothing) for layers that cannot take them: styles on nulls, cameras, lights and adjustment
//    layers, New Mask on nulls, cameras and lights; the Home tab's New Mask too, and an adjustment
//    layer's Properties offer no style to add. With several layers selected they follow what the
//    command does: styles when one of them takes styles (added to that one), New Mask when the
//    first selected layer takes masks. New Mask's shortcut on a null still says why.
// 2. The layer-styles sentence names every kind it refuses (zh and en).
// 3. A text layer named after its text goes back to the default text-layer name when its text is
//    emptied in the Source Text box (Undo and Redo bring the names back with the text); a name
//    given by hand stays; the same when its text is deleted with the Text tool on the canvas.
//    Results are read from the engine (read-only queries), the elements and their text. Prints
//    PASS or FAIL per step.
import { readFileSync } from 'node:fs'
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27

/** Opens the sample afresh in `locale` (the harness opens it in zh-CN). */
async function reopen(locale) {
  const url = new URL(page.url())
  url.searchParams.set('locale', locale)
  await page.goto(url.toString())
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
}
const ids = async () => (await h.layers()).map((layer) => layer.id)
const names = async () => (await h.layers()).map((layer) => layer.name)
const nameOf = async (id) => (await h.layer(id))?.name
const revision = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).revision
const textOf = async (id) => {
  const read = await h.query('prop.get', { layer: id, path: 'text/sourceText' })
  const value = read?.value
  return typeof value === 'string' ? value : (value?.text ?? '')
}
const toasts = () => kit.locator('.kui-toast').count()
const toastAfter = async (count) => {
  await h.until(async () => (await toasts()) > count, 40)
  return ((await kit.locator('.kui-toast').last().textContent()) ?? '').trim()
}
const closeToasts = async () => {
  for (let i = 0; i < 5 && (await toasts()) > 0; i += 1) await kit.locator('.kui-toast').last().getByRole('button').first().click().catch(() => undefined)
}
/** A new layer of `kind` from the transport's New Layer menu (selected); its id. */
const newLayer = async (kind) => {
  const before = await ids()
  await kit.getByTestId('motion-new-layer').click()
  await kit.getByTestId(`motion-new-${kind}`).click()
  await h.until(async () => (await ids()).length === before.length + 1)
  const id = (await ids()).find((each) => !before.includes(each))
  await h.until(async () => (await h.layer(id))?.selected)
  await page.waitForTimeout(300)
  return id
}
/** A click on a layer's name in the timeline (with `modifiers`: added to the selection). */
const pick = async (id, modifiers = []) => {
  await kit.getByTestId(`motion-layer-${id}`).locator('.motion-row-name').first().click({ modifiers })
  await page.waitForTimeout(300)
}
const selectedIds = async () => (await h.layers()).filter((layer) => layer.selected).map((layer) => layer.id)
/** The timeline's empty top-left corner: a field loses the focus. */
const away = async () => {
  await kit.getByTestId('motion-timeline').click({ position: { x: 5, y: 5 } }).catch(() => undefined)
  await page.waitForTimeout(500)
}
const MASK_ROW = ['menu:Layer/Mask', 'command:layer.addMask']
const STYLE_ROWS = ['dropShadow', 'innerShadow', 'outerGlow', 'innerGlow', 'bevelEmboss', 'satin', 'colorOverlay', 'gradientOverlay', 'stroke'].map((id) => ['menu:Layer/Layer Styles', `command:layer.style.${id}`])
/**
 * Opens Layer ▸ `submenu`; the rows asked for, each `{ row, disabled, opacity }` (`disabled`: the
 * row takes no click and looks it, faded as the frame's disabled rows are); the menu stays open.
 */
const rows = async (submenu, wanted) => {
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator(`[role=menu] [data-kit-ui-id="${submenu}"]`).first().hover()
  await kit.locator(`[role=menu] [data-kit-ui-id="${wanted[0]}"]`).last().waitFor()
  const out = []
  for (const row of wanted) {
    const element = kit.locator(`[role=menu] [data-kit-ui-id="${row}"]`).last()
    const opacity = Number(await element.evaluate((node) => getComputedStyle(node).opacity))
    out.push({ row, disabled: (await element.isDisabled()) && opacity < 1, opacity })
  }
  return out
}
const closeMenus = async () => {
  for (let i = 0; i < 3 && (await kit.locator('[role=menu]').count()) > 0; i += 1) {
    await page.keyboard.press('Escape')
    await page.waitForTimeout(150)
  }
}
/** A real mouse click on Layer ▸ `submenu` ▸ `row` (an open menu); then the menus closed. */
const clickRow = async (submenu, row) => {
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator(`[role=menu] [data-kit-ui-id="${submenu}"]`).first().hover()
  // A greyed row takes no Playwright click: the mouse is moved there and pressed as a person would.
  const box = await kit.locator(`[role=menu] [data-kit-ui-id="${row}"]`).last().boundingBox()
  if (box) {
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2)
  }
  await page.waitForTimeout(800)
  await closeMenus()
}
const stylesOf = async (id) => ((await h.query('layer.style.list', { layer: id }))?.styles ?? []).map((style) => style.style)

try {
  await h.open()

  // 1. Unavailable rows, in both languages.
  for (const locale of ['zh-CN', 'en']) {
    await reopen(locale)
    for (const [kind, styles, masks] of [
      ['null', false, false],
      ['camera', false, false],
      ['light', false, false],
      ['adjustment', false, true],
    ]) {
      const id = await newLayer(kind)
      const maskRow = (await rows(MASK_ROW[0], [MASK_ROW[1]]))[0]
      await closeMenus()
      const styleRows = await rows(STYLE_ROWS[0][0], STYLE_ROWS.map(([, row]) => row))
      await closeMenus()
      h.step(`${locale}: ${masks ? '(kept) ' : ''}${kind}: Layer ▸ Mask ▸ New Mask is ${masks ? 'available' : 'unavailable (greyed)'}`, maskRow.disabled === !masks, JSON.stringify(maskRow))
      h.step(`${locale}: ${kind}: every Layer ▸ Layer Styles row is unavailable (greyed)`, styleRows.every((row) => row.disabled), styleRows.filter((row) => !row.disabled).map((row) => row.row).join(', '))
      // A click on the greyed rows does nothing and says nothing.
      await closeToasts()
      const before = await revision()
      const count = await toasts()
      await clickRow(...STYLE_ROWS[0])
      if (!masks) await clickRow(...MASK_ROW)
      h.step(`${locale}: ${kind}: a click on the greyed rows changes nothing and shows no refusal`, (await revision()) === before && (await toasts()) === count && (await stylesOf(id)).length === 0, `revision ${before} → ${await revision()} · notices ${count} → ${await toasts()}`)
      if (kind === 'null') {
        // The Home tab's New Mask button too.
        await kit.getByTestId('ribbon-tab-home').click()
        await kit.getByTestId('ribbon-drop-home').waitFor()
        const home = kit.getByTestId('ribbon-drop-home').locator('[data-kit-ui-id="command:layer.addMask"]').first()
        const homeDisabled = (await home.count()) ? await home.isDisabled() : null
        h.step(`${locale}: null: the Home tab's New Mask is unavailable`, homeDisabled === true, String(homeDisabled))
        await page.keyboard.press('Escape')
        await page.waitForTimeout(200)
        // New Mask's shortcut still says why.
        await h.focusEditor()
        const shown = await toasts()
        await page.keyboard.press(`${h.primary}+Alt+Shift+KeyN`)
        const said = await toastAfter(shown)
        const sentence = locale === 'en' ? 'Masks go only on layers with a picture, not on cameras, lights, null objects or 3D models.' : '蒙版只能加在有画面的图层上，摄像机、灯光、空对象和 3D 模型都不行。'
        h.step(`${locale}: (kept) null: Cmd/Ctrl+Alt+Shift+N says “${sentence}”`, said === sentence, said)
        await closeToasts()
      }
      if (kind === 'adjustment') {
        // Its Properties offer no style to add (it would be refused).
        const add = kit.getByTestId('motion-style-add')
        const offered = (await add.count()) > 0 && !(await add.isDisabled())
        h.step(`${locale}: adjustment layer: the Properties panel offers no layer style to add`, !offered, `Add style buttons ${await add.count()}`)
      }
    }

    // Several layers: the shape layer and a null.
    const nul = await newLayer('null')
    await pick(SHAPE, [h.primary])
    await h.until(async () => (await selectedIds()).length === 2)
    const nullFirst = (await rows(MASK_ROW[0], [MASK_ROW[1]]))[0]
    await closeMenus()
    const stylesMixed = (await rows(STYLE_ROWS[0][0], [STYLE_ROWS[0][1]]))[0]
    await closeMenus()
    h.step(`${locale}: null then shape layer selected: New Mask (it adds to the first, the null) is unavailable`, nullFirst.disabled === true, `${JSON.stringify(nullFirst)} · selected ${(await selectedIds()).join(', ')}`)
    h.step(`${locale}: (kept) null then shape layer selected: Drop Shadow (it goes on the shape layer) is available`, stylesMixed.disabled === false, JSON.stringify(stylesMixed))
    await clickRow(...STYLE_ROWS[0])
    await h.until(async () => (await stylesOf(SHAPE)).includes('dropShadow'))
    h.step(`${locale}: (kept) Drop Shadow with both selected goes on the shape layer only`, (await stylesOf(SHAPE)).includes('dropShadow') && (await stylesOf(nul)).length === 0, `${(await stylesOf(SHAPE)).join(', ')} · null ${(await stylesOf(nul)).join(', ')}`)
    await pick(SHAPE)
    await pick(nul, [h.primary])
    await h.until(async () => (await selectedIds()).length === 2)
    const shapeFirst = (await rows(MASK_ROW[0], [MASK_ROW[1]]))[0]
    await closeMenus()
    h.step(`${locale}: (kept) shape layer then null selected: New Mask is available`, shapeFirst.disabled === false, JSON.stringify(shapeFirst))
  }

  // 2. The layer-styles sentence names every kind it refuses (no editor path reaches it any more:
  // its rows are greyed; it is the editor's sentence for the engine's refusal, from its strings).
  const strings = readFileSync(new URL('../src/strings.ts', import.meta.url), 'utf8')
  for (const sentence of [
    '"error.noStyles": "Layer styles go only on footage, solid, pre-comp, text and shape layers, not on cameras, lights, null objects or adjustment layers."',
    '"error.noStyles": "图层样式只能加在素材、纯色、预合成、文本和形状图层上，摄像机、灯光、空对象和调整图层都不行。"',
  ]) {
    h.step(`the layer-styles sentence reads ${sentence.slice(19)}`, strings.includes(sentence))
  }

  // 3. A text layer emptied in the Source Text box.
  for (const [locale, plain] of [
    ['zh-CN', '文本'],
    ['en', 'Text'],
  ]) {
    await reopen(locale)
    const isDefault = (name) => name === plain || new RegExp(`^${plain} \\d+$`).test(name ?? '')
    const text = await newLayer('text')
    const box = kit.getByTestId('motion-text-content')
    await box.waitFor()
    await box.click()
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.type('ab')
    await h.until(async () => (await nameOf(text)) === 'ab')
    await away()
    h.step(`${locale}: (set-up) New Layer ▸ Text, “ab” typed, the timeline clicked: the layer is named “ab”`, (await nameOf(text)) === 'ab', await nameOf(text))
    await box.click()
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.press('Backspace')
    await away()
    await h.until(async () => (await textOf(text)) === '' && isDefault(await nameOf(text)))
    const row = ((await kit.getByTestId(`motion-layer-${text}`).locator('.motion-row-name').first().textContent()) ?? '').trim()
    h.step(`${locale}: select all, delete, click the timeline: the empty layer is named “${plain}” again (the timeline too), not “ab”`, (await textOf(text)) === '' && isDefault(await nameOf(text)) && row.includes(plain), `${JSON.stringify(await textOf(text))} · ${await nameOf(text)} · row ${row}`)
    await h.undo()
    await h.until(async () => (await textOf(text)) === 'ab')
    h.step(`${locale}: (kept) Cmd/Ctrl+Z brings back “ab” and its name with it`, (await textOf(text)) === 'ab' && (await nameOf(text)) === 'ab', `${JSON.stringify(await textOf(text))} · ${await nameOf(text)}`)
    await h.focusEditor()
    await page.keyboard.press(`${h.primary}+Shift+KeyZ`)
    await h.until(async () => (await textOf(text)) === '')
    h.step(`${locale}: Cmd/Ctrl+Shift+Z empties it again, named “${plain}”`, (await textOf(text)) === '' && isDefault(await nameOf(text)), `${JSON.stringify(await textOf(text))} · ${await nameOf(text)}`)

    // A name given by hand stays.
    const named = await newLayer('text')
    const nameField = kit.getByTestId('motion-layer-name')
    await nameField.waitFor()
    await nameField.click()
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.type('Title')
    await page.keyboard.press('Enter')
    await h.until(async () => (await nameOf(named)) === 'Title')
    await box.click()
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.type('cd')
    await h.until(async () => (await textOf(named)) === 'cd')
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.press('Backspace')
    await away()
    await h.until(async () => (await textOf(named)) === '')
    h.step(`${locale}: (kept) a layer named “Title” by hand keeps its name when its text is emptied`, (await textOf(named)) === '' && (await nameOf(named)) === 'Title', `${JSON.stringify(await textOf(named))} · ${await nameOf(named)}`)

    // Typing on the canvas: the Text tool in an existing text layer named after its text, all of
    // it selected and deleted, Esc: the layer stays, named “Text” again; Undo brings both back.
    await reopen(locale)
    const typedOn = await newLayer('text')
    await box.waitFor()
    await box.click()
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.type('ab')
    await h.until(async () => (await nameOf(typedOn)) === 'ab')
    await away()
    const position = (await h.query('prop.get', { layer: typedOn, path: 'transform/position' }))?.value ?? [960, 540]
    await kit.getByTestId('motion-tool-text').click()
    const spot = await h.at(position[0], position[1] - 15)
    await page.mouse.click(spot.x, spot.y)
    await page.waitForTimeout(400)
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.press('Backspace')
    await h.until(async () => (await textOf(typedOn)) === '')
    await page.keyboard.press('Escape')
    await page.waitForTimeout(400)
    await kit.getByTestId('motion-tool-selection').click()
    const emptied = await nameOf(typedOn)
    h.step(`${locale}: Text tool in “ab”, select all, delete, Esc: the layer stays, empty, named “${plain}”`, (await ids()).includes(typedOn) && (await textOf(typedOn)) === '' && isDefault(emptied), `${JSON.stringify(await textOf(typedOn))} · ${emptied}`)
    await h.undo()
    await h.until(async () => (await textOf(typedOn)) === 'ab')
    h.step(`${locale}: (kept) Cmd/Ctrl+Z brings back “ab” named “ab”`, (await textOf(typedOn)) === 'ab' && (await nameOf(typedOn)) === 'ab', `${JSON.stringify(await textOf(typedOn))} · ${await nameOf(typedOn)}`)
  }
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
