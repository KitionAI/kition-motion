// The two-view layout by hand, in a real browser through the development host: node tests/twoViews.mjs [chromium|webkit]
// View ▸ Two Views (only with 3D layers) shows a second 3D view beside the viewer: Top, with its
// ground and the 3D layers' outlines; its drop-down picks another view; Make Main swaps it with
// the viewer's; View ▸ One View and its close button hide it. Round 7g: it starts at about 32 % of
// a 1440 px window, its drawing area covers the pane on the viewer's background (the picture fitted in it), a click on
// a layer in it selects that layer, the bar on its left edge drags it wider or narrower (at least
// 200 px, the viewer keeps 45 %, kept for the session), and while playback runs it says it is not
// updated. Results are read from the engine (`view.get3D`, `comp.info`) and the second view's own
// attributes. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const second = kit.getByTestId('motion-second-view')
const viewRow = (text) => kit.locator('[role=menu] [role=menuitem]', { hasText: text }).first()
async function viewMenu() {
  await kit.getByTestId('motion-menu-View').click()
  await kit.locator('[role=menu]').first().waitFor()
}

try {
  await h.open()
  await viewMenu()
  h.step('without 3D layers View has no Two Views', (await viewRow('两个视图').count()) === 0)
  await page.keyboard.press('Escape')
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await kit.getByTestId('motion-3d-view').waitFor()
  const before = await kit.getByTestId('kit-canvas').boundingBox()

  // 1. View ▸ Two Views: Top beside the viewer, with its ground.
  await viewMenu()
  await viewRow('两个视图').click()
  await second.waitFor()
  await h.until(async () => Number(await second.getAttribute('data-lines')) > 10)
  h.step('View ▸ 两个视图 shows the Top view beside the viewer', (await second.getAttribute('data-view')) === 'top' && Number(await second.getAttribute('data-lines')) > 10, `${await second.getAttribute('data-view')} ${await second.getAttribute('data-lines')}`)
  const after = await kit.getByTestId('kit-canvas').boundingBox()
  const side = await second.boundingBox()
  h.step('the viewer makes room: the second view is to its right, both inside the window', after.width < before.width - 100 && side.x >= after.x + after.width - 1 && side.x + side.width <= 1440, JSON.stringify({ before: before.width, after: after.width, side }))
  h.step('its picture is drawn', ((await second.getAttribute('data-size')) ?? '').split(' ').every((n) => Number(n) > 50), await second.getAttribute('data-size'))
  h.step('the viewer keeps its own view', (await h.query('view.get3D', {}))?.view === 'activeCamera')

  // Round 7g. Its width, its background, a click to select, the divider, the note while playing.
  const room = (await kit.getByTestId('motion-views').boundingBox()).width
  const width = async () => Number(await second.getAttribute('data-width'))
  const expected = Math.min(Math.round(1440 * 0.32), Math.floor(room * 0.55))
  h.step('it starts at about 32 % of the window (the viewer keeps at least 45 %)', Math.abs((await width()) - expected) <= 2 && Math.abs(side.width - expected) <= 3, `${await width()} (${side.width} px) of ${room}, expected ${expected}`)
  const body = second.locator('.motion-second-body')
  const looks = await body.evaluate((element, viewerSelector) => {
    const viewer = element.ownerDocument.querySelector(viewerSelector)
    const canvas = element.querySelector('canvas')
    const box = element.getBoundingClientRect()
    const drawn = canvas.getBoundingClientRect()
    return { background: getComputedStyle(element).backgroundColor, viewer: viewer ? getComputedStyle(viewer).backgroundColor : null, box: [box.top, box.height, box.width], canvas: [drawn.top, drawn.height, drawn.width] }
  }, '.motion-views > .kui-canvas-region')
  h.step('its drawing area covers the whole pane, on the viewer’s background (the picture fitted in it)', looks.background === looks.viewer && Math.abs(looks.box[0] - looks.canvas[0]) <= 1 && Math.abs(looks.box[1] - looks.canvas[1]) <= 1 && Math.abs(looks.box[2] - looks.canvas[2]) <= 1 && looks.box[1] > 150, JSON.stringify(looks))
  // A click on the layer in the Top view selects it (another layer selected first).
  const others = (await h.layers()).filter((layer) => layer.id !== SHAPE)
  await kit.getByTestId(`motion-layer-${others[0].id}`).locator('.motion-row-name').first().click()
  await h.until(async () => !(await h.layer(SHAPE))?.selected && ((await second.getAttribute('data-layers')) ?? '').split(' ').includes(String(SHAPE)))
  const centre = ((await second.getAttribute('data-centres')) ?? '').split(' ').find((each) => each.startsWith(`${SHAPE}:`))
  const [cx, cy] = (centre ?? ':0,0').split(':')[1].split(',').map(Number)
  const canvasBox = await kit.getByTestId('motion-second-view-canvas').boundingBox()
  await page.mouse.click(canvasBox.x + cx, canvasBox.y + cy)
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  h.step('a click on a layer in the second view selects it', (await h.layer(SHAPE))?.selected === true && !(await h.layer(others[0].id))?.selected, centre)
  await h.until(async () => ((await second.getAttribute('data-layers')) ?? '').includes(`${SHAPE}*`))
  h.step('and the second view marks it selected', ((await second.getAttribute('data-layers')) ?? '').includes(`${SHAPE}*`))
  // The divider: wider, then as narrow as it goes, then as wide as the viewer allows.
  const divider = kit.getByTestId('motion-second-view-divider')
  const grip = await h.centre(divider)
  const was = await width()
  await h.drag(grip, { x: grip.x - 120, y: grip.y })
  await h.until(async () => (await width()) > was + 100)
  h.step('dragging the divider left widens the second view', Math.abs((await width()) - (was + 120)) <= 4, `${was} → ${await width()}`)
  await h.drag(await h.centre(divider), { x: 1430, y: grip.y })
  await h.until(async () => (await width()) === 200)
  h.step('it is never narrower than 200 px', (await width()) === 200, String(await width()))
  await h.drag(await h.centre(divider), { x: 10, y: grip.y })
  await h.until(async () => (await width()) > 400)
  const viewerNow = (await kit.locator('.motion-views > .kui-canvas-region').boundingBox()).width
  h.step('the viewer keeps at least 45 % however far it is dragged', viewerNow >= room * 0.45 - 2, `${viewerNow} of ${room}`)
  await h.drag(await h.centre(divider), { x: (await h.centre(divider)).x + 150, y: grip.y })
  await page.waitForTimeout(300)
  const kept = await width()
  // While playback runs it keeps its picture and says so.
  await h.focusEditor()
  await page.keyboard.press('Space')
  await h.until(async () => (await kit.getByTestId('motion-second-view-paused').count()) === 1, 30)
  h.step('while playing it says it is not updated', ((await kit.getByTestId('motion-second-view-paused').textContent().catch(() => '')) ?? '').includes('播放时不更新'))
  await page.keyboard.press('Space')
  await h.until(async () => (await kit.getByTestId('motion-second-view-paused').count()) === 0, 30)

  // 2. Its drop-down picks another view.
  await kit.getByTestId('motion-second-view-select').selectOption('left')
  await h.until(async () => (await second.getAttribute('data-view')) === 'left')
  const labels = await kit.getByTestId('motion-second-view-select').locator('option').allTextContents()
  h.step('its drop-down picks another view, in Chinese', (await second.getAttribute('data-view')) === 'left' && labels.some((label) => /左/.test(label)), labels.join(' '))

  // 3. Make Main swaps it with the viewer's view.
  await kit.getByTestId('motion-second-view-main').click()
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'left')
  await h.until(async () => (await second.getAttribute('data-view')) === 'activeCamera')
  h.step('Make Main shows it in the viewer and the viewer’s view here', (await h.query('view.get3D', {}))?.view === 'left' && (await second.getAttribute('data-view')) === 'activeCamera')

  // 4. View ▸ One View hides it; the close button too.
  await viewMenu()
  await viewRow('一个视图').click()
  await h.until(async () => (await second.count()) === 0)
  h.step('View ▸ 一个视图 hides it', (await second.count()) === 0)
  await viewMenu()
  await viewRow('两个视图').click()
  await second.waitFor()
  await h.until(async () => Number(await second.getAttribute('data-width')) > 0)
  h.step('shown again, it keeps the width it was dragged to', Math.abs(Number(await second.getAttribute('data-width')) - kept) <= 1, `${await second.getAttribute('data-width')} (was ${kept})`)
  await kit.getByTestId('motion-second-view-close').click()
  await h.until(async () => (await second.count()) === 0)
  h.step('its close button hides it too', (await second.count()) === 0)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
