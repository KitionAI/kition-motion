// The fixes after the round 7n hand test, by hand, in a real browser through the development host: node tests/fixes7m.mjs [chromium|webkit]
// 1. Cmd/Ctrl+Z in the Source Text box undoes only what was typed there since it took the focus,
//    and never writes another text layer's text into this one. 2. After a keyframe move is undone,
//    a click on that keyframe selects it once: its menu offers 编辑值…, and Wiggler says why it
//    cannot run. 3. (5 in the audit) A number field: Cmd/Ctrl+Z while typing goes back only to the
//    value it showed when it took the focus; Esc puts that value back and changes nothing. 4. The
//    Render Queue: a sequence folder name in either language counts as automatic, and the folder a
//    sequence job writes to is kept in the job (the same in every interface language). 5. A text
//    layer's default name in either language (numbered or not) follows what is typed, and the
//    Text tool names its layer as Layer ▸ New ▸ Text does. 6. The font box has no × that does
//    nothing; English says “Remove All Styles” in both places; a bare undefined name in an
//    expression says its line. Results are read from the engine (read-only queries), the elements
//    and their text. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const POSITION = 'transform/position'

/** Opens the sample afresh in `locale` (the harness opens it in zh-CN). */
async function reopen(locale) {
  const url = new URL(page.url())
  url.searchParams.set('locale', locale)
  await page.goto(url.toString())
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
}
const sourceText = async (id) => {
  const value = (await h.query('prop.get', { layer: id, path: 'text/sourceText' }))?.value
  return typeof value === 'string' ? value : (value?.text ?? '')
}
const nameOf = async (id) => (await h.layer(id))?.name
/** The test id of the element with the focus inside the editor's frame. */
const focused = () => page.evaluate(() => document.getElementById('kit')?.contentDocument?.activeElement?.getAttribute('data-testid') ?? '')
/** Layer ▸ New ▸ Text from the transport's New Layer menu; answers the new layer once its Source Text box has the focus. */
async function newText() {
  const before = (await h.layers()).map((layer) => layer.id)
  await kit.getByTestId('motion-new-layer').click()
  await kit.getByTestId('motion-new-text').click()
  await h.until(async () => (await h.layers()).length === before.length + 1)
  await h.until(async () => (await focused()) === 'motion-text-content')
  return (await h.layers()).find((layer) => !before.includes(layer.id))
}

