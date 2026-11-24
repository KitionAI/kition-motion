// 3D by hand, in a real browser through the development host: node tests/threeD.mjs [chromium|webkit]
// The New Layer menu adds a camera and a light (their options in the Properties panel); a 3D
// layer's Z position, orientation and X/Y rotation show in the panel and the timeline; the status
// bar's 3D view menu and View ▸ 3D View switch between the active camera, Front, Top and the
// custom views; the camera tools (on the rail, and C to cycle them) orbit, pan and dolly the
// active camera (one undo step per drag) or a custom view (the picture changes, no edit).
// Results are read from the engine (read-only queries) and the canvas's pixels. Prints PASS or FAIL.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const SHAPE = 27
const view3d = async () => h.query('view.get3D', {})
const value = async (layer, path) => (await h.query('prop.get', { layer, path }))?.value
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)
const activeTool = async () => (await kit.locator('[data-testid^="motion-tool-"][aria-pressed="true"]').getAttribute('data-testid'))?.replace('motion-tool-', '')

/** A drag across the middle of the canvas, `dx`, `dy` screen pixels, in small steps. */
async function dragCanvas(dx, dy) {
  const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
  const from = { x: surface.x + surface.width / 2, y: surface.y + surface.height / 2 }
  await h.drag(from, { x: from.x + dx, y: from.y + dy }, 12)
}

