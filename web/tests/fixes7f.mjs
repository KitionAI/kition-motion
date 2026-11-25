// The fixes after the round 7f hand test, by hand, in a real browser through the development host: node tests/fixes7f.mjs [chromium|webkit]
// Convert Expression to Keyframes on a property whose expression is switched off says to turn it
// on first; the status bar says what the 3D handle under the pointer does (the depth dot too); the
// Smoother on four keys along a line gives no more keys and a path that never runs back; opening the
// graph editor fits its time to the keys and closing it puts the layer bars' zoom back (适合全部
// too); the timeline starts at about 38 % of the window with a visible grip on its divider; Export…
// offers 三分之一 and 四分之一 as the Render Queue does; a Render Queue job shows the folder it
// writes to; the English strings spell "color"; and the default new composition (1920 × 1080, 10 s,
// 30 fps) caches from end to end in the background, cached frames look the same as rendered ones,
// and when a heavy picture fills the cache the button says so and one click on 把预览分辨率改为
// 二分之一 caches several times as many frames. Results are read from the engine (`prop.get`,
// `comp.info`, the frame cache) and the elements' own attributes. Prints PASS or FAIL per step, and
// MEASURE lines with the cache's numbers.
import { readFileSync } from 'node:fs'
import { inflateSync } from 'node:zlib'
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const near = (a, b, eps) => Math.abs(a - b) <= eps
const value = async (layer, path, time) => (await h.query('prop.get', time === undefined ? { layer, path } : { layer, path, time })).value
const keysOf = async (layer, path) => ((await h.query('prop.get', { layer, path })).keys ?? []).length
/** A menu bar menu, then its rows down `path` (submenus hovered), the last one clicked. */
async function menu(name, path, last) {
  await kit.getByTestId(`motion-menu-${name}`).click()
  let at = `menu:${name}`
  for (const sub of path) {
    at = `${at}/${sub}`
    await kit.locator(`[data-kit-ui-id="${at}"]`).hover()
  }
  await kit.locator(`[role=menu] [data-testid="${last}"]`).first().click()
}
/** A PNG's size and scanlines (its IDAT data inflated): two screenshots of the same pixels have the same. */
function scanlines(png) {
  const chunks = []
  let size = ''
  for (let at = 8; at < png.length; ) {
    const length = png.readUInt32BE(at)
    const type = png.toString('latin1', at + 4, at + 8)
    if (type === 'IHDR') size = `${png.readUInt32BE(at + 8)}x${png.readUInt32BE(at + 12)}`
    if (type === 'IDAT') chunks.push(png.subarray(at + 8, at + 8 + length))
    at += 12 + length
  }
  return { size, lines: inflateSync(Buffer.concat(chunks)) }
}
async function selectShape() {
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').first().click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
}
const button = kit.getByTestId('motion-cache-ahead')
const attr = async (locator, name) => Number(await locator.getAttribute(name))

