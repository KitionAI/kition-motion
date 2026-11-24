// The editor's flow in a real browser, through the development host: node tests/flow.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html in Chinese, and prints PASS or FAIL per step.
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'
import { delayEngineReplies } from './slowEngine.mjs'

const which = process.argv[2] ?? 'chromium'
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1', watch: null }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
await delayEngineReplies(context)
if (which === 'chromium') await context.grantPermissions(['clipboard-read', 'clipboard-write'])
const page = await context.newPage()
const errors = []
let failures = 0
page.on('pageerror', (error) => errors.push(error.message))
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` - ${detail}` : ''}`)
}
const until = async (check, tries = 60) => {
  for (let i = 0; i < tries; i += 1) {
    if (await check().catch(() => false)) return true
    await page.waitForTimeout(100)
  }
  return false
}
const inspect = () => page.evaluate(async () => window.devHost.connection().ai.inspect())
const details = async () => (await inspect()).details

try {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  const kit = page.frameLocator('#kit')
  await page.waitForTimeout(1000)
  step('Chinese menu names', (await kit.getByTestId('motion-menu-Composition').textContent()) === '合成')
  step('status describes the composition', (await kit.getByTestId('motion-status').textContent())?.includes('3 个图层'))
  const shape = (await details()).info.layers.find((layer) => layer.name.startsWith('Shape'))
  await kit.getByTestId(`motion-layer-${shape.id}`).click()
  await kit.getByTestId('motion-value-position-0').waitFor()
  step('the Properties panel shows the selection', await kit.getByTestId('motion-value-position-0').isVisible())
  const ruler = await kit.getByTestId('motion-ruler').boundingBox()
  await page.mouse.click(ruler.x + ruler.width * 0.5, ruler.y + ruler.height / 2)
  await until(async () => (await details()).time > 1.3)
  step('scrubs the ruler', (await details()).time > 1.3 && (await details()).time < 1.7, String((await details()).time))
  await kit.getByTestId('motion-play').click()
  await page.waitForTimeout(600)
  await kit.getByTestId('motion-play').click()
  step('plays and pauses', (await details()).time > 1.6, String((await details()).time))
  // Where playback stopped depends on the machine's speed (it may stop on the key at 2.5 s, where
  // an edit changes that key): the edit happens at 1.8 s, between the layer's keys.
  await page.mouse.click(ruler.x + ruler.width * 0.6, ruler.y + ruler.height / 2)
  await until(async () => Math.abs((await details()).time - 1.8) < 0.05)
  await kit.getByTestId('motion-value-position-1').fill('200')
  await kit.getByTestId('motion-value-position-1').press('Enter')
  const keys = async () => (await details()).info.layers.find((layer) => layer.id === shape.id).animated.find((property) => property.name === 'Position')?.times.length ?? 0
  await until(async () => (await keys()) >= 3)
  step('sets a keyframe by editing a value', (await keys()) >= 3, String(await keys()))
  await kit.getByTestId('motion-new-layer').click()
  await kit.getByTestId('motion-new-text').click()
  await until(async () => (await details()).info.layers.length === 4)
  step('adds a text layer', (await details()).info.layers.length === 4)
  // Export opens its dialog; Render with its settings (the whole composition, MP4) writes the movie.
  await kit.getByTestId('motion-render').click()
  await kit.getByTestId('motion-export-dialog-ok').click()
  await until(async () => (await page.evaluate(() => [...window.devHost.files.keys()])).some((name) => name.endsWith('.mp4')), 600)
  const movie = await page.evaluate(() => {
    const name = [...window.devHost.files.keys()].find((key) => key.endsWith('.mp4'))
    return name ? { name, size: window.devHost.files.get(name).bytes.byteLength } : null
  })
  step('renders an MP4 next to the project', movie && movie.size > 1000, JSON.stringify(movie))
  const run = await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'layer.newNull', params: {} }], false))
  await page.waitForTimeout(300)
  step('the agent edits the live project', run.completed[0].changed && (await page.evaluate(() => window.devHost.changes.at(-1).by)) === 'agent')
  await kit.getByTestId('motion-save').click()
  await until(async () => (await page.evaluate(() => window.devHost.files.get('Sample.kmotion').version)) > 1)
  step('saves through the host', (await page.evaluate(() => window.devHost.files.get('Sample.kmotion').version)) > 1)
  // Focus shows the composition alone: the transport and the timeline are the kit's own bars.
  const app = kit.locator('.kui-app')
  await app.focus()
  // The editor takes Cmd or Ctrl from navigator.platform (@kition/sdk/editor's isMac), while
  // Playwright's ControlOrMeta follows the operating system running the test, and the two can
  // disagree (WebKit on Linux). The shortcut presses the modifier the editor reads.
  const primaryKey = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  await page.keyboard.press(`${primaryKey}+Backslash`)
  await until(async () => (await app.getAttribute('data-mode')) === 'focus')
  const inFocus = { transport: await kit.getByTestId('motion-transport').count(), timeline: await kit.getByTestId('motion-timeline').count() }
  step('Focus hides the transport and the timeline', (await app.getAttribute('data-mode')) === 'focus' && inFocus.transport === 0 && inFocus.timeline === 0, JSON.stringify(inFocus))
  await page.keyboard.press('Escape')
  await until(async () => (await app.getAttribute('data-mode')) !== 'focus')
  step('leaving Focus shows them again', (await kit.getByTestId('motion-transport').count()) === 1 && (await kit.getByTestId('motion-timeline').count()) === 1)
} catch (error) {
  failures += 1
  console.log(`FAIL ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
