// The fixes after the round 7k hand test, by hand, in a real browser through the development host: node tests/fixes7k.mjs [chromium|webkit]
// A Render Queue job's folder box that is empty is automatic: typed into and emptied again, a
// change to PNG sequence gives it the sequence folder `<comp>_序列`, and its frames land there; a
// sequence job whose box is left empty still writes into `<comp>_序列` (its row says so before it
// renders; since 7n the job keeps that folder, so the project holds where it renders); a folder a new job took from the job before it is a default, so a change to a sequence
// gives it a sequence folder and a change back gives the inherited folder again, while a folder
// typed in the job's own dialog stays. (7m) A box holding the composition's sequence folder is
// automatic too, even after typing or when typed by hand: a movie job never saves into it. The cache-full ▾'s three rows start their text at one left
// edge, with Loop only the cached part off and on. Results are read from the engine
// (`renderQueue.list`), the host's files and the elements' own boxes. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h

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

try {
  await h.open()

  // 1. The Render Queue's folder box.
  const jobs = async () => (await h.query('renderQueue.list', {})).items ?? []
  const job = async (id) => (await jobs()).find((each) => each.id === id)
  const addJob = async () => {
    await kit.getByTestId('motion-menu-File').click()
    await kit.locator('[data-kit-ui-id="menu:File/Export"]').hover()
    await kit.locator('[role=menu] [data-testid="motion-control-add-to-render-queue"]').first().click()
  }
  const folderOf = (id) => kit.getByTestId(`motion-queue-item-${id}-folder`)
  const folderField = () => kit.getByTestId('motion-job-folder')
  const openSettings = async (id) => {
    await kit.getByTestId(`motion-queue-item-${id}-settings`).click()
    await kit.getByTestId('motion-job-dialog').waitFor()
  }
  const closeWith = async (button) => {
    await kit.getByTestId(button).click()
    await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
  }
  /** Answers the folder box once two reads agree (the dialog has settled). */
  const settledFolder = async () => {
    let value = await folderField().inputValue()
    await h.until(async () => {
      const again = await folderField().inputValue()
      const same = again === value
      value = again
      return same
    })
    return value
  }
  /** Picks `format` in the open job dialog and answers the folder box once it has settled. */
  const pickFormat = async (format) => {
    await kit.getByTestId('motion-job-format').selectOption(format)
    await h.until(async () => (await kit.getByTestId('motion-job-format').inputValue()) === format)
    return settledFolder()
  }
  /** Types `text` into the folder box with the keyboard, at its end. */
  const typeFolder = async (text) => {
    await folderField().click()
    await page.keyboard.press('End')
    await page.keyboard.type(text)
    await h.until(async () => (await folderField().inputValue()).endsWith(text))
  }
  /** Empties the folder box with the keyboard (select all, Backspace). */
  const emptyFolder = async () => {
    await folderField().click()
    await page.keyboard.press(`${h.primary}+KeyA`)
    await page.keyboard.press('Backspace')
    await h.until(async () => (await folderField().inputValue()) === '')
  }
  /** Two frames at a quarter of the size, in the open dialog. */
  const twoFrames = async () => {
    await kit.getByTestId('motion-job-span').selectOption('custom')
    await kit.getByTestId('motion-job-start').fill('0:00:00:00')
    await kit.getByTestId('motion-job-end').fill('0:00:00:02')
    await kit.getByTestId('motion-job-size').selectOption('quarter')
  }
  const files = async () => page.evaluate(() => [...window.devHost.files.keys()])
  const state = (id) => kit.getByTestId(`motion-queue-item-${id}-status`).getAttribute('data-state')
  /** Renders the queue; answers the files the render wrote that were not there before. */
  const renderNew = async (id) => {
    const before = new Set(await files())
    await kit.getByTestId('motion-queue-render').click()
    await h.until(async () => ['done', 'failed'].includes(await state(id)), 900)
    await h.until(async () => (await kit.getByTestId('motion-queue-render').count()) === 1, 300)
    return (await files()).filter((path) => !before.has(path)).sort()
  }

  // A movie job (into the project's folder); a character typed into its folder box and deleted
  // again, then PNG sequence: the box is empty, so automatic: the sequence folder.
  await addJob()
  await h.until(async () => (await jobs()).length === 1)
  const first = (await jobs())[0]
  const own = `${first.compName}_序列`
  await h.until(async () => (await folderOf(first.id).count()) === 1)
  h.step('(set-up) a new job is a movie (H.264) into the project’s folder', first.output?.format === 'H264' && (await folderOf(first.id).getAttribute('data-folder')) === '', `${first.output?.format} · ${await folderOf(first.id).textContent()}`)
  await openSettings(first.id)
  await typeFolder('x')
  await page.keyboard.press('Backspace')
  await h.until(async () => (await folderField().inputValue()) === '')
  const afterEmptied = await pickFormat('png')
  h.step(`typed x, deleted it, then PNG sequence: the folder box becomes ${own}`, afterEmptied === own, JSON.stringify(afterEmptied))
  await twoFrames()
  await closeWith('motion-job-dialog-ok')
  await h.until(async () => (await folderOf(first.id).getAttribute('data-folder')) === own && (await job(first.id))?.output?.format === 'PngSequence')
  const firstFiles = await renderNew(first.id)
  h.step(`rendered (2 frames), the frames land in ${own}/ and nowhere else`, (await state(first.id)) === 'done' && firstFiles.length === 2 && firstFiles.every((path) => path.startsWith(`${own}/`) && path.endsWith('.png')), firstFiles.join(' | '))
  console.log(`FILES ${h.which}: ${firstFiles.join(' | ')}`)

  // A sequence job whose folder box is left empty at OK: its row says it goes into the sequence
  // folder, and the render puts the frames there (never loose in the project's folder). (7n) The
  // job keeps that folder (not one named at render time in the interface's language).
  await addJob()
  await h.until(async () => (await jobs()).length === 2)
  const second = (await jobs())[1]
  await h.until(async () => (await folderOf(second.id).count()) === 1)
  h.step(`(set-up) the next job is a PNG sequence into ${own} 2/`, second.output?.format === 'PngSequence' && (await folderOf(second.id).getAttribute('data-folder')) === `${own} 2`, await folderOf(second.id).textContent())
  await openSettings(second.id)
  await emptyFolder()
  await kit.getByTestId('motion-job-name').click()
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('loose')
  await h.until(async () => (await kit.getByTestId('motion-job-name').inputValue()) === 'loose')
  await twoFrames()
  await closeWith('motion-job-dialog-ok')
  await h.until(async () => ((await job(second.id))?.output?.output ?? '').startsWith(`${own}/loose`))
  const stored = (await job(second.id))?.output?.output ?? ''
  await h.until(async () => (await folderOf(second.id).getAttribute('data-folder')) === own)
  h.step(`OK with the box empty: the job keeps the sequence folder ${own}/ itself, and its row says 存到：${own}/`, stored.startsWith(`${own}/loose`) && (await folderOf(second.id).getAttribute('data-folder')) === own && ((await folderOf(second.id).textContent()) ?? '').includes(`${own}/`), `${stored} · ${await folderOf(second.id).textContent()}`)
  const secondFiles = (await renderNew(second.id)).filter((path) => path.includes('loose'))
  h.step(`rendered (2 frames), the frames land in ${own}/ as the row said, none loose`, (await state(second.id)) === 'done' && secondFiles.length === 2 && secondFiles.every((path) => path.startsWith(`${own}/loose_`)), secondFiles.join(' | '))
  console.log(`FILES ${h.which}: ${secondFiles.join(' | ')}`)

  // A folder typed in a job's own dialog is that job's: it stays through a change of format. Saved
  // with a movie format, it is also the folder the next new job takes (a default for that job).
  await addJob()
  await h.until(async () => (await jobs()).length === 3)
  const third = (await jobs())[2]
  await h.until(async () => (await folderOf(third.id).count()) === 1)
  await openSettings(third.id)
  await pickFormat('h264')
  await emptyFolder()
  await typeFolder('renders')
  const typedPng = await pickFormat('png')
  const typedMovie = await pickFormat('h264')
  h.step('a folder typed in the job’s dialog (renders) stays through PNG sequence and H.264', typedPng === 'renders' && typedMovie === 'renders', `${typedPng} / ${typedMovie}`)
  await closeWith('motion-job-dialog-ok')
  await h.until(async () => (await folderOf(third.id).getAttribute('data-folder')) === 'renders' && (await job(third.id))?.output?.format === 'H264')
  await openSettings(third.id)
  const reopened = await pickFormat('png')
  h.step('opened again, the job’s own typed folder (renders) stays when it becomes a sequence', reopened === 'renders', JSON.stringify(reopened))
  await closeWith('motion-job-dialog-cancel')

  await addJob()
  await h.until(async () => (await jobs()).length === 4)
  const fourth = (await jobs())[3]
  await h.until(async () => (await folderOf(fourth.id).getAttribute('data-folder')) === 'renders')
  h.step('(set-up) the next new job is a movie into renders/ (the folder used last)', fourth.output?.format === 'H264' && (await folderOf(fourth.id).getAttribute('data-folder')) === 'renders', await folderOf(fourth.id).textContent())
  await openSettings(fourth.id)
  const inheritedPng = await pickFormat('png')
  h.step(`the inherited folder is a default: PNG sequence gives ${own} 2 (${own} is taken; the second job’s box was emptied)`, inheritedPng === `${own} 2`, JSON.stringify(inheritedPng))
  const inheritedBack = await pickFormat('h264')
  h.step('and back to H.264 it is renders again', inheritedBack === 'renders', JSON.stringify(inheritedBack))
  await closeWith('motion-job-dialog-cancel')
  h.step('Cancel changes nothing', (await folderOf(fourth.id).getAttribute('data-folder')) === 'renders' && (await job(fourth.id))?.output?.format === 'H264', `${await folderOf(fourth.id).getAttribute('data-folder')} · ${(await job(fourth.id))?.output?.format}`)

  // (7m) A box holding the composition's sequence folder is automatic whatever was typed before: in
  // one visit typed into, emptied, PNG sequence (the box becomes <comp>_序列), then H.264: the box
  // goes back to the inherited folder, and OK keeps the movie out of the sequence folder. The first
  // two jobs go first, so the sequence folder is <comp>_序列 itself, unnumbered.
  for (const gone of [first, second]) {
    await kit.getByTestId(`motion-queue-item-${gone.id}-remove`).click()
    await h.until(async () => !(await job(gone.id)))
  }
  h.step('(set-up) the first two jobs removed', (await jobs()).length === 2, JSON.stringify((await jobs()).map((each) => each.id)))
  await addJob()
  await h.until(async () => (await jobs()).length === 3)
  const fifth = (await jobs())[2]
  await h.until(async () => (await folderOf(fifth.id).getAttribute('data-folder')) === 'renders')
  h.step('(set-up) the new job is a movie into renders/ (the folder used last)', fifth.output?.format === 'H264' && (await folderOf(fifth.id).getAttribute('data-folder')) === 'renders', await folderOf(fifth.id).textContent())
  await openSettings(fifth.id)
  await typeFolder('x')
  await emptyFolder()
  const roundPng = await pickFormat('png')
  h.step(`typed x, emptied the box, then PNG sequence: the box becomes ${own}`, roundPng === own, JSON.stringify(roundPng))
  const roundMovie = await pickFormat('h264')
  h.step(`then H.264: the box no longer holds ${own}, it is the inherited renders again`, roundMovie === 'renders', JSON.stringify(roundMovie))
  await closeWith('motion-job-dialog-ok')
  await h.until(async () => (await job(fifth.id))?.output?.format === 'H264' && (await folderOf(fifth.id).getAttribute('data-folder')) === 'renders')
  const fifthOut = (await job(fifth.id))?.output?.output ?? ''
  h.step(`OK: the movie job is saved into renders/, its row says so, nothing goes into ${own}`, (await folderOf(fifth.id).getAttribute('data-folder')) === 'renders' && fifthOut.startsWith('renders/') && !fifthOut.includes(own), `${fifthOut} · ${await folderOf(fifth.id).textContent()}`)

  // Typed by hand, <comp>_序列 cannot be told apart from the generated name: for a movie it counts
  // as automatic too (the folder hint says so), and OK saves the movie into its default folder.
  await openSettings(fifth.id)
  await emptyFolder()
  await typeFolder(own)
  await closeWith('motion-job-dialog-ok')
  const typedOwnOut = (await job(fifth.id))?.output?.output ?? ''
  const typedOwnRow = await folderOf(fifth.id).getAttribute('data-folder')
  h.step(`${own} typed for a movie counts as automatic: OK keeps it in renders/, never in ${own}/`, typedOwnRow === 'renders' && typedOwnOut.startsWith('renders/') && !typedOwnOut.includes(own), `${typedOwnOut} · ${await folderOf(fifth.id).textContent()}`)
  const hint = (await folderOf(fifth.id).getAttribute('title')) ?? ''
  h.step('the folder hint says a typed sequence-folder name counts as not set', hint.includes('手动输入的也算') && hint.includes('单个文件不会存进这个文件夹'), hint)

  // 2. The cache-full ▾: the three rows' texts start at one left edge, with the loop off and on.
  await page.context().addInitScript(() => Object.defineProperty(Navigator.prototype, 'deviceMemory', { get: () => undefined, configurable: true }))
  for (const locale of ['zh-CN', 'en']) {
    await reopen(locale, 1440, 900)
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
    await cacheButton.click()
    await h.until(async () => (await cacheButton.getAttribute('data-full')) === 'true', 3000)
    h.step(`${locale}: (set-up) the cache fills up`, (await cacheButton.getAttribute('data-full')) === 'true', await cacheButton.textContent())
    await kit.getByTestId('motion-cache-full-menu').click()
    const rows = ['motion-cache-lower', 'motion-cache-more-memory', 'motion-cache-loop-cached'].map((id) => kit.getByTestId(id))
    await h.until(async () => (await Promise.all(rows.map((row) => row.count()))).every((count) => count === 1))
    const loopCached = rows[2]
    /** Each row's text: where it starts (its label's left edge, in px) and what it says, once two reads agree. */
    const edges = async () => {
      const read = () => Promise.all(rows.map((row) => row.evaluate((element) => {
        const label = [...element.querySelectorAll(':scope > span')].find((span) => span.textContent.trim()) ?? element
        return { left: label.getBoundingClientRect().left, text: label.textContent.trim() }
      })))
      let now = await read()
      await h.until(async () => {
        const again = await read()
        const same = JSON.stringify(again) === JSON.stringify(now)
        now = again
        return same
      })
      return now
    }
    const lined = (all) => Math.max(...all.map((each) => each.left)) - Math.min(...all.map((each) => each.left)) <= 1
    const show = (all) => all.map((each) => `${each.text} @ ${each.left.toFixed(1)}`).join(' | ')
    await page.mouse.move(5, 5)
    const off = await edges()
    h.step(`${locale}: loop off, the three rows’ texts start at one left edge (within 1 px)`, (await loopCached.getAttribute('aria-checked')) === 'false' && off.every((each) => each.text) && lined(off), show(off))
    await loopCached.click()
    await h.until(async () => (await loopCached.getAttribute('aria-checked')) === 'true')
    await page.mouse.move(5, 5)
    const on = await edges()
    h.step(`${locale}: loop on (a click), the tick shows and the texts still start at one left edge, where they were`, (await loopCached.getAttribute('aria-checked')) === 'true' && (await loopCached.locator('svg').count()) === 1 && lined(on) && Math.abs(on[0].left - off[0].left) <= 1, show(on))
    await loopCached.click()
    await h.until(async () => (await loopCached.getAttribute('aria-checked')) === 'false')
    await page.keyboard.press('Escape')
    await cacheButton.click()
    await h.until(async () => (await cacheButton.getAttribute('aria-pressed')) === 'false')
  }
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
