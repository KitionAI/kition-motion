// Shape layers by hand, in a real browser through the development host:
// node tests/shapes.mjs [chromium|webkit]
// The Add menu (in the Properties panel and on the timeline's Contents row) adds shapes, paints
// and path operators; a shape item's buttons move it up and down, turn it off and delete it; a
// gradient's stops are added, dragged and removed in the Properties panel and its start point is
// dragged on the canvas; a shape path's vertices are selected (click, Shift-click, box), moved,
// added on a segment with the Pen, converted with Alt-click and deleted with Delete. Each change
// is one undo step. Results are read from the engine (read-only queries). Prints PASS or FAIL.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const SHAPE = 27
const near = (a, b, eps = 1.5) => typeof a === 'number' && Math.abs(a - b) <= eps

/** A composition point on the page (as tests/canvas.mjs). */
async function at(cx, cy) {
  const [zoom, x, y] = (await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ').map(Number)
  const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
  return { x: surface.x + (cx - x) * zoom, y: surface.y + (cy - y) * zoom }
}
async function click(cx, cy, modifiers = []) {
  const p = await at(cx, cy)
  for (const key of modifiers) await page.keyboard.down(key)
  await page.mouse.click(p.x, p.y)
  for (const key of modifiers) await page.keyboard.up(key)
  await page.waitForTimeout(120)
}
async function dragComp(from, to) {
  await h.drag(await at(...from), await at(...to))
  await page.waitForTimeout(150)
}
/** The node at `path` of a layer's tree, and every item group (in order) under a contents path. */
function find(node, path) {
  if (node?.path === path) return node
  for (const child of node?.children ?? []) {
    const found = find(child, path)
    if (found) return found
  }
  return null
}
const tree = async (layer) => (await h.query('layer.tree', { layer })).properties
const items = async (layer, path) => (find(await tree(layer), path)?.children ?? []).filter((child) => child.children).map((child) => child.match)
/** The first shape Path item's vertices (its own space). */
async function shapePath(layer) {
  const walk = (node) => (node?.match === 'path' && node.children ? node : (node?.children ?? []).map(walk).find(Boolean))
  const item = walk(await tree(layer))
  return item?.children?.find((child) => child.match === 'path')?.value?.v ?? null
}

try {
  await h.open()
  const inner = 'contents/group/contents'

  // 1. The Properties panel's Add menu puts an operator into the shape's group.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await until(async () => (await h.layer(SHAPE))?.selected)
  await kit.getByTestId('motion-shape-add').click()
  await kit.getByTestId('motion-shape-add-trim').click()
  await until(async () => (await items(SHAPE, inner)).includes('trim'))
  step('Add ▸ Trim Paths in the Properties panel adds it to the shape’s group', (await items(SHAPE, inner)).includes('trim'), JSON.stringify(await items(SHAPE, inner)))
  await h.undo()
  await until(async () => !(await items(SHAPE, inner)).includes('trim'))
  step('one undo takes it away', !(await items(SHAPE, inner)).includes('trim'))

  // 2. The timeline's Contents row has an Add menu too.
  await kit.getByTestId(`motion-twirl-layer-${SHAPE}`).click()
  await kit.getByTestId(`motion-row-shape-add-${SHAPE}-contents`).waitFor({ timeout: 5000 })
  await kit.getByTestId(`motion-row-shape-add-${SHAPE}-contents`).click()
  await kit.getByTestId(`motion-row-shape-add-${SHAPE}-contents-repeater`).click()
  await until(async () => (await items(SHAPE, 'contents')).includes('repeater'))
  step('the timeline’s Contents row ▸ Repeater adds one to the layer’s contents', (await items(SHAPE, 'contents')).includes('repeater'), JSON.stringify(await items(SHAPE, 'contents')))

  // 3. Item buttons in the Properties panel: up, off, delete.
  const order = await items(SHAPE, 'contents')
  const repeaterId = 'contents.repeater'
  await kit.getByTestId(`motion-group-${repeaterId}`).waitFor({ timeout: 5000 })
  await kit.getByTestId(`motion-item-${repeaterId}-up`).click()
  await until(async () => (await items(SHAPE, 'contents'))[0] === 'repeater')
  step('the item’s up button moves the repeater above the group', (await items(SHAPE, 'contents'))[0] === 'repeater', `${JSON.stringify(order)} → ${JSON.stringify(await items(SHAPE, 'contents'))}`)
  await kit.getByTestId(`motion-item-${repeaterId}-enabled`).click()
  await until(async () => find(await tree(SHAPE), 'contents/repeater')?.enabled === false)
  step('its eye turns it off', find(await tree(SHAPE), 'contents/repeater')?.enabled === false)
  await kit.getByTestId(`motion-item-${repeaterId}-remove`).click()
  await until(async () => !(await items(SHAPE, 'contents')).includes('repeater'))
  step('its bin deletes it', !(await items(SHAPE, 'contents')).includes('repeater'))
  await h.undo()
  await until(async () => (await items(SHAPE, 'contents')).includes('repeater'))
  step('one undo brings it back', (await items(SHAPE, 'contents')).includes('repeater'))
  await kit.getByTestId(`motion-item-${repeaterId}-remove`).click()
  await until(async () => !(await items(SHAPE, 'contents')).includes('repeater'))

  // 4. A gradient fill: stops added, dragged and removed in the panel.
  await kit.getByTestId('motion-shape-add').click()
  await kit.getByTestId('motion-shape-add-gfill').click()
  await until(async () => (await items(SHAPE, inner)).includes('gfill'))
  const colors = `${inner}/gfill/colors`
  const stops = async () => find(await tree(SHAPE), colors)?.value?.v?.colors ?? []
  for (const toggle of ['contents', 'contents.group', 'contents.group.contents', 'contents.group.contents.gfill']) {
    const button = kit.getByTestId(`motion-group-toggle-${toggle}`).first()
    if ((await button.count()) && (await button.getAttribute('aria-expanded')) === 'false') await button.click()
  }
  const editor = 'motion-value-contents.group.contents.gfill.colors'
  await kit.getByTestId(`${editor}-bar`).waitFor({ timeout: 5000 })
  // The editor's fields below the bar in view too (the panel scrolls), so the stops are on screen.
  await kit.getByTestId(`${editor}-remove`).scrollIntoViewIfNeeded()
  const bar = await kit.getByTestId(`${editor}-bar`).boundingBox()
  await page.mouse.click(bar.x + bar.width / 2, bar.y + bar.height / 2)
  await until(async () => (await stops()).length === 3)
  step('a click on the gradient bar adds a stop there', (await stops()).length === 3 && near((await stops())[1][0], 0.5, 0.02), JSON.stringify(await stops()))
  const marker = await h.centre(kit.getByTestId(`${editor}-stop-1`))
  await h.drag(marker, { x: bar.x + bar.width * 0.25, y: marker.y })
  await until(async () => near((await stops())[1]?.[0], 0.25, 0.03))
  step('a stop drags along the bar', near((await stops())[1]?.[0], 0.25, 0.03), JSON.stringify(await stops()))
  await kit.getByTestId(`${editor}-remove`).click()
  await until(async () => (await stops()).length === 2)
  step('the bin removes the picked stop', (await stops()).length === 2)
  await h.undo()
  await until(async () => (await stops()).length === 3)
  step('one undo brings it back', (await stops()).length === 3)

  // 5. A new rectangle's gradient start point drags on the canvas.
  await click(-80, 300)
  await h.focusEditor()
  await page.keyboard.press('q')
  const before = (await h.layers()).map((layer) => layer.id)
  await dragComp([700, 100], [1000, 300])
  await until(async () => (await h.layers()).length === before.length + 1)
  const rect = (await h.layers()).find((layer) => !before.includes(layer.id)).id
  await kit.getByTestId('motion-shape-add').click()
  await kit.getByTestId('motion-shape-add-gfill').click()
  await until(async () => (await items(rect, inner)).includes('gfill'))
  const startPath = `${inner}/gfill/start`
  const startAt = (await h.query('prop.get', { layer: rect, path: startPath })).value
  const position = (await h.query('prop.get', { layer: rect, path: 'transform/position' })).value
  // The group sits at the layer's origin, the layer's origin at its position: the point is position + start.
  const from = [position[0] + startAt[0], position[1] + startAt[1]]
  await page.waitForTimeout(300)
  await dragComp(from, [from[0] - 40, from[1] + 30])
  await until(async () => !near((await h.query('prop.get', { layer: rect, path: startPath })).value[0], startAt[0], 0.5))
  const moved = (await h.query('prop.get', { layer: rect, path: startPath })).value
  step('the gradient’s start point drags on the canvas', near(moved[0], startAt[0] - 40, 2) && near(moved[1], startAt[1] + 30, 2), `${JSON.stringify(startAt)} → ${JSON.stringify(moved)}`)
  await h.undo()
  await until(async () => near((await h.query('prop.get', { layer: rect, path: startPath })).value[0], startAt[0], 0.5))
  step('one undo puts it back', near((await h.query('prop.get', { layer: rect, path: startPath })).value[0], startAt[0], 0.5))

  // 6. A path drawn with the Pen and nothing selected: a new shape layer; then its vertices.
  await click(-80, 300)
  await until(async () => (await h.layers()).every((layer) => !layer.selected))
  await kit.getByTestId('motion-tool-pen').click()
  const hint = (await kit.getByTestId('motion-tool-status').textContent().catch(() => '')) ?? ''
  step('the Pen says in the status bar that its path will be a new shape layer', hint.includes('新的形状图层'), hint)
  const count = (await h.layers()).length
  for (const [x, y] of [[400, 450], [600, 450], [600, 650], [400, 650], [401, 451]]) await click(x, y)
  await until(async () => (await h.layers()).length === count + 1)
  const pathLayer = (await h.layers()).find((layer) => layer.selected)
  step('the Pen with nothing selected draws a new shape layer', (await h.layers()).length === count + 1 && pathLayer?.type === 'Shape', JSON.stringify(pathLayer?.type))
  const first = await shapePath(pathLayer.id)
  step('its path has the four points, closed', first?.vertices?.length === 4 && first.closed === true, JSON.stringify(first))
  await kit.getByTestId('motion-tool-selection').click()
  await click(400, 450)
  await click(600, 450, ['Shift'])
  await dragComp([600, 450], [620, 430])
  await until(async () => !near((await shapePath(pathLayer.id))?.vertices?.[1]?.[0], first.vertices[1][0], 0.5))
  let now = await shapePath(pathLayer.id)
  step('click, Shift-click and a drag move both top vertices', [0, 1].every((i) => near(now.vertices[i][0], first.vertices[i][0] + 20) && near(now.vertices[i][1], first.vertices[i][1] - 20)) && near(now.vertices[2][0], first.vertices[2][0], 0.5), JSON.stringify(now.vertices))
  await h.undo()
  await until(async () => near((await shapePath(pathLayer.id))?.vertices?.[1]?.[0], first.vertices[1][0], 0.5))
  step('one undo puts them back', near((await shapePath(pathLayer.id))?.vertices?.[0]?.[0], first.vertices[0][0], 0.5))
  await click(600, 650, ['Alt'])
  await until(async () => {
    const tangent = (await shapePath(pathLayer.id))?.in_tangents?.[2] ?? [0, 0]
    return Math.abs(tangent[0]) + Math.abs(tangent[1]) > 1
  })
  now = await shapePath(pathLayer.id)
  step('Alt-click turns a corner smooth', Math.abs(now.in_tangents[2][0]) + Math.abs(now.in_tangents[2][1]) > 1, JSON.stringify(now.in_tangents))
  await h.undo()
  await kit.getByTestId('motion-tool-pen').click()
  await click(600, 550)
  await until(async () => (await shapePath(pathLayer.id))?.vertices?.length === 5)
  step('the Pen on a segment adds a vertex there', (await shapePath(pathLayer.id))?.vertices?.length === 5)
  await kit.getByTestId('motion-tool-selection').click()
  await click(600, 550)
  await dragComp([350, 620], [650, 700])
  await page.waitForTimeout(200)
  await h.focusEditor()
  await page.keyboard.press('Delete')
  await until(async () => (await shapePath(pathLayer.id))?.vertices?.length === 3)
  step('a box over the bottom vertices and Delete removes them (the layer stays)', (await shapePath(pathLayer.id))?.vertices?.length === 3 && (await h.layer(pathLayer.id)) != null, JSON.stringify((await shapePath(pathLayer.id))?.vertices))
  await h.undo()
  await until(async () => (await shapePath(pathLayer.id))?.vertices?.length === 5)
  step('one undo brings them back', (await shapePath(pathLayer.id))?.vertices?.length === 5)
  const pre = await shapePath(pathLayer.id)
  await dragComp([500, 650], [500, 700])
  await until(async () => !near((await shapePath(pathLayer.id))?.vertices?.[0]?.[1], pre.vertices[0][1], 0.5))
  now = await shapePath(pathLayer.id)
  step('a drag on a segment moves the whole path', now.vertices.length === pre.vertices.length && now.vertices.every((v, i) => near(v[0], pre.vertices[i][0]) && near(v[1], pre.vertices[i][1] + 50, 2)), `${JSON.stringify(pre.vertices)} → ${JSON.stringify(now.vertices)}`)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
