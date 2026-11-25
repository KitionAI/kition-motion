// The effect browser by hand, in a real browser through the development host: node tests/effectBrowser.mjs [chromium|webkit]
// Find Effect lists the categories (opened with a click), each effect with a one-line description
// in Chinese; the star adds an effect to Favourites (kept when the browser opens again) and takes
// it out; an effect applied shows under Recently used; typing still finds an effect and Enter
// applies it. Round 7g: with every category open, no two effects share a name, in Chinese or in
// English, and every effect has its own description in both (in American spelling), wrapped to two
// lines rather than cut. Results are read from the engine (`layer.tree`'s effects). Prints PASS or
// FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const GAUSSIAN = 'ec.blur.gaussian'
const GLOW = 'ec.stylize.glow'
const effects = async () => ((await h.query('layer.tree', { layer: SHAPE })).properties.children ?? []).find((node) => node.match === 'effects')?.children ?? []
const dialog = () => kit.getByTestId('motion-effect-dialog')
/** Opens Find Effect from the menu bar (in the page's language: `openBrowserHere`). */
let openBrowserHere = async () => {
  await kit.getByTestId('motion-menu-Effect').click()
  await kit.locator('[role=menu] [role=menuitem]', { hasText: '查找效果' }).first().click()
  await dialog().waitFor()
}
const openBrowser = () => openBrowserHere()

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

  // 5. Round 7g: every effect listed, in Chinese and in English: distinct names, its own description.
  const everyEffect = async () => {
    await openBrowser()
    const closed = kit.locator('[data-testid^="motion-effect-head-category-"][aria-expanded="false"]')
    for (let i = 0; i < 40 && (await closed.count()) > 0; i += 1) await closed.first().click()
    const rows = await kit.locator('[data-testid^="motion-effect-option-"]').evaluateAll((options) =>
      options.map((option) => ({ id: option.getAttribute('data-testid').slice('motion-effect-option-'.length), name: option.querySelector('.motion-effect-name > span')?.textContent ?? '', about: option.querySelector('.motion-effect-about')?.textContent ?? '' })),
    )
    return rows
  }
  const checkLocale = async (locale, own) => {
    const rows = await everyEffect()
    const byName = new Map()
    for (const row of rows) byName.set(row.name, [...(byName.get(row.name) ?? []), row.id])
    const shared = [...byName].filter(([, ids]) => ids.length > 1)
    h.step(`${locale}: no two effects share a name`, rows.length > 250 && shared.length === 0, `${rows.length} effects; ${shared.map(([name, ids]) => `${name}: ${ids.join(', ')}`).join(' | ')}`)
    const without = rows.filter((row) => !own(row.about.split(' · ')[0] ?? '') || !row.about.includes(' · '))
    h.step(`${locale}: every effect has its own description`, without.length === 0, `${without.length} without: ${without.slice(0, 6).map((row) => row.id).join(', ')}`)
    return rows
  }
  const zhRows = await checkLocale('zh-CN', (line) => /[㐀-鿿]/.test(line))
  // The settings listed in Chinese are all named in Chinese (none left out, none in English).
  const fourColor = zhRows.find((row) => row.id === 'ec.generate.fourcolor')
  const fastBox = zhRows.find((row) => row.id === 'ec.blur.fastbox')
  h.step('zh-CN: the settings are listed in Chinese, none left out', fourColor?.about.includes('点 1') && fastBox?.about.includes('模糊半径') && fastBox?.about.includes('迭代') && !zhRows.some((row) => /[A-Za-z][a-z]{5,}/.test(row.about.split(' · ')[1] ?? '')), `${fourColor?.about} | ${fastBox?.about} | ${zhRows.filter((row) => /[A-Za-z][a-z]{5,}/.test(row.about.split(' · ')[1] ?? '')).slice(0, 4).map((row) => row.about).join(' | ')}`)
  // A long description wraps to a second line instead of being cut.
  const keylight = kit.getByTestId('motion-effect-about-ec.keying.keylight').first()
  await keylight.scrollIntoViewIfNeeded()
  const lines = await keylight.evaluate((element) => element.getBoundingClientRect().height / parseFloat(getComputedStyle(element).lineHeight))
  h.step('a long description wraps to two lines', lines > 1.6 && lines < 2.4, `${lines.toFixed(2)} lines`)
  // Searching by the English name still finds an effect renamed in Chinese.
  await kit.getByTestId('motion-effect-search').fill('CC Radial Blur')
  await kit.getByTestId('motion-effect-option-ec.blur.ccradial').waitFor()
  h.step('zh-CN: the English name still finds it', ((await kit.getByTestId('motion-effect-option-ec.blur.ccradial').textContent()) ?? '').includes('放射状模糊'))
  await page.keyboard.press('Escape')
  await page.goto(page.url().replace('locale=zh-CN', 'locale=en'))
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  await kit.getByTestId('motion-timeline').waitFor()
  await page.waitForTimeout(500)
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  const openBrowserEn = async () => {
    await kit.getByTestId('motion-menu-Effect').click()
    await kit.locator('[role=menu] [role=menuitem]', { hasText: 'Find Effect' }).first().click()
    await dialog().waitFor()
  }
  openBrowserHere = openBrowserEn
  const enRows = await checkLocale('en', (line) => /[A-Za-z]{3}/.test(line) && !/[㐀-鿿]/.test(line))
  h.step('en: the descriptions spell "color", never "colour"', enRows.length > 250 && !enRows.some((row) => /colour/i.test(row.about)), enRows.filter((row) => /colour/i.test(row.about)).map((row) => row.id).join(', '))
  await page.keyboard.press('Escape')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
