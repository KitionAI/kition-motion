// Every menu item and Home button by hand, in a real browser through the development host:
// node tests/menus.mjs [chromium|webkit]
// Walks every leaf of every menu and of Home's ribbon (its drop-downs too) with nothing selected and
// with a layer selected, clicks each enabled one with the pointer, and fails when one ends in an
// error notice or changes nothing (no document, selection, time, view, playback or command change,
// no dialog). A dialog is filled, confirmed, and its result checked (and undone where it edits).
// In zh-CN every label must have Chinese characters, effect names excepted. Cmd+K, Cmd+Y and
// Alt+Shift+J open their dialogs; Backspace deletes like Delete; the splitter resizes the timeline.
// MENUS_OUT=<file> writes every leaf's result as JSON (the walkthrough's menu table is made from it).
import { writeFileSync } from 'node:fs'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const WORKERS = Number(process.env.MENUS_WORKERS ?? 4)
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1', watch: null }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
let failures = 0
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` - ${detail}` : ''}`)
}
const CJK = /[㐀-鿿]/
const PICTURE = 'iVBORw0KGgoAAAANSUhEUgAAAEAAAAAwCAIAAAAuKetIAAAACXBIWXMAAAABAAAAAQBPJcTWAAAAT0lEQVR4nO3PwQkAIBDAMAX3X/kcwkcQmgnaPetvRwe8akBrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQLufsAG9SQcS0wAAAABJRU5ErkJggg=='

/** A fresh page on the sample: the playhead at the middle, and in `layer` the shape layer selected by a click on its row. */
async function open(state) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
  const page = await context.newPage()
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  const kit = page.frameLocator('#kit')
  await kit.getByTestId('motion-timeline').waitFor()
  // A 64 × 48 red picture in the workspace, for File ▸ Import's picker to pick.
  await page.evaluate((data) => window.devHost.files.set('picture.png', { bytes: Uint8Array.from(atob(data), (c) => c.charCodeAt(0)).buffer, version: 1 }), PICTURE)
  await page.waitForTimeout(400)
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  const ruler = await kit.getByTestId('motion-ruler').boundingBox()
  await page.mouse.click(ruler.x + ruler.width * 0.5, ruler.y + ruler.height / 2)
  const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
  const shape = (await details()).info.layers.find((layer) => layer.name.startsWith('Shape'))
  if (state === 'layer') await kit.getByTestId(`motion-layer-${shape.id}`).click()
  if (state === 'solid') await kit.getByTestId(`motion-layer-${(await details()).info.layers.find((layer) => layer.type === 'Solid').id}`).click()
  await page.waitForTimeout(400)
  return { context, page, kit, errors, primary, details, shape }
}

/** What a click may change: the document, the selection, the time, the comp, the view, playback, the commands that can run, the files. */
async function snapshot({ page, kit }) {
  const inspected = await page.evaluate(async () => window.devHost.connection().ai.inspect())
  const enabled = await page.evaluate(async () => (await window.devHost.connection().ai.commands('')).filter((command) => command.enabled !== false).map((command) => command.id).sort().join(' '))
  const files = await page.evaluate(() => [...window.devHost.files.keys()].sort().join(' '))
  const view = await kit.getByTestId('motion-workspace').getAttribute('data-view')
  const play = await kit.getByTestId('motion-play').getAttribute('aria-label')
  const render = await kit.getByTestId('motion-render').textContent()
  const d = inspected.details ?? {}
  const layers = (d.info?.layers ?? []).map((layer) => `${layer.id}:${layer.name}:${layer.selected}:${JSON.stringify(layer.switches)}`).join('|')
  return { revision: inspected.revision, comp: JSON.stringify(d.comp), time: d.time, layers, enabled, files, view, play, render }
}
const changes = (a, b) => Object.keys(a).filter((key) => a[key] !== b[key])
const errorToasts = (kit) => kit.locator('.kui-toast[data-level="error"]').count()
const dialogOpen = (kit) => kit.locator('.kui-dialog').count()

/** The leaves of a popover menu, submenus walked by hovering their rows. */
async function walkMenu(page, kit, popover, path, uipath, out) {
  const items = await popover.evaluate((element) =>
    [...element.children].flatMap((child) => {
      if (child.classList.contains('kui-submenu')) {
        const row = child.querySelector(':scope > [role=menuitem]')
        return [{ kind: 'submenu', uiid: row.getAttribute('data-kit-ui-id'), label: row.textContent.trim() }]
      }
      const item = child.matches('[role^=menuitem]') ? child : child.querySelector(':scope > [role^=menuitem]')
      if (!item) return []
      const spans = [...item.querySelectorAll(':scope > span')].map((span) => span.textContent.trim()).filter(Boolean)
      return [{ kind: 'leaf', testid: item.getAttribute('data-testid'), label: spans[0] ?? item.textContent.trim(), shortcut: item.querySelector('kbd')?.textContent.trim() ?? '', disabled: item.disabled || item.getAttribute('aria-disabled') === 'true' }]
    }),
  )
  for (const item of items) {
    if (item.kind === 'leaf') {
      out.push({ ...item, path: [...path, item.label], uipath })
      continue
    }
    out.push({ kind: 'submenu', label: item.label, path: [...path, item.label] })
    const row = kit.locator(`[data-kit-ui-id="${item.uiid}"]`)
    await row.hover()
    const sub = row.locator('xpath=..').locator(':scope > [role=menu]')
    await sub.waitFor({ timeout: 3000 })
    await walkMenu(page, kit, sub, [...path, item.label], [...uipath, item.uiid], out)
  }
}

/** Every leaf of the menu bar and of Home, with its label and whether it is enabled here. */
async function enumerate({ page, kit }) {
  const out = []
  const tops = await kit.locator('[data-testid^="motion-menu-"]').evaluateAll((buttons) => buttons.map((button) => ({ id: button.getAttribute('data-testid').slice('motion-menu-'.length), label: button.textContent.trim() })).filter((top) => !top.id.startsWith('bar')))
  for (const top of tops) {
    out.push({ kind: 'submenu', label: top.label, path: [top.label] })
    await kit.getByTestId(`motion-menu-${top.id}`).click()
    const popover = kit.locator('.kui-popover[role=menu]:not(.kui-submenu-popover)').first()
    await popover.waitFor()
    const leaves = []
    await walkMenu(page, kit, popover, [top.label], [], leaves)
    out.push(...leaves.map((leaf) => ({ ...leaf, top: top.id })))
    await page.keyboard.press('Escape')
    await page.waitForTimeout(100)
  }
  await kit.getByTestId('ribbon-tab-home').click()
  const panel = kit.getByTestId('ribbon-drop-home')
  await panel.waitFor()
  // A drop-down (RibbonMenu) is a button pressed while its menu is open; a command button is not.
  const buttons = await panel.locator('button[data-testid]').evaluateAll((all) => all.map((button) => ({ testid: button.getAttribute('data-testid'), label: (button.getAttribute('aria-label') || button.textContent).trim(), disabled: button.disabled, menu: button.hasAttribute('aria-pressed') })))
  for (const button of buttons) {
    if (!button.menu) {
      out.push({ kind: 'leaf', top: 'Home', home: true, testid: button.testid, label: button.label, disabled: button.disabled, path: ['开始', button.label] })
      continue
    }
    out.push({ kind: 'submenu', label: button.label, path: ['开始', button.label] })
    if (button.disabled) continue
    await panel.getByTestId(button.testid).click()
    const menu = kit.locator('.kui-popover[role=menu]').last()
    await menu.waitFor()
    const leaves = []
    await walkMenu(page, kit, menu, ['开始', button.label], [], leaves)
    out.push(...leaves.map((leaf) => ({ ...leaf, top: 'Home', home: true, parent: button.testid })))
    await page.keyboard.press('Escape')
    await page.waitForTimeout(100)
    if (!(await panel.isVisible())) await kit.getByTestId('ribbon-tab-home').click()
  }
  await page.keyboard.press('Escape')
  return out
}

/** Opens a leaf's menu path and clicks it. */
async function click({ page, kit }, leaf) {
  if (leaf.home) {
    await kit.getByTestId('ribbon-tab-home').click()
    const panel = kit.getByTestId('ribbon-drop-home')
    await panel.waitFor()
    if (leaf.parent) {
      await panel.getByTestId(leaf.parent).click()
      await kit.locator(`[role=menu] [data-testid="${leaf.testid}"]`).first().click()
    } else await panel.getByTestId(leaf.testid).click()
    return
  }
  await kit.getByTestId(`motion-menu-${leaf.top}`).click()
  for (const uiid of leaf.uipath) await kit.locator(`[data-kit-ui-id="${uiid}"]`).hover()
  await kit.locator(`[role=menu] [data-testid="${leaf.testid}"]`).first().click()
}

const undo = async ({ kit, page }) => {
  await kit.getByTestId('motion-undo').click()
  await page.waitForTimeout(500)
}

/** A dialog the click opened: filled, confirmed, and its result checked. Answers a note, or throws. */
async function finishDialog(ctx, leaf, state) {
  const { kit, page, details } = ctx
  const before = await details()
  const revision = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).revision
  const startRevision = await revision()
  const field = (testId) => kit.getByTestId(testId)
  const confirm = async () => {
    await page.keyboard.press('Enter')
    await page.waitForTimeout(700)
    if (await dialogOpen(kit)) throw new Error('the dialog stayed open after Enter')
  }
  if (await field('motion-comp-dialog').count()) {
    const newComp = leaf.testid.includes('comp.new')
    await field('motion-comp-width').fill('1000')
    await field('motion-comp-height').fill('500')
    await field('motion-comp-name').click()
    await confirm()
    const after = await details()
    if (after.comp.width !== 1000 || after.comp.height !== 500) throw new Error(`the composition is not 1000 × 500: ${JSON.stringify(after.comp)}`)
    if (newComp && after.comp.id === before.comp.id) throw new Error('no new composition')
    await undo(ctx)
    const undone = await details()
    if (!newComp && undone.comp.width !== before.comp.width) throw new Error(`undo did not restore the width: ${undone.comp.width}`)
    return newComp ? 'new composition 1000 × 500 made and opened' : 'settings 1000 × 500 applied; undo restored them'
  }
  if (await field('motion-solid-dialog').count()) {
    const newSolid = !leaf.testid.includes('layer.settings')
    await field('motion-solid-color').fill('#ff0000')
    await field('motion-solid-width').fill('400')
    await field('motion-solid-name').click()
    await confirm()
    const after = await details()
    if (newSolid ? after.info.layers.length !== before.info.layers.length + 1 : (await revision()) === startRevision) throw new Error(newSolid ? 'no new solid' : 'the solid did not change')
    if (newSolid && !after.info.layers.some((layer) => layer.type === 'Solid' && layer.selected)) throw new Error('the new solid is not selected')
    await undo(ctx)
    if (newSolid && (await details()).info.layers.length !== before.info.layers.length) throw new Error('undo did not remove the solid')
    return newSolid ? 'red solid 400 px wide added; undo removed it' : 'solid settings applied'
  }
  if (await field('motion-time-dialog').count()) {
    await field('motion-time-field').fill('0:00:02:00')
    await confirm()
    const time = (await details()).time
    if (Math.abs(time - 2) > 1e-3) throw new Error(`time is ${time}, not 2 s`)
    return 'went to 0:00:02:00'
  }
  if (await field('motion-rename-dialog').count()) {
    await field('motion-rename-field').fill('Renamed by test')
    await confirm()
    if (!(await details()).info.layers.some((layer) => layer.name === 'Renamed by test')) throw new Error('the layer was not renamed')
    return 'layer renamed'
  }
  if (await field('motion-effect-dialog').count()) {
    if (state === 'none') {
      const hint = await field('motion-effect-dialog').textContent()
      if (!CJK.test(hint) || !(await field('motion-effect-dialog-ok').isDisabled())) throw new Error('without a selection, Find Effect should say to select a layer and not apply')
      await page.keyboard.press('Escape')
      await page.waitForTimeout(200)
      if (await dialogOpen(kit)) throw new Error('Escape did not close Find Effect')
      return 'lists effects and asks for a selection first'
    }
    await page.keyboard.type('高斯')
    const first = await field('motion-effect-list').locator('[role=option]').first().textContent()
    if (!first.includes('高斯模糊')) throw new Error(`typing 高斯 does not find Gaussian Blur first: ${first}`)
    await confirm()
    if ((await revision()) === startRevision) throw new Error('no effect applied')
    return 'found 高斯模糊 by typing and applied it'
  }
  if (await field('motion-export-dialog').count()) {
    const movies = async () => (await page.evaluate(() => [...window.devHost.files.keys()])).filter((name) => name.endsWith('.mp4'))
    await field('motion-export-dialog-ok').click()
    for (let i = 0; i < 300 && !(await movies()).length; i += 1) await page.waitForTimeout(100)
    if (!(await movies()).length) throw new Error('Export wrote no MP4')
    if (await dialogOpen(kit)) throw new Error('the Export dialog stayed open after rendering')
    return `exported ${(await movies())[0]}`
  }
  throw new Error('an unknown dialog opened')
}

/** Clicks one leaf on a fresh page and classifies what happened. */
async function run(state, leaf) {
  const ctx = await open(state)
  try {
    const before = await snapshot(ctx)
    await click(ctx, leaf)
    await ctx.page.waitForTimeout(700)
    if (await errorToasts(ctx.kit)) return { result: 'error', note: (await ctx.kit.locator('.kui-toast').first().textContent()).trim() }
    if (await dialogOpen(ctx.kit)) {
      const note = await finishDialog(ctx, leaf, state)
      if (await errorToasts(ctx.kit)) return { result: 'error', note: `after the dialog: ${(await ctx.kit.locator('.kui-toast').first().textContent()).trim()}` }
      return { result: 'dialog', note }
    }
    const after = await snapshot(ctx)
    const changed = changes(before, after)
    if (ctx.errors.length) return { result: 'error', note: `page error: ${ctx.errors[0]}` }
    return changed.length ? { result: 'changed', note: changed.join(', ') } : { result: 'nothing', note: '' }
  } catch (error) {
    return { result: 'error', note: String(error.message ?? error).split('\n')[0] }
  } finally {
    await ctx.context.close()
  }
}

/** Applies every effect leaf on one page, undoing each after: a change and no error notice each. */
async function runEffects(leaves) {
  const ctx = await open('layer')
  const results = []
  try {
    for (const leaf of leaves) {
      const revision = async () => (await ctx.page.evaluate(async () => window.devHost.connection().ai.inspect())).revision
      const before = await revision()
      const toasts = await errorToasts(ctx.kit)
      await click(ctx, leaf)
      let after = before
      for (let i = 0; i < 40 && after === before; i += 1) {
        await ctx.page.waitForTimeout(100)
        after = await revision()
      }
      await ctx.page.waitForTimeout(150)
      if ((await errorToasts(ctx.kit)) > toasts) results.push({ leaf, result: 'error', note: (await ctx.kit.locator('.kui-toast').last().textContent()).trim() })
      else if (after === before) results.push({ leaf, result: 'nothing', note: '' })
      else results.push({ leaf, result: 'changed', note: 'effect applied' })
      await undo(ctx)
    }
  } finally {
    await ctx.context.close()
  }
  return results
}

const all = { none: null, layer: null }
const report = []
try {
  for (const state of ['none', 'layer']) {
    const ctx = await open(state)
    const leaves = await enumerate(ctx)
    await ctx.context.close()
    all[state] = leaves
    const effectNames = new Set(
      (await (async () => {
        const probe = await open('none')
        const list = (await probe.page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'effect.list', params: {} }], false))).completed[0].result
        await probe.context.close()
        return list
      })()).map((effect) => effect.name),
    )
    const english = leaves.filter((leaf) => !CJK.test(leaf.label) && !effectNames.has(leaf.label))
    step(`${state}: every label is in Chinese (effect names excepted)`, english.length === 0, english.map((leaf) => leaf.path.join(' › ')).slice(0, 12).join('; '))
    const enabled = leaves.filter((leaf) => leaf.kind === 'leaf' && !leaf.disabled)
    const effects = enabled.filter((leaf) => leaf.testid?.startsWith('motion-control-effect:'))
    const others = enabled.filter((leaf) => !effects.includes(leaf))
    console.log(`${state}: ${leaves.filter((leaf) => leaf.kind === 'leaf').length} leaves, ${enabled.length} enabled (${effects.length} effects)`)
    const results = []
    const queue = [...others]
    await Promise.all(
      Array.from({ length: WORKERS }, async () => {
        while (queue.length) {
          const leaf = queue.shift()
          results.push({ leaf, ...(await run(state, leaf)) })
        }
      }),
    )
    if (effects.length) {
      const chunks = Array.from({ length: WORKERS }, (_, index) => effects.filter((_, at) => at % WORKERS === index))
      for (const chunk of await Promise.all(chunks.map((chunk) => (chunk.length ? runEffects(chunk) : [])))) results.push(...chunk)
    }
    for (const { leaf, result, note } of results) {
      const ok = result === 'changed' || result === 'dialog'
      if (!ok || process.env.MENUS_VERBOSE) step(`${state}: ${leaf.path.join(' › ')} (${result})`, ok, note)
      report.push({ state, path: leaf.path, testid: leaf.testid, shortcut: leaf.shortcut ?? '', result, note })
    }
    const bad = results.filter((item) => item.result !== 'changed' && item.result !== 'dialog')
    step(`${state}: every enabled item does something (${results.length} clicked)`, bad.length === 0, `${bad.length} did not`)
    for (const leaf of leaves.filter((item) => item.kind === 'leaf' && item.disabled)) report.push({ state, path: leaf.path, testid: leaf.testid, shortcut: leaf.shortcut ?? '', result: 'disabled', note: '' })
  }

  // Shortcuts: the dialogs open from the keyboard, as their menu rows say.
  {
    const ctx = await open('layer')
    const { kit, page, primary } = ctx
    await kit.getByTestId('motion-timeline-rows').click({ position: { x: 600, y: 200 } })
    await kit.getByTestId(`motion-layer-${ctx.shape.id}`).click()
    const items = async () => (await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'project.summary', params: {} }], false))).completed[0].result.items.length
    const itemCount = await items()
    for (const [keys, testId] of [[`${primary}+k`, 'motion-comp-dialog'], [`${primary}+y`, 'motion-solid-dialog'], ['Alt+Shift+J', 'motion-time-dialog'], [`${primary}+Alt+KeyN`, 'motion-comp-dialog']]) {
      await page.keyboard.press(keys)
      await page.waitForTimeout(400)
      step(`${keys} opens its dialog`, (await kit.getByTestId(testId).count()) === 1)
      await page.keyboard.press('Escape')
      await page.waitForTimeout(300)
      step(`Escape closes it`, (await dialogOpen(kit)) === 0)
    }
    // Cmd+N is the host's and the browser's: New Composition is Cmd+Alt+N, and that key no longer replaces the project (the engine's New Project).
    step(`${primary}+Alt+N left the project as it was`, (await items()) === itemCount && (await ctx.details()).info.layers.length > 0)
    await kit.getByTestId('motion-menu-Composition').click()
    const newComp = kit.locator('[role=menu] [data-testid="motion-cmd-comp.new"]')
    step('New Composition shows the shortcut that runs it', (await newComp.locator('kbd').textContent()).replace(/\s/g, '').includes('Alt+N'), await newComp.locator('kbd').textContent())
    await page.keyboard.press('Escape')
    // Backspace deletes the selected layer as Delete does.
    const count = (await ctx.details()).info.layers.length
    await page.keyboard.press('Backspace')
    await page.waitForTimeout(500)
    step('Backspace deletes the selected layer', (await ctx.details()).info.layers.length === count - 1)
    await ctx.context.close()
  }

  // With three layers selected: Sequence Layers puts them one after another, Distribute spaces them (each one undo step).
  {
    const ctx = await open('none')
    const { kit, page, primary } = ctx
    const all = (await ctx.details()).info.layers
    await kit.getByTestId(`motion-layer-${all[0].id}`).click()
    for (const layer of all.slice(1)) await kit.getByTestId(`motion-layer-${layer.id}`).click({ modifiers: [primary] })
    await page.waitForTimeout(600)
    for (const [menu, sub, testid, what] of [['Animation', null, 'motion-control-sequence', 'Sequence Layers'], ['Layer', 'menu:Layer/Align', 'motion-control-distribute-hcenter', 'Distribute Horizontal Centers']]) {
      const before = (await page.evaluate(async () => window.devHost.connection().ai.inspect())).revision
      await kit.getByTestId(`motion-menu-${menu}`).click()
      if (sub) await kit.locator(`[data-kit-ui-id="${sub}"]`).hover()
      const row = kit.locator(`[role=menu] [data-testid="${testid}"]`).first()
      const shown = (await row.count()) === 1
      if (shown) await row.click()
      else await page.keyboard.press('Escape')
      await page.waitForTimeout(600)
      const after = (await page.evaluate(async () => window.devHost.connection().ai.inspect())).revision
      step(`three layers: ${what} is shown and changes the composition`, shown && after !== before && (await errorToasts(kit)) === 0)
      await undo(ctx)
      step(`and one undo takes it back`, (await page.evaluate(async () => window.devHost.connection().ai.inspect())).revision !== after)
    }
    await ctx.context.close()
  }

  // Solid Settings, on a selected solid.
  {
    const ctx = await open('solid')
    const leaf = { top: 'Layer', uipath: [], testid: 'motion-cmd-layer.settings', path: ['图层', '纯色设置…'] }
    const result = await run('solid', leaf)
    step(`solid: Solid Settings (${result.result})`, result.result === 'dialog', result.note)
    report.push({ state: 'solid', path: leaf.path, testid: leaf.testid, shortcut: '', ...result })
    await ctx.context.close()
  }

  // The splitter between the composition and the timeline drags the timeline's height, kept for the session.
  {
    const ctx = await open('none')
    const { kit, page } = ctx
    const splitter = await kit.getByTestId('motion-splitter').boundingBox()
    const height = (await kit.getByTestId('motion-timeline').boundingBox()).height
    await page.mouse.move(splitter.x + splitter.width / 2, splitter.y + splitter.height / 2)
    await page.mouse.down()
    await page.mouse.move(splitter.x + splitter.width / 2, splitter.y - 100, { steps: 8 })
    await page.mouse.up()
    const taller = (await kit.getByTestId('motion-timeline').boundingBox()).height
    step('dragging the splitter up makes the timeline taller', Math.abs(taller - height - 100) <= 4, `${height} → ${taller}`)
    await page.reload()
    await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
    await kit.getByTestId('motion-timeline').waitFor()
    const kept = (await kit.getByTestId('motion-timeline').boundingBox()).height
    step('the height is kept for the session', Math.abs(kept - taller) <= 2, `${kept}`)
    await ctx.context.close()
  }
} finally {
  if (process.env.MENUS_OUT) writeFileSync(process.env.MENUS_OUT, JSON.stringify({ leaves: all, results: report }, null, 1))
  await browser.close()
  await server.close()
}
console.log(failures ? `${failures} failed` : 'all passed')
process.exit(failures ? 1 : 0)
