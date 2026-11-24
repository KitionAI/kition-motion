// Info and audio by hand, in a real browser through the development host: node tests/info.mjs [chromium|webkit]
// The status bar reads the composition pixel under the pointer and its colour (0-255, or percent
// after a click); L shows a sound layer's Audio Levels and LL its waveform too; while it plays the
// transport's meters show the sound, and muting the layer (its audio switch) or soloing another
// silences it at once, each undone. Prints PASS or FAIL per step.
import { start, toneWav } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const info = kit.getByTestId('motion-info')
const meters = kit.getByTestId('motion-meters')
const level = async () => Math.max(Number(await meters.getAttribute('data-left')), Number(await meters.getAttribute('data-right')))

try {
  await h.open()
  // 1. The pointer over the blue shape: its position and colour in the status bar.
  const centre = (await h.query('prop.get', { layer: SHAPE, path: 'transform/position' })).value
  // A whole page pixel (WebKit's pointer events have whole coordinates), and the composition point under it.
  const near = await h.at(centre[0], centre[1])
  const p = { x: Math.round(near.x), y: Math.round(near.y) }
  const [zoom, viewX, viewY] = (await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ').map(Number)
  const surface = await kit.getByTestId('kit-canvas-surface').boundingBox()
  const at = [Math.floor(viewX + (p.x - surface.x) / zoom), Math.floor(viewY + (p.y - surface.y) / zoom)]
  await page.mouse.move(p.x - 30, p.y - 30)
  await page.mouse.move(p.x, p.y, { steps: 4 })
  // The readout follows the last move (one question to the engine at a time).
  await h.until(async () => (await info.count()) === 1 && (await info.getAttribute('data-rgba')) != null && Math.abs(Number(await info.getAttribute('data-x')) - at[0]) <= 1 && Math.abs(Number(await info.getAttribute('data-y')) - at[1]) <= 1)
  const x = Number(await info.getAttribute('data-x'))
  const y = Number(await info.getAttribute('data-y'))
  const rgba = ((await info.getAttribute('data-rgba')) ?? '').split(' ').map(Number)
  const seen = await h.rgb(at[0] + 0.5, at[1] + 0.5)
  h.step('the status bar reads the composition pixel under the pointer', Math.abs(x - at[0]) <= 1 && Math.abs(y - at[1]) <= 1, `${x} ${y} for ${at}`)
  h.step('and the colour the viewer shows there', rgba.length === 4 && rgba.slice(0, 3).every((value, channel) => Math.abs(value - seen[channel]) <= 6) && rgba[3] === 255, `${rgba} vs ${seen}`)
  h.step('in 0-255', (await info.textContent()).includes(`${rgba[0]} ${rgba[1]} ${rgba[2]} 255`), await info.textContent())
  await info.click()
  await h.until(async () => (await info.getAttribute('aria-pressed')) === 'true')
  h.step('a click shows percent, and the button says so', (await info.textContent()).includes(`${Math.round((rgba[0] / 255) * 100)}% ${Math.round((rgba[1] / 255) * 100)}%`) && (await info.getAttribute('aria-pressed')) === 'true', await info.textContent())
  await info.click()
  await h.until(async () => (await info.getAttribute('aria-pressed')) === 'false')
  h.step('another click goes back to 0-255', (await info.textContent()).includes(`${rgba[0]} ${rgba[1]} ${rgba[2]} 255`))
  h.step('the readout never took the focus into a text field', await page.evaluate(() => !(document.getElementById('kit').contentDocument.activeElement instanceof HTMLInputElement)))

  // 2. A sound layer: L shows its Audio Levels, LL its waveform too.
  await h.putFile('tone.wav', toneWav(2))
  const tone = (await h.query('file.import', { paths: ['tone.wav'] })).items?.[0]
  await h.setUp([{ command: 'layer.addItem', params: { item: tone, time: 0 } }])
  await h.until(async () => (await h.layers()).some((layer) => layer.name === 'tone.wav'))
  const sound = (await h.layers()).find((layer) => layer.name === 'tone.wav')
  await kit.getByTestId(`motion-layer-${sound.id}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(sound.id))?.selected)
  await page.keyboard.press('KeyL')
  await kit.getByTestId(`motion-prop-row-${sound.id}-audio.levels`).waitFor()
  h.step('L shows Audio Levels', (await kit.getByTestId(`motion-prop-row-${sound.id}-audio.levels`).count()) === 1 && (await kit.getByTestId(`motion-waveform-row-${sound.id}`).count()) === 0)
  // L twice in a row (a moment after the first L): LL.
  await page.waitForTimeout(600)
  await page.keyboard.press('KeyL')
  await page.keyboard.press('KeyL')
  await kit.getByTestId(`motion-waveform-row-${sound.id}`).waitFor()
  const wave = kit.getByTestId(`motion-waveform-${sound.id}`)
  await h.until(async () => Number(await wave.getAttribute('data-peaks')) > 0)
  h.step('LL shows its waveform, in Chinese', (await kit.getByTestId(`motion-waveform-row-${sound.id}`).textContent()).includes('波形') && Number(await wave.getAttribute('data-peaks')) > 100, await wave.getAttribute('data-peaks'))
  h.step('the waveform has the tone’s level', Number(await wave.getAttribute('data-loudest')) > 0.3, await wave.getAttribute('data-loudest'))
  const bar = await kit.getByTestId(`motion-layer-${sound.id}`).locator('.motion-bar').boundingBox()
  const drawn = await wave.boundingBox()
  h.step('drawn along the layer’s bar', bar && drawn && Math.abs(drawn.x - bar.x) < 4 && Math.abs(drawn.x + drawn.width - (bar.x + bar.width)) < 4, JSON.stringify({ bar, drawn }))

  // 3. The meters while it plays; muting the layer silences them at once, and so does soloing another.
  await h.setUp([{ command: 'time.set', params: { time: 0 } }])
  await h.focusEditor()
  await page.keyboard.press('Space')
  await h.until(async () => (await meters.getAttribute('data-idle')) === null)
  await h.until(async () => (await level()) > 0.1, 80)
  h.step('the meters show the sound while it plays', (await level()) > 0.1, String(await level()))
  h.step('with the loudest peak held', Number(await meters.getAttribute('data-hold-left')) >= Number(await meters.getAttribute('data-left')))
  await kit.getByTestId(`motion-switch-audio-${sound.id}`).click()
  await h.until(async () => (await h.layer(sound.id))?.switches?.audio === false)
  await h.until(async () => (await level()) < 0.001, 40)
  h.step('muting the layer while it plays silences it', (await level()) < 0.001, String(await level()))
  await kit.getByTestId(`motion-switch-audio-${sound.id}`).click()
  await h.until(async () => (await level()) > 0.1, 40)
  h.step('turning it on again is heard again', (await level()) > 0.1, String(await level()))
  await kit.getByTestId(`motion-switch-solo-${SHAPE}`).click()
  await h.until(async () => (await level()) < 0.001, 40)
  h.step('soloing another layer silences it', (await level()) < 0.001, String(await level()))
  await h.focusEditor()
  await page.keyboard.press('Space')
  await h.until(async () => (await meters.getAttribute('data-idle')) !== null)
  h.step('the meters hide when it stops (keeping their place)', (await meters.getAttribute('data-idle')) !== null && !(await meters.isVisible()))
  await h.undo()
  await h.until(async () => (await h.layer(SHAPE))?.switches?.solo === false)
  h.step('undo takes the solo off', (await h.layer(SHAPE))?.switches?.solo === false)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
