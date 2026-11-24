// The effect browser by hand, in a real browser through the development host: node tests/effectBrowser.mjs [chromium|webkit]
// Find Effect lists the categories (opened with a click), each effect with a one-line description
// in Chinese; the star adds an effect to Favourites (kept when the browser opens again) and takes
// it out; an effect applied shows under Recently used; typing still finds an effect and Enter
// applies it. Results are read from the engine (`layer.tree`'s effects). Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const GAUSSIAN = 'ec.blur.gaussian'
const GLOW = 'ec.stylize.glow'
const effects = async () => ((await h.query('layer.tree', { layer: SHAPE })).properties.children ?? []).find((node) => node.match === 'effects')?.children ?? []
const dialog = () => kit.getByTestId('motion-effect-dialog')
async function openBrowser() {
  await kit.getByTestId('motion-menu-Effect').click()
  await kit.locator('[role=menu] [role=menuitem]', { hasText: '查找效果' }).first().click()
  await dialog().waitFor()
}

try {
  await h.open()
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)

  // 1. The categories, closed, in Chinese.
  await openBrowser()
  const heads = await kit.locator('[data-testid^="motion-effect-head-category-"]').allTextContents()
  h.step('Find Effect lists the categories, in Chinese', heads.length >= 15 && heads.some((head) => head.includes('模糊和锐化')), heads.slice(0, 4).join(' | '))
  h.step('nothing starred or used yet: no Favourites or Recently used', (await kit.getByTestId('motion-effect-group-favourites').count()) === 0 && (await kit.getByTestId('motion-effect-group-recent').count()) === 0)
  const blurHead = kit.getByTestId('motion-effect-head-category-Blur & Sharpen')
  await blurHead.click()
  const gaussian = kit.getByTestId(`motion-effect-option-${GAUSSIAN}`)
  await gaussian.waitFor()
  h.step('a click opens a category', (await blurHead.getAttribute('aria-expanded')) === 'true')
  const about = (await kit.getByTestId(`motion-effect-about-${GAUSSIAN}`).first().textContent()) ?? ''
  // Round 7f: its own line (what Gaussian Blur does, not its category's words) before its settings.
  const radial = (await kit.getByTestId('motion-effect-about-ec.blur.ccradial').first().textContent()) ?? ''
  h.step('each effect has a one-line description in Chinese, its own', /[㐀-鿿]/.test(about.split(' · ')[0]) && !about.includes('模糊或锐化画面') && about.includes('参数：') && about.includes('模糊度') && radial.split(' · ')[0] !== about.split(' · ')[0], `${about} | ${radial}`)

  // 2. The star: Favourites, kept when the browser opens again.
  await kit.getByTestId(`motion-effect-star-category-Blur & Sharpen-${GAUSSIAN}`).click()
  await kit.getByTestId('motion-effect-group-favourites').waitFor()
  h.step('the star adds it to Favourites, at the top', (await kit.getByTestId(`motion-effect-favourites-${GAUSSIAN}`).count()) === 1 && (await kit.getByTestId(`motion-effect-star-category-Blur & Sharpen-${GAUSSIAN}`).getAttribute('aria-pressed')) === 'true')
  await page.keyboard.press('Escape')
  await h.until(async () => (await dialog().count()) === 0)
  await openBrowser()
  h.step('Favourites are kept when Find Effect opens again', (await kit.getByTestId(`motion-effect-favourites-${GAUSSIAN}`).count()) === 1)

  // 3. Typing finds an effect; Enter applies it; it then shows under Recently used.
  const before = (await effects()).length
  await kit.getByTestId('motion-effect-search').fill('发光')
  await kit.getByTestId(`motion-effect-option-${GLOW}`).waitFor()
  // Round 7f: Glow's own line (it brightens and spreads the bright parts), not the Stylize category's.
  h.step('typing lists the matches with their descriptions', ((await kit.getByTestId(`motion-effect-about-${GLOW}`).textContent()) ?? '').includes('亮'), await kit.getByTestId(`motion-effect-about-${GLOW}`).textContent())
  await page.keyboard.press('Enter')
  await h.until(async () => (await effects()).length === before + 1)
  h.step('Enter applies the first match', (await effects()).length === before + 1)
  await openBrowser()
  h.step('it shows under Recently used', (await kit.getByTestId(`motion-effect-recent-${GLOW}`).count()) === 1)
  // A click on a favourite applies it.
  await kit.getByTestId(`motion-effect-favourites-${GAUSSIAN}`).click()
  await h.until(async () => (await effects()).length === before + 2)
  h.step('a click on a favourite applies it', (await effects()).length === before + 2)
  await h.undo()
  await h.until(async () => (await effects()).length === before + 1)
  h.step('undo takes it off', (await effects()).length === before + 1)

  // 4. The star again takes it out of Favourites.
  await openBrowser()
  await kit.getByTestId(`motion-effect-star-favourites-${GAUSSIAN}`).click()
  await h.until(async () => (await kit.getByTestId('motion-effect-group-favourites').count()) === 0)
  h.step('starred again, it leaves Favourites', (await kit.getByTestId('motion-effect-group-favourites').count()) === 0)
  await page.keyboard.press('Escape')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
