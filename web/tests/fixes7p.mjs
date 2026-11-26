// The fixes after the round 7p hand test, by hand, in a real browser through the development host: node tests/fixes7p.mjs [chromium|webkit]
// 1. A focused field taken out of the page with its layer (the editor's Undo pressed in the
//    Source Text box, a number field or the name field, with nothing typed, removes the layer):
//    the editor's keys work at once (Redo brings the layer back, Undo takes it again), with no
//    click first. 2. The Text tool: typing deleted again and left with Esc leaves no undo step:
//    Undo takes back what was done before the click, and never brings back an empty layer named
//    after the deleted text. 3. Enable Time Remapping (Cmd/Ctrl+Alt+T) on a shape layer, Add
//    Expression (Alt+Shift+=) with no property selected, and New Mask's shortcut on a null object
//    say why in the editor's words (zh and en); a layer style's row is unavailable on it (7q).
//    4. A Render Queue job that fails (the host cannot write its file) says so in the interface's
//    language: no host or engine text in the row or its tooltip; that text goes to the console.
//    Results are read from the engine (read-only queries), the elements and their text. Prints
//    PASS or FAIL per step.
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
/** The test id of the element with the focus inside the editor's frame, or its tag. */
const focused = () =>
  page.evaluate(() => {
    const active = document.getElementById('kit')?.contentDocument?.activeElement
    return active?.getAttribute('data-testid') || active?.className || active?.tagName || ''
  })
const names = async () => (await h.layers()).map((layer) => layer.name)
const ids = async () => (await h.layers()).map((layer) => layer.id)
const opacityNow = async () => (await h.query('prop.get', { layer: SHAPE, path: 'transform/opacity' }))?.value
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
/** A new layer of `kind` from the transport's New Layer menu; its id. */
const newLayer = async (kind) => {
  const before = await ids()
  await kit.getByTestId('motion-new-layer').click()
  await kit.getByTestId(`motion-new-${kind}`).click()
  await h.until(async () => (await ids()).length === before.length + 1)
  return (await ids()).find((id) => !before.includes(id))
}
const toastAfter = async (count) => {
  await h.until(async () => (await kit.locator('.kui-toast').count()) > count, 40)
  return ((await kit.locator('.kui-toast').last().textContent()) ?? '').trim()
}

