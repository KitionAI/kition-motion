// The two-view layout by hand, in a real browser through the development host: node tests/twoViews.mjs [chromium|webkit]
// View ▸ Two Views (only with 3D layers) shows a second 3D view beside the viewer: Top, with its
// ground and the 3D layers' outlines; its drop-down picks another view; Make Main swaps it with
// the viewer's; View ▸ One View and its close button hide it. Results are read from the engine
// (`view.get3D`) and the second view's own attributes. Prints PASS or FAIL per step.
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
  await kit.getByTestId('motion-second-view-close').click()
  await h.until(async () => (await second.count()) === 0)
  h.step('its close button hides it too', (await second.count()) === 0)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