try {
  await h.open()

  // 1. Cmd/Ctrl+Z in the Source Text box stays within this layer and this visit.
  const alpha = await newText()
  await page.keyboard.type('Alpha')
  await h.until(async () => (await sourceText(alpha.id)) === 'Alpha')
  await h.focusEditor()
  const second = await newText()
  const startText = await kit.getByTestId('motion-text-content').inputValue()
  await page.keyboard.type('xyz')
  await h.until(async () => (await sourceText(second.id)) === 'xyz')
  const seen = []
  for (let i = 0; i < 4; i += 1) {
    await page.keyboard.press(`${h.primary}+KeyZ`)
    await page.waitForTimeout(250)
    seen.push(await kit.getByTestId('motion-text-content').inputValue())
  }
  await h.until(async () => (await sourceText(second.id)) === startText)
  const secondText = await sourceText(second.id)
  h.step('Cmd/Ctrl+Z four times in the Source Text box: back to the text it had when it took the focus, never another layer’s', !seen.includes('Alpha') && seen.at(-1) === startText && secondText === startText && (await nameOf(second.id)) !== 'Alpha', `${JSON.stringify(seen)} · layer ${JSON.stringify(secondText)} (${await nameOf(second.id)})`)
  h.step('the other text layer keeps its text and name', (await sourceText(alpha.id)) === 'Alpha' && (await nameOf(alpha.id)) === 'Alpha', `${await sourceText(alpha.id)} (${await nameOf(alpha.id)})`)
  await page.keyboard.press(`${h.primary}+Shift+KeyZ`)
  await h.until(async () => (await kit.getByTestId('motion-text-content').inputValue()) === 'x')
  h.step('Cmd/Ctrl+Shift+Z in the box redoes the typing one step', (await kit.getByTestId('motion-text-content').inputValue()) === 'x', await kit.getByTestId('motion-text-content').inputValue())
  await h.focusEditor()

  // 2. A keyframe move undone, then a click on that keyframe: it is selected once.
  await h.setUp([{ command: 'prop.set', params: { layer: SHAPE, path: POSITION, value: [500, 300], time: 1 } }])
  const keyTimes = async () => ((await h.query('prop.get', { layer: SHAPE, path: POSITION })).keys ?? []).map((key) => Math.round((key.compTime ?? key.time) * 1000) / 1000)
  await h.until(async () => (await keyTimes()).includes(1))
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  await page.keyboard.press('KeyP')
  const mark = (ms) => kit.getByTestId(`motion-key-${SHAPE}-transform.position-${ms}`)
  await mark(1000).waitFor()
  const keysBefore = await keyTimes()
  await mark(1000).click()
  await h.until(async () => (await mark(1000).getAttribute('aria-selected')) === 'true')
  const from = await h.centre(mark(1000))
  await h.drag(from, { x: from.x + 60, y: from.y })
  await h.until(async () => !(await keyTimes()).includes(1))
  const moved = await keyTimes()
  h.step('(set-up) the keyframe at 1 s dragged later', !moved.includes(1) && moved.length === keysBefore.length, `${JSON.stringify(keysBefore)} → ${JSON.stringify(moved)}`)
  await h.undo()
  await h.until(async () => (await keyTimes()).includes(1))
  await mark(1000).waitFor()
  h.step('(set-up) one undo puts it back at 1 s', JSON.stringify(await keyTimes()) === JSON.stringify(keysBefore), JSON.stringify(await keyTimes()))
  await mark(1000).click()
  await page.waitForTimeout(400)
  await mark(1000).click({ button: 'right' })
  await kit.getByTestId('motion-key-menu').waitFor()
  await h.until(async () => (await kit.getByTestId('motion-key-menu-edit').count()) === 1, 30)
  const wigglerRow = kit.getByTestId('motion-key-menu-wiggler')
  h.step('then a click on it selects it once: its menu offers 编辑值…', (await kit.getByTestId('motion-key-menu-edit').count()) === 1, (await kit.getByTestId('motion-key-menu').textContent()) ?? '')
  h.step('and its Wiggler row says it needs two keyframes', ((await wigglerRow.textContent()) ?? '').includes('至少两个关键帧'), (await wigglerRow.textContent()) ?? '')
  await page.keyboard.press('Escape')
  await h.until(async () => (await kit.getByTestId('motion-key-menu').count()) === 0)
  const toasts = await kit.locator('.kui-toast').count()
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-control-wiggler"]').first().click()
  await h.until(async () => (await kit.locator('.kui-toast').count()) > toasts, 30)
  h.step('Animation ▸ Keyframe Assistant ▸ Wiggler then says why, and opens nothing', (await kit.getByTestId('motion-wiggler-dialog').count()) === 0 && ((await kit.locator('.kui-toast').last().textContent()) ?? '').includes('至少两个关键帧'), (await kit.locator('.kui-toast').last().textContent().catch(() => '')) ?? '')
  await page.keyboard.press('Escape')

  // 3. A number field: Cmd/Ctrl+Z while typing, and Esc.
  await reopen('zh-CN')
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  const opacity = kit.getByTestId('motion-value-opacity-0')
  await opacity.waitFor()
  const opacityNow = async () => (await h.query('prop.get', { layer: SHAPE, path: 'transform/opacity' }))?.value
  const typeInto = async (field, text) => {
    await field.click()
    await h.until(async () => (await focused()) === (await field.getAttribute('data-testid')))
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.type(text)
  }
  await typeInto(opacity, '50')
  await page.keyboard.press('Enter')
  await h.until(async () => (await opacityNow()) === 50)
  h.step('(set-up) Opacity typed 50 and Enter', (await opacityNow()) === 50, JSON.stringify(await opacityNow()))
  await typeInto(opacity, '7')
  await h.until(async () => (await opacity.inputValue()) === '7')
  await page.keyboard.press(`${h.primary}+KeyZ`)
  await page.waitForTimeout(250)
  const afterUndo = await opacity.inputValue()
  await page.keyboard.press(`${h.primary}+KeyZ`)
  await page.waitForTimeout(250)
  const afterTwo = await opacity.inputValue()
  // 7o: with nothing typed left to undo, the second Cmd/Ctrl+Z is the editor's Undo (of the 50), and the field shows the document's value.
  await h.until(async () => (await opacity.inputValue()) === '100')
  h.step('typed 7, Cmd/Ctrl+Z twice in the field: 50 (the value when it took the focus), then the editor’s Undo and the field shows the document’s 100', afterUndo === '50' && (await opacity.inputValue()) === '100' && (await opacityNow()) === 100, `${afterUndo} / ${afterTwo} → ${await opacity.inputValue()} (${JSON.stringify(await opacityNow())})`)
  await page.keyboard.press(`${h.primary}+Shift+KeyZ`)
  await h.until(async () => (await opacity.inputValue()) === '50')
  h.step('(set-up) Cmd/Ctrl+Shift+Z there is the editor’s Redo: 50 again', (await opacity.inputValue()) === '50' && (await opacityNow()) === 50, `${await opacity.inputValue()} · ${JSON.stringify(await opacityNow())}`)
  await page.keyboard.type('8')
  await h.until(async () => (await opacity.inputValue()) === '508' || (await opacity.inputValue()) === '8')
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
  h.step('then typed again and Esc: the field shows 50 and nothing is set', (await opacity.inputValue()) === '50' && (await opacityNow()) === 50 && (await focused()) !== 'motion-value-opacity-0', `${await opacity.inputValue()} · ${JSON.stringify(await opacityNow())}`)
  await typeInto(opacity, '12')
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
  h.step('typed 12 and Esc at once: still 50 (Esc never commits)', (await opacity.inputValue()) === '50' && (await opacityNow()) === 50, `${await opacity.inputValue()} · ${JSON.stringify(await opacityNow())}`)
  await typeInto(opacity, '30')
  await h.until(async () => (await opacity.inputValue()) === '30')
  for (let i = 0; i < 3; i += 1) {
    await page.keyboard.press(`${h.primary}+KeyZ`)
    await page.waitForTimeout(250)
  }
  const shownBack = await opacity.inputValue()
  await kit.getByTestId('motion-timeline').click({ position: { x: 5, y: 5 } }).catch(() => undefined)
  await page.waitForTimeout(400)
  // 7o: the first Cmd/Ctrl+Z takes the typing back to 50, the second is the editor's Undo (100), the third has nothing left to undo.
  h.step('typed 30, Cmd/Ctrl+Z three times, then a click elsewhere: the field and Opacity are the document’s 100 (never the 50 or 30 the field showed)', shownBack === '100' && (await opacityNow()) === 100 && (await opacity.inputValue()) === '100', `${shownBack} · ${JSON.stringify(await opacityNow())}`)

  // 4. The Render Queue's sequence folder in either language.
  const jobs = async () => (await h.query('renderQueue.list', {})).items ?? []
  const job = async (id) => (await jobs()).find((each) => each.id === id)
  const addJob = async () => {
    const count = (await jobs()).length
    await kit.getByTestId('motion-menu-File').click()
    await kit.locator('[data-kit-ui-id="menu:File/Export"]').hover()
    await kit.locator('[role=menu] [data-testid="motion-control-add-to-render-queue"]').first().click()
    await h.until(async () => (await jobs()).length === count + 1)
    return (await jobs()).at(-1)
  }
  const folderField = () => kit.getByTestId('motion-job-folder')
  const openSettings = async (id) => {
    await kit.getByTestId(`motion-queue-item-${id}-settings`).click()
    await kit.getByTestId('motion-job-dialog').waitFor()
  }
  const ok = async () => {
    await kit.getByTestId('motion-job-dialog-ok').click()
    await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
  }
  const settled = async (read) => {
    let value = await read()
    await h.until(async () => {
      const again = await read()
      const same = again === value
      value = again
      return same
    })
    return value
  }
  const pickFormat = async (format) => {
    await kit.getByTestId('motion-job-format').selectOption(format)
    await h.until(async () => (await kit.getByTestId('motion-job-format').inputValue()) === format)
    return settled(() => folderField().inputValue())
  }
  const emptyFolder = async () => {
    await folderField().click()
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.press('Backspace')
    await h.until(async () => (await folderField().inputValue()) === '')
  }
  const typeFolder = async (text) => {
    await emptyFolder()
    await page.keyboard.type(text)
    await h.until(async () => (await folderField().inputValue()) === text)
  }
  const stored = async (id) => (await job(id))?.output?.output ?? ''
  for (const [locale, own, other] of [
    ['zh-CN', '_序列', '_sequence'],
    ['en', '_sequence', '_序列'],
  ]) {
    if (locale !== 'zh-CN') {
      await page.evaluate(() => localStorage.clear())
      await reopen(locale)
    }
    const movie = await addJob()
    const comp = movie.compName
    h.step(`${locale}: (set-up) a new job is a movie (H.264) into the project’s folder`, movie.output?.format === 'H264' && !(await stored(movie.id)).includes('/'), await stored(movie.id))
    for (const typed of [`${comp}${other}`, `${comp}${other} 2`, `${comp}${own} 3`]) {
      await openSettings(movie.id)
      await typeFolder(typed)
      await ok()
      await page.waitForTimeout(300)
      const out = await stored(movie.id)
      h.step(`${locale}: “${typed}” typed for a movie counts as automatic: OK keeps the movie out of it`, !out.includes(`${typed}/`) && !out.includes('/'), out)
    }
    // A movie job made a PNG sequence: its folder box shows the sequence folder, and OK keeps it in the job.
    await openSettings(movie.id)
    const box = await pickFormat('png')
    await ok()
    await h.until(async () => (await job(movie.id))?.output?.format === 'PngSequence')
    h.step(`${locale}: made a PNG sequence, the job keeps its folder ${comp}${own}/ (written in the project)`, box === `${comp}${own}` && (await stored(movie.id)).startsWith(`${comp}${own}/`), `${box} · ${await stored(movie.id)}`)
    // Its folder box emptied at OK: the job still keeps a sequence folder of its own (not one picked at render time).
    await openSettings(movie.id)
    await emptyFolder()
    await ok()
    await page.waitForTimeout(300)
    const emptied = await stored(movie.id)
    h.step(`${locale}: the sequence job’s folder box emptied at OK: the job keeps ${comp}${own}/`, emptied.startsWith(`${comp}${own}/`), emptied)
    // A new job after a sequence is a sequence too: its folder is in the job from the start.
    const next = await addJob()
    const nextOut = await stored(next.id)
    h.step(`${locale}: the next new job (a PNG sequence) has its folder in the job: ${comp}${own} 2/`, next.output?.format === 'PngSequence' && nextOut.startsWith(`${comp}${own} 2/`), nextOut)
  }

  // 5. Text layer names in either language.
  for (const [locale, base] of [
    ['zh-CN', '文本'],
    ['en', 'Text'],
  ]) {
    await reopen(locale)
    const first = await newText()
    h.step(`${locale}: (set-up) Layer ▸ New ▸ Text makes “${base}”`, first.name === base, first.name)
    await h.focusEditor()
    const twice = await newText()
    h.step(`${locale}: (set-up) the next one is “${base} 2”`, twice.name === `${base} 2`, twice.name)
    await page.keyboard.type('Hey')
    await h.until(async () => (await nameOf(twice.id)) === 'Hey')
    h.step(`${locale}: typing “Hey” in its Source Text box names “${base} 2” Hey`, (await nameOf(twice.id)) === 'Hey', await nameOf(twice.id))
    await h.focusEditor()
    await kit.getByTestId('motion-tool-text').click()
    const before = (await h.layers()).map((layer) => layer.id)
    const spot = await h.at(640, 650)
    await page.mouse.click(spot.x, spot.y)
    await h.until(async () => (await h.layers()).length === before.length + 1)
    const tool = (await h.layers()).find((layer) => !before.includes(layer.id))
    h.step(`${locale}: the Text tool names its layer as Layer ▸ New ▸ Text does (“${base} 2”, the next free one)`, tool?.name === `${base} 2`, tool?.name)
    await page.waitForTimeout(200)
    await page.keyboard.type('Yo')
    await h.until(async () => (await nameOf(tool.id)) === 'Yo')
    h.step(`${locale}: typing “Yo” on the canvas names it Yo`, (await nameOf(tool.id)) === 'Yo', await nameOf(tool.id))
    await page.keyboard.press('Escape')
    await kit.getByTestId('motion-tool-selection').click()
  }

  // 6. The font box, the Remove All Styles label (in English) and a bare undefined name's line.
  await reopen('en')
  const label = await newText()
  await h.focusEditor()
  await kit.getByTestId('motion-text-font').waitFor()
  h.step('the font box shows no clear button (a text layer always has a font)', (await kit.getByTestId('motion-text-font').locator('.kui-combobox-clear').filter({ visible: true }).count()) === 0)
  await kit.getByTestId('motion-style-add').click()
  await kit.getByTestId('motion-style-add-dropShadow').click()
  await h.until(async () => ((await h.query('layer.tree', { layer: label.id, depth: 3 }))?.properties?.children ?? []).some((child) => child.match === 'layerStyles' && (child.children ?? []).some((style) => style.match === 'dropShadow')))
  await kit.getByTestId('motion-style-add').click()
  await kit.getByTestId('motion-style-remove-all').waitFor()
  const inPanel = ((await kit.getByTestId('motion-style-remove-all').textContent()) ?? '').trim()
  await page.keyboard.press('Escape')
  await h.focusEditor()
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[data-kit-ui-id="menu:Layer/Layer Styles"]').hover()
  const row = kit.locator('[role=menu] [data-testid="motion-cmd-layer.style.removeAll"]').first()
  await row.waitFor()
  const inMenu = ((await row.textContent()) ?? '').trim()
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  h.step('English: the Properties add-style menu and Layer ▸ Layer Styles call it the same: Remove All Styles', inPanel === 'Remove All Styles' && inMenu.startsWith('Remove All Styles'), `${inPanel} / ${inMenu}`)

  await reopen('zh-CN')
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  const diamond = kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.rotation`)
  await diamond.waitFor()
  await diamond.click({ button: 'right' })
  await kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.rotation-menu-expression`).click()
  const exprRow = `motion-expr-${SHAPE}-transform.rotation`
  await kit.getByTestId(`${exprRow}-field`).waitFor()
  await kit.getByTestId(`${exprRow}-field`).click()
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('wobble')
  await page.keyboard.press('Enter')
  await h.until(async () => ((await kit.getByTestId(`${exprRow}-error`).textContent().catch(() => '')) ?? '').includes('wobble'))
  const said = (await kit.getByTestId(`${exprRow}-error`).textContent().catch(() => '')) ?? ''
  h.step('an expression that is only an undefined name (wobble) says its line: 第 1 行', said.includes('wobble') && said.includes('第 1 行'), said)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
