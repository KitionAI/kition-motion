// Footage and compositions by hand, in a real browser through the development host: node tests/footage.mjs [chromium|webkit] [shots]
// Imports a picture through the host's picker (File ▸ Import, Cmd+I, the Project panel's button),
// places it from the Project panel by double-click and by dragging, moves it on the canvas, opens
// and closes the panel, switches compositions from it, renames and deletes items, pre-composes and
// goes into the precomp and back, exports the work area through the Export dialog, and adds,
// moves, renames and deletes a marker, undoing each. Every step uses the pointer and the keyboard;
// the results are read from the engine (read-only ai.run queries), the host's files, and the
// canvas's pixels. With `shots`, it also writes the help screenshots. Prints PASS or FAIL per step.
import { deflateSync } from 'node:zlib'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'
import { delayEngineReplies } from './slowEngine.mjs'

const which = process.argv[2] ?? 'chromium'
const shots = process.argv[3] === 'shots' ? new URL('../../../kition-plan/kit-docs/motion/images/', import.meta.url).pathname : null
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1', watch: null }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
await delayEngineReplies(context)
const page = await context.newPage()
const errors = []
let failures = 0
page.on('pageerror', (error) => errors.push(error.message))
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` - ${detail}` : ''}`)
}
const until = async (check, tries = 80) => {
  for (let i = 0; i < tries; i += 1) {
    if (await check().catch(() => false)) return true
    await page.waitForTimeout(100)
  }
  return false
}
const near = (a, b, eps) => Math.abs(a - b) <= eps
/** A read-only engine query (the agent's own context: never the person's selection). */
const query = async (command, params) => (await page.evaluate(async (steps) => window.devHost.connection().ai.run(steps, false), [{ command, params }])).completed[0].result
const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
const layers = async () => (await details()).info.layers
const activeComp = async () => (await details()).comp.id
const items = async () => (await query('project.summary', {})).items
const position = async (layer) => (await query('prop.get', { layer, path: 'transform/position' })).value
const compMarkers = async () => await query('markers.list', {})
const kit = page.frameLocator('#kit')
let primary = 'Meta'

/** A solid-colour PNG, `width` × `height`, made here (no file from anywhere else). */
function png(width, height, [r, g, b]) {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    return c >>> 0
  })
  const crc = (bytes) => {
    let c = 0xffffffff
    for (const byte of bytes) c = crcTable[(c ^ byte) & 255] ^ (c >>> 8)
    return (c ^ 0xffffffff) >>> 0
  }
  const chunk = (type, data) => {
    const out = Buffer.alloc(12 + data.length)
    out.writeUInt32BE(data.length, 0)
    out.write(type, 4, 'ascii')
    data.copy(out, 8)
    out.writeUInt32BE(crc(out.subarray(4, 8 + data.length)), 8 + data.length)
    return out
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(width, 0)
  header.writeUInt32BE(height, 4)
  header[8] = 8
  header[9] = 2
  const rows = Buffer.alloc((width * 3 + 1) * height)
  for (let y = 0; y < height; y += 1) for (let x = 0; x < width; x += 1) rows.set([r, g, b], y * (width * 3 + 1) + 1 + x * 3)
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', header), chunk('IDAT', deflateSync(rows)), chunk('IEND', Buffer.alloc(0))])
}

/** A tiny WebM with one VP8 video track (a codec the build names but cannot decode). */
function vp8Webm(width = 64, height = 48) {
  const size = (n) => { // EBML variable-size integer, 8 bytes
    const out = [0x01]; for (let i = 6; i >= 0; i -= 1) out.push(Math.floor(n / 2 ** (8 * i)) & 0xff); return out }
  const uint = (n, bytes = 4) => Array.from({ length: bytes }, (_, i) => Math.floor(n / 2 ** (8 * (bytes - 1 - i))) & 0xff)
  const el = (id, body) => [...id, ...size(body.length), ...body]
  const str = (s) => [...Buffer.from(s, 'ascii')]
  const frame = (key) => {
    // VP8 key frame: a 3-byte frame tag, the start code, then 14-bit width and height.
    const tag = [0x10, 0x02, 0x00]
    return [...tag, 0x9d, 0x01, 0x2a, width & 0xff, width >> 8, height & 0xff, height >> 8, ...new Array(20).fill(0)]
  }
  const block = (time) => el([0xa3], [0x81, ...uint(time, 2), 0x80, ...frame(true)])
  const header = el([0x1a, 0x45, 0xdf, 0xa3], [...el([0x42, 0x86], [1]), ...el([0x42, 0xf7], [1]), ...el([0x42, 0xf2], [4]), ...el([0x42, 0xf3], [8]), ...el([0x42, 0x82], str('webm')), ...el([0x42, 0x87], [2]), ...el([0x42, 0x85], [2])])
  const info = el([0x15, 0x49, 0xa9, 0x66], [...el([0x2a, 0xd7, 0xb1], uint(1000000)), ...el([0x44, 0x89], [0x40, 0x8f, 0x40, 0, 0, 0, 0, 0])])
  const video = el([0xe0], [...el([0xb0], uint(width, 2)), ...el([0xba], uint(height, 2))])
  const track = el([0xae], [...el([0xd7], [1]), ...el([0x73, 0xc5], [1]), ...el([0x83], [1]), ...el([0x86], str('V_VP8')), ...el([0x23, 0xe3, 0x83], uint(100000000)), ...video])
  const tracks = el([0x16, 0x54, 0xae, 0x6b], track)
  const cluster = el([0x1f, 0x43, 0xb6, 0x75], [...el([0xe7], [0]), ...block(0), ...block(100), ...block(200), ...block(300), ...block(400), ...block(500), ...block(600), ...block(700), ...block(800), ...block(900)])
  const segment = el([0x18, 0x53, 0x80, 0x67], [...info, ...tracks, ...cluster])
  return Buffer.from([...header, ...segment])
}

/** A mono 16-bit WAV of a 440 Hz tone, `seconds` long. */
function toneWav(seconds = 1, rate = 48000) {
  const frames = Math.round(seconds * rate)
  const out = Buffer.alloc(44 + frames * 2)
  out.write('RIFF', 0, 'ascii')
  out.writeUInt32LE(36 + frames * 2, 4)
  out.write('WAVEfmt ', 8, 'ascii')
  out.writeUInt32LE(16, 16)
  out.writeUInt16LE(1, 20)
  out.writeUInt16LE(1, 22)
  out.writeUInt32LE(rate, 24)
  out.writeUInt32LE(rate * 2, 28)
  out.writeUInt16LE(2, 32)
  out.writeUInt16LE(16, 34)
  out.write('data', 36, 'ascii')
  out.writeUInt32LE(frames * 2, 40)
  for (let i = 0; i < frames; i += 1) out.writeInt16LE(Math.round(Math.sin((2 * Math.PI * 440 * i) / rate) * 12000), 44 + i * 2)
  return out
}

/** Puts a file in the host's workspace, where its picker finds it (the first picture there). */
const putFile = (name, bytes) => page.evaluate(([path, data]) => window.devHost.files.set(path, { bytes: new Uint8Array(data).buffer, version: 1 }), [name, [...bytes]])
const removeFile = (name) => page.evaluate((path) => window.devHost.files.delete(path), name)

/** The canvas view: zoom and the composition point at the surface's top left, and the surface's place on the page. */
async function view() {
  const [zoom, x, y] = (await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ').map(Number)
  const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
  return { zoom, x, y, left: surface.x, top: surface.y }
}
/** A composition point on the page. */
async function at(cx, cy) {
  const v = await view()
  return { x: v.left + (cx - v.x) * v.zoom, y: v.top + (cy - v.y) * v.zoom }
}
/** The colour the canvas shows at a composition point, [r, g, b]. */
async function rgb(cx, cy) {
  const p = await at(cx, cy)
  const shot = await page.screenshot({ clip: { x: Math.round(p.x), y: Math.round(p.y), width: 1, height: 1 } })
  return page.evaluate(async (data) => {
    const image = await createImageBitmap(await (await fetch(`data:image/png;base64,${data}`)).blob())
    const canvas = new OffscreenCanvas(1, 1)
    const context = canvas.getContext('2d')
    context.drawImage(image, 0, 0)
    return [...context.getImageData(0, 0, 1, 1).data.slice(0, 3)]
  }, shot.toString('base64'))
}
const isRed = ([r, g, b]) => r > 200 && g < 60 && b < 60
/** A press at page point `a`, moved in small steps to `b`, released; `held` keys are down for the move only. */
async function drag(a, b, held = []) {
  await page.mouse.move(a.x, a.y)
  await page.mouse.down()
  for (const key of held) await page.keyboard.down(key)
  for (let i = 1; i <= 10; i += 1) {
    await page.mouse.move(a.x + ((b.x - a.x) * i) / 10, a.y + ((b.y - a.y) * i) / 10)
    await page.waitForTimeout(16)
  }
  // A drag-and-drop needs a move over the target after it was entered: WebKit drops only where a
  // dragover has been taken, and the last step of the path may be the one that enters the target.
  await page.mouse.move(b.x + 1, b.y + 1)
  await page.waitForTimeout(16)
  await page.mouse.move(b.x, b.y)
  await page.waitForTimeout(100)
  await page.mouse.up()
  for (const key of held) await page.keyboard.up(key)
  await page.waitForTimeout(150)
}
const centre = (box) => ({ x: box.x + box.width / 2, y: box.y + box.height / 2 })
async function focusEditor() {
  await kit.locator('.kui-app').focus()
}
/** A click on the ruler at `x` pixels in, and the time once the engine has moved there (it seeks asynchronously). */
async function seekRuler(x) {
  const was = (await details()).time
  await kit.getByTestId('motion-ruler').click({ position: { x, y: 10 } })
  await until(async () => (await details()).time !== was)
  return (await details()).time
}
async function undo() {
  await focusEditor()
  await page.keyboard.press(`${primary}+KeyZ`)
}
const newest = async (before) => (await layers()).find((layer) => !before.some((old) => old.id === layer.id))

async function open() {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(800)
  primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
}

const RED = png(64, 48, [255, 0, 0])
const GREEN = png(40, 40, [0, 200, 0])

try {
  await open()
  const comp1 = await activeComp()

  // 1. Import through the host's picker: Cmd+I picks the first picture in the workspace.
  await putFile('red.png', RED)
  await focusEditor()
  await page.keyboard.press(`${primary}+KeyI`)
  await until(async () => (await items()).some((item) => item.name === 'red.png'))
  const red = (await items()).find((item) => item.name === 'red.png')
  step('Cmd+I imports the picture the host picks', red?.type === 'Image' && red.size?.[0] === 64 && red.size?.[1] === 48, JSON.stringify(red))
  step('and says so in Chinese', (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('已导入 red.png')))
  await undo()
  await until(async () => !(await items()).some((item) => item.name === 'red.png'))
  step('undo takes the import back', !(await items()).some((item) => item.name === 'red.png'))
  // File ▸ Import does the same.
  await kit.getByTestId('motion-menu-File').click()
  const importRow = kit.locator('[role=menu] [data-testid="motion-cmd-file.import"]')
  step('File ▸ Import reads 导入…', (await importRow.textContent()).includes('导入…'))
  await importRow.click()
  await until(async () => (await items()).some((item) => item.name === 'red.png'))
  const image = (await items()).find((item) => item.name === 'red.png')
  step('File ▸ Import imports it again', image?.type === 'Image')

  // 2. The Project panel: shown from the top bar, listing the project with sizes.
  step('the Project panel starts hidden', (await kit.getByTestId('motion-project-panel').count()) === 0)
  await kit.getByTestId('motion-project-toggle').click()
  await kit.getByTestId('motion-project-panel').waitFor()
  const row = kit.getByTestId(`motion-item-${image.id}`)
  step('the panel lists the picture with its size', (await row.textContent()).includes('64 × 48'), await row.textContent())
  step('and the composition with its size and length', (await kit.getByTestId(`motion-item-${comp1}`).textContent()).includes('1280 × 720'))

  // 3. Double-click places the picture at the current time; the canvas draws it.
  let before = await layers()
  await row.dblclick()
  await until(async () => (await layers()).length === before.length + 1)
  let placed = await newest(before)
  step('a double-click on the picture adds it as a layer', placed?.type === 'Footage' && placed.name === 'red.png', JSON.stringify(placed))
  // Inside the picture (608–672 × 336–384), clear of the selection's anchor mark at its centre.
  await until(async () => isRed(await rgb(652, 374)))
  step('the canvas draws it at the composition centre', isRed(await rgb(652, 374)) && isRed(await rgb(615, 342)) && !isRed(await rgb(600, 330)), JSON.stringify(await rgb(652, 374)))
  if (shots) await page.screenshot({ path: `${shots}phase4-project-panel.png` })

  // 4. Moving it on the canvas with the Selection tool: one undo step.
  const from = await at(640, 360)
  const to = await at(740, 410)
  // Cmd or Ctrl held: no snapping to the composition's centre lines on the way.
  await drag(from, to, [primary])
  await until(async () => (await position(placed.id))[0] > 700)
  const moved = await position(placed.id)
  step('the footage layer drags on the canvas like any layer', near(moved[0], 740, 2) && near(moved[1], 410, 2), JSON.stringify(moved))
  await undo()
  await until(async () => near((await position(placed.id))[0], 640, 0.5))
  step('undo puts it back', near((await position(placed.id))[0], 640, 0.5))
  await undo()
  await until(async () => (await layers()).length === before.length)
  step('undo removes the placed layer', (await layers()).length === before.length)

  // 5. Dragging the picture from the panel onto the composition: a layer where it was dropped.
  before = await layers()
  await drag(centre(await row.boundingBox()), await at(300, 200))
  await until(async () => (await layers()).length === before.length + 1)
  placed = await newest(before)
  const dropped = placed ? await position(placed.id) : null
  step('dragging it onto the composition places it where it was dropped', placed?.type === 'Footage' && dropped && near(dropped[0], 300, 3) && near(dropped[1], 200, 3), JSON.stringify(dropped))
  await undo()
  await until(async () => (await layers()).length === before.length)
  step('undo removes it', (await layers()).length === before.length)
  // And onto the timeline: a layer starting at the time under the pointer where it is let go.
  // The first press after the drag-and-drop above goes straight to the ruler (WebKit gives it no
  // pointerdown; the editor makes one from its mousedown, pointerFallback.ts).
  const now = await seekRuler(200)
  step('the first press after a drag-and-drop moves the current time', now > 0, String(now))
  const target = centre(await kit.getByTestId('motion-timeline-rows').boundingBox())
  const rulerBox = await kit.getByTestId('motion-ruler').boundingBox()
  const viewStart = Number(await kit.getByTestId('motion-ruler').getAttribute('data-view-start'))
  const viewSpan = Number(await kit.getByTestId('motion-ruler').getAttribute('data-view-span'))
  const fps = (await details()).comp.frameRate ?? 30
  const under = Math.round((viewStart + ((target.x - rulerBox.x) / rulerBox.width) * viewSpan) * fps) / fps
  await drag(centre(await row.boundingBox()), target)
  await until(async () => (await layers()).length === before.length + 1)
  placed = await newest(before)
  step('dragging it onto the timeline starts it at the time under the pointer', placed && Math.abs(under - now) > 0.2 && near(placed.in, under, 1.01 / fps), JSON.stringify({ in: placed?.in, under, now }))
  await undo()
  await until(async () => (await layers()).length === before.length)

  // 6. The panel's Import button, with a second picture.
  await removeFile('red.png')
  await putFile('green.png', GREEN)
  await kit.getByTestId('motion-project-import').click()
  await until(async () => (await items()).some((item) => item.name === 'green.png'))
  const green = (await items()).find((item) => item.name === 'green.png')
  step("the panel's Import button imports through the host", green?.type === 'Image')

  // 7. Rename with F2 and Enter; undo.
  await kit.getByTestId(`motion-item-${green.id}`).click()
  await page.keyboard.press('F2')
  const field = kit.getByTestId(`motion-item-rename-${green.id}`)
  await field.waitFor()
  await field.fill('草地')
  await page.keyboard.press('Enter')
  await until(async () => (await items()).find((item) => item.id === green.id)?.name === '草地')
  step('F2 renames the selected item', (await items()).find((item) => item.id === green.id)?.name === '草地')
  await undo()
  await until(async () => (await items()).find((item) => item.id === green.id)?.name === 'green.png')
  step('undo restores its name', (await items()).find((item) => item.id === green.id)?.name === 'green.png')

  // 8. Delete: at once when nothing uses it, after asking when a layer does; undo.
  await kit.getByTestId(`motion-item-${green.id}`).click()
  await page.keyboard.press('Delete')
  await until(async () => !(await items()).some((item) => item.id === green.id))
  step('Delete removes an unused item without asking', !(await items()).some((item) => item.id === green.id) && (await kit.getByTestId('motion-delete-dialog').count()) === 0)
  await undo()
  await until(async () => (await items()).some((item) => item.id === green.id))
  step('undo brings it back', (await items()).some((item) => item.id === green.id))
  before = await layers()
  await kit.getByTestId(`motion-item-${green.id}`).dblclick()
  await until(async () => (await layers()).length === before.length + 1)
  await kit.getByTestId(`motion-item-${green.id}`).click()
  await page.keyboard.press('Delete')
  await kit.getByTestId('motion-delete-dialog').waitFor()
  step('deleting an item a layer uses asks first, in Chinese', (await kit.getByTestId('motion-delete-dialog').textContent()).includes('1 个图层'), await kit.getByTestId('motion-delete-dialog').textContent())
  await kit.getByTestId('motion-delete-dialog-ok').click()
  await until(async () => !(await items()).some((item) => item.id === green.id))
  step('confirmed, the item and its layer go', !(await items()).some((item) => item.id === green.id) && (await layers()).length === before.length)
  await undo()
  await until(async () => (await layers()).length === before.length + 1)
  step('undo brings both back', (await items()).some((item) => item.id === green.id) && (await layers()).length === before.length + 1)

  // 9. A second composition from the panel, and back by double-click.
  await kit.getByTestId('motion-project-new-comp').click()
  await kit.getByTestId('motion-comp-dialog').waitFor()
  await page.keyboard.press('Enter')
  await until(async () => (await activeComp()) !== comp1)
  const comp2 = await activeComp()
  step("the panel's New Composition makes and opens one", comp2 !== comp1 && (await items()).some((item) => item.id === comp2))
  await kit.getByTestId(`motion-item-${comp1}`).dblclick()
  await until(async () => (await activeComp()) === comp1)
  step('a double-click on a composition opens it', (await activeComp()) === comp1)
  await kit.getByTestId(`motion-item-${comp2}`).dblclick()
  await until(async () => (await activeComp()) === comp2)
  await kit.getByTestId(`motion-item-${comp1}`).dblclick()
  await until(async () => (await activeComp()) === comp1)
  step('and back again', (await activeComp()) === comp1)

  // 10. The panel closes and opens again; its state lasts the session.
  await kit.getByTestId('motion-project-toggle').click()
  await until(async () => (await kit.getByTestId('motion-project-panel').count()) === 0)
  step('the top-bar button hides the panel', (await kit.getByTestId('motion-project-panel').count()) === 0)
  await kit.getByTestId('motion-project-toggle').click()
  await kit.getByTestId('motion-project-panel').waitFor()
  const edge = await kit.getByTestId('motion-project-edge').boundingBox()
  const width = (await kit.getByTestId('motion-project-panel').boundingBox()).width
  await drag(centre(edge), { x: centre(edge).x + 60, y: centre(edge).y })
  const wider = (await kit.getByTestId('motion-project-panel').boundingBox()).width
  step('its right edge drags it wider', near(wider, width + 60, 3), `${width} → ${wider}`)

  // 11. Pre-compose from the layer menu, go in by double-clicking its bar, and back by the breadcrumb.
  const shape = (await layers()).find((layer) => layer.type === 'Shape')
  await kit.getByTestId(`motion-layer-${shape.id}`).click({ button: 'right' })
  await kit.getByTestId('motion-layer-menu-precompose').click()
  await until(async () => (await layers()).some((layer) => layer.type === 'Composition'))
  const precomp = (await layers()).find((layer) => layer.type === 'Composition')
  step('Pre-compose replaces the layer with a precomp layer', precomp && !(await layers()).some((layer) => layer.id === shape.id))
  step('the new composition is named in Chinese (预合成 1)', (await items()).some((item) => item.name === '预合成 1') && precomp?.name === '预合成 1', JSON.stringify({ layer: precomp?.name, items: (await items()).map((item) => item.name) }))
  step('and says how to go into it', (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('双击')))
  await kit.getByTestId(`motion-bar-${precomp.id}`).dblclick()
  await until(async () => (await activeComp()) !== comp1)
  const nested = await activeComp()
  step('a double-click on its bar opens its composition', nested !== comp1 && (await layers()).some((layer) => layer.type === 'Shape'))
  await kit.getByTestId('motion-comp-path').waitFor()
  step('the breadcrumb names the composition to go back to', (await kit.getByTestId(`motion-comp-crumb-${comp1}`).textContent()).includes('Comp 1'))
  if (shots) await page.screenshot({ path: `${shots}phase4-precomp-breadcrumb.png` })
  await kit.getByTestId('motion-comp-back').click()
  await until(async () => (await activeComp()) === comp1)
  step('Back returns to the outer composition', (await activeComp()) === comp1 && (await kit.getByTestId('motion-comp-path').count()) === 0)
  // On the canvas: a double-click on the precomp layer (on its blue square, clear of the green picture) goes in too.
  await page.mouse.dblclick((await at(150, 300)).x, (await at(150, 300)).y)
  await until(async () => (await activeComp()) === nested)
  step('a double-click on the precomp layer on the canvas opens it', (await activeComp()) === nested && (await kit.getByTestId('motion-comp-path').count()) === 1)
  await kit.getByTestId(`motion-comp-crumb-${comp1}`).click()
  await until(async () => (await activeComp()) === comp1)
  step('the breadcrumb goes back', (await activeComp()) === comp1)
  await undo()
  await until(async () => (await layers()).some((layer) => layer.id === shape.id))
  step('undo takes the pre-compose back', (await layers()).some((layer) => layer.id === shape.id) && !(await layers()).some((layer) => layer.type === 'Composition'))

  // 12. Markers: * adds one at the current time; drag, double-click to name, Delete; undo each.
  const markAt = await seekRuler(150)
  // A click on the canvas outside the composition deselects every layer: the marker is the composition's.
  await page.mouse.click((await at(-30, -30)).x, (await at(-30, -30)).y)
  await until(async () => !(await layers()).some((layer) => layer.selected))
  await focusEditor()
  await page.keyboard.press('Shift+Digit8')
  await until(async () => (await compMarkers()).length === 1)
  step('* adds a composition marker at the current time', near((await compMarkers())[0]?.time ?? -1, markAt, 1e-3), JSON.stringify(await compMarkers()))
  const flag = kit.getByTestId('motion-marker-comp-0')
  await flag.waitFor()
  const ruler = await kit.getByTestId('motion-ruler').boundingBox()
  await drag(centre(await flag.boundingBox()), { x: centre(await flag.boundingBox()).x + ruler.width * 0.2, y: centre(await flag.boundingBox()).y })
  await until(async () => (await compMarkers())[0]?.time > markAt + 0.3)
  const movedTo = (await compMarkers())[0]?.time
  step('dragging the marker moves it in time', movedTo > markAt + 0.3, `${markAt} → ${movedTo}`)
  await undo()
  await until(async () => near((await compMarkers())[0]?.time ?? -1, markAt, 1e-3))
  step('undo moves it back', near((await compMarkers())[0]?.time ?? -1, markAt, 1e-3))
  await flag.dblclick()
  await kit.getByTestId('motion-marker-dialog').waitFor()
  await kit.getByTestId('motion-marker-name').fill('开场')
  await page.keyboard.press('Enter')
  await until(async () => (await compMarkers())[0]?.comment === '开场')
  step('a double-click names the marker', (await compMarkers())[0]?.comment === '开场' && (await flag.textContent()) === '开场')
  if (shots) await page.screenshot({ path: `${shots}phase4-markers.png` })
  await undo()
  await until(async () => (await compMarkers())[0]?.comment === '')
  step('undo takes the name back', (await compMarkers())[0]?.comment === '')
  await flag.click()
  await page.keyboard.press('Delete')
  await until(async () => (await compMarkers()).length === 0)
  step('Delete removes the clicked marker, not the layers', (await compMarkers()).length === 0 && (await layers()).length === before.length + 1)
  await undo()
  await until(async () => (await compMarkers()).length === 1)
  step('undo brings it back', (await compMarkers()).length === 1)
  // A layer marker: * with a layer selected.
  const text = (await layers()).find((layer) => layer.type === 'Text')
  await kit.getByTestId(`motion-layer-${text.id}`).click()
  // * goes to the layers the timeline shows selected: press it once the row shows the click's
  // selection, as a person would (the engine answers the click asynchronously).
  await until(async () => (await kit.getByTestId(`motion-layer-${text.id}`).getAttribute('data-selected')) === 'true')
  await focusEditor()
  await page.keyboard.press('Shift+Digit8')
  await until(async () => (await query('markers.list', { layer: text.id })).length === 1)
  await kit.getByTestId(`motion-marker-${text.id}-0`).waitFor({ timeout: 5000 }).catch(() => undefined)
  step('with a layer selected, * marks the layer', (await query('markers.list', { layer: text.id })).length === 1 && (await kit.getByTestId(`motion-marker-${text.id}-0`).count()) === 1, JSON.stringify({ markers: await query('markers.list', { layer: text.id }), comp: await compMarkers(), flags: await kit.getByTestId(`motion-marker-${text.id}-0`).count() }))
  await undo()

  // 13. Export: the dialog renders the work area at half size to an MP4 beside the project.
  await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'comp.workArea', params: { start: 1, end: 2 } }], false))
  await kit.getByTestId('motion-render').click()
  await kit.getByTestId('motion-export-dialog').waitFor()
  step('the top-bar button opens the Export dialog', (await kit.getByTestId('motion-export-dialog').textContent()).includes('工作区域'))
  // The whole composition at full size: the bar counts frames up, the engine still answers
  // other questions between its steps, and Cancel stops the render and writes no file.
  const exportProgress = async () => {
    const bar = kit.getByTestId('motion-export-progress')
    return (await bar.count()) ? { done: Number(await bar.getAttribute('data-done')), total: Number(await bar.getAttribute('data-total')) } : null
  }
  await kit.getByTestId('motion-export-dialog-ok').click()
  await until(async () => (await exportProgress())?.done > 0, 150)
  const first = await exportProgress()
  const asked = Date.now()
  const answered = await query('project.summary', {}).then(() => Date.now() - asked).catch(() => -1)
  await until(async () => (await exportProgress())?.done > first.done, 150)
  const later = await exportProgress()
  if (shots) await page.screenshot({ path: `${shots}phase6-export-progress.png` })
  step('the export shows the frames it has rendered, counting up', first?.total > 0 && later?.done > first.done && later.done < later.total, JSON.stringify({ first, later }))
  step('the engine answers while it exports', answered >= 0 && answered < 3000, `${answered} ms`)
  const stopLabel = (await kit.getByTestId('motion-export-dialog-cancel').textContent()) ?? ''
  await kit.getByTestId('motion-export-dialog-cancel').click()
  await until(async () => (await kit.getByTestId('motion-export-busy').count()) === 0, 100)
  await until(async () => (await kit.getByTestId('motion-export-stopped').count()) === 1, 50)
  step('while it renders the button reads 停止导出, and once stopped the dialog says so above its settings', stopLabel.includes('停止导出') && (await kit.getByTestId('motion-export-stopped').count()) === 1 && (await kit.getByTestId('motion-export-dialog-cancel').textContent()) === '取消', stopLabel)
  const stopped = await page.evaluate(() => [...window.devHost.files.keys()].filter((name) => /\.(mp4|webm|gif)$/.test(name)))
  step('Cancel stops the export, writes no file and keeps the dialog', stopped.length === 0 && (await kit.getByTestId('motion-export-dialog').count()) === 1 && (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('已取消导出')), JSON.stringify(stopped))
  await kit.getByTestId('motion-export-range').selectOption('workArea')
  await kit.getByTestId('motion-export-size').selectOption('half')
  if (shots) await page.screenshot({ path: `${shots}phase4-export-dialog.png` })
  await kit.getByTestId('motion-export-dialog-ok').click()
  await until(async () => (await page.evaluate(() => [...window.devHost.files.keys()])).some((name) => name.endsWith('.mp4')), 300)
  const movie = await page.evaluate(() => {
    const name = [...window.devHost.files.keys()].find((key) => key.endsWith('.mp4'))
    return name ? { name, size: window.devHost.files.get(name).bytes.byteLength } : null
  })
  step('Render writes an MP4 beside the project', movie?.name === 'Comp 1.mp4' && movie.size > 1000, JSON.stringify(movie))
  step('says which file it wrote, and closes', (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('Comp 1.mp4')) && (await kit.getByTestId('motion-export-dialog').count()) === 0)

  // 14. A file that is not a picture, movie or sound, dropped from the computer: a message, nothing imported.
  const count = (await items()).length
  await kit.getByTestId('motion-workspace').evaluate((target) => {
    const data = new DataTransfer()
    data.items.add(new File(['hello'], 'notes.txt', { type: 'text/plain' }))
    const canvas = target.querySelector('[data-testid="kit-canvas-surface"]')
    canvas.dispatchEvent(new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: data }))
    canvas.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: data }))
  })
  await until(async () => (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('notes.txt')))
  step('a dropped file of another type says it cannot be imported', (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('无法导入 notes.txt')) && (await items()).length === count)
  // A VP8 movie: refused with a message in Chinese, not imported to draw black.
  await kit.getByTestId('motion-workspace').evaluate((target, bytes) => {
    const data = new DataTransfer()
    data.items.add(new File([new Uint8Array(bytes)], 'clip.webm', { type: 'video/webm' }))
    const canvas = target.querySelector('[data-testid="kit-canvas-surface"]')
    canvas.dispatchEvent(new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: data }))
    canvas.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: data }))
  }, [...vp8Webm()])
  await until(async () => (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('clip.webm')))
  step('a VP8 movie is refused, saying so in Chinese', (await kit.locator('.kui-toast').allTextContents()).some((text) => text.includes('未导入 clip.webm') && text.includes('VP8')) && (await items()).length === count, (await kit.locator('.kui-toast').allTextContents()).join(' | '))
  // A sound placed at the start plays with the preview: the engine mixes it and the page schedules it.
  await putFile('tone.wav', toneWav(2))
  const tone = (await query('file.import', { paths: ['tone.wav'] })).items?.[0]
  await page.evaluate(async (item) => window.devHost.connection().ai.run([{ command: 'layer.addItem', params: { item, time: 0 } }], false), tone)
  await kit.getByTestId('motion-workspace').evaluate((element) => element.removeAttribute('data-audio'))
  await kit.getByTestId('motion-play').click()
  await until(async () => Number(await kit.getByTestId('motion-workspace').getAttribute('data-audio')) >= 1, 60)
  await kit.getByTestId('motion-play').click()
  step('a placed sound plays during preview', Number(await kit.getByTestId('motion-workspace').getAttribute('data-audio')) >= 1, String(await kit.getByTestId('motion-workspace').getAttribute('data-audio')))
  // A picture dropped from the computer: written beside the project through the host, imported and placed.
  before = await layers()
  await kit.getByTestId('motion-workspace').evaluate((target, bytes) => {
    const data = new DataTransfer()
    data.items.add(new File([new Uint8Array(bytes)], 'dropped.png', { type: 'image/png' }))
    const canvas = target.querySelector('[data-testid="kit-canvas-surface"]')
    canvas.dispatchEvent(new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: data }))
    canvas.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: data }))
  }, [...RED])
  await until(async () => (await layers()).length === before.length + 1)
  step('a picture dropped from the computer is written beside the project, imported and placed', (await page.evaluate(() => window.devHost.files.has('dropped.png'))) && (await items()).some((item) => item.name === 'dropped.png') && (await layers()).length === before.length + 1)

  // 15. The panel stays open after a reload (the session remembers it).
  await open()
  step('the panel is still open after a reload', (await kit.getByTestId('motion-project-panel').count()) === 1)
  step('no page errors', errors.length === 0, errors.join('; '))
} catch (error) {
  step('ran to the end', false, String(error?.stack ?? error).split('\n').slice(0, 3).join(' '))
} finally {
  await browser.close()
  await server.close()
}
console.log(failures ? `${failures} failed (${which})` : `all passed (${which})`)
process.exit(failures ? 1 : 0)
