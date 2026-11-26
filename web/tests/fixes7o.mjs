// The fixes after the round 7o hand test, by hand, in a real browser through the development host: node tests/fixes7o.mjs [chromium|webkit]
// 1. A refusal's toast is in the interface's language only: the editor's own sentence, with no
//    engine text or command id under it (zh and en). 2. The Text tool clicked on the canvas and
//    left without typing (Esc, or a click elsewhere) leaves no undo step: Cmd/Ctrl+Z undoes what
//    was done before the click, and Redo does not bring the empty layer back. 3. Cmd/Ctrl+Z in a
//    number, name or Source Text field with nothing typed in this visit is the editor's Undo (and
//    Redo likewise); the field then shows the document's value and commits nothing older. Results
//    are read from the engine (read-only queries), the elements and their text. Prints PASS or FAIL per step.
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
/** The test id of the element with the focus inside the editor's frame. */
const focused = () => page.evaluate(() => document.getElementById('kit')?.contentDocument?.activeElement?.getAttribute('data-testid') ?? '')
const toastTexts = async () => (await kit.locator('.kui-toast').allTextContents()).join(' | ')
const opacityNow = async () => (await h.query('prop.get', { layer: SHAPE, path: 'transform/opacity' }))?.value
const sourceText = async (id) => {
  const value = (await h.query('prop.get', { layer: id, path: 'text/sourceText' }))?.value
  return typeof value === 'string' ? value : (value?.text ?? '')
}
const names = async () => (await h.layers()).map((layer) => layer.name)
const selectShape = async () => {
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
}
/** Clicks `field` and waits until it has the focus, typing nothing. */
const enter = async (field) => {
  await field.click()
  await h.until(async () => (await focused()) === (await field.getAttribute('data-testid')))
}
const press = async (keys) => {
  await page.keyboard.press(keys.replace('Primary', h.primary))
  await page.waitForTimeout(300)
}
/** A click on the timeline's empty top-left corner: the field loses the focus. */
const away = async () => {
  await kit.getByTestId('motion-timeline').click({ position: { x: 5, y: 5 } }).catch(() => undefined)
  await page.waitForTimeout(400)
}

