// Caching ahead by hand, in a real browser through the development host: node tests/cacheAhead.mjs [chromium|webkit]
// View ▸ Cache Work Area in Background renders the work area's frames while nothing plays (the
// cache bar fills, none is shown), playback then takes every frame from the cache at the
// composition's rate, an edit empties the cache and it fills again by itself, and the status bar's
// Cache ahead button shows the state and turns it off (an edit then leaves the cache empty).
// Results are read from the engine's frame cache (the workspace's data-preview). Prints PASS or
// FAIL per step and the frame rates measured.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h
const SHAPE = 27
const button = kit.getByTestId('motion-cache-ahead')
const playing = async () => (await kit.getByTestId('motion-play').getAttribute('aria-label')) === '暂停'

/** Plays from the current time to the end (Loop off); answers the frame rate the transport showed while it played (the median). */
async function playRate() {
  const rates = []
  await kit.getByTestId('motion-play').click()
  await until(playing, 30)
  for (let i = 0; i < 200 && (await playing()); i += 1) {
    const fps = Number(await kit.getByTestId('motion-fps').getAttribute('data-fps', { timeout: 200 }).catch(() => NaN))
    if (Number.isFinite(fps)) rates.push(fps)
    await page.waitForTimeout(100)
  }
  await until(async () => !(await playing()), 30)
  await page.waitForTimeout(600)
  // The middle of the rates read while it played (not the last read alone: at the very end the
  // transport's rate falls to 0 as playback stops).
  const heard = rates.filter((fps) => fps > 0).sort((a, b) => a - b)
  return heard.length ? heard[Math.floor(heard.length / 2)] : NaN
}

try {
  await h.open()
  const comp = (await h.details()).comp
  const frames = Math.round(comp.duration * comp.frameRate)
  // Playback here runs to the end and stops: Loop off.
  await kit.getByTestId('motion-loop').click()
  await until(async () => (await kit.getByTestId('motion-loop').getAttribute('aria-pressed')) === 'false')
  step('Cache ahead is off by default, and says so', (await button.getAttribute('aria-pressed')) === 'false')
  const before = await h.cache()

  // 1. View ▸ Cache Work Area in Background: the frames are rendered with nothing playing.
  await kit.getByTestId('motion-menu-View').click()
  await kit.locator('[role=menu] [role=menuitem]', { hasText: '在后台缓存工作区' }).first().click()
  await until(async () => (await button.getAttribute('aria-pressed')) === 'true')
  step('the View row turns it on (the status bar button shows it)', (await button.getAttribute('aria-pressed')) === 'true')
  await until(async () => (await h.cache()).cached >= frames, 300)
  const filled = await h.cache()
  step('every frame of the work area is cached without playing', filled.cached >= frames, `${filled.cached} of ${frames}`)
  step('none of them was shown', filled.shown - before.shown <= 1, `${filled.shown - before.shown} shown`)
  step('the cache bar over the ruler shows them all', Number(await kit.getByTestId('motion-cache-bar').getAttribute('data-frames')) >= frames)
  step('the button says how many', Number(await button.getAttribute('data-done')) >= frames && ((await button.getAttribute('title')) ?? '').includes(`${frames}`), await button.getAttribute('title'))
  // The View row now offers to stop.
  await kit.getByTestId('motion-menu-View').click()
  step('the View row now says Stop Caching in Background', (await kit.locator('[role=menu] [role=menuitem]', { hasText: '停止后台缓存' }).count()) === 1)
  await page.keyboard.press('Escape')

  // 2. Playback takes them from the cache, at the composition's rate.
  await h.focusEditor()
  await page.keyboard.press('Home')
  await until(async () => (await h.details()).time === 0)
  const start0 = await h.cache()
  const fps = await playRate()
  const played = await h.cache()
  step('playback renders nothing: every frame comes from the cache', played.rendered - start0.rendered <= 1 && played.hits - start0.hits >= frames / 3, JSON.stringify({ rendered: played.rendered - start0.rendered, hits: played.hits - start0.hits }))
  console.log(`MEASURE ${h.which}: cached ahead, playback ${fps.toFixed?.(1)} fps of ${comp.frameRate}`)

  // 3. An edit empties the cache; it fills again by itself.
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await until(async () => (await h.layer(SHAPE))?.selected)
  await h.focusEditor()
  await page.keyboard.press('ArrowRight')
  await until(async () => (await h.cache()).cached < frames, 50)
  step('an edit empties the cache', (await h.cache()).cached < frames, String((await h.cache()).cached))
  await until(async () => (await h.cache()).cached >= frames, 300)
  step('and it fills again by itself', (await h.cache()).cached >= frames, String((await h.cache()).cached))
  await h.undo()

  // 4. Off from the status bar: an edit leaves the cache empty.
  await button.click()
  await until(async () => (await button.getAttribute('aria-pressed')) === 'false')
  step('the status bar button turns it off', (await button.getAttribute('aria-pressed')) === 'false')
  await h.focusEditor()
  await page.keyboard.press('ArrowLeft')
  await until(async () => (await h.cache()).cached <= 1, 50)
  await page.waitForTimeout(1500)
  step('off, an edit leaves the cache as playback fills it (nothing ahead)', (await h.cache()).cached <= 1, String((await h.cache()).cached))
  await h.undo()
  await until(async () => (await h.cache()).cached <= 1, 50)

  // 5. The rate of step 2, against the same composition played here without the cache (each frame
  // rendered while it plays). On a fast machine cached playback keeps the composition's rate. On a
  // shared CI runner painting with the CPU (WebKit there) the rate is the runner's painting, not
  // the cache's, so there it must be at least the uncached rate measured in this same run (the
  // cache is not slower than rendering) and a third of the composition's rate; that every frame
  // came from the cache is checked strictly above.
  await h.focusEditor()
  await page.keyboard.press('Home')
  await until(async () => (await h.details()).time === 0)
  const uncached = await playRate()
  console.log(`MEASURE ${h.which}: not cached, playback ${uncached.toFixed?.(1)} fps of ${comp.frameRate}`)
  // With the engine's answers slowed down on purpose (ENGINE_DELAY_MS), each frame waits for one
  // answer: the rate that allows, at most.
  const delay = Number(process.env.ENGINE_DELAY_MS ?? 0)
  const reachable = delay > 0 ? Math.min(comp.frameRate, 1000 / (delay * 1.5)) : comp.frameRate
  const full = fps >= reachable * 0.8
  const asFastAsTheRunner = fps >= uncached * 0.9 && fps >= reachable / 3
  step('cached playback runs at (nearly) the composition’s rate, or (on a slow runner) no slower than uncached playback', full || asFastAsTheRunner, `${fps} of ${comp.frameRate}, uncached ${uncached}${delay > 0 ? ` (at most ${reachable.toFixed(1)} with answers ${delay} ms late)` : ''}`)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
