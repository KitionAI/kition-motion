// The fixes after the round 7h hand test, by hand, in a real browser through the development host: node tests/fixes7h.mjs [chromium|webkit]
// A layer setting's source choice offers its three named values (Source / Masks / Effects & Masks);
// effect settings read in Chinese; options that are waves read as waves, and the misleading ones
// say what they do; two options no longer carry another product's name; A shows the Anchor Point
// and R on a 3D layer shows Orientation and the three rotations; the graph editor picks its layer
// itself; a Render Queue job after an image sequence does not write into the sequence's folder;
// and a full cache says exactly what it holds and can loop only that. Results are read from the
// engine (`prop.get`, `renderQueue.list`, the inspected state) and the elements' own attributes.
// Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const NIGHT = 4

/** Opens the sample afresh at `width` × `height` in `locale` (the harness opens it in zh-CN at 1440 × 900). */
async function reopen(locale, width, height) {
  await page.setViewportSize({ width, height })
  const url = new URL(page.url())
  url.searchParams.set('locale', locale)
  await page.goto(url.toString())
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
}
const rowName = (id) => kit.getByTestId(`motion-layer-${id}`).locator('.motion-row-name').first()
async function selectOnly(id) {
  await rowName(id).click()
  await h.until(async () => (await h.layers()).filter((layer) => layer.selected).map((layer) => layer.id).join() === String(id))
}
/** Opens effect card `index` (1-based) of the selected layer and answers it once its drop-downs are drawn. */
async function card(index) {
  const toggle = kit.getByTestId(`motion-effect-toggle-${index}`)
  await toggle.waitFor()
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click()
  const box = kit.getByTestId(`motion-effect-card-${index}`)
  await h.until(async () => (await box.locator('select').count()) > 0)
  return box
}
/** A drop-down's option texts and the chosen one, read again if the card was redrawn while being read. */
async function readSelect(select) {
  let out = { options: [], chosen: '' }
  await h.until(async () => {
    out = await select.evaluate((element) => ({ options: [...element.options].map((option) => option.textContent ?? ''), chosen: element.selectedOptions[0]?.textContent ?? '' }))
    return out.options.length > 0
  })
  return out
}
const same = (a, b) => a.length === b.length && a.every((each, at) => each === b[at])

