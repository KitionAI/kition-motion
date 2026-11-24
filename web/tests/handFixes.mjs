// The phase 7c hand-test fixes by hand, in a real browser through the development host:
// node tests/handFixes.mjs [chromium|webkit]
// The transport keeps its buttons in place while the frame rate shows (a second click on Play
// pauses), Loop goes round the work area, switch buttons look pressed, the status bar keeps to one
// line, Purge says so; a new light is a point light that keeps a 3D layer lit and its type changes
// in the panel; the camera tools show only with something 3D; a 3D view other than the camera's
// leaves 2D layers out and Alt+Shift+1 picks Front; a gradient on top is what the shape's quick
// settings edit, and they stay after Convert to Bezier Path; a property added to a text animator
// takes the focus; a drop shadow shows and Layer ▸ Layer Styles marks it; the Time section says
// the source time and what Freeze Frame did; the names split keeps both sides readable.
// Results are read from the engine (read-only queries), the page and its pixels. Prints PASS or FAIL.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const SHAPE = 27
const workspace = () => kit.getByTestId('motion-workspace')
const playing = async () => (await kit.getByTestId('motion-play').getAttribute('aria-label')) === '暂停'
const toasts = async () => (await kit.locator('.kui-toast').allTextContents()).join(' | ')
const background = (locator) => locator.evaluate((element) => getComputedStyle(element).backgroundColor)
/** The node at `path` in a layer's property tree. */
function node(tree, path) {
  if (tree?.path === path) return tree
  for (const child of tree?.children ?? []) {
    const found = node(child, path)
    if (found) return found
  }
  return null
}
const tree = async (layer) => h.query('layer.tree', { layer, depth: 12 })
/** Selects a layer by a click on its name in the timeline. */
async function pick(layer) {
  await kit.getByTestId(`motion-layer-${layer}`).locator('.motion-row-name').click()
  await until(async () => (await h.layer(layer))?.selected)
}

