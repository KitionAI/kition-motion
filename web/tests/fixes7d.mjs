// The phase 7d hand-test fixes, by hand in a real browser through the development host: node tests/fixes7d.mjs [chromium|webkit]
// With the Project panel open the transport stays on one line (the composition's name readable, the
// duration unwrapped) and its buttons keep their place while playing; the Render Queue's title,
// job names and Stop read well in the narrow column; the keyframe dialog names the property in
// Chinese and a 2D layer's Position has x and y only; Rove Across Time is off (and says why) on a
// first keyframe; File ▸ New ▸ New Folder starts renaming; the pointer readout empties when the
// pointer leaves the composition; the switch columns' short names are unambiguous; the shape Add
// menu's submenus say what they hold and its search finds Gradient Fill; Alt+Shift+1 and F10 both
// give the Front view; a Project item dragged onto the timeline with one plain press, move and
// release makes a layer; a text selection follows characters an animator moves. Prints PASS or
// FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const newest = (before, layers) => layers.find((layer) => !before.some((old) => old.id === layer.id))
const box = (testId) => kit.getByTestId(testId).boundingBox()

try {
  await h.open()

  // 1. The Project panel open (the Properties panel is open too): the transport stays on one line.
  await kit.getByTestId('motion-project-toggle').click()
  await kit.getByTestId('motion-project-panel').waitFor().catch(() => undefined)
  await page.waitForTimeout(400)
  const transport = await box('motion-transport')
  const switcher = kit.getByTestId('motion-comp-switcher')
  const shownName = await switcher.evaluate((select) => {
    const text = select.options[select.selectedIndex]?.text ?? ''
    const style = getComputedStyle(select)
    const probe = document.createElement('span')
    probe.style.font = style.font
    probe.style.position = 'absolute'
    probe.style.whiteSpace = 'pre'
    probe.textContent = text
    document.body.append(probe)
    const needed = probe.getBoundingClientRect().width + parseFloat(style.paddingLeft) + parseFloat(style.paddingRight)
    probe.remove()
    return { text, room: select.clientWidth, needed }
  })
  h.step('the composition’s name fits its drop-down beside the panels', shownName.text === 'Comp 1' && shownName.room >= shownName.needed - 1, JSON.stringify(shownName))
  const time = await box('motion-timecode')
  const duration = await box('motion-duration')
  h.step('the time and the duration are on one line', Math.abs(time.y - duration.y) < 2 && duration.x > time.x, JSON.stringify({ time, duration }))
  h.step('the transport is one line high', transport.height < 44, String(transport.height))
  const overflow = await kit.getByTestId('motion-transport').evaluate((bar) => [...bar.children].filter((child) => child.getBoundingClientRect().right > bar.getBoundingClientRect().right + 1).map((child) => child.className))
  h.step('nothing sticks out of it', overflow.length === 0, overflow.join(','))
  const play = await box('motion-play')
  await kit.getByTestId('motion-play').click()
  await h.until(async () => (await kit.getByTestId('motion-fps').getAttribute('data-shown')) === 'true', 40)
  const playing = await box('motion-play')
  h.step('the play button keeps its place while playing', Math.abs(playing.x - play.x) < 1, `${play.x} → ${playing.x}`)
  await kit.getByTestId('motion-play').click()
  await h.until(async () => (await kit.getByTestId('motion-play').getAttribute('aria-label')) === '播放')

  // 2. The Render Queue in the side column: one-line title, the job's name readable, Stop a stop button.
  await h.setUp([{ command: 'renderQueue.add', params: { comp: (await h.details()).comp.id, timeSpan: 'comp', format: 'gif', output: 'renders/长的.gif' } }])
  await kit.getByTestId('motion-menu-File').click()
  await kit.locator('[role=menu] [role=menuitem]', { hasText: /^导出(?!…)/ }).first().hover()
  await kit.locator('[role=menu] [data-testid="motion-control-render-queue"]').first().click()
  await kit.getByTestId('motion-queue-panel').waitFor()
  const jobs = (await h.query('renderQueue.list', {})).items ?? []
  const job = jobs[0]
  const name = kit.getByTestId(`motion-queue-item-${job.id}`).locator('.motion-queue-comp')
  const fits = async (locator) => locator.evaluate((element) => element.scrollWidth <= element.clientWidth + 1)
  h.step('the job’s composition name is not cut', (await name.textContent()) === 'Comp 1' && (await fits(name)), await name.textContent())
  const titleHeight = async () => (await kit.getByTestId('motion-queue-panel').locator('.motion-queue-title').boundingBox()).height
  await kit.getByTestId('motion-queue-render').click()
  await kit.getByTestId('motion-queue-stop').waitFor()
  h.step('while rendering the title stays on one line', (await titleHeight()) < 22, String(await titleHeight()))
  const stop = kit.getByTestId('motion-queue-stop')
  const fill = await stop.locator('svg').evaluate((svg) => getComputedStyle(svg).fill)
  h.step('Stop shows a filled square, apart from the job’s check box', fill !== 'none' && (await stop.boundingBox()).y + 4 < (await kit.getByTestId(`motion-queue-item-${job.id}-on`).boundingBox()).y, fill)
  h.step('and its name and Stop All fit', (await fits(stop)) && (await fits(kit.getByTestId('motion-queue-stop-all'))))
  await kit.getByTestId('motion-queue-stop-all').click()
  await h.until(async () => (await kit.getByTestId('motion-queue-render').count()) === 1, 200)
  await kit.getByTestId('motion-queue-close').click()

  // 3. The keyframe dialog: the property's name in Chinese; a 2D layer's Position has x and y only.
  const keyTimes = async () => ((await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).keys ?? []).map((key) => key.compTime ?? key.time)
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  await page.keyboard.press('KeyP')
  const last = Math.max(...(await keyTimes()))
  const lastMark = kit.getByTestId(`motion-key-${SHAPE}-transform.position-${Math.round(last * 1000)}`)
  await lastMark.waitFor()
  await lastMark.dblclick()
  await kit.getByTestId('motion-key-dialog').waitFor()
  const title = await kit.getByTestId('motion-key-dialog').locator('h2, [role=heading], .kui-dialog-title').first().textContent().catch(async () => kit.getByTestId('motion-key-dialog').textContent())
  h.step('the dialog names Position in Chinese', title.includes('位置') && !title.includes('Position'), title)
  h.step('a 2D layer’s Position shows X and Y, no Z', (await kit.getByTestId('motion-key-value-1').count()) === 1 && (await kit.getByTestId('motion-key-value-2').count()) === 0)
  const before = (await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).keys.find((key) => Math.abs((key.compTime ?? key.time) - last) < 1e-3).value
  await kit.getByTestId('motion-key-value-0').fill(String(before[0] + 25))
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).keys.some((key) => Math.abs(key.value[0] - before[0] - 25) < 1e-3))
  const typed = (await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).keys.find((key) => Math.abs((key.compTime ?? key.time) - last) < 1e-3).value
  h.step('typing x keeps y and the hidden z', Math.abs(typed[0] - before[0] - 25) < 1e-3 && Math.abs(typed[1] - before[1]) < 1e-3 && Math.abs((typed[2] ?? 0) - (before[2] ?? 0)) < 1e-3, JSON.stringify({ before, typed }))
  await h.undo()
  await h.until(async () => (await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).keys.some((key) => Math.abs(key.value[0] - before[0]) < 1e-3))
  h.step('undo puts x back', (await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).keys.some((key) => Math.abs(key.value[0] - before[0]) < 1e-3))

  // 4. Rove Across Time on the first keyframe: off, saying why.
  const first = Math.min(...(await keyTimes()))
  const firstMark = kit.getByTestId(`motion-key-${SHAPE}-transform.position-${Math.round(first * 1000)}`)
  await firstMark.click()
  await firstMark.click({ button: 'right' })
  await kit.getByTestId('motion-key-menu-rove').waitFor()
  await h.until(async () => (await kit.getByTestId('motion-key-menu-rove').isDisabled()))
  h.step('Rove Across Time is off on the first keyframe', await kit.getByTestId('motion-key-menu-rove').isDisabled())
  h.step('its tooltip says why, in Chinese', ((await kit.getByTestId('motion-key-menu-rove-row').getAttribute('title')) ?? '').includes('第一个和最后一个关键帧'))
  await page.keyboard.press('Escape')

  // 5. File ▸ New ▸ New Folder: its name is ready to type over.
  let items = await h.items()
  await kit.getByTestId('motion-menu-File').click()
  await kit.locator('[data-kit-ui-id="menu:File/New"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-cmd-project.newFolder"]').first().click()
  await h.until(async () => (await h.items()).length === items.length + 1)
  const folder = (await h.items()).find((item) => !items.some((old) => old.id === item.id))
  const field = kit.getByTestId(`motion-item-rename-${folder?.id}`)
  await h.until(async () => (await field.count()) === 1)
  h.step('File ▸ New ▸ New Folder starts renaming it', (await field.count()) === 1)
  await page.keyboard.type('素材')
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.items()).some((item) => item.id === folder.id && item.name === '素材'))
  h.step('typing names it', (await h.items()).some((item) => item.id === folder.id && item.name === '素材'))
  await h.undo()
  await h.until(async () => (await h.items()).some((item) => item.id === folder.id && item.name !== '素材'))
  await h.undo()
  await h.until(async () => (await h.items()).length === items.length)
  h.step('undo takes the name and the folder away', (await h.items()).length === items.length)

  // 6. The pointer readout empties when the pointer leaves the composition.
  const info = kit.getByTestId('motion-info')
  const centre = await h.at(640, 360)
  await page.mouse.move(centre.x - 20, centre.y - 20)
  await page.mouse.move(centre.x, centre.y, { steps: 3 })
  await h.until(async () => (await info.count()) === 1)
  h.step('the readout shows over the composition', (await info.count()) === 1)
  const rows = await box('motion-timeline-rows')
  await page.mouse.move(rows.x + rows.width / 2, rows.y + 10, { steps: 4 })
  await h.until(async () => (await info.count()) === 0)
  h.step('and empties over the timeline', (await info.count()) === 0)

  // 7. The switch columns' short names: two characters where one was ambiguous.
  await kit.getByTestId('motion-column-header').click({ button: 'right' })
  await kit.getByTestId('motion-column-text').click()
  await h.until(async () => (await kit.locator('.motion-column-short').count()) > 0)
  const heads = await kit.locator('.motion-column-short').allTextContents()
  h.step('Motion Blur reads 模糊, not 模 (beside 模式)', heads.includes('模糊') && !heads.includes('模'), heads.join(' '))
  h.step('and Shy 消隐, effects 效果', heads.includes('消隐') && heads.includes('效果'), heads.join(' '))
  await kit.getByTestId('motion-column-header').click({ button: 'right' })
  await kit.getByTestId('motion-column-text').click()
  await h.until(async () => (await kit.locator('.motion-column-short').count()) === 0)

  // 8. The shape Add menu: submenus say what they hold; its search finds Gradient Fill.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  const add = kit.getByTestId('motion-shape-add')
  await add.scrollIntoViewIfNeeded()
  await add.click()
  const paints = kit.getByTestId('motion-shape-add-menu-paints')
  await paints.waitFor()
  h.step('the paints submenu says it holds gradients', (await paints.textContent()).includes('渐变'), await paints.textContent())
  const search = kit.getByTestId('motion-shape-add-search')
  h.step('the menu did not put the focus in its search field', !(await search.evaluate((input) => input === document.activeElement)))
  await search.click()
  await page.keyboard.type('渐变填充')
  await h.until(async () => (await kit.getByTestId('motion-shape-add-gfill').count()) === 1)
  h.step('typing 渐变填充 lists Gradient Fill', (await kit.getByTestId('motion-shape-add-gfill').count()) === 1 && (await kit.getByTestId('motion-shape-add-menu-paints').count()) === 0)
  const contents = async () => JSON.stringify(await h.query('layer.tree', { layer: SHAPE }))
  const was = await contents()
  await page.keyboard.press('Enter')
  await h.until(async () => (await contents()) !== was && (await contents()).includes('gfill'))
  h.step('Enter adds it', (await contents()).includes('gfill'))
  await h.undo()
  await h.until(async () => (await contents()) === was)
  h.step('undo removes it', (await contents()) === was)

  // 9. The 3D views' keys: Alt+Shift+1 and F10 both give the Front view.
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await kit.getByTestId('motion-3d-view').waitFor()
  await h.focusEditor()
  await page.keyboard.press('Alt+Shift+Digit1')
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'front')
  h.step('Alt+Shift+1 gives the Front view', (await h.query('view.get3D', {}))?.view === 'front')
  await page.keyboard.press('Alt+Shift+Digit3')
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'activeCamera')
  await page.keyboard.press('F10')
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'front')
  h.step('F10 too', (await h.query('view.get3D', {}))?.view === 'front')
  await page.keyboard.press('Alt+Shift+Digit3')
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'activeCamera')
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: false } }])

  // 10. A Project item onto the timeline with one plain press, move and release.
  await kit.getByTestId('motion-project-toggle').isVisible()
  if ((await kit.getByTestId('motion-project-panel').count()) === 0) await kit.getByTestId('motion-project-toggle').click()
  const solid = (await h.items()).find((item) => item.type === 'Solid')
  const solidsFolder = (await h.items()).find((item) => item.id === solid.parent)
  if (solidsFolder) await kit.getByTestId(`motion-item-twirl-${solidsFolder.id}`).click()
  const source = kit.getByTestId(`motion-item-${solid.id}`)
  await source.waitFor()
  let layers = await h.layers()
  const from = await h.centre(source)
  const ruler = await box('motion-ruler')
  const to = { x: ruler.x + ruler.width / 2, y: rows.y + rows.height / 2 }
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  await page.mouse.move(to.x, to.y, { steps: 6 })
  await page.mouse.up()
  await h.until(async () => (await h.layers()).length === layers.length + 1)
  h.step('a plain press, move and release onto the timeline adds the layer', (await h.layers()).length === layers.length + 1, h.which)
  if ((await h.layers()).length === layers.length + 1) {
    await h.undo()
    await h.until(async () => (await h.layers()).length === layers.length)
  }
  // Without dragging: the item's menu adds it too.
  await source.click({ button: 'right' })
  await kit.getByTestId('motion-item-menu-add').click()
  await h.until(async () => (await h.layers()).length === layers.length + 1)
  h.step('its menu’s Add to Composition adds it without a drag', (await h.layers()).length === layers.length + 1)
  await h.undo()
  await h.until(async () => (await h.layers()).length === layers.length)
  h.step('undo removes it', (await h.layers()).length === layers.length)

  // 11. A text selection follows characters an animator moves (the canvas's highlight).
  await kit.getByTestId('motion-tool-text').click()
  layers = await h.layers()
  const typedAt = await h.at(300, 600)
  await page.mouse.click(typedAt.x, typedAt.y)
  await page.keyboard.type('ABCD')
  await h.until(async () => (await h.layers()).length === layers.length + 1)
  const text = newest(layers, await h.layers())
  await page.keyboard.press('Shift+Home')
  // The highlight is drawn on the canvas: the topmost row of a column through the text that has
  // its blue (a translucent rgb(62 136 255) over the dark background), in page pixels.
  const column = await h.at(306, 520)
  const highlightTop = async () => {
    const clip = { x: Math.round(column.x), y: Math.round(column.y), width: 1, height: 260 }
    const shot = await page.screenshot({ clip })
    const top = await page.evaluate(async (data) => {
      const image = await createImageBitmap(await (await fetch(`data:image/png;base64,${data}`)).blob())
      const canvas = new OffscreenCanvas(image.width, image.height)
      const context = canvas.getContext('2d')
      context.drawImage(image, 0, 0)
      const pixels = context.getImageData(0, 0, image.width, image.height).data
      for (let y = 0; y < image.height; y += 1) {
        const [r, g, b] = pixels.slice(y * image.width * 4, y * image.width * 4 + 3)
        if (b > 130 && b - r > 80 && g > 55 && g < 120) return y / (image.height / 260)
      }
      return null
    }, shot.toString('base64'))
    return top == null ? null : clip.y + top
  }
  // The selection drawn and settled (two reads apart the same) before it is measured.
  let seen = null
  await h.until(async () => {
    const now = await highlightTop()
    const settled = now != null && now === seen
    seen = now
    return settled
  })
  const still = await highlightTop()
  await h.setUp([
    { command: 'layer.addTextAnimator', params: { layer: text.id, properties: ['position'] } },
    { command: 'prop.set', params: { layer: text.id, path: 'text/animators/#1/properties/position', value: [0, 80, 0] } },
  ])
  const [zoom] = (await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ').map(Number)
  await h.until(async () => Math.abs((await highlightTop()) - still - 80 * zoom) < 3)
  const moved = await highlightTop()
  h.step('the selection’s highlight moves down with the characters', still != null && Math.abs(moved - still - 80 * zoom) < 3, `${still} → ${moved} (zoom ${zoom})`)
  await page.keyboard.press('Escape')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