try {
  await h.open()

  // 1. The Wiggler refused (keyframes of two properties): only the editor's sentence, in either language.
  for (const [locale, said, english] of [
    ['zh-CN', '请先选中同一个属性的至少两个关键帧。', false],
    ['en', 'Select two or more keyframes of one property first.', true],
  ]) {
    await reopen(locale)
    await h.setUp([
      { command: 'prop.set', params: { layer: SHAPE, path: 'transform/position', value: [500, 300], time: 1 } },
      { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/opacity', value: 100, time: 0 } },
      { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/opacity', value: 40, time: 1 } },
    ])
    await selectShape()
    await h.focusEditor()
    await page.keyboard.press('KeyU')
    const mark = (path) => kit.getByTestId(`motion-key-${SHAPE}-${path}-1000`)
    await mark('transform.position').waitFor()
    await mark('transform.opacity').waitFor()
    await mark('transform.position').click()
    await mark('transform.opacity').click({ modifiers: ['Shift'] })
    await h.until(async () => (await mark('transform.opacity').getAttribute('aria-selected')) === 'true')
    const warnings = []
    const listen = (message) => message.type() === 'warning' && warnings.push(message.text())
    page.on('console', listen)
    const before = await kit.locator('.kui-toast').count()
    await kit.getByTestId('motion-menu-Animation').click()
    await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
    await kit.locator('[role=menu] [data-testid="motion-control-wiggler"]').first().click()
    await kit.getByTestId('motion-wiggler-dialog').waitFor()
    await kit.getByTestId('motion-wiggler-dialog').getByRole('button', { name: english ? 'OK' : '确定' }).click()
    await h.until(async () => (await kit.locator('.kui-toast').count()) > before, 40)
    const text = (await kit.locator('.kui-toast').last().textContent()) ?? ''
    await h.until(async () => warnings.some((line) => line.includes('keyframe.wiggle')), 20)
    page.off('console', listen)
    h.step(`${locale}: Wiggler on keyframes of two properties: the toast says “${said}” and nothing else (no engine sentence, no command id)`, text.trim() === said && !/invalid parameters|keyframe\.wiggle|select at least/i.test(text), text)
    h.step(`${locale}: the engine's own words go to the developer console`, warnings.some((line) => line.includes('keyframe.wiggle')), warnings.join(' | '))
    await page.keyboard.press('Escape')
  }

  // 2. The Text tool clicked on the canvas and left without typing: no undo step at all.
  for (const [locale, end] of [
    ['zh-CN', 'Escape'],
    ['en', 'click'],
  ]) {
    await reopen(locale)
    await selectShape()
    const opacity = kit.getByTestId('motion-value-opacity-0')
    await opacity.waitFor()
    await enter(opacity)
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.type('60')
    await page.keyboard.press('Enter')
    await h.until(async () => (await opacityNow()) === 60)
    h.step(`${locale}: (set-up) Opacity typed 60 and Enter`, (await opacityNow()) === 60, JSON.stringify(await opacityNow()))
    const layersBefore = await names()
    await kit.getByTestId('motion-tool-text').click()
    const spot = await h.at(950, 600)
    await page.mouse.click(spot.x, spot.y)
    await h.until(async () => (await h.layers()).length === layersBefore.length + 1)
    h.step(`${locale}: (set-up) a click with the Text tool makes a layer to type in`, (await h.layers()).length === layersBefore.length + 1, (await names()).join(', '))
    await page.waitForTimeout(300)
    if (end === 'Escape') await page.keyboard.press('Escape')
    else {
      const elsewhere = await h.at(200, 650)
      await page.mouse.click(elsewhere.x, elsewhere.y)
    }
    await h.until(async () => (await h.layers()).length === layersBefore.length)
    await kit.getByTestId('motion-tool-selection').click()
    h.step(`${locale}: (set-up) left with ${end === 'Escape' ? 'Esc' : 'a click elsewhere'}, the empty layer goes`, JSON.stringify(await names()) === JSON.stringify(layersBefore), (await names()).join(', '))
    await h.undo()
    await h.until(async () => (await opacityNow()) !== 60)
    const undone = (await opacityNow()) === 100 && JSON.stringify(await names()) === JSON.stringify(layersBefore)
    h.step(`${locale}: then Cmd/Ctrl+Z undoes what was done before the click (Opacity 100), and no empty text layer comes back`, undone, `${JSON.stringify(await opacityNow())} · ${(await names()).join(', ')}`)
    await h.focusEditor()
    await press('Primary+Shift+KeyZ')
    await h.until(async () => (await opacityNow()) === 60)
    await press('Primary+Shift+KeyZ')
    const after = await h.details()
    h.step(`${locale}: Redo brings Opacity 60 back, never the empty layer, and then has nothing left`, undone && (await opacityNow()) === 60 && JSON.stringify(await names()) === JSON.stringify(layersBefore) && after.canRedo === false, `${JSON.stringify(await opacityNow())} · ${(await names()).join(', ')} · canRedo ${after.canRedo}`)
  }

  // 3. Cmd/Ctrl+Z in a field with nothing typed in this visit is the editor's Undo.
  await reopen('zh-CN')
  await selectShape()
  const opacity = kit.getByTestId('motion-value-opacity-0')
  await opacity.waitFor()
  await enter(opacity)
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('50')
  await page.keyboard.press('Enter')
  await h.until(async () => (await opacityNow()) === 50)
  h.step('(set-up) Opacity typed 50 and Enter', (await opacityNow()) === 50, JSON.stringify(await opacityNow()))
  await enter(opacity)
  await press('Primary+KeyZ')
  await h.until(async () => (await opacityNow()) === 100 && (await opacity.inputValue()) === '100')
  const fieldUndid = (await opacityNow()) === 100 && (await opacity.inputValue()) === '100' && (await focused()) === 'motion-value-opacity-0'
  h.step('number field, nothing typed: Cmd/Ctrl+Z is the editor’s Undo (Opacity 100), and the field shows 100', fieldUndid, `${JSON.stringify(await opacityNow())} · ${await opacity.inputValue()} · focus ${await focused()}`)
  await press('Primary+Shift+KeyZ')
  await h.until(async () => (await opacityNow()) === 50 && (await opacity.inputValue()) === '50')
  const fieldRedid = fieldUndid && (await opacityNow()) === 50
  h.step('and Cmd/Ctrl+Shift+Z there is the editor’s Redo: 50 in the document and the field', fieldRedid && (await opacity.inputValue()) === '50', `${JSON.stringify(await opacityNow())} · ${await opacity.inputValue()}`)
  await away()
  h.step('a click elsewhere then commits nothing: still 50', fieldRedid && (await opacityNow()) === 50 && (await opacity.inputValue()) === '50', `${JSON.stringify(await opacityNow())} · ${await opacity.inputValue()}`)
  await enter(opacity)
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('7')
  await h.until(async () => (await opacity.inputValue()) === '7')
  await press('Primary+KeyZ')
  const fieldBack = await opacity.inputValue()
  const valueBack = await opacityNow()
  await press('Primary+KeyZ')
  await h.until(async () => (await opacityNow()) === 100)
  h.step('typed 7: the first Cmd/Ctrl+Z takes the typing back (50, nothing undone), the second is the editor’s Undo (100)', fieldBack === '50' && valueBack === 50 && (await opacityNow()) === 100 && (await opacity.inputValue()) === '100', `${fieldBack} (${valueBack}) → ${await opacity.inputValue()} (${JSON.stringify(await opacityNow())})`)
  await away()
  h.step('a click elsewhere then: Opacity stays 100, the 50 the field showed before is not committed', (await opacityNow()) === 100 && (await opacity.inputValue()) === '100', `${JSON.stringify(await opacityNow())} · ${await opacity.inputValue()}`)

  // The layer's name field.
  const nameField = kit.getByTestId('motion-layer-name')
  const original = await nameField.inputValue()
  await enter(nameField)
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('Box')
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.layer(SHAPE))?.name === 'Box')
  h.step('(set-up) the layer renamed Box in its name field', (await h.layer(SHAPE))?.name === 'Box', (await h.layer(SHAPE))?.name)
  await enter(nameField)
  await press('Primary+KeyZ')
  await h.until(async () => (await h.layer(SHAPE))?.name === original && (await nameField.inputValue()) === original)
  h.step(`name field, nothing typed: Cmd/Ctrl+Z is the editor’s Undo, and the field shows “${original}”`, (await h.layer(SHAPE))?.name === original && (await nameField.inputValue()) === original, `${(await h.layer(SHAPE))?.name} · ${await nameField.inputValue()}`)
  await away()
  h.step('a click elsewhere then renames nothing', (await h.layer(SHAPE))?.name === original, (await h.layer(SHAPE))?.name)

  // The Source Text box: another text layer's text never comes into this one.
  const newText = async () => {
    const before = (await h.layers()).map((layer) => layer.id)
    await kit.getByTestId('motion-new-layer').click()
    await kit.getByTestId('motion-new-text').click()
    await h.until(async () => (await h.layers()).length === before.length + 1)
    await h.until(async () => (await focused()) === 'motion-text-content')
    return (await h.layers()).find((layer) => !before.includes(layer.id))
  }
  const alpha = await newText()
  await page.keyboard.type('Alpha')
  await h.until(async () => (await sourceText(alpha.id)) === 'Alpha')
  await h.focusEditor()
  const beta = await newText()
  const betaStart = await sourceText(beta.id)
  await page.keyboard.type('Beta')
  await h.until(async () => (await sourceText(beta.id)) === 'Beta')
  await away()
  const box = kit.getByTestId('motion-text-content')
  await enter(box)
  await press('Primary+KeyZ')
  await h.until(async () => (await sourceText(beta.id)) === betaStart && (await box.inputValue()) === betaStart)
  const boxUndid = (await sourceText(beta.id)) === betaStart && (await box.inputValue()) === betaStart
  h.step(`Source Text box, nothing typed: Cmd/Ctrl+Z is the editor’s Undo (the typing “Beta” undone), and the box shows “${betaStart}”`, boxUndid, `${JSON.stringify(await sourceText(beta.id))} · ${JSON.stringify(await box.inputValue())}`)
  h.step('the other text layer keeps its text through that Undo', boxUndid && (await sourceText(alpha.id)) === 'Alpha', await sourceText(alpha.id))
  if ((await focused()) === 'motion-text-content') {
    await press('Primary+Shift+KeyZ')
    await h.until(async () => (await sourceText(beta.id)) === 'Beta' && (await box.inputValue()) === 'Beta')
  }
  const boxRedid = boxUndid && (await sourceText(beta.id)) === 'Beta'
  h.step('Cmd/Ctrl+Shift+Z there is the editor’s Redo: Beta again in the layer and the box', boxRedid && (await box.inputValue()) === 'Beta', `${JSON.stringify(await sourceText(beta.id))} · ${JSON.stringify(await box.inputValue())}`)
  await away()
  h.step('a click elsewhere then changes nothing', boxRedid && (await sourceText(beta.id)) === 'Beta' && (await sourceText(alpha.id)) === 'Alpha', `${await sourceText(beta.id)} / ${await sourceText(alpha.id)}`)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