try {
  // 1. The transport: the frame rate's room is kept, a second click pauses, Loop goes round.
  await h.open()
  await h.focusEditor()
  const loop = kit.getByTestId('motion-loop')
  step('Loop is on by default and looks pressed', (await loop.getAttribute('aria-pressed')) === 'true')
  const pressedLook = await background(loop)
  await loop.click()
  await until(async () => (await loop.getAttribute('aria-pressed')) === 'false')
  await page.mouse.move(10, 10)
  await page.waitForTimeout(200)
  step('a switch button looks different on and off (Loop)', (await background(loop)) !== pressedLook, `${pressedLook} / ${await background(loop)}`)
  await loop.click()
  await until(async () => (await loop.getAttribute('aria-pressed')) === 'true')
  const before = await kit.getByTestId('motion-play').boundingBox()
  await kit.getByTestId('motion-play').click()
  await until(playing, 30)
  await until(async () => (await kit.getByTestId('motion-fps').getAttribute('data-shown')) === 'true', 40)
  const during = await kit.getByTestId('motion-play').boundingBox()
  step('the play button stays where it was once the frame rate shows', Math.abs(during.x - before.x) < 1 && (await kit.getByTestId('motion-fps').getAttribute('data-shown')) === 'true', `${before.x} → ${during.x}`)
  // Looping: the time comes round to the start again (the sample is 3 s long).
  let wrapped = false
  let last = -1
  for (let i = 0; i < 60 && !wrapped; i += 1) {
    const now = (await kit.getByTestId('motion-timecode').textContent()) ?? ''
    const seconds = Number(now.split(':')[2]) + Number(now.split(':')[3]) / 30
    if (seconds < last - 0.5) wrapped = true
    last = seconds
    await page.waitForTimeout(100)
  }
  step('with Loop on, playback goes round the work area', wrapped && (await playing()))
  await page.mouse.click(during.x + during.width / 2, during.y + during.height / 2)
  await until(async () => !(await playing()), 30)
  step('a second click on the same place pauses', !(await playing()))
  // Purge Preview Cache says so.
  await until(async () => Number(await workspace().getAttribute('data-cached')) > 0)
  await kit.getByTestId('motion-menu-View').click()
  await kit.locator('[role=menu] [data-testid="motion-control-purge-cache"]').first().click()
  await until(async () => (await toasts()).includes('已清除预览缓存'))
  step('Purge Preview Cache says it purged the frames', (await toasts()).includes('已清除预览缓存'), await toasts())

  // 2. The status bar keeps to one line with the Pen's long hint; the hint is in its tooltip.
  await pick(SHAPE)
  await h.focusEditor()
  await page.keyboard.press('g')
  await kit.getByTestId('motion-tool-status').waitFor({ timeout: 5000 })
  const bar = await kit.locator('.kui-status-bar').boundingBox()
  const hint = kit.getByTestId('motion-tool-status')
  step('the status bar stays one line high with the Pen’s hint', bar.height < 40, String(bar.height))
  step('the hint’s whole text is in its tooltip', ((await hint.getAttribute('title')) ?? '').includes('画蒙版') && (await hint.textContent()) === (await hint.getAttribute('title')))
  step('Draw mask is a switch that looks pressed when on', (await kit.getByTestId('motion-pen-mask').getAttribute('aria-pressed')) === 'false')
  await page.keyboard.press('v')

  // 3. A light: a point light named in Chinese that keeps a 3D layer lit; its type changes in the panel.
  step('the camera tools are off the rail with nothing 3D', (await kit.getByTestId('motion-tool-orbit').count()) === 0)
  // The shape's middle (its position), where the light falls.
  const centre = ((await h.query('prop.get', { layer: SHAPE, path: 'transform/position' }))?.value ?? [0, 0]).slice(0, 2)
  await kit.getByTestId(`motion-switch-threeD-${SHAPE}`).click()
  await until(async () => (await h.layer(SHAPE)).switches.three_d === true)
  await kit.getByTestId('motion-tool-orbit').waitFor({ timeout: 5000 }).catch(() => null)
  step('they are there once a layer is 3D', (await kit.getByTestId('motion-tool-orbit').count()) === 1)
  await page.waitForTimeout(500)
  const unlit = await h.rgb(...centre)
  await kit.getByTestId('motion-new-layer').click()
  await kit.getByTestId('motion-new-light').click()
  await until(async () => (await h.layers()).some((layer) => layer.type === 'Light'))
  const light = (await h.layers()).find((layer) => layer.type === 'Light')
  step('a new light is named 灯光 1 and is a point light', light.name === '灯光 1' && (await tree(light.id))?.lightKind === 'Point', `${light.name} ${(await tree(light.id))?.lightKind}`)
  await page.waitForTimeout(800)
  const lit = await h.rgb(...centre)
  const sum = (rgb) => rgb[0] + rgb[1] + rgb[2]
  step('the 3D layer stays lit (not black) under the new light', sum(unlit) > 150 && sum(lit) > sum(unlit) * 0.6, `${JSON.stringify(unlit)} → ${JSON.stringify(lit)}`)
  await pick(light.id)
  const type = kit.getByTestId('motion-light-type')
  await type.waitFor({ timeout: 5000 })
  step('the light’s panel has its type, with no blending mode, effects or layer styles', (await type.inputValue()) === 'Point' && (await kit.getByTestId('motion-blend-mode').count()) === 0 && (await kit.getByTestId('motion-section-layerStyles').count()) === 0)
  await type.selectOption('Spot')
  await until(async () => (await tree(light.id))?.lightKind === 'Spot')
  step('Light Type ▸ 聚光 makes it a spot light', (await tree(light.id))?.lightKind === 'Spot')

  // 4. A 3D view other than the camera's leaves the 2D layers out; Alt+Shift+1 picks Front.
  const corner = [20, 20]
  const withNight = await h.rgb(...corner)
  await kit.getByTestId('motion-3d-view').selectOption('top')
  await until(async () => (await workspace().getAttribute('data-view3d')) === 'top')
  await until(async () => JSON.stringify(await h.rgb(...corner)) !== JSON.stringify(withNight))
  step('the Top view does not draw the 2D background layer', JSON.stringify(await h.rgb(...corner)) !== JSON.stringify(withNight), `${JSON.stringify(withNight)} → ${JSON.stringify(await h.rgb(...corner))}`)
  await h.focusEditor()
  await page.keyboard.press('Alt+Shift+Digit1')
  await until(async () => (await workspace().getAttribute('data-view3d')) === 'front')
  step('Alt+Shift+1 switches to the Front view', (await workspace().getAttribute('data-view3d')) === 'front')
  await page.keyboard.press('Alt+Shift+Digit3')
  await until(async () => (await workspace().getAttribute('data-view3d')) === 'activeCamera')
  step('Alt+Shift+3 back to the active camera', (await workspace().getAttribute('data-view3d')) === 'activeCamera')

  // 5. Shapes: a gradient on top is what the quick settings edit; they stay after a convert.
  await h.open()
  await pick(SHAPE)
  await kit.getByTestId('motion-shape-add').click()
  await kit.getByTestId('motion-shape-add-menu-paints').hover()
  await kit.getByTestId('motion-shape-add-gfill').click()
  const quick = kit.getByTestId('motion-shape-controls')
  await until(async () => (await quick.locator('[data-testid="motion-value-quick.contents.group.contents.gfill.colors"]').count()) === 1)
  step('with a gradient fill on top, the quick settings edit its colours', (await quick.locator('[data-testid="motion-value-quick.contents.group.contents.gfill.colors"]').count()) === 1 && (await quick.locator('[data-testid="motion-prop-contents.group.contents.fill.color"]').count()) === 0)
  await h.undo()
  await until(async () => (await quick.locator('[data-testid="motion-prop-contents.group.contents.fill.color"]').count()) === 1)
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[data-kit-ui-id="menu:Layer/Mask and Shape Path"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-cmd-path.convertToBezier"]').first().click()
  await until(async () => node((await tree(SHAPE))?.properties, 'contents/group/contents/path') != null)
  await page.waitForTimeout(400)
  step('after Convert to Bezier Path the quick settings (fill colour) stay', (await quick.locator('[data-testid="motion-prop-contents.group.contents.fill.color"]').count()) === 1)
  const groupName = (await quick.locator('.motion-shape-quick-name').allTextContents()).join(' ')
  step('a group’s name over its quick settings reads in Chinese', !/Rectangle/.test(groupName), groupName)

  // 6. A property added to a text animator takes the focus; the next step is said.
  const text = (await h.layers()).find((layer) => layer.type === 'Text').id
  await pick(text)
  await kit.getByTestId('motion-text-animate').click()
  await kit.getByTestId('motion-text-animate-position').click()
  await until(async () => ((await kit.locator(':focus').getAttribute('data-testid').catch(() => '')) ?? '').startsWith('motion-value-text.animators.animator.properties.position'))
  step('Animate ▸ Position puts the focus in the new Position value', ((await kit.locator(':focus').getAttribute('data-testid').catch(() => '')) ?? '').startsWith('motion-value-text.animators.animator.properties.position'))
  step('the animator says what to do next, and its name shows once', (await kit.getByTestId('motion-animator-1').locator('.motion-animator-hint').count()) === 1 && (await kit.getByTestId('motion-animator-1').getByText('动画制作工具 1', { exact: true }).count()) === 1)
  await page.keyboard.type('0')
  await page.keyboard.press('Escape')

  // 7. A drop shadow shows, and Layer ▸ Layer Styles marks it.
  await pick(SHAPE)
  const add = kit.getByTestId('motion-style-add')
  await add.scrollIntoViewIfNeeded()
  await add.click()
  await kit.getByTestId('motion-style-add-dropShadow').click()
  await until(async () => (await h.query('prop.get', { layer: SHAPE, path: 'layerStyles/dropShadow/distance' }))?.value === 20)
  step('a new drop shadow is 20 px away, 20 px soft and fully dark', (await h.query('prop.get', { layer: SHAPE, path: 'layerStyles/dropShadow/distance' }))?.value === 20 && (await h.query('prop.get', { layer: SHAPE, path: 'layerStyles/dropShadow/opacity' }))?.value === 100)
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[data-kit-ui-id="menu:Layer/Layer Styles"]').hover()
  const row = kit.locator('[role=menu] [data-testid="motion-cmd-layer.style.dropShadow"]').first()
  await row.waitFor({ timeout: 3000 })
  step('Layer ▸ Layer Styles marks the drop shadow as added', ((await row.textContent()) ?? '').includes('已添加'), await row.textContent())
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')

  // 8. Time: the source time as a timecode; Freeze Frame says what it did.
  await h.setUp([{ command: 'layer.precompose', params: { layers: [SHAPE], name: 'Pre' } }])
  await until(async () => (await h.layers()).some((layer) => layer.name === 'Pre'))
  const pre = (await h.layers()).find((layer) => layer.name === 'Pre').id
  await pick(pre)
  const remap = kit.getByTestId('motion-remap-toggle')
  await remap.scrollIntoViewIfNeeded()
  await remap.click()
  await kit.getByTestId('motion-remap-time').waitFor({ timeout: 5000 }).catch(() => null)
  step('time remapping shows its source time as a timecode', /\d+:\d\d:\d\d:\d\d/.test((await kit.getByTestId('motion-remap-time').textContent().catch(() => '')) ?? ''))
  const freeze = kit.getByTestId('motion-freeze-frame')
  const box = await freeze.boundingBox()
  const panel = await kit.getByTestId('motion-properties-body').boundingBox()
  step('the Time buttons sit inside the panel', box.x >= panel.x + 4 && box.x + box.width <= panel.x + panel.width, `${box.x}+${box.width} in ${panel.x}+${panel.width}`)
  await freeze.click()
  await until(async () => (await toasts()).includes('已冻结'))
  step('Freeze Frame says which frame it froze', (await toasts()).includes('已冻结'), await toasts())

  // 9. The names split keeps the names readable however far it is dragged.
  const split = kit.getByTestId('motion-names-split')
  const grip = await split.boundingBox()
  await h.drag({ x: grip.x + grip.width / 2, y: grip.y + grip.height / 2 }, { x: grip.x - 600, y: grip.y + grip.height / 2 })
  await page.waitForTimeout(300)
  const name = await kit.getByTestId(`motion-layer-${pre}`).locator('.motion-row-name').boundingBox()
  step('dragged far to the left, the names keep at least 100 px', name.width >= 100, String(name.width))
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
