// The fixes after the round 7g hand test, by hand, in a real browser through the development host: node tests/fixes7g.mjs [chromium|webkit]
// The second view's divider and the timeline's divider take their press for themselves (no text
// selection starts, so WebKit cannot take the pointer away mid-drag) and stop following the pointer
// when the press ends; the graph editor does not re-fit its time after a change made in it (only
// after one made elsewhere). Results are read from the engine (`prop.get`, `comp.info`) and the
// elements' own attributes. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const near = (a, b, eps) => Math.abs(a - b) <= eps
const viewRow = (text) => kit.locator('[role=menu] [role=menuitem]', { hasText: text }).first()
async function viewMenu() {
  await kit.getByTestId('motion-menu-View').click()
  await kit.locator('[role=menu]').first().waitFor()
}
/** Records, in the editor's page, whether the next presses were taken (their default prevented). */
async function watchPresses() {
  await kit.locator('body').evaluate(() => {
    window.__presses = []
    if (window.__watching) return
    window.__watching = true
    window.addEventListener('pointerdown', (event) => window.__presses.push(event.defaultPrevented))
  })
}
const presses = () => kit.locator('body').evaluate(() => window.__presses)
const selectionText = () => kit.locator('body').evaluate(() => String(window.getSelection() ?? ''))

/** Opens the sample afresh at `width` × `height` in `locale` (the harness opens it in zh-CN at 1440 × 900). */
async function reopen(locale, width, height) {
  await page.setViewportSize({ width, height })
  const url = new URL(page.url())
  url.searchParams.set('locale', locale)
  await page.goto(url.toString())
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
}
const positionOf = async (layer) => (await h.query('prop.get', { layer, path: 'transform/position' })).value

