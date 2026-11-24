// Text animators by hand, in a real browser through the development host:
// node tests/animators.mjs [chromium|webkit]
// The Text section's Animate menu adds an animator (by the pointer and by the keyboard), its range
// selector's Start and Advanced settings change, properties and selectors are added and removed
// one at a time, the animator goes, and the Presets menu applies a built-in text animation. Results
// are read from the engine (read-only queries). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const TEXT = 73

/**
 * Opens animator 1's Remove menu and picks `part` in it; on failure says which removal it was,
 * with the panel's test ids and a picture of the page, so a missing card shows why.
 */
async function removeFromAnimator(part, which) {
  try {
    await kit.getByTestId('motion-animator-remove-1').click({ timeout: 10000 })
    await kit.getByTestId(`motion-animator-remove-1-${part}`).click({ timeout: 10000 })
  } catch (error) {
    const ids = await kit.getByTestId('motion-properties-body').locator('[data-testid]').evaluateAll((all) => all.map((each) => each.getAttribute('data-testid')).filter((id) => /animator/.test(id ?? ''))).catch(() => [])
    const shot = `/tmp/animators-${h.which}-${part}.png`
    await page.screenshot({ path: shot }).catch(() => null)
    throw new Error(`the ${which} removal (${part}) failed: ${String(error?.message ?? error).split('\n')[0]}; panel ids: ${ids.join(' ')}; picture: ${shot}`)
  }
}

/** The layer's animators: each `{uid, selectors: [match], properties: [match]}`. */
async function animators() {
  const tree = (await h.query('layer.tree', { layer: TEXT, depth: 8 }))?.properties
  const text = (tree?.children ?? []).find((child) => child.match === 'text')
  const group = (text?.children ?? []).find((child) => child.path === 'text/animators')
  return (group?.children ?? [])
    .filter((child) => child.children)
    .map((animator) => {
      const part = (match) => ((animator.children ?? []).find((child) => child.match === match)?.children ?? []).map((child) => child.match)
      return { uid: animator.uid, node: animator, selectors: part('selectors'), properties: part('properties') }
    })
}

