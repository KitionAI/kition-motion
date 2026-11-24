// Layer styles and time remapping by hand, in a real browser through the development host:
// node tests/layerStyles.mjs [chromium|webkit]
// The Properties panel's Layer Styles section adds a style (and Layer ▸ Layer Styles does too),
// edits and keyframes its parameters, turns it off and removes it; the timeline shows its rows.
// The Time section of a composition layer enables time remapping, freezes a frame, reverses the
// layer and removes the remapping. Results are read from the engine (read-only queries). Prints
// PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const SHAPE = 27
const SOLID = 4

/** The node at `path` in a layer's property tree, or null. */
async function node(layer, path) {
  const tree = (await h.query('layer.tree', { layer, depth: 6 }))?.properties
  const find = (each) => (each?.path === path ? each : (each?.children ?? []).map(find).find(Boolean) ?? null)
  return find(tree)
}
const styles = async (layer) => ((await node(layer, 'layerStyles'))?.children ?? []).filter((child) => child.children).map((child) => child.match)
const value = async (layer, path) => (await h.query('prop.get', { layer, path }))?.value

try {
  await h.open()
  // 1. The Layer Styles section: Add style ▸ Drop Shadow.
  await kit.getByTestId(`motion-layer-${SHAPE}`).click()
  await kit.getByTestId('motion-section-layerStyles').waitFor()
  await kit.getByTestId('motion-style-add').click()
  await kit.getByTestId('motion-style-add-dropShadow').click()
  await until(async () => (await styles(SHAPE)).includes('dropShadow'))
  step('Add style ▸ 投影 gives the layer a drop shadow (with Blending Options)', JSON.stringify(await styles(SHAPE)) === JSON.stringify(['blendingOptions', 'dropShadow']), JSON.stringify(await styles(SHAPE)))
  await kit.getByTestId('motion-style-dropShadow').waitFor()
  step('the section shows it with its parameters', (await kit.getByTestId('motion-prop-layerStyles.dropShadow.distance').count()) === 1)

  // 2. Its Distance typed in, and its Opacity keyframed from its diamond.
  const distance = kit.getByTestId('motion-value-layerStyles.dropShadow.distance-0')
  await distance.click()
  await distance.fill('24')
  await distance.press('Enter')
  await until(async () => (await value(SHAPE, 'layerStyles/dropShadow/distance')) === 24)
  step('typing 24 in Distance sets it', (await value(SHAPE, 'layerStyles/dropShadow/distance')) === 24, JSON.stringify(await value(SHAPE, 'layerStyles/dropShadow/distance')))
  await kit.getByTestId('motion-add-key-layerStyles.dropShadow.opacity').click()
  await until(async () => (await h.layer(SHAPE)).animated.some((property) => property.path === 'layerStyles/dropShadow/opacity'))
  step('the Opacity diamond keyframes it', (await h.layer(SHAPE)).animated.some((property) => property.path === 'layerStyles/dropShadow/opacity'))

  // 3. Turned off and on by its eye.
  await kit.getByTestId('motion-style-enabled-dropShadow').click()
  await until(async () => (await node(SHAPE, 'layerStyles/dropShadow'))?.enabled === false)
  step('its eye turns the drop shadow off', (await node(SHAPE, 'layerStyles/dropShadow'))?.enabled === false)
  await kit.getByTestId('motion-style-enabled-dropShadow').click()
  await until(async () => (await node(SHAPE, 'layerStyles/dropShadow'))?.enabled !== false)
  step('and on again', (await node(SHAPE, 'layerStyles/dropShadow'))?.enabled !== false)

  // 4. Layer ▸ Layer Styles ▸ Stroke from the menu bar.
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[role=menu] [role=menuitem]', { hasText: '图层样式' }).first().hover()
  await kit.locator('[role=menu] [data-testid="motion-cmd-layer.style.stroke"]').first().click()
  await until(async () => (await styles(SHAPE)).includes('stroke'))
  step('Layer ▸ 图层样式 ▸ 描边 adds a stroke', (await styles(SHAPE)).includes('stroke'), JSON.stringify(await styles(SHAPE)))

  // 5. The timeline shows the styles under the layer.
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  await kit.getByTestId(`motion-group-${SHAPE}-layerStyles`).waitFor({ timeout: 3000 }).catch(() => null)
  step('the timeline lists Layer Styles under the layer', (await kit.getByTestId(`motion-group-${SHAPE}-layerStyles`).count()) === 1)
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()

  // 6. Removed by its trash button; one undo brings it back. (The layer selected again first: the
  // twirl's clicks may have left the selection to a slow engine.)
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await until(async () => (await h.layer(SHAPE))?.selected)
  await kit.getByTestId('motion-style-remove-dropShadow').waitFor({ timeout: 5000 }).catch(() => null)
  await kit.getByTestId('motion-style-remove-dropShadow').click()
  await until(async () => !(await styles(SHAPE)).includes('dropShadow'))
  step('the trash button removes the drop shadow', !(await styles(SHAPE)).includes('dropShadow') && (await styles(SHAPE)).includes('stroke'), JSON.stringify(await styles(SHAPE)))
  await h.undo()
  await until(async () => (await styles(SHAPE)).includes('dropShadow'))
  step('one undo brings it back with its distance', (await styles(SHAPE)).includes('dropShadow') && (await value(SHAPE, 'layerStyles/dropShadow/distance')) === 24)

  // 7. Remove all styles from the Add menu, by the keyboard.
  await kit.getByTestId('motion-style-add').focus()
  await page.keyboard.press('Enter')
  await kit.getByTestId('motion-style-remove-all').waitFor()
  await kit.getByTestId('motion-style-remove-all').click()
  await until(async () => (await styles(SHAPE)).filter((id) => id !== 'blendingOptions').length === 0)
  step('Remove all styles leaves none', (await styles(SHAPE)).filter((id) => id !== 'blendingOptions').length === 0, JSON.stringify(await styles(SHAPE)))

  // 8. Time remapping on a composition layer (the solid pre-composed).
  await h.setUp([{ command: 'layer.precompose', params: { layers: [SOLID], name: 'Remap me' } }])
  await until(async () => (await h.layers()).some((layer) => layer.type === 'Composition'))
  const comp = (await h.layers()).find((layer) => layer.type === 'Composition')
  await kit.getByTestId(`motion-layer-${comp.id}`).click()
  await kit.getByTestId('motion-section-time').waitFor()
  await kit.getByTestId('motion-remap-toggle').click()
  await until(async () => (await node(comp.id, 'timeRemap')) !== null)
  const remap = await node(comp.id, 'timeRemap')
  step('启用时间重映射 gives the layer Time Remap with a key at each end', (remap?.keys ?? []).length === 2, JSON.stringify(remap?.keys))
  await kit.getByTestId('motion-prop-timeRemap').waitFor({ timeout: 3000 }).catch(() => null)
  step('the Time section shows Time Remap with its diamond', (await kit.getByTestId('motion-prop-timeRemap').count()) === 1 && (await kit.getByTestId('motion-add-key-timeRemap').count()) === 1)
  const ruler = await kit.getByTestId('motion-ruler').boundingBox()
  await page.mouse.click(ruler.x + ruler.width * 0.3, ruler.y + ruler.height / 2)
  await until(async () => (await h.details()).time > 0.1)
  const now = (await h.details()).time
  await kit.getByTestId('motion-freeze-frame').click()
  await until(async () => ((await node(comp.id, 'timeRemap'))?.keys ?? []).length === 1)
  const frozen = await node(comp.id, 'timeRemap')
  step('冻结帧 holds the frame at the current time (one hold key)', (frozen?.keys ?? []).length === 1 && Math.abs((frozen?.value ?? -1) - now) < 0.1, JSON.stringify({ keys: frozen?.keys, now }))
  await h.undo()
  await until(async () => ((await node(comp.id, 'timeRemap'))?.keys ?? []).length === 2)
  step('one undo takes the freeze back', ((await node(comp.id, 'timeRemap'))?.keys ?? []).length === 2)
  const stretchOf = async () => (await h.query('comp.info', {})).layers.find((layer) => layer.id === comp.id)?.stretch
  await kit.getByTestId('motion-time-reverse').click()
  await until(async () => (await stretchOf()) < 0)
  step('时间反向图层 reverses the layer', (await stretchOf()) < 0, String(await stretchOf()))
  await h.undo()
  await until(async () => (await stretchOf()) > 0)
  await kit.getByTestId('motion-remap-toggle').click()
  await until(async () => (await node(comp.id, 'timeRemap')) === null)
  step('移除时间重映射 takes Time Remap away', (await node(comp.id, 'timeRemap')) === null)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