try {
  await h.open()

  // 1. Layer settings' source choice: three named values, not the digits 0 to 9; a choice is stored as its value.
  await h.setUp([
    { command: 'effect.apply', params: { effect: 'ec.channel.blend', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.audio.distortion', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.channel.arithmetic', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.distort.wavewarp', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.stylize.glow', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.color.cccoloroffset', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.distort.ccbender', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.distort.magnify', layers: [SHAPE] } },
    { command: 'edit.deselectAll', params: {} },
  ])
  await selectOnly(SHAPE)
  const blend = await card(1)
  const source = blend.locator('select[aria-label="与图层混合 来源"]')
  await h.until(async () => (await source.count()) === 1)
  const sourceRead = await readSelect(source)
  h.step('Blend ▸ 与图层混合 来源 offers 源 / 蒙版 / 效果和蒙版 (效果和蒙版 chosen)', same(sourceRead.options, ['源', '蒙版', '效果和蒙版']) && sourceRead.chosen === '效果和蒙版', JSON.stringify(sourceRead))
  await source.selectOption({ label: '蒙版' })
  await h.until(async () => (await h.query('prop.get', { layer: SHAPE, path: 'effects/#1/blendWithLayerSource' })).value === 1)
  h.step('choosing 蒙版 stores the value 1', (await h.query('prop.get', { layer: SHAPE, path: 'effects/#1/blendWithLayerSource' })).value === 1, JSON.stringify((await h.query('prop.get', { layer: SHAPE, path: 'effects/#1/blendWithLayerSource' })).value))

  // 2. Setting names in Chinese (a few the hand test named).
  const distortion = await card(2)
  const distortionText = (await distortion.textContent()) ?? ''
  h.step('Distortion’s settings read 混合量, 音量, 分辨率（位）, 降采样（x）', ['混合量', '音量', '分辨率（位）', '降采样（x）'].every((name) => distortionText.includes(name)) && !/\b(Mix|Volume|Resolution|Downsample)\b/.test(distortionText), distortionText.slice(0, 200))
  const arithmetic = await card(3)
  const operator = await readSelect(arithmetic.locator('select').first())
  const arithmeticText = (await arithmetic.textContent()) ?? ''
  h.step('Arithmetic reads 蓝色值, and Block Above / Below read 高于时清零 / 低于时清零', arithmeticText.includes('蓝色值') && operator.options.includes('高于时清零') && operator.options.includes('低于时清零') && !operator.options.some((option) => option.includes('阻止')), operator.options.join(' | '))

  // 3. Waves read as waves; a square that is a shape stays a square.
  const wave = await readSelect((await card(4)).locator('select').first())
  h.step('Wave Warp ▸ 波浪类型 reads 正弦波 / 方波 / 三角波 / 锯齿波 … 凹半圆形', ['正弦波', '方波', '三角波', '锯齿波', '半圆形', '凹半圆形'].every((label) => wave.options.includes(label)) && !wave.options.includes('正方形'), wave.options.join(' | '))
  const glow = await card(5)
  const glowOptions = await glow.locator('select option').allTextContents()
  h.step('Glow: 在上面 / 在后面 / 无, and its loops 三角波 A>B>A, 锯齿波 A>B', ['在上面', '在后面', '无', '三角波 A>B>A', '锯齿波 A>B'].every((label) => glowOptions.includes(label)) && !glowOptions.includes('顶端'), glowOptions.slice(0, 8).join(' | '))
  const offset = await card(6)
  const offsetOptions = await offset.locator('select option').allTextContents()
  h.step('CC Color Offset ▸ 溢出 reads 回绕 / 折回 / 二值', ['折回', '二值'].every((label) => offsetOptions.includes(label)) && !offsetOptions.includes('曝光过度'), offsetOptions.join(' | '))
  const bender = await readSelect((await card(7)).locator('select').first())
  h.step('CC Bender ▸ 样式 names its curves by shape (中部鼓起, S 形)', bender.options.includes('中部鼓起') && bender.options.includes('S 形') && !bender.options.some((option) => /玛丽莲|拳击手/.test(option)), bender.options.join(' | '))
  const magnify = await readSelect((await card(8)).locator('select').first())
  h.step('Magnify ▸ 形状 keeps 圆形 / 正方形 (a shape, not a wave)', magnify.options.includes('正方形') && !magnify.options.includes('方波'), magnify.options.join(' | '))

  // 4. A shows only the Anchor Point; R on a 3D layer shows Orientation and the three rotations.
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await selectOnly(SHAPE)
  await h.focusEditor()
  await page.keyboard.press('KeyA')
  const row = (path) => kit.getByTestId(`motion-prop-row-${SHAPE}-${path.replaceAll('/', '.')}`)
  await h.until(async () => (await row('transform/anchor').count()) === 1)
  h.step('A shows the Anchor Point alone', (await row('transform/anchor').count()) === 1 && (await row('transform/position').count()) === 0)
  await h.focusEditor()
  await page.keyboard.press('KeyR')
  const rotations = ['transform/orientation', 'transform/rotationX', 'transform/rotationY', 'transform/rotation']
  await h.until(async () => (await row('transform/orientation').count()) === 1)
  const counts = await Promise.all(rotations.map(async (path) => (await row(path).count())))
  h.step('R on a 3D layer shows Orientation and X, Y and Z Rotation', counts.every((count) => count === 1) && (await row('transform/anchor').count()) === 0, counts.join(' '))
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: false } }])
  await h.until(async () => (await row('transform/orientation').count()) === 0)
  h.step('on a 2D layer R shows Rotation only', (await row('transform/rotation').count()) === 1 && (await row('transform/orientation').count()) === 0)

  // 5. The graph editor's own layer picker: choosing a layer selects it and shows its animated properties.
  await h.setUp([
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 0 } },
    { command: 'prop.addKey', params: { layer: SHAPE, path: 'transform/position', time: 1, value: [700, 400] } },
    { command: 'prop.addKey', params: { layer: NIGHT, path: 'transform/opacity', time: 0, value: 100 } },
    { command: 'prop.addKey', params: { layer: NIGHT, path: 'transform/opacity', time: 1, value: 20 } },
  ])
  await selectOnly(SHAPE)
  await kit.getByTestId('motion-graph-toggle').click()
  const picker = kit.getByTestId('motion-graph-layer')
  await picker.waitFor()
  h.step('the graph shows a layer picker with the selected layer chosen', (await picker.inputValue()) === String(SHAPE))
  await picker.selectOption(String(NIGHT))
  await h.until(async () => (await h.layers()).filter((layer) => layer.selected).map((layer) => layer.id).join() === String(NIGHT))
  const legend = kit.locator('.motion-graph-legend li')
  await h.until(async () => (await legend.count()) > 0 && ((await legend.first().textContent()) ?? '').includes('不透明度'))
  const legendText = (await legend.allTextContents()).join(' | ')
  h.step('choosing another layer there selects it and shows its curves', (await picker.inputValue()) === String(NIGHT) && legendText.includes('不透明度') && !legendText.includes('位置'), legendText)
  await kit.getByTestId('motion-graph-toggle').click()

  // 6. The English editor: neutral option names.
  await reopen('en', 1280, 800)
  await h.setUp([
    { command: 'effect.apply', params: { effect: 'ec.color.equalize', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.stylize.ccglass', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.channel.blend', layers: [SHAPE] } },
    { command: 'edit.deselectAll', params: {} },
  ])
  await selectOnly(SHAPE)
  const equalize = await readSelect((await card(1)).locator('select').first())
  const glass = await card(2)
  const glassOptions = await glass.locator('select option').allTextContents()
  h.step('Equalize offers Image Editor Style; CC Glass offers Composition Lights', equalize.options.includes('Image Editor Style') && glassOptions.includes('Composition Lights'), `${equalize.options.join(' | ')} / ${glassOptions.filter((option) => /Light/.test(option)).join(' | ')}`)
  const englishSource = await readSelect((await card(3)).locator('select[aria-label="Blend With Layer Source"]'))
  h.step('Blend With Layer Source offers Source / Masks / Effects & Masks', same(englishSource.options, ['Source', 'Masks', 'Effects & Masks']), englishSource.options.join(' | '))

  // 7. The Render Queue: a job added after an image sequence in its own folder writes to the project's folder.
  await reopen('zh-CN', 1440, 900)
  const jobs = async () => (await h.query('renderQueue.list', {})).items ?? []
  const addJob = async () => {
    await kit.getByTestId('motion-menu-File').click()
    await kit.locator('[data-kit-ui-id="menu:File/Export"]').hover()
    await kit.locator('[role=menu] [data-testid="motion-control-add-to-render-queue"]').first().click()
  }
  await addJob()
  await h.until(async () => (await jobs()).length === 1)
  const first = (await jobs())[0]
  await kit.getByTestId(`motion-queue-item-${first.id}-settings`).click()
  await kit.getByTestId('motion-job-dialog').waitFor()
  await kit.getByTestId('motion-job-format').selectOption('png')
  await kit.getByTestId('motion-job-folder').fill('job-png')
  await kit.getByTestId('motion-job-dialog-ok').click()
  await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
  const folderOf = (id) => kit.getByTestId(`motion-queue-item-${id}-folder`)
  await h.until(async () => (await folderOf(first.id).getAttribute('data-folder')) === 'job-png')
  h.step('the PNG sequence writes into job-png/', (await folderOf(first.id).getAttribute('data-folder')) === 'job-png', await folderOf(first.id).textContent())
  await addJob()
  await h.until(async () => (await jobs()).length === 2)
  const second = (await jobs())[1]
  await h.until(async () => (await folderOf(second.id).count()) === 1)
  h.step('the next job writes to the project’s folder, not the sequence’s', (await folderOf(second.id).getAttribute('data-folder')) === '' && ((await folderOf(second.id).textContent()) ?? '').includes('项目文件夹'), await folderOf(second.id).textContent())
  await kit.getByTestId(`motion-queue-item-${second.id}-settings`).click()
  await kit.getByTestId('motion-job-dialog').waitFor()
  await kit.getByTestId('motion-job-format').selectOption('gif')
  await kit.getByTestId('motion-job-folder').fill('gifs')
  await kit.getByTestId('motion-job-dialog-ok').click()
  await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
  await addJob()
  await h.until(async () => (await jobs()).length === 3)
  const third = (await jobs())[2]
  await h.until(async () => (await folderOf(third.id).count()) === 1)
  h.step('a folder chosen for a movie is still taken by the next job', (await folderOf(third.id).getAttribute('data-folder')) === 'gifs', await folderOf(third.id).textContent())

  // 8. A full cache says what it holds (frames and seconds) and can loop only that.
  await page.context().addInitScript(() => Object.defineProperty(Navigator.prototype, 'deviceMemory', { get: () => undefined, configurable: true }))
  await reopen('zh-CN', 1440, 900)
  await kit.getByTestId('motion-menu-Composition').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-comp.new"]').first().click()
  await page.waitForTimeout(400)
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.details()).comp?.width === 1920 && (await h.layers()).length === 0)
  await h.setUp([
    { command: 'layer.newSolid', params: { name: 'Ground', color: '#2a6fdb' } },
    { command: 'effect.apply', params: { effect: 'ec.noise.noise', layers: ['#1'], params: { amount: 100 } } },
    { command: 'edit.deselectAll', params: {} },
  ])
  const cacheButton = kit.getByTestId('motion-cache-ahead')
  const numberOf = async (name) => Number(await cacheButton.getAttribute(name))
  await cacheButton.click()
  await h.until(async () => (await cacheButton.getAttribute('data-full')) === 'true', 3000)
  const held = await numberOf('data-done')
  const seconds = (Math.round((held / 30) * 10) / 10).toFixed(1)
  const said = `已缓存前 ${held} 帧（${seconds} 秒）`
  await h.until(async () => ((await cacheButton.getAttribute('title')) ?? '').includes(said))
  h.step(`the full cache says exactly what it holds (${said})`, ((await cacheButton.getAttribute('title')) ?? '').includes(said) && ((await cacheButton.getAttribute('title')) ?? '').includes('共 300 帧'), await cacheButton.getAttribute('title'))
  const loopCached = kit.getByTestId('motion-cache-loop-cached')
  h.step('beside it: 只循环已缓存的部分 (off)', (await loopCached.count()) === 1 && (await loopCached.textContent()) === '只循环已缓存的部分' && (await loopCached.getAttribute('aria-pressed')) === 'false', await loopCached.textContent().catch(() => '(none)'))
  const end = held / 30
  const timeNow = async () => (await h.details()).time
  const play = () => kit.getByTestId('motion-play').click()
  const stopped = () => h.until(async () => (await kit.getByTestId('motion-play').getAttribute('aria-label')) === '播放')
  // On: looping playback goes round the cached frames, from 0 to where the cache ends.
  await loopCached.click()
  await h.until(async () => (await loopCached.getAttribute('data-area')) !== null)
  const area = (await loopCached.getAttribute('data-area')) ?? ''
  h.step('on: the loop is the cached frames', (await loopCached.getAttribute('aria-pressed')) === 'true' && area === `0.000 ${end.toFixed(3)}`, area)
  await h.setUp([{ command: 'time.set', params: { time: Math.max(0, end - 0.3) } }])
  await play()
  const times = []
  for (let i = 0; i < 50; i += 1) {
    times.push(await timeNow())
    await page.waitForTimeout(100)
  }
  await play()
  await stopped()
  const wrapped = times.some((time, at) => at > 0 && time < times[at - 1] - 0.1)
  h.step('and playback goes round inside them', times.every((time) => time <= end + 1e-6) && wrapped, times.map((time) => time.toFixed(2)).join(' '))
  // Off: playback goes on past the cached frames (drawn as it reaches them) rather than going round early.
  await loopCached.click()
  await h.until(async () => (await loopCached.getAttribute('aria-pressed')) === 'false' && (await loopCached.getAttribute('data-area')) === null)
  await h.setUp([{ command: 'time.set', params: { time: Math.max(0, end - 0.3) } }])
  await play()
  const past = await h.until(async () => (await timeNow()) > end + 0.05, 600)
  await play()
  await stopped()
  h.step('off: playback goes on past the cached frames', past, `${await timeNow()} s, the cache ends at ${end.toFixed(2)} s`)
  await cacheButton.click()
  await h.until(async () => (await cacheButton.getAttribute('aria-pressed')) === 'false')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