try {
  await h.open()
  await kit.getByTestId(`motion-layer-${TEXT}`).click()
  await kit.getByTestId('motion-section-text').waitFor()
  step('the text layer has no animator to start with', (await animators()).length === 0)

  // 1. Animate ▸ Position: an animator with a Range Selector and Position.
  await kit.getByTestId('motion-text-animate').click()
  await kit.getByTestId('motion-text-animate-position').click()
  await until(async () => (await animators()).length === 1)
  const [first] = await animators()
  step('动画 ▸ 位置 adds an animator with a range selector animating Position', first?.selectors.join() === 'rangeSelector' && first?.properties.join() === 'position', JSON.stringify(first && { s: first.selectors, p: first.properties }))
  await kit.getByTestId('motion-animator-1').waitFor()
  step('the panel shows the animator under the text settings', (await kit.getByTestId('motion-animator-1').count()) === 1)

  // 2. The range selector's Start typed in, and its Advanced Based On chosen.
  const startPath = 'text.animators.animator.selectors.rangeSelector.start'
  const startField = kit.getByTestId(`motion-value-${startPath}-0`)
  // Its groups open one by one (Selectors, then Range Selector 1) until Start shows.
  for (const group of ['text.animators.animator.selectors', 'text.animators.animator.selectors.rangeSelector']) {
    if (await startField.count()) break
    const toggle = kit.getByTestId(`motion-group-toggle-${group}`)
    await toggle.waitFor({ timeout: 3000 })
    if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click()
  }
  await startField.waitFor({ timeout: 3000 })
  await startField.click()
  await startField.fill('40')
  await startField.press('Enter')
  const startValue = async () => (await h.query('prop.get', { layer: TEXT, path: 'text/animators/animator/selectors/rangeSelector/start' }))?.value
  await until(async () => (await startValue()) === 40)
  step('typing 40 in Start sets the range’s start', (await startValue()) === 40, JSON.stringify(await startValue()))
  const advanced = kit.getByTestId('motion-group-toggle-text.animators.animator.selectors.rangeSelector.advanced')
  await advanced.click()
  const based = kit.getByTestId('motion-value-text.animators.animator.selectors.rangeSelector.advanced.basedOn')
  await based.waitFor({ timeout: 3000 })
  const options = await based.locator('option').evaluateAll((all) => all.map((option) => option.value))
  await based.selectOption(options[options.length - 1])
  const basedValue = async () => (await h.query('prop.get', { layer: TEXT, path: 'text/animators/animator/selectors/rangeSelector/advanced/basedOn' }))?.value
  await until(async () => (await basedValue()) === Number(options[options.length - 1]))
  step('Advanced ▸ Based On changes to the last choice (lines)', (await basedValue()) === Number(options[options.length - 1]), JSON.stringify({ value: await basedValue(), options }))

  // 3. A property and a selector added, then each removed alone.
  await kit.getByTestId('motion-animator-add-1').click()
  await kit.getByTestId('motion-animator-add-1-opacity').click()
  await until(async () => (await animators())[0]?.properties.includes('opacity'))
  step('Add ▸ 不透明度 adds Opacity to the animator', (await animators())[0]?.properties.join() === 'position,opacity', JSON.stringify((await animators())[0]?.properties))
  await kit.getByTestId('motion-animator-add-1').click()
  await kit.getByTestId('motion-animator-selector-1-wiggly').click()
  await until(async () => (await animators())[0]?.selectors.includes('wigglySelector'))
  step('Add ▸ 摆动选择器 adds a wiggly selector', (await animators())[0]?.selectors.join() === 'rangeSelector,wigglySelector', JSON.stringify((await animators())[0]?.selectors))
  await removeFromAnimator('opacity', 'first')
  await until(async () => !(await animators())[0]?.properties.includes('opacity'))
  step('Remove ▸ 不透明度 removes only Opacity', (await animators())[0]?.properties.join() === 'position', JSON.stringify((await animators())[0]?.properties))
  await removeFromAnimator('wigglySelector', 'second')
  await until(async () => !(await animators())[0]?.selectors.includes('wigglySelector'))
  step('and the wiggly selector alone', (await animators())[0]?.selectors.join() === 'rangeSelector')
  await h.undo()
  await until(async () => (await animators())[0]?.selectors.includes('wigglySelector'))
  step('one undo brings the selector back', (await animators())[0]?.selectors.join() === 'rangeSelector,wigglySelector')

  // 4. The animator's Position offset typed in moves the characters it covers.
  const offset = kit.getByTestId('motion-value-text.animators.animator.properties.position-1')
  const propertiesToggle = kit.getByTestId('motion-group-toggle-text.animators.animator.properties')
  if (!(await offset.count()) && (await propertiesToggle.getAttribute('aria-expanded')) !== 'true') await propertiesToggle.click()
  await offset.waitFor({ timeout: 3000 }).catch(() => null)
  if ((await offset.count()) === 1) {
    await offset.click()
    await offset.fill('-30')
    await offset.press('Enter')
    const position = async () => (await h.query('prop.get', { layer: TEXT, path: 'text/animators/animator/properties/position' }))?.value
    await until(async () => (await position())?.[1] === -30)
    step('the animator’s Position y typed in', (await position())?.[1] === -30, JSON.stringify(await position()))
  } else step('the animator’s Position has x and y fields', false, String(await offset.count()))

  // 5. The whole animator removed.
  await removeFromAnimator('all', 'third')
  await until(async () => (await animators()).length === 0)
  step('Remove ▸ the animator takes it away', (await animators()).length === 0)

  // 6. By the keyboard: the Animate menu opens with Enter and takes an arrow and Enter.
  await kit.getByTestId('motion-text-animate').focus()
  await page.keyboard.press('Enter')
  await kit.getByTestId('motion-text-animate-position').waitFor()
  await page.keyboard.press('ArrowDown')
  await page.keyboard.press('Enter')
  await until(async () => (await animators()).length === 1)
  const typed = (await animators())[0]
  step('Enter, an arrow and Enter in the Animate menu add an animator', typed?.properties.length === 1, JSON.stringify(typed?.properties))
  await h.undo()
  await until(async () => (await animators()).length === 0)

  // 7. Presets ▸ Fade Up Characters: an animation from the current time, keyframed.
  await kit.getByTestId('motion-text-presets').click()
  await kit.getByTestId('motion-text-preset-fadeUpCharacters').click()
  await until(async () => (await animators()).length > 0)
  const animated = (await h.layer(TEXT)).animated.filter((property) => property.path.startsWith('text/animators'))
  step('预设 ▸ 逐字淡入上升 adds an animator with keyframes', (await animators()).length > 0 && animated.length > 0, JSON.stringify(animated.map((property) => property.path)))
  await h.undo()
  await until(async () => (await animators()).length === 0)
  step('one undo takes the preset back', (await animators()).length === 0)
  await kit.getByTestId('motion-text-presets').click()
  await kit.getByTestId('motion-text-preset-typewriter').click()
  await until(async () => (await animators()).length > 0)
  step('预设 ▸ 打字机 adds an animator', (await animators()).length > 0)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