try {
  await h.open()

  // 1. The dividers: the press is theirs, and once released they no longer follow the pointer.
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await kit.getByTestId('motion-3d-view').waitFor()
  await viewMenu()
  await viewRow('两个视图').click()
  const second = kit.getByTestId('motion-second-view')
  await second.waitFor()
  await h.until(async () => Number(await second.getAttribute('data-width')) > 0)
  const width = async () => Number(await second.getAttribute('data-width'))
  const divider = kit.getByTestId('motion-second-view-divider')
  await watchPresses()
  const grip = await h.centre(divider)
  const was = await width()
  // Over the viewer and back: a text selection here would let WebKit cancel the pointer.
  await h.drag(grip, { x: grip.x - 90, y: grip.y })
  await h.until(async () => (await width()) > was + 60)
  const dragged = await width()
  h.step('the second view’s divider takes its press (no text selection starts)', (await presses())[0] === true && (await selectionText()) === '', JSON.stringify(await presses()))
  h.step('and the drag widens the second view by what it moved', near(dragged, was + 90, 4), `${was} → ${dragged}`)
  // The pointer goes back over the divider and across the viewer with no button held.
  const now = await h.centre(divider)
  for (const x of [now.x, now.x + 40, now.x - 200, now.x + 300]) {
    await page.mouse.move(x, now.y)
    await page.waitForTimeout(30)
  }
  h.step('once released it no longer follows the pointer', (await width()) === dragged, `${await width()} (was ${dragged})`)
  // Its picture has the pane's shape: drawn over the whole pane, no bands above and below.
  const paneCanvas = kit.getByTestId('motion-second-view-canvas')
  await h.until(async () => {
    const box = await paneCanvas.boundingBox()
    const [dw, dh] = ((await second.getAttribute('data-size')) ?? '0 0').split(' ').map(Number)
    return Math.abs(dw - box.width) <= 1 && Math.abs(dh - box.height) <= 1
  }, 50)
  const pane = await paneCanvas.boundingBox()
  const drawnSize = ((await second.getAttribute('data-size')) ?? '0 0').split(' ').map(Number)
  const pixel = async (x, y) => {
    const shot = await page.screenshot({ clip: { x: Math.round(x), y: Math.round(y), width: 1, height: 1 } })
    return page.evaluate(async (data) => {
      const image = await createImageBitmap(await (await fetch(`data:image/png;base64,${data}`)).blob())
      const canvas = new OffscreenCanvas(1, 1)
      const context = canvas.getContext('2d')
      context.drawImage(image, 0, 0)
      return [...context.getImageData(0, 0, 1, 1).data.slice(0, 3)]
    }, shot.toString('base64'))
  }
  // What shows where nothing is drawn: the first background behind the canvas.
  const paneBackground = await paneCanvas.evaluate((element) => {
    for (let at = element.parentElement; at; at = at.parentElement) {
      const color = getComputedStyle(at).backgroundColor
      if (color && color !== 'transparent' && !/rgba\(.*, 0\)$/.test(color)) return color
    }
    return ''
  })
  const corners = [await pixel(pane.x + 4, pane.y + 4), await pixel(pane.x + pane.width - 5, pane.y + pane.height - 5)]
  const bare = paneBackground.match(/\d+/g)?.slice(0, 3).map(Number) ?? []
  const isBare = (rgb) => bare.length === 3 && rgb.every((value, index) => Math.abs(value - bare[index]) <= 3)
  h.step('the second view’s picture fills its pane (drawn at the pane’s size, its corners show the scene)', Math.abs(drawnSize[0] - pane.width) <= 1 && Math.abs(drawnSize[1] - pane.height) <= 1 && corners.every((rgb) => !isBare(rgb)), JSON.stringify({ pane: [Math.round(pane.width), Math.round(pane.height)], drawn: drawnSize, corners, paneBackground }))
  await kit.getByTestId('motion-second-view-close').click()
  await h.until(async () => (await second.count()) === 0)

  const splitter = kit.getByTestId('motion-splitter')
  const height = async () => Number(await splitter.getAttribute('aria-valuenow'))
  await watchPresses()
  const bar = await h.centre(splitter)
  const tall = await height()
  await h.drag(bar, { x: bar.x + 120, y: bar.y - 60 })
  await h.until(async () => (await height()) > tall + 40)
  const taller = await height()
  h.step('the timeline’s divider takes its press too, and the drag makes the timeline taller', (await presses())[0] === true && (await selectionText()) === '' && near(taller, tall + 60, 3), `${tall} → ${taller}, ${JSON.stringify(await presses())}`)
  for (const y of [bar.y - 60, bar.y - 20, bar.y - 150]) {
    await page.mouse.move(bar.x, y)
    await page.waitForTimeout(30)
  }
  h.step('and once released it no longer follows the pointer', (await height()) === taller, `${await height()} (was ${taller})`)

  // 2. The graph editor re-fits its time only after a change made outside it.
  const before = (await h.layers()).map((layer) => layer.id)
  await h.setUp([
    { command: 'layer.newSolid', params: { name: 'Keys', color: '#2a6fdb', width: 200, height: 200 } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/position', time: 0.2, value: [500, 300] } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/position', time: 0.8, value: [700, 300] } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/opacity', time: 2, value: 100 } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/opacity', time: 2.6, value: 20 } },
  ])
  await h.until(async () => (await h.layers()).some((layer) => !before.includes(layer.id)))
  const solid = (await h.layers()).find((layer) => !before.includes(layer.id)).id
  await kit.getByTestId(`motion-layer-${solid}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layers()).filter((layer) => layer.selected).map((layer) => layer.id).join() === String(solid))
  const timeline = kit.getByTestId('motion-timeline')
  const viewOf = async () => (await timeline.getAttribute('data-view')).split(' ').map(Number)
  const unfitted = await viewOf()
  await kit.getByTestId('motion-graph-toggle').click()
  await h.until(async () => (await viewOf()).join() !== unfitted.join(), 80)
  await h.until(async () => (await kit.getByTestId(`motion-graph-key-${solid}-transform.opacity-0-2600`).count()) === 1)
  const fitted = await viewOf()
  h.step('opening the graph editor fits the keys (0.2–2.6 s)', fitted[0] <= 0.2 && fitted[0] + fitted[1] >= 2.6 && fitted[1] < 4, String(fitted))
  await kit.getByTestId(`motion-graph-key-${solid}-transform.opacity-0-2000`).click()
  await kit.getByTestId(`motion-graph-key-${solid}-transform.opacity-0-2600`).click({ modifiers: ['Shift'] })
  await page.waitForTimeout(400)
  await h.focusEditor()
  await page.keyboard.press('Delete')
  await h.until(async () => !(await h.layer(solid)).animated.some((property) => property.path === 'transform/opacity'))
  await h.until(async () => (await kit.getByTestId(`motion-graph-key-${solid}-transform.opacity-0-2600`).count()) === 0)
  await page.waitForTimeout(500)
  h.step('deleting keys in the graph (Opacity no longer animated) leaves its time where it was', !(await h.layer(solid)).animated.some((property) => property.path === 'transform/opacity') && (await viewOf()).join() === fitted.join(), `${fitted} → ${await viewOf()}`)
  // Elsewhere: a click in the viewer on another layer fits its keys, not the ones shown before.
  await h.setUp([
    { command: 'layer.newSolid', params: { name: 'Late', color: '#f2c14e', width: 160, height: 160 } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/position', time: 1.6, value: [900, 420] } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/position', time: 1.9, value: [960, 420] } },
  ])
  await h.until(async () => (await h.layers()).some((layer) => layer.name === 'Late'))
  const late = (await h.layers()).find((layer) => layer.name === 'Late').id
  // The time stands before both layers' keys, where each sits at its first key.
  await h.until(async () => (await h.layer(solid))?.selected && !(await h.layer(late))?.selected)
  const lateAt = await h.at(900, 420)
  await page.mouse.click(lateAt.x, lateAt.y)
  await h.until(async () => (await h.layer(late))?.selected && !(await h.layer(solid))?.selected)
  await h.until(async () => near((await viewOf())[0] + (await viewOf())[1] / 2, 1.75, 0.3), 50)
  const lateView = await viewOf()
  h.step('a click in the viewer on another layer fits that layer’s keys (1.6–1.9 s), not the ones shown before', lateView[0] <= 1.6 && lateView[0] + lateView[1] >= 1.9 && lateView[0] > 0.8, `${fitted} → ${lateView}`)
  const solidAt = await h.at(500, 300)
  await page.mouse.click(solidAt.x, solidAt.y)
  await h.until(async () => (await h.layer(solid))?.selected && !(await h.layer(late))?.selected)
  await h.until(async () => (await viewOf())[0] + (await viewOf())[1] < 1.2, 50)
  const refit = await viewOf()
  h.step('and a click back on the first layer fits its keys again (0.2–0.8 s)', refit[0] <= 0.2 && refit[0] + refit[1] >= 0.8 && refit[1] < 2, `${lateView} → ${refit}`)
  await kit.getByTestId('motion-graph-toggle').click()

  // 3. A 3D layer's three numbers in the timeline: on one line, readable, scrubbed and typed, at
  // 1440 × 900 and 1280 × 800, in Chinese and in English.
  for (const [locale, width, height] of [['zh-CN', 1440, 900], ['en', 1280, 800], ['zh-CN', 1280, 800], ['en', 1440, 900]]) {
    const at = `${locale} ${width}×${height}`
    await reopen(locale, width, height)
    await h.setUp([
      { command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } },
      { command: 'prop.set', params: { layer: SHAPE, path: 'transform/position', value: [1412.5123, 358.987, -250.25] } },
    ])
    await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
    await h.until(async () => (await h.layer(SHAPE))?.selected)
    await h.focusEditor()
    await page.keyboard.press('p')
    const row = kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`)
    await row.waitFor()
    const field = (index) => kit.getByTestId(`motion-row-value-${SHAPE}-transform.position-${index}`)
    await h.until(async () => (await field(2).count()) === 1 && near(Number(await field(0).getAttribute('data-value')), 1412.51, 0.01))
    const looks = await row.evaluate((element) => {
      const box = (node) => node.getBoundingClientRect()
      const fields = [...element.querySelectorAll('input.motion-number')]
      const names = element.querySelector('.motion-timeline-names') ?? element.firstElementChild
      return {
        row: [box(element).top, box(element).bottom],
        names: box(names).right,
        fields: fields.map((input) => ({ top: box(input).top, bottom: box(input).bottom, left: box(input).left, right: box(input).right, fits: input.scrollWidth <= input.clientWidth, text: input.value, title: input.title })),
      }
    })
    const [x, y, z] = looks.fields
    const oneLine = looks.fields.length === 3 && looks.fields.every((each) => near(each.top, x.top, 1) && each.top >= looks.row[0] - 1 && each.bottom <= looks.row[1] + 1 && each.right - each.left >= 40)
    h.step(`${at}: the three numbers of a 3D layer’s Position sit on one line inside the row and the names column`, oneLine && z.right <= looks.names + 1 && x.right <= y.left && y.right <= z.left, JSON.stringify(looks))
    const real = [1412.5123, 358.987, -250.25]
    h.step(`${at}: each shows its value without clipping (decimals dropped to fit), the whole value in its tooltip`, looks.fields.every((each, index) => each.fits && near(Number(each.text), real[index], 0.5)) && x.title.includes('1412.51'), looks.fields.map((each) => `${each.text} [${each.title}]`).join(' '))
    for (const index of [0, 1, 2]) {
      const axis = 'XYZ'[index]
      const before = await positionOf(SHAPE)
      const centre = await h.centre(field(index))
      await h.drag(centre, { x: centre.x + 20, y: centre.y })
      await h.until(async () => near((await positionOf(SHAPE))[index], before[index] + 20, 1.5))
      const scrubbed = await positionOf(SHAPE)
      h.step(`${at}: dragging ${axis} scrubs only ${axis}`, near(scrubbed[index], before[index] + 20, 1.5) && scrubbed.every((value, other) => other === index || near(value, before[other], 1e-6)), `${JSON.stringify(before)} → ${JSON.stringify(scrubbed)}`)
      await field(index).click()
      await h.until(async () => field(index).evaluate((input) => document.activeElement === input))
      const typedFrom = await field(index).inputValue()
      await page.keyboard.press(`${h.primary}+A`)
      const entry = `-${1000 + index}.25`
      await page.keyboard.type(entry)
      const whileTyping = await field(index).inputValue()
      await page.keyboard.press('Enter')
      await h.until(async () => near((await positionOf(SHAPE))[index], -1000.25 - index, 1e-6))
      const typed = await positionOf(SHAPE)
      h.step(`${at}: a click on ${axis} shows its whole value to type over, the typed text whole, and Enter sets it`, near(typed[index], -1000.25 - index, 1e-6) && whileTyping === entry && near(Number(typedFrom), scrubbed[index], 0.006) && typed.every((value, other) => other === index || near(value, scrubbed[other], 1e-6)), `${typedFrom} → ${whileTyping} → ${JSON.stringify(typed)} (shows ${await field(index).inputValue()})`)
    }
  }
  await page.setViewportSize({ width: 1440, height: 900 })

  // 4. Chinese: every effect name and every drop-down option reads in Chinese, except proper nouns
  // and abbreviations; Glow's 颜色循环 and the legacy tracked-shape effect among them.
  await reopen('zh-CN', 1440, 900)
  await h.setUp([
    { command: 'effect.apply', params: { effect: 'ec.stylize.glow', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.obsolete.mochashape', layers: [SHAPE] } },
  ])
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await kit.getByTestId('motion-effect-card-2').count()) === 1)
  const glow = kit.getByTestId('motion-effect-card-1')
  if ((await kit.getByTestId('motion-effect-toggle-1').getAttribute('aria-expanded')) !== 'true') await kit.getByTestId('motion-effect-toggle-1').click()
  await h.until(async () => (await glow.locator('select').count()) > 3)
  const chosen = await glow.locator('select').evaluateAll((selects) => selects.map((select) => select.selectedOptions[0]?.textContent ?? ''))
  const looping = await glow.locator('select option').allTextContents()
  // (7i: the loops are waves, so 三角波 / 锯齿波 rather than the shapes 三角形 / 锯齿.)
  h.step('Glow’s 颜色循环 reads 三角波 A>B>A, its other choices in Chinese too', chosen.includes('三角波 A>B>A') && ['锯齿波 A>B', '锯齿波 B>A', '三角波 B>A>B'].every((label) => looping.includes(label)), chosen.join(' | '))
  const legacy = (await kit.getByTestId('motion-effect-toggle-2').textContent()) ?? ''
  h.step('the legacy tracked-shape effect is named in Chinese', legacy.includes('跟踪形状'), legacy)
  const ALLOWED = new Set(["1.0", "10%", "100%", "105%", "109%", "1x", "2.0", "20%", "2x", "3%", "3.0", "3x", "4x", "5%", "90%", "95%", "ACES2065-1", "ACEScct", "ACEScg", "ARGB", "ASC CDL v1.2", "Alpha", "Alpha 1", "Alpha 2", "CIE-XYZ-D65", "CryptoAsset", "CryptoMaterial", "CryptoObject", "Cryptomatte", "Display P3", "EXtractoR", "HDTV (Rec. 709)", "HLS", "IDentifier", "Lab", "NTSC", "PAL", "RGB", "RGBA", "Rec. 2020", "Rec.1886 Rec.709", "Rec.2100-PQ", "SMPTE HH:MM:SS:FF", "X", "XY", "XZ", "Y", "YIQ", "YUV", "YZ", "Z", "sRGB", "sRGB IEC61966-2.1", "z = z^2 + c", "z = z^3 + c", "z = z^4 + c"])
  // Setting names that are abbreviations or letters only (Color's H, S and L keys of its HSL secondary).
  const NAMES_ALLOWED = new Set(['Alpha', 'CCC ID', 'FOV', 'H', 'ID', 'L', 'LUT', 'RGB', 'S', 'X', 'Y', 'Z'])
  const list = await h.query('effect.list', {})
  const untranslated = await kit.locator('body').evaluate(async (_, effects) => {
    const { PHRASES } = await import('/src/strings.ts')
    const { LEARNED_NAMES_ZH, shapedZh } = await import('/src/effectNames.ts')
    const zh = PHRASES['zh-CN']
    const shown = (text) => LEARNED_NAMES_ZH.get(text) ?? (zh[text] && zh[text] !== text ? zh[text] : (shapedZh(text) ?? text))
    const out = new Set()
    // A numbered name the phrases lack reads as its translated base and the number (`main.tsx`).
    const named = (text) => {
      const whole = shown(text)
      const match = whole === text ? /^(.*\S) (\d+)$/.exec(text) : null
      return match && shown(match[1]) !== match[1] ? `${shown(match[1])} ${match[2]}` : whole
    }
    const names = new Set()
    for (const effect of effects) {
      if (shown(effect.name) === effect.name) out.add(effect.name)
      const ids = new Set((effect.params ?? []).map((param) => param.id))
      for (const param of effect.params ?? []) {
        for (const option of param.options ?? []) if (shown(option.label) === option.label) out.add(option.label)
        // Every setting's name, and a layer setting's source choice (`<name> Source`) beside it.
        const settings = param.type === 'layer' && !ids.has(`${param.id}Source`) ? [param.name, `${param.name} Source`] : [param.name]
        for (const name of settings) if (named(name) === name) names.add(name)
      }
    }
    return [[...out], [...names]]
  }, list)
  const [left, leftNames] = [untranslated[0].filter((text) => !ALLOWED.has(text)), untranslated[1].filter((text) => !NAMES_ALLOWED.has(text))]
  h.step(`every effect name and drop-down option (${list.length} effects) reads in Chinese, but for proper nouns and abbreviations`, left.length === 0, left.slice(0, 20).join(' | '))
  const settingCount = list.reduce((sum, effect) => sum + (effect.params ?? []).length, 0)
  h.step(`(7i) every setting name of every effect (${settingCount} settings) reads in Chinese, but for abbreviations`, leftNames.length === 0, leftNames.slice(0, 20).join(' | '))

  // 5. Layers with different labels selected: neither the right-click menu nor the menu bar ticks a colour.
  const NIGHT = 4
  await h.setUp([
    { command: 'edit.label', params: { label: 'Blue', layers: [SHAPE], target: 'layers' } },
    { command: 'edit.label', params: { label: 'Green', layers: [NIGHT], target: 'layers' } },
  ])
  const rowName = (id) => kit.getByTestId(`motion-layer-${id}`).locator('.motion-row-name').first()
  const menuTicks = async () => {
    await rowName(SHAPE).click({ button: 'right' })
    await kit.getByTestId('motion-layer-menu').waitFor()
    await kit.getByTestId('motion-layer-menu-label').hover()
    await kit.getByTestId('motion-layer-menu-label-Blue').waitFor()
    const ticked = await kit.locator('[role=menu] .motion-label-icon[data-current]').count()
    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await h.until(async () => (await kit.getByTestId('motion-layer-menu').count()) === 0, 20)
    return ticked
  }
  await rowName(SHAPE).click()
  await h.until(async () => (await h.layers()).filter((layer) => layer.selected).map((layer) => layer.id).join() === String(SHAPE))
  const alone = await menuTicks()
  await rowName(NIGHT).click({ modifiers: ['Shift'] })
  await h.until(async () => (await h.layer(NIGHT))?.selected && (await h.layer(SHAPE))?.selected)
  const mixed = await menuTicks()
  h.step('the right-click Label menu ticks the colour of one blue layer, and none for a blue and a green one', alone === 1 && mixed === 0, `${alone} then ${mixed}`)
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[data-kit-ui-id="menu:Layer/Label"]').hover()
  const rows = kit.locator('[role=menu] [data-kit-ui-id^="control:label-"]')
  await h.until(async () => (await rows.count()) === 17)
  const checkedRows = await rows.evaluateAll((items) => items.filter((item) => item.getAttribute('aria-checked') === 'true').map((item) => item.getAttribute('data-kit-ui-id')))
  h.step('and Layer ▸ 标签 in the menu bar ticks none either', (await rows.count()) === 17 && checkedRows.length === 0, checkedRows.join(' '))
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')

  // 6. Animation ▸ Keyframe Assistant: rows that cannot run yet look it (muted, no hand cursor) and
  // still say why when chosen; once they can run they look like any other row.
  await h.setUp([
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 0 } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 2, value: [700, 400] } },
  ])
  await rowName(SHAPE).click()
  await h.until(async () => (await h.layers()).filter((layer) => layer.selected).map((layer) => layer.id).join() === String(SHAPE))
  // (7i: greyed as the SDK greys a disabled row, `.kui-menu-item:disabled`, rather than in the muted text colour.)
  const assistant = async () => {
    let looks = {}
    await h.until(async () => {
      await page.keyboard.press('Escape')
      await page.keyboard.press('Escape')
      await kit.getByTestId('motion-menu-Animation').click()
      await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
      await kit.locator('[role=menu] [data-testid="motion-control-smoother"]').first().waitFor({ timeout: 3000 })
      looks = await kit.locator('[role=menu]').last().evaluate((menu) => {
        const out = {}
        const look = (element) => ({ color: getComputedStyle(element).color, cursor: getComputedStyle(element).cursor, opacity: getComputedStyle(element).opacity, text: element.textContent })
        for (const id of ['wiggler', 'smoother', 'convert-expression']) {
          const element = menu.querySelector(`[data-testid="motion-control-${id}"]`)
          if (element) out[id] = look(element)
        }
        // A row the SDK itself disables, in the same menu, for the look to match.
        const probe = document.createElement('button')
        probe.className = 'kui-menu-item'
        probe.disabled = true
        menu.appendChild(probe)
        out.disabled = look(probe)
        probe.remove()
        return out
      })
      // A menu redrawn while it is read gives detached rows (no computed style): read it again.
      return Object.keys(looks).length === 4 && Object.values(looks).every((look) => look.color !== '')
    }, 10)
    return looks
  }
  const waiting = await assistant()
  const disabled = waiting.disabled
  const isMuted = (look) => look.color === disabled.color && look.opacity === disabled.opacity && look.cursor === 'default'
  h.step('with no keyframes selected Wiggler, Smoother and Convert Expression to Keyframes look greyed as the SDK’s disabled rows, saying why', isMuted(waiting.wiggler) && isMuted(waiting.smoother) && isMuted(waiting['convert-expression']) && waiting.wiggler.text.includes('至少两个关键帧'), JSON.stringify(waiting))
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-control-wiggler"]').first().click()
  await h.until(async () => (await kit.locator('.kui-toast').count()) > 0, 20)
  h.step('choosing a greyed row still says why, and opens nothing', (await kit.getByTestId('motion-wiggler-dialog').count()) === 0 && ((await kit.locator('.kui-toast').last().textContent()) ?? '').includes('至少两个关键帧'))
  await h.focusEditor()
  await page.keyboard.press('KeyP')
  const mark = (ms) => kit.getByTestId(`motion-key-${SHAPE}-transform.position-${ms}`)
  await mark(0).waitFor()
  await mark(0).click()
  await mark(2000).click({ modifiers: ['Shift'] })
  await h.until(async () => (await kit.locator('.motion-key[aria-selected="true"]').count()) === 2)
  await h.until(async () => !(await kit.getByTestId('motion-waiting-rows').textContent()).includes('control:wiggler'))
  const ready = await assistant()
  h.step('with two keyframes selected Wiggler looks like any other row (Smoother, needing three, stays greyed)', !isMuted(ready.wiggler) && ready.wiggler.cursor !== 'default' && isMuted(ready.smoother) && !ready.wiggler.text.includes('至少'), JSON.stringify(ready))
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')


  // 7a. A new full-frame solid over other layers says once per session that it covers them, with
  // 移到最下面 (one undo step); none in an empty composition.
  const newSolid = async () => {
    const before = (await h.layers()).map((layer) => layer.id)
    await kit.getByTestId('motion-menu-Layer').click()
    await kit.locator('[data-kit-ui-id="menu:Layer/New"]').hover()
    await kit.locator('[role=menu] [data-testid="motion-cmd-layer.newSolid"]').first().click()
    await kit.getByTestId('motion-solid-dialog').waitFor()
    await page.keyboard.press('Enter')
    await h.until(async () => (await h.layers()).some((layer) => !before.includes(layer.id)))
    await page.waitForTimeout(600)
    return (await h.layers()).find((layer) => !before.includes(layer.id))?.id
  }
  const coverNotices = () => kit.locator('.kui-toast', { hasText: '纯色图层盖住了下面的图层' })
  await kit.getByTestId('motion-menu-Composition').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-comp.new"]').first().click()
  await page.waitForTimeout(400)
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.details()).comp?.width === 1920 && (await h.layers()).length === 0)
  await newSolid()
  h.step('a solid in an empty composition says nothing', (await coverNotices().count()) === 0)
  const covering = await newSolid()
  await h.until(async () => (await coverNotices().count()) === 1, 30)
  const back = kit.locator('.kui-toast button', { hasText: '移到最下面' })
  h.step('a full-frame solid over another layer says it covers the layers below, offering 移到最下面', (await coverNotices().count()) === 1 && (await back.count()) === 1 && (await h.layers())[0]?.id === covering, await coverNotices().first().textContent().catch(() => '(none)'))
  await back.click()
  await h.until(async () => (await h.layers()).at(-1)?.id === covering)
  h.step('移到最下面 puts it under the other layers', (await h.layers()).at(-1)?.id === covering, (await h.layers()).map((layer) => layer.name).join(' / '))
  await h.undo()
  await h.until(async () => (await h.layers())[0]?.id === covering)
  h.step('and one undo puts it back on top', (await h.layers())[0]?.id === covering && (await h.layers()).length === 2)
  await h.until(async () => (await coverNotices().count()) === 0, 150)
  await newSolid()
  await page.waitForTimeout(800)
  h.step('the next solid in this session says nothing (once per session)', (await coverNotices().count()) === 0)
  // 7. A browser that does not report its memory (Safari; here every browser, made to say nothing):
  // the cache keeps to 400 MB, and when it is full 使用更多内存（1 GB） raises it for the session.
  await page.context().addInitScript(() => Object.defineProperty(Navigator.prototype, 'deviceMemory', { get: () => undefined, configurable: true }))
  await reopen('zh-CN', 1440, 900)
  // Composition ▸ New Composition… with Enter (1920 × 1080, 10 s, 30 fps), a solid with Noise at 100 over it.
  await kit.getByTestId('motion-menu-Composition').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-comp.new"]').first().click()
  await page.waitForTimeout(400)
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.details()).comp?.width === 1920 && (await h.layers()).length === 0)
  await h.setUp([
    { command: 'layer.newSolid', params: { name: 'Ground', color: '#2a6fdb' } },
    { command: 'effect.apply', params: { effect: 'ec.noise.noise', layers: ['#1'], params: { amount: 100 } } },
    { command: 'edit.deselectAll', params: {} },
  ])
  const cacheButton = kit.getByTestId('motion-cache-ahead')
  const numberOf = async (name) => Number(await cacheButton.getAttribute(name))
  await cacheButton.click()
  await h.until(async () => (await cacheButton.getAttribute('data-full')) === 'true', 3000)
  const fullAt = await numberOf('data-done')
  const more = kit.getByTestId('motion-cache-more-memory')
  h.step('the cache fills at 400 MB and offers 使用更多内存（1 GB）, saying it uses more memory', (await numberOf('data-budget')) === 400 * 1048576 && (await more.count()) === 1 && (await more.textContent()) === '使用更多内存（1 GB）' && ((await more.getAttribute('title')) ?? '').includes('更多电脑内存'), `${fullAt} frames, budget ${await numberOf('data-budget')}, ${await more.textContent().catch(() => '(none)')} | ${await more.getAttribute('title').catch(() => '')}`)
  await more.click()
  await h.until(async () => (await numberOf('data-budget')) === 1024 * 1048576 && (await numberOf('data-done')) > fullAt && ((await cacheButton.getAttribute('data-full')) === 'true' || (await numberOf('data-done')) >= 300), 3000)
  const moreAt = await numberOf('data-done')
  h.step('one click raises the budget to 1 GB for the session and caches on (the action goes away)', (await numberOf('data-budget')) === 1024 * 1048576 && moreAt > fullAt * 2 && (await more.count()) === 0, `${fullAt} → ${moreAt} frames, budget ${await numberOf('data-budget')}`)
  await cacheButton.click()
  await h.until(async () => (await cacheButton.getAttribute('aria-pressed')) === 'false')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
