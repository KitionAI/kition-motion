// Preview by hand, in a real browser through the development host: node tests/preview.mjs [chromium|webkit]
// Playback keeps the frames it drew (the frame cache: a bar over the ruler), plays them again
// from the cache, and an edit empties it; the transport shows the frame rate playback reaches and
// its Skip frames switch; the status bar's Region button draws a region of interest on the canvas
// (only it is rendered, the rest grey; Escape stops drawing, a second click clears it). Prints PASS
// or FAIL per step, and the measured frame rates and render times.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const SHAPE = 27
const workspace = () => kit.getByTestId('motion-workspace')
const playing = async () => (await kit.getByTestId('motion-play').getAttribute('aria-label')) === '暂停'

/** Plays from the start to the end; answers the frame rates the transport showed and the cache counters before and after. */
async function playThrough() {
  await page.keyboard.press('Home')
  await until(async () => (await h.details()).time === 0)
  await page.waitForTimeout(400)
  const before = await h.cache()
  const rates = []
  await kit.getByTestId('motion-play').click()
  await until(playing, 30)
  for (let i = 0; i < 200 && (await playing()); i += 1) {
    const fps = Number(await kit.getByTestId('motion-fps').getAttribute('data-fps', { timeout: 200 }).catch(() => NaN))
    if (Number.isFinite(fps)) rates.push(fps)
    await page.waitForTimeout(100)
  }
  await page.waitForTimeout(600)
  const after = await h.cache()
  return { rates, before, after, fps: rates.length ? rates[rates.length - 1] : NaN }
}

try {
  await h.open()
  const comp = (await h.details()).comp
  const frames = Math.round(comp.duration * comp.frameRate)
  await h.focusEditor()

  // 1. The first playback renders each frame it shows and keeps it.
  const first = await playThrough()
  const kept = (await h.cache()).cached
  step('a playback keeps the frames it drew (the cache bar over the ruler counts them)', kept > frames / 4 && Number(await kit.getByTestId('motion-cache-bar').getAttribute('data-frames')) === kept, `${kept} of ${frames} frames`)
  step('the transport showed the frame rate while playing', first.rates.length > 0, JSON.stringify(first.rates.slice(-3)))

  // 2. Playing again shows the kept frames from the cache.
  const second = await playThrough()
  step('playing again takes the kept frames from the cache', second.after.hits - second.before.hits > 0, JSON.stringify({ hits: second.after.hits - second.before.hits, rendered: second.after.rendered - second.before.rendered }))
  const renderMs = first.after.rendered > first.before.rendered ? (first.after.renderMs - first.before.renderMs) / (first.after.rendered - first.before.rendered) : NaN
  console.log(`MEASURE ${h.which}: ${frames} frames at ${comp.frameRate} fps; first playback ${first.fps.toFixed(1)} fps (${first.after.rendered - first.before.rendered} rendered, ${renderMs.toFixed(1)} ms each), second ${second.fps.toFixed(1)} fps (${second.after.hits - second.before.hits} from the cache, ${second.after.rendered - second.before.rendered} rendered)`)

  // 3. An edit empties the cache.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await until(async () => (await h.layer(SHAPE))?.selected)
  await h.focusEditor()
  await page.keyboard.press('ArrowRight')
  await until(async () => (await h.cache()).cached <= 1)
  step('an edit (the layer nudged) empties the cache', (await h.cache()).cached <= 1, String((await h.cache()).cached))

  // 4. Skip frames off: playback shows every frame.
  const skip = kit.getByTestId('motion-skip-frames')
  step('Skip frames is on by default', (await skip.getAttribute('aria-pressed')) === 'true')
  await skip.click()
  await until(async () => (await skip.getAttribute('aria-pressed')) === 'false')
  await h.focusEditor()
  const every = await playThrough()
  const shown = every.after.shown - every.before.shown
  step('with Skip frames off, playback shows every frame', (await skip.getAttribute('aria-pressed')) === 'false' && shown >= frames - 2, `${shown} of ${frames} frames shown`)
  await skip.click()
  await until(async () => (await skip.getAttribute('aria-pressed')) === 'true')

  // 5. The region of interest: drawn on the canvas, only it rendered.
  await h.focusEditor()
  await page.keyboard.press('Home')
  await until(async () => (await h.details()).time === 0)
  await kit.getByTestId('motion-roi').click()
  await until(async () => (await workspace().getAttribute('data-roi')) === 'drawing')
  await page.keyboard.press('Escape')
  await until(async () => (await workspace().getAttribute('data-roi')) === null)
  step('Region starts drawing and Escape stops it', (await workspace().getAttribute('data-roi')) === null)
  const outside = [comp.width - 40, comp.height - 40]
  const colourBefore = await h.rgb(...outside)
  await kit.getByTestId('motion-roi').click()
  await until(async () => (await workspace().getAttribute('data-roi')) === 'drawing')
  await h.drag(await h.at(100, 100), await h.at(500, 400))
  await until(async () => /^\d+ \d+ \d+ \d+$/.test((await workspace().getAttribute('data-roi')) ?? ''))
  const region = ((await workspace().getAttribute('data-roi')) ?? '').split(' ').map(Number)
  step('a drag over the composition sets the region of interest', region.length === 4 && Math.abs(region[0] - 100) <= 2 && Math.abs(region[2] - 400) <= 3, JSON.stringify(region))
  await until(async () => (await h.rgb(...outside)).every((part) => Math.abs(part - 48) <= 3))
  const grey = await h.rgb(...outside)
  step('outside the region the picture is grey: it is not rendered', grey.every((part) => Math.abs(part - 48) <= 3) && JSON.stringify(grey) !== JSON.stringify(colourBefore), `${JSON.stringify(colourBefore)} → ${JSON.stringify(grey)}`)
  step('the region is outlined on the canvas', (await kit.getByTestId('motion-roi-box').count()) === 1)
  await kit.getByTestId('motion-roi').click()
  await until(async () => (await workspace().getAttribute('data-roi')) === null)
  await until(async () => JSON.stringify(await h.rgb(...outside)) === JSON.stringify(colourBefore))
  step('a second click clears it and the whole picture renders again', JSON.stringify(await h.rgb(...outside)) === JSON.stringify(colourBefore), JSON.stringify(await h.rgb(...outside)))
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