try {
  // 0. The English strings and descriptions spell "color" (American), never "colour".
  const source = (file) => readFileSync(new URL(`../src/${file}`, import.meta.url), 'utf8')
  const values = (text) => [...text.matchAll(/^\s*"(?:[^"\\]|\\.)*":\s*"((?:[^"\\]|\\.)*)",?\s*$/gm)].map((match) => match[1])
  const strings = source('strings.ts')
  const english = values(strings.slice(0, strings.indexOf('const zhCN'))).concat(values(source('effectAbout.ts').split('export const EFFECT_ABOUT_ZH')[0]), values(strings.slice(strings.indexOf('const AMERICAN'), strings.indexOf('export const PHRASES'))))
  const british = english.filter((text) => /colour|favourite|centre/i.test(text))
  h.step('the English strings spell "color", "favorite" and "center"', english.length > 1000 && british.length === 0, `${english.length} strings; ${british.slice(0, 4).join(' | ')}`)

  await h.open()

  // 1. The timeline starts at about 38 % of the window's height; its divider has a visible grip.
  const timelineHeight = Number(await kit.getByTestId('motion-splitter').getAttribute('aria-valuenow'))
  const editorHeight = await kit.locator('body').evaluate(() => window.innerHeight)
  h.step('the timeline starts at about 38 % of the editor’s height', near(timelineHeight, editorHeight * 0.38, 2), `${timelineHeight} of ${editorHeight}`)
  const grip = await kit.getByTestId('motion-splitter').evaluate((element) => {
    const style = getComputedStyle(element, '::before')
    return { content: style.content, width: parseFloat(style.width), height: parseFloat(style.height), opacity: Number(style.opacity), color: style.backgroundColor }
  })
  h.step('the divider above the timeline shows a grip', grip.content !== 'none' && grip.width >= 30 && grip.height >= 2 && grip.opacity >= 0.5 && grip.color !== 'rgba(0, 0, 0, 0)', JSON.stringify(grip))
  const splitter = await h.centre(kit.getByTestId('motion-splitter'))
  await h.drag(splitter, { x: splitter.x + 200, y: splitter.y - 60 })
  await h.until(async () => Number(await kit.getByTestId('motion-splitter').getAttribute('aria-valuenow')) > timelineHeight + 40)
  h.step('dragging the grip up makes the timeline taller', Number(await kit.getByTestId('motion-splitter').getAttribute('aria-valuenow')) > timelineHeight + 40, await kit.getByTestId('motion-splitter').getAttribute('aria-valuenow'))

  // 2. Convert Expression to Keyframes on an expression switched off: turn it on first.
  await h.setUp([{ command: 'prop.setExpression', params: { layer: SHAPE, path: 'transform/rotation', expression: 'time * 90', enabled: false } }])
  await selectShape()
  await h.focusEditor()
  await page.keyboard.press('KeyR')
  const rotationName = kit.getByTestId(`motion-prop-row-${SHAPE}-transform.rotation`).locator('.motion-prop-label')
  await rotationName.waitFor()
  await rotationName.click()
  await h.until(async () => ((await h.layer(SHAPE))?.selectedProps ?? []).includes('transform/rotation'))
  await page.waitForTimeout(600)
  await kit.getByTestId('motion-menu-Animation').click()
  await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
  const convertRow = kit.locator('[role=menu] [data-testid="motion-control-convert-expression"]').first()
  await convertRow.waitFor()
  await h.until(async () => ((await convertRow.textContent()) ?? '').includes('先打开表达式'), 30)
  h.step('Convert says to turn the expression on first', ((await convertRow.textContent()) ?? '').includes('先打开表达式'), await convertRow.textContent())
  await convertRow.click()
  await h.until(async () => (await kit.locator('.kui-toast').count()) > 0, 20)
  h.step('choosing it says so too, and makes no keyframes', ((await kit.locator('.kui-toast').last().textContent()) ?? '').includes('先打开表达式') && (await keysOf(SHAPE, 'transform/rotation')) === 0)
  await h.setUp([{ command: 'prop.setExpression', params: { layer: SHAPE, path: 'transform/rotation', expression: '' } }])

  // 3. The 3D handles: the status bar says what the handle under the pointer does.
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await selectShape()
  const handles = async () => JSON.parse((await kit.getByTestId('motion-workspace').getAttribute('data-gizmo')) || '{}')
  await h.until(async () => (await handles()).z != null && (await handles()).x != null)
  const status = kit.getByTestId('motion-tool-status')
  const hover = async (part) => {
    const spot = (await handles())[part]
    const at = await h.at(spot[0], spot[1])
    await page.mouse.move(at.x + 20, at.y + 20)
    await page.mouse.move(at.x, at.y, { steps: 4 })
  }
  await hover('z')
  await h.until(async () => ((await status.textContent().catch(() => '')) ?? '').includes('深度'))
  h.step('over the depth dot the status bar says it moves forward or back', ((await status.textContent().catch(() => '')) ?? '').includes('前后移动（深度）'), await status.textContent().catch(() => '(none)'))
  await hover('x')
  await h.until(async () => ((await status.textContent().catch(() => '')) ?? '').includes('沿 X'))
  h.step('over the X arrow it says it moves along X', ((await status.textContent().catch(() => '')) ?? '').includes('沿 X 移动'), await status.textContent().catch(() => '(none)'))
  await hover('ry')
  await h.until(async () => ((await status.textContent().catch(() => '')) ?? '').includes('绕 Y'))
  h.step('over the Y ring it says it rotates around Y', ((await status.textContent().catch(() => '')) ?? '').includes('绕 Y 旋转'), await status.textContent().catch(() => '(none)'))
  const corner = await h.at(5, 5)
  await page.mouse.move(corner.x, corner.y, { steps: 4 })
  await h.until(async () => (await status.count()) === 0)
  h.step('off the handles the status bar says nothing of them', (await status.count()) === 0)
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: false } }])

  // 4. The Smoother on four Position keys along one line (200, 300, 340, 900): no more keys, and
  // the layer never runs back along the line.
  await h.setUp([
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 0, value: [200, 360] } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 1, value: [300, 360] } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 1.4, value: [340, 360] } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 2.5, value: [900, 360] } },
  ])
  await selectShape()
  await h.focusEditor()
  await page.keyboard.press('KeyP')
  const positionName = kit.getByTestId(`motion-prop-row-${SHAPE}-transform.position`).locator('.motion-prop-label')
  await positionName.waitFor()
  await positionName.click()
  await h.until(async () => ((await h.query('keys.info', {})) ?? []).length === 4)
  // The menu's row names the Smoother once the editor has read the four selected keyframes (until
  // then it says it waits for three): opened again until it does.
  const smootherRow = kit.locator('[role=menu] [data-testid="motion-control-smoother"]').first()
  for (let i = 0; i < 20; i += 1) {
    await kit.getByTestId('motion-menu-Animation').click()
    await kit.locator('[data-kit-ui-id="menu:Animation/Keyframe Assistant"]').hover()
    await smootherRow.waitFor()
    if (!((await smootherRow.textContent()) ?? '').includes('（')) break
    await page.keyboard.press('Escape')
    await page.waitForTimeout(300)
  }
  await smootherRow.click()
  await kit.getByTestId('motion-smoother-dialog').waitFor()
  await page.keyboard.press('Enter')
  await h.until(async () => (await kit.getByTestId('motion-smoother-dialog').count()) === 0)
  await page.waitForTimeout(500)
  const smoothed = await keysOf(SHAPE, 'transform/position')
  h.step('the Smoother gives no more keys than the four it was given', smoothed >= 2 && smoothed <= 4, String(smoothed))
  const xs = []
  for (let frame = 0; frame <= 75; frame += 3) xs.push((await value(SHAPE, 'transform/position', frame / 30))[0])
  const back = xs.findIndex((x, at) => at > 0 && x < xs[at - 1] - 1e-6)
  h.step('the smoothed path never runs back and stays between 200 and 900', back < 0 && Math.min(...xs) >= 200 - 1e-6 && Math.max(...xs) <= 900 + 1e-6, xs.map((x) => x.toFixed(1)).join(' '))

  // 5. The graph editor fits its time to the keys when it opens, and closing it puts the layer bars' zoom back.
  const timeline = kit.getByTestId('motion-timeline')
  const viewOf = async () => (await timeline.getAttribute('data-view')).split(' ').map(Number)
  const before = await viewOf()
  const duration = (await h.details()).comp.duration
  await kit.getByTestId('motion-graph-toggle').click()
  await h.until(async () => (await viewOf())[1] < before[1] - 0.2, 80)
  const fitted = await viewOf()
  h.step('opening the graph editor fits its time to the keys (0–2.5 s of a 3 s composition)', fitted[1] < duration - 0.2 && fitted[0] <= 1e-6 && fitted[0] + fitted[1] >= 2.5 - 1e-6, `${before} → ${fitted}`)
  await kit.getByTestId('motion-graph-fit').click()
  await page.waitForTimeout(300)
  await kit.getByTestId('motion-graph-toggle').click()
  await h.until(async () => (await viewOf()).join() === before.join())
  h.step('closing it puts the layer bars’ zoom back, 适合全部 included', (await viewOf()).join() === before.join(), `${await viewOf()} (was ${before})`)

  // 6. The default new composition caches from end to end.
  await kit.getByTestId('motion-menu-Composition').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-comp.new"]').first().click()
  await page.waitForTimeout(400)
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.details()).comp?.width === 1920 && (await h.details()).comp?.name !== '短片')
  const comp = (await h.details()).comp
  h.step('Composition ▸ New Composition… with Enter makes 1920 × 1080, 10 s, 30 fps', comp.width === 1920 && comp.height === 1080 && near(comp.duration, 10, 1e-6) && near(comp.frameRate, 30, 1e-6), JSON.stringify({ w: comp.width, h: comp.height, d: comp.duration, fps: comp.frameRate }))
  await h.setUp([
    { command: 'layer.newSolid', params: { name: 'Ground', color: '#2a6fdb' } },
    { command: 'layer.newSolid', params: { name: 'Card', color: '#f2c14e', width: 600, height: 300 } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/position', time: 0, value: [400, 300] } },
    { command: 'prop.addKey', params: { layer: '#1', path: 'transform/position', time: 9, value: [1500, 800] } },
    { command: 'edit.deselectAll', params: {} },
  ])
  await button.click()
  await h.until(async () => (await attr(button, 'data-done')) >= 300 || (await button.getAttribute('data-full')) === 'true', 1800)
  const done = await attr(button, 'data-done')
  const bytes = await attr(button, 'data-bytes')
  const budget = await attr(button, 'data-budget')
  console.log(`MEASURE ${h.which}: default composition at Full: ${done}/300 frames held in ${(bytes / 1048576).toFixed(1)} MB of a ${(budget / 1048576).toFixed(0)} MB budget`)
  h.step('a default composition caches all 300 frames at Full, within the budget', done === 300 && (await button.getAttribute('data-full')) !== 'true' && bytes > 0 && bytes <= budget, `${done} frames, ${bytes} bytes of ${budget}`)
  h.step('the button says so', ((await button.textContent()) ?? '').includes('300/300'), await button.textContent())

  // 7. A frame from the cache looks as it does rendered again.
  /** A click on the ruler at `seconds` (the playhead goes there). */
  const seek = async (seconds) => {
    const ruler = await kit.locator('.motion-ruler').first().boundingBox()
    await page.mouse.click(ruler.x + (seconds * ruler.width) / 10, ruler.y + ruler.height - 4)
    await h.until(async () => near((await h.details()).time, seconds, 0.05))
  }
  const hitsAtFirst = (await h.cache()).hits
  await seek(4)
  await h.until(async () => (await h.cache()).hits > hitsAtFirst)
  const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
  const clip = { x: Math.round(surface.x + surface.width / 4), y: Math.round(surface.y + surface.height / 4), width: Math.round(surface.width / 2), height: Math.round(surface.height / 2) }
  // A picture once two screenshots a moment apart agree (the frame drawn, nothing moving).
  const stableShot = async () => {
    let last = scanlines(await page.screenshot({ clip }))
    for (let i = 0; i < 30; i += 1) {
      await page.waitForTimeout(200)
      const next = scanlines(await page.screenshot({ clip }))
      if (next.size === last.size && next.lines.equals(last.lines)) return next
      last = next
    }
    return last
  }
  const shotCached = await stableShot()
  const hitsBefore = (await h.cache()).hits - hitsAtFirst
  await button.click()
  await h.until(async () => (await button.getAttribute('aria-pressed')) === 'false')
  await menu('View', [], 'motion-control-purge-cache')
  await h.until(async () => (await h.cache()).cached <= 1)
  await seek(3)
  await seek(4)
  const shotRendered = await stableShot()
  h.step('a frame from the cache is the same picture as the frame rendered again', hitsBefore > 0 && shotCached.size === shotRendered.size && shotCached.lines.equals(shotRendered.lines), `${shotCached.size}, ${shotCached.lines.length} bytes of scanlines; ${hitsBefore} frames shown from the cache`)

  // 8. A heavy picture (noise over the whole frame) fills the cache: the button says what to do,
  // and one click on it caches several times as many frames at Half.
  const ground = (await h.layers()).find((layer) => layer.name === 'Ground').id
  const applied = await h.setUp([{ command: 'effect.apply', params: { effect: 'ec.noise.noise', layers: [ground], params: { amount: 100 } } }, { command: 'edit.deselectAll', params: {} }])
  h.step('(set-up) Noise on the background', applied.completed?.length === 2, JSON.stringify(applied).slice(0, 200))
  await button.click()
  await h.until(async () => (await button.getAttribute('data-full')) === 'true' || (await attr(button, 'data-done')) >= 300, 3000)
  const fullAt = await attr(button, 'data-done')
  const fullBytes = await attr(button, 'data-bytes')
  console.log(`MEASURE ${h.which}: noise at Full: ${fullAt}/300 frames fit (${(fullBytes / 1048576).toFixed(1)} MB of ${(budget / 1048576).toFixed(0)} MB)`)
  h.step('the cache fills at Full, within its budget', (await button.getAttribute('data-full')) === 'true' && fullAt > 10 && fullAt < 300 && fullBytes <= budget, `${fullAt} frames, ${fullBytes} bytes`)
  const lower = kit.getByTestId('motion-cache-lower')
  h.step('the button says the cache is full and how far it got', ((await button.textContent()) ?? '').includes(`缓存已满 ${fullAt}/300`) && ((await button.getAttribute('title')) ?? '').includes('把预览分辨率改为二分之一可缓存更多'), `${await button.textContent()} | ${await button.getAttribute('title')}`)
  h.step('beside it: 把预览分辨率改为二分之一可缓存更多', (await lower.count()) === 1 && ((await lower.textContent()) ?? '') === '把预览分辨率改为二分之一可缓存更多', await lower.textContent().catch(() => '(none)'))
  await lower.click()
  await h.until(async () => (await kit.getByTestId('motion-resolution').inputValue()) === '2')
  h.step('one click sets the preview resolution to Half', (await kit.getByTestId('motion-resolution').inputValue()) === '2')
  await h.until(async () => (await attr(button, 'data-done')) > fullAt && ((await button.getAttribute('data-full')) === 'true' || (await attr(button, 'data-done')) >= 300), 3000)
  const halfAt = await attr(button, 'data-done')
  console.log(`MEASURE ${h.which}: noise at Half: ${halfAt}/300 frames fit`)
  h.step('at Half about four times as many frames fit (or all of them)', halfAt >= Math.min(300, fullAt * 3), `${fullAt} at Full, ${halfAt} at Half`)
  await kit.getByTestId('motion-resolution').selectOption('4')
  await h.until(async () => (await attr(button, 'data-done')) >= 300 || (await button.getAttribute('data-full')) === 'true', 3000)
  const quarterAt = await attr(button, 'data-done')
  console.log(`MEASURE ${h.which}: noise at Quarter: ${quarterAt}/300 frames fit`)
  h.step('at Quarter more still (all 300 here)', quarterAt >= Math.min(300, halfAt * 3), `${quarterAt}`)
  await button.click()
  await h.until(async () => (await button.getAttribute('aria-pressed')) === 'false')

  // 9. Export… offers Third and Quarter, as the Render Queue does.
  await menu('File', [], 'motion-cmd-file.exportMovie')
  const size = kit.getByTestId('motion-export-size')
  await size.waitFor()
  const sizes = await size.locator('option').allTextContents()
  h.step('Export… offers 完整, 二分之一, 三分之一 and 四分之一', ['完整', '二分之一', '三分之一（640 × 360）', '四分之一（480 × 270）'].every((label) => sizes.some((each) => each.startsWith(label))), sizes.join(' | '))
  await size.selectOption('quarter')
  h.step('四分之一 can be chosen', (await size.inputValue()) === 'quarter')
  await page.keyboard.press('Escape')
  await h.until(async () => (await size.count()) === 0)

  // 10. The Render Queue: a job shows the folder it writes to; a new one takes the folder used last.
  await menu('File', ['Export'], 'motion-control-add-to-render-queue')
  await kit.getByTestId('motion-queue-panel').waitFor()
  const jobs = async () => (await h.query('renderQueue.list', {})).items ?? []
  await h.until(async () => (await jobs()).length === 1)
  const first = (await jobs())[0]
  await kit.getByTestId(`motion-queue-item-${first.id}-settings`).click()
  const folderField = kit.getByTestId('motion-job-folder')
  await folderField.waitFor()
  await folderField.fill('job-png')
  await kit.getByTestId('motion-job-dialog-ok').click()
  await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
  await h.until(async () => ((await kit.getByTestId(`motion-queue-item-${first.id}-folder`).textContent()) ?? '').includes('job-png'))
  h.step('the job’s row says where it writes', ((await kit.getByTestId(`motion-queue-item-${first.id}-folder`).textContent()) ?? '').includes('存到：job-png/'), await kit.getByTestId(`motion-queue-item-${first.id}-folder`).textContent())
  await menu('File', ['Export'], 'motion-control-add-to-render-queue')
  await h.until(async () => (await jobs()).length === 2)
  const second = (await jobs())[1]
  await h.until(async () => (await kit.getByTestId(`motion-queue-item-${second.id}-folder`).count()) === 1)
  h.step('a second job shows the folder it took from the first', ((await kit.getByTestId(`motion-queue-item-${second.id}-folder`).textContent()) ?? '').includes('job-png/'), await kit.getByTestId(`motion-queue-item-${second.id}-folder`).textContent())

} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