try {
  await h.open()

  // 1. New Layer ▸ Camera and Light; their options in the Properties panel.
  const count = (await h.layers()).length
  await kit.getByTestId('motion-new-layer').click()
  await kit.getByTestId('motion-new-camera').click()
  await until(async () => (await h.layers()).some((layer) => layer.type === 'Camera'))
  const camera = (await h.layers()).find((layer) => layer.type === 'Camera')
  step('新建图层 ▸ 摄像机 adds a camera layer', !!camera && (await h.layers()).length === count + 1)
  await kit.getByTestId(`motion-layer-${camera.id}`).locator('.motion-row-name').click()
  await kit.getByTestId('motion-section-cameraOptions').waitFor({ timeout: 5000 }).catch(() => null)
  step('the Properties panel shows its Camera Options and its Point of Interest', (await kit.getByTestId('motion-section-cameraOptions').count()) === 1 && (await kit.getByTestId('motion-prop-poi').count()) === 1)
  await kit.getByTestId('motion-new-layer').click()
  await kit.getByTestId('motion-new-light').click()
  await until(async () => (await h.layers()).some((layer) => layer.type === 'Light'))
  const light = (await h.layers()).find((layer) => layer.type === 'Light')
  await kit.getByTestId(`motion-layer-${light.id}`).locator('.motion-row-name').click()
  await kit.getByTestId('motion-section-lightOptions').waitFor({ timeout: 5000 }).catch(() => null)
  step('新建图层 ▸ 灯光 adds a light with its Light Options in the panel', (await kit.getByTestId('motion-section-lightOptions').count()) === 1)

  // 2. A 3D layer's Z position, orientation and rotations in the panel and on the timeline.
  await kit.getByTestId(`motion-switch-threeD-${SHAPE}`).click()
  await until(async () => (await h.layer(SHAPE)).switches.three_d === true)
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await kit.getByTestId('motion-prop-orientation').waitFor({ timeout: 5000 }).catch(() => null)
  step('a 3D layer shows Orientation and X and Y Rotation in the panel', (await kit.getByTestId('motion-prop-orientation').count()) === 1 && (await kit.getByTestId('motion-prop-rotationX').count()) === 1 && (await kit.getByTestId('motion-prop-rotationY').count()) === 1)
  const z = kit.getByTestId('motion-value-position-2')
  await z.click()
  await z.fill('-200')
  await z.press('Enter')
  await until(async () => (await value(SHAPE, 'transform/position'))?.[2] === -200)
  step('typing −200 in Position Z moves it in depth', (await value(SHAPE, 'transform/position'))?.[2] === -200, JSON.stringify(await value(SHAPE, 'transform/position')))
  const rx = kit.getByTestId('motion-value-rotationX-0')
  await rx.click()
  await rx.fill('30')
  await rx.press('Enter')
  await until(async () => (await value(SHAPE, 'transform/rotationX')) === 30)
  step('typing 30 in X Rotation turns it', (await value(SHAPE, 'transform/rotationX')) === 30)
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  await kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.orientation`).waitFor({ timeout: 5000 }).catch(() => null)
  step('the timeline shows its Orientation row with a stopwatch', (await kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.orientation`).count()) === 1)
  await kit.getByTestId(`motion-row-add-key-${SHAPE}-transform.rotationY`).click()
  await until(async () => (await h.layer(SHAPE)).animated.some((property) => property.path === 'transform/rotationY'))
  step('its Y Rotation stopwatch on the timeline keyframes it', (await h.layer(SHAPE)).animated.some((property) => property.path === 'transform/rotationY'))
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()

  // 3. The 3D views: the status bar's menu, and View ▸ 3D View.
  const views = kit.getByTestId('motion-3d-view')
  await views.waitFor({ timeout: 5000 })
  step('the status bar offers the 3D views, at Active Camera', (await views.inputValue()) === 'activeCamera' && (await view3d()).view === 'activeCamera')
  /** The canvas as it shows now (a PNG of its surface). */
  const picture = async () => (await kit.getByTestId('kit-canvas-surface').screenshot()).toString('base64')
  await views.selectOption('top')
  await until(async () => (await view3d()).view === 'top')
  step('choosing 顶部 shows the Top view', (await view3d()).view === 'top' && (await kit.getByTestId('motion-workspace').getAttribute('data-view3d')) === 'top')
  await kit.getByTestId('motion-menu-View').click()
  await kit.locator('[role=menu] [role=menuitem]', { hasText: '3D 视图' }).first().hover()
  await kit.locator('[role=menu] [data-testid="motion-cmd-view.3d.front"]').first().click()
  await until(async () => (await view3d()).view === 'front')
  step('View ▸ 3D 视图 ▸ 正面 shows the Front view', (await view3d()).view === 'front')
  await views.selectOption('activeCamera')
  await until(async () => (await view3d()).view === 'activeCamera')

  // 4. The camera tools: C picks Orbit, then Pan, then Dolly; each drag moves the active camera.
  await h.focusEditor()
  await page.keyboard.press('KeyC')
  await until(async () => (await activeTool()) === 'orbit')
  step('C picks the Orbit Camera tool', (await activeTool()) === 'orbit', await activeTool())
  const eye = async () => value(camera.id, 'transform/position')
  const start0 = await eye()
  await dragCanvas(120, 0)
  await until(async () => !same(await eye(), start0))
  step('a drag with Orbit turns the camera around its point of interest', !same(await eye(), start0), `${JSON.stringify(start0)} → ${JSON.stringify(await eye())}`)
  await h.undo()
  await until(async () => same(await eye(), start0))
  step('one undo takes the whole orbit back', same(await eye(), start0))
  await h.focusEditor()
  await page.keyboard.press('KeyC')
  await until(async () => (await activeTool()) === 'track')
  const poiBefore = await value(camera.id, 'transform/poi')
  await dragCanvas(0, 80)
  await until(async () => !same(await eye(), start0))
  const [moved, poiAfter] = [await eye(), await value(camera.id, 'transform/poi')]
  step('C again picks Pan, and a drag moves the camera and its point of interest together', (await activeTool()) === 'track' && Math.abs(moved[1] - start0[1] - (poiAfter[1] - poiBefore[1])) < 1e-6 && Math.abs(moved[1] - start0[1]) > 1, `${JSON.stringify(moved)} poi ${JSON.stringify(poiAfter)}`)
  await h.undo()
  await until(async () => same(await eye(), start0))
  await kit.getByTestId('motion-tool-dolly').click()
  await dragCanvas(100, 0)
  await until(async () => !same(await eye(), start0))
  const dollied = await eye()
  step('a drag with Dolly moves the camera along its view', Math.abs(dollied[2] - start0[2]) > 1, `${JSON.stringify(start0)} → ${JSON.stringify(dollied)}`)
  await h.undo()
  await until(async () => same(await eye(), start0))

  // 5. In a custom view the tools move the view, not the camera layer, and the picture follows.
  await views.selectOption('custom1')
  await until(async () => (await view3d()).view === 'custom1')
  await page.waitForTimeout(500)
  const shownBefore = await picture()
  const viewEye = (await view3d()).camera.eye
  await kit.getByTestId('motion-tool-orbit').click()
  await dragCanvas(160, 40)
  await until(async () => !same((await view3d()).camera.eye, viewEye))
  step('Orbit in 自定义视图 1 moves the view and leaves the camera layer', !same((await view3d()).camera.eye, viewEye) && same(await eye(), start0))
  await until(async () => (await picture()) !== shownBefore, 30)
  step('and the canvas shows the new view', (await picture()) !== shownBefore)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