try {
  await h.open()

  // 1. The field goes with its layer: the keys are the editor's at once.
  // The Source Text box (the 7p hand test: Redo did nothing until a click).
  const typed = await newLayer('text')
  await h.until(async () => (await focused()) === 'motion-text-content')
  await page.keyboard.type('xyz')
  await h.until(async () => (await h.layer(typed))?.name === 'xyz')
  await away()
  const box = kit.getByTestId('motion-text-content')
  await enter(box)
  await press('Primary+KeyZ')
  await h.until(async () => (await box.inputValue()) !== 'xyz')
  h.step('(set-up) Source Text box, nothing typed: the first Cmd/Ctrl+Z undoes the typing', (await box.inputValue()) !== 'xyz', JSON.stringify(await box.inputValue()))
  await press('Primary+KeyZ')
  await h.until(async () => !(await ids()).includes(typed))
  h.step('(set-up) the second removes the text layer, and its box goes with it', !(await ids()).includes(typed) && (await box.count()) === 0, `${(await names()).join(', ')} · box ${await box.count()}`)
  const keeper = await focused()
  await press('Primary+Shift+KeyZ')
  await h.until(async () => (await ids()).includes(typed))
  h.step('Source Text box gone with its layer: Cmd/Ctrl+Shift+Z right away brings the layer back (no click first)', (await ids()).includes(typed), `focus ${keeper} · ${(await names()).join(', ')}`)
  const redone = (await ids()).includes(typed)
  await press('Primary+KeyZ')
  await h.until(async () => !(await ids()).includes(typed))
  h.step('and Cmd/Ctrl+Z takes it again', redone && !(await ids()).includes(typed), (await names()).join(', '))

  // A number field of the Properties panel (a new null's Opacity).
  const nul = await newLayer('null')
  const opacity = kit.getByTestId('motion-value-opacity-0')
  await opacity.waitFor()
  await enter(opacity)
  await press('Primary+KeyZ')
  await h.until(async () => !(await ids()).includes(nul))
  h.step('(set-up) a new null’s Opacity field, nothing typed: Cmd/Ctrl+Z removes the null', !(await ids()).includes(nul), (await names()).join(', '))
  const afterNumber = await focused()
  await press('Primary+Shift+KeyZ')
  await h.until(async () => (await ids()).includes(nul))
  h.step('number field gone with its layer: Cmd/Ctrl+Shift+Z right away brings the null back', (await ids()).includes(nul), `focus ${afterNumber} · ${(await names()).join(', ')}`)

  // The layer's name field (a new shape layer).
  await reopen('zh-CN')
  const made = await newLayer('shape')
  await h.until(async () => (await h.layer(made))?.selected)
  const nameField = kit.getByTestId('motion-layer-name')
  await nameField.waitFor()
  await enter(nameField)
  await press('Primary+KeyZ')
  await h.until(async () => !(await ids()).includes(made))
  h.step('(set-up) a new shape layer’s name field, nothing typed: Cmd/Ctrl+Z removes the shape layer', !(await ids()).includes(made), (await names()).join(', '))
  const afterName = await focused()
  await press('Primary+Shift+KeyZ')
  await h.until(async () => (await ids()).includes(made))
  h.step('name field gone with its layer: Cmd/Ctrl+Shift+Z right away brings the shape layer back', (await ids()).includes(made), `focus ${afterName} · ${(await names()).join(', ')}`)

  // 2. The Text tool: "ab" typed, deleted again, Esc: no undo step of the visit stays.
  for (const locale of ['zh-CN', 'en']) {
    await reopen(locale)
    await selectShape()
    const field = kit.getByTestId('motion-value-opacity-0')
    await field.waitFor()
    await enter(field)
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
    await page.waitForTimeout(300)
    await page.keyboard.type('ab')
    await h.until(async () => (await names()).includes('ab'))
    h.step(`${locale}: (set-up) the Text tool’s layer is named after the typing (“ab”)`, (await names()).includes('ab'), (await names()).join(', '))
    await page.keyboard.press('Backspace')
    await page.keyboard.press('Backspace')
    await page.waitForTimeout(300)
    await page.keyboard.press('Escape')
    await h.until(async () => (await h.layers()).length === layersBefore.length)
    await kit.getByTestId('motion-tool-selection').click()
    h.step(`${locale}: (set-up) left empty with Esc, the layer goes`, JSON.stringify(await names()) === JSON.stringify(layersBefore), (await names()).join(', '))
    await h.undo()
    await h.until(async () => (await opacityNow()) !== 60 || (await h.layers()).length !== layersBefore.length)
    const undone = (await opacityNow()) === 100 && JSON.stringify(await names()) === JSON.stringify(layersBefore)
    h.step(`${locale}: the first Cmd/Ctrl+Z undoes what was done before the click (Opacity 100); no empty text layer (named “a” or anything) comes back`, undone, `${JSON.stringify(await opacityNow())} · ${(await names()).join(', ')}`)
    await h.focusEditor()
    await press('Primary+Shift+KeyZ')
    await h.until(async () => (await opacityNow()) === 60)
    const after = await h.details()
    h.step(`${locale}: Redo brings Opacity 60 back and then has nothing left (no typing, no empty layer)`, undone && (await opacityNow()) === 60 && JSON.stringify(await names()) === JSON.stringify(layersBefore) && after.canRedo === false, `${JSON.stringify(await opacityNow())} · ${(await names()).join(', ')} · canRedo ${after.canRedo}`)
  }

  // 3. Refusals reached with the keys and a layer the command cannot take: the editor's sentence alone.
  for (const [locale, remap, expression, masks, styles] of [
    ['zh-CN', '只有素材图层和合成图层能做时间重映射（影片、声音、图片、预合成）。', '请先在时间轴上选中一个属性，表达式会加在它上面。', '蒙版只能加在有画面的图层上，摄像机、灯光、空对象和 3D 模型都不行。', '图层样式只能加在素材、纯色、预合成、文本和形状图层上，摄像机、灯光、空对象和调整图层都不行。'],
    ['en', 'Only footage and composition layers can be time-remapped (movies, sounds, pictures, pre-comps).', 'Select a property in the timeline first: the expression goes on it.', 'Masks go only on layers with a picture, not on cameras, lights, null objects or 3D models.', 'Layer styles go only on footage, solid, pre-comp, text and shape layers, not on cameras, lights, null objects or adjustment layers.'],
  ]) {
    await reopen(locale)
    await selectShape()
    const warnings = []
    const listen = (message) => message.type() === 'warning' && warnings.push(message.text())
    page.on('console', listen)
    await h.focusEditor()
    let before = await kit.locator('.kui-toast').count()
    await page.keyboard.press(`${h.primary}+Alt+KeyT`)
    const remapSaid = await toastAfter(before)
    h.step(`${locale}: Cmd/Ctrl+Alt+T on a shape layer says “${remap}” and nothing else`, remapSaid === remap, remapSaid)
    await kit.locator('.kui-toast').last().getByRole('button').first().click().catch(() => undefined)
    await h.focusEditor()
    before = await kit.locator('.kui-toast').count()
    await page.keyboard.press('Alt+Shift+Equal')
    const expressionSaid = await toastAfter(before)
    h.step(`${locale}: Alt+Shift+= with no property selected says “${expression}” and nothing else`, expressionSaid === expression, expressionSaid)
    await h.until(async () => warnings.some((line) => line.includes('time remapping needs')) && warnings.some((line) => line.includes('prop.setExpression')), 20)
    page.off('console', listen)
    h.step(`${locale}: the engine’s own words for both go to the developer console`, warnings.some((line) => line.includes('time remapping needs')) && warnings.some((line) => line.includes('prop.setExpression')), warnings.join(' | '))
    // A null object selected: New Mask from its shortcut says why (since 7q its Layer ▸ Mask row
    // and the Layer ▸ Layer Styles rows are greyed for a null: fixes7q.mjs checks them, and the
    // reworded layer-styles sentence).
    const nul = await newLayer('null')
    await h.until(async () => (await h.layer(nul))?.selected)
    await kit.locator('.kui-toast').last().getByRole('button').first().click().catch(() => undefined)
    await h.focusEditor()
    before = await kit.locator('.kui-toast').count()
    await page.keyboard.press(`${h.primary}+Alt+Shift+KeyN`)
    const text = await toastAfter(before)
    h.step(`${locale}: New Mask (Cmd/Ctrl+Alt+Shift+N) on a null object says “${masks}” and nothing else`, text === masks, text)
    await kit.getByTestId('motion-menu-Layer').click()
    await kit.locator('[role=menu] [data-kit-ui-id="menu:Layer/Layer Styles"]').first().hover()
    const styleRow = kit.locator('[role=menu] [data-kit-ui-id="command:layer.style.dropShadow"]').last()
    await styleRow.waitFor()
    h.step(`${locale}: Layer ▸ Layer Styles ▸ Drop Shadow on a null object is unavailable (its sentence: ${styles})`, await styleRow.isDisabled())
    for (let i = 0; i < 3 && (await kit.locator('[role=menu]').count()) > 0; i += 1) await page.keyboard.press('Escape')
  }

  // 4. A Render Queue job the host cannot write: the row says so in the interface's language.
  for (const [locale, said] of [
    ['zh-CN', '失败：具体原因已写到开发者控制台。'],
    ['en', 'Failed: The reason is written to the developer console.'],
  ]) {
    await reopen(locale)
    // The host refuses to write anything into the folder `full` (as a full disk would).
    await page.evaluate(() => {
      const files = window.devHost.files
      const set = files.set.bind(files)
      files.set = (path, entry) => {
        if (String(path).includes('full/')) throw new Error('no space left on the device for full/')
        return set(path, entry)
      }
    })
    const warnings = []
    const listen = (message) => message.type() === 'warning' && warnings.push(message.text())
    page.on('console', listen)
    await kit.getByTestId('motion-menu-File').click()
    await kit.locator('[role=menu] [role=menuitem]', { hasText: locale === 'en' ? /^Export(?!…)/ : /^导出(?!…)/ }).first().hover()
    await kit.locator('[role=menu] [data-testid="motion-control-add-to-render-queue"]').first().click()
    await kit.getByTestId('motion-queue-panel').waitFor()
    const jobs = async () => (await h.query('renderQueue.list', {}))?.items ?? []
    await h.until(async () => (await jobs()).length === 1)
    const job = (await jobs())[0]
    await kit.getByTestId(`motion-queue-item-${job.id}-settings`).click()
    await kit.getByTestId('motion-job-dialog').waitFor()
    await kit.getByTestId('motion-job-span').selectOption('custom')
    await kit.getByTestId('motion-job-start').fill('0:00:00:00')
    await kit.getByTestId('motion-job-end').fill('0:00:00:03')
    await kit.getByTestId('motion-job-format').selectOption('gif')
    await kit.getByTestId('motion-job-size').selectOption('quarter')
    await kit.getByTestId('motion-job-name').fill('short')
    await kit.getByTestId('motion-job-folder').fill('full')
    await kit.getByTestId('motion-job-dialog-ok').click()
    await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
    await kit.getByTestId('motion-queue-render').click()
    const status = kit.getByTestId(`motion-queue-item-${job.id}-status`)
    await h.until(async () => ['done', 'failed'].includes(await status.getAttribute('data-state').catch(() => null)), 600)
    const row = ((await status.textContent()) ?? '').trim()
    const tip = await status.evaluate((element) => [...element.querySelectorAll('[title]')].map((each) => each.getAttribute('title')).join(' | '))
    h.step(`${locale}: (set-up) the job fails (the host cannot write into full/)`, (await status.getAttribute('data-state')) === 'failed', row)
    h.step(`${locale}: its row says “${said}”, no host or engine text in it or its tooltip`, row === said && !/no space|full\//.test(tip), `${row} · tooltip ${JSON.stringify(tip)}`)
    await h.until(async () => warnings.some((line) => line.includes('no space left')), 20)
    page.off('console', listen)
    h.step(`${locale}: the host’s own words go to the developer console`, warnings.some((line) => line.includes('no space left')), warnings.join(' | '))
  }
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
