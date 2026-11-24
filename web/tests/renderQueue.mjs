// The Render Queue by hand, in a real browser through the development host: node tests/renderQueue.mjs [chromium|webkit]
// Adds a job from File ▸ Export ▸ Add to Render Queue and from the Project panel, changes its
// settings (range, format, size, file name and folder) in its dialog, saves and reuses a preset,
// duplicates, reorders and removes jobs (each undone), then renders: two jobs one after another
// while a layer is deleted by hand (the jobs render the project as it was when Render was pressed;
// the deletion stays), Stop on a long job (the next still renders) and Stop All. Results are read
// from the engine (`renderQueue.list`), the host's files and the panel. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const jobs = async () => (await h.query('renderQueue.list', {})).items ?? []
const job = async (id) => (await jobs()).find((each) => each.id === id)
const status = (id) => kit.getByTestId(`motion-queue-item-${id}-status`)
const state = async (id) => (await status(id).count()) ? status(id).getAttribute('data-state') : null
const file = (path) => page.evaluate((name) => {
  const entry = window.devHost.files.get(name)
  return entry ? [...new Uint8Array(entry.bytes).slice(0, 3)] : null
}, path)
const isGif = (bytes) => bytes != null && String.fromCharCode(...bytes) === 'GIF'

/** Opens a job's settings, sets what `fill` sets, and confirms. */
async function settings(id, fill) {
  await kit.getByTestId(`motion-queue-item-${id}-settings`).click()
  await kit.getByTestId('motion-job-dialog').waitFor()
  await fill()
  await kit.getByTestId('motion-job-dialog-ok').click()
  await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
}
/** A short GIF: frames 0 to 4, a quarter of the size, named `name`, in the folder renders. */
async function short(name, end = '0:00:00:05') {
  await kit.getByTestId('motion-job-span').selectOption('custom')
  await kit.getByTestId('motion-job-start').fill('0:00:00:00')
  await kit.getByTestId('motion-job-end').fill(end)
  await kit.getByTestId('motion-job-format').selectOption('gif')
  await kit.getByTestId('motion-job-size').selectOption('quarter')
  await kit.getByTestId('motion-job-name').fill(name)
  await kit.getByTestId('motion-job-folder').fill('renders')
}

try {
  await h.open()
  const comp = (await h.details()).comp
  // 1. File ▸ Export ▸ Add to Render Queue: a job for the active composition; the panel opens.
  await kit.getByTestId('motion-menu-File').click()
  await kit.locator('[role=menu] [role=menuitem]', { hasText: /^导出(?!…)/ }).first().hover()
  const addRow = kit.locator('[role=menu] [data-testid="motion-control-add-to-render-queue"]').first()
  h.step('File ▸ 导出 ▸ 添加到渲染队列 is there', (await addRow.textContent()).includes('添加到渲染队列'))
  await addRow.click()
  await h.until(async () => (await jobs()).length === 1)
  const first = (await jobs())[0]
  h.step('it adds a job for the composition', first?.comp === comp.id, JSON.stringify(first?.compName))
  await kit.getByTestId('motion-queue-panel').waitFor()
  h.step('the Render Queue panel opens, in Chinese', (await kit.getByTestId('motion-queue-panel').textContent()).includes('渲染队列'))
  await h.undo()
  await h.until(async () => (await jobs()).length === 0)
  h.step('undo removes the job', (await jobs()).length === 0)
  await page.keyboard.press(`${h.primary}+Shift+KeyZ`)
  await h.until(async () => (await jobs()).length === 1)
  h.step('redo adds it again', (await jobs()).length === 1)

  // 2. Its settings: frames 0-4 as a GIF at a quarter of the size, named and in a folder.
  await settings(first.id, () => short('短片'))
  await h.until(async () => (await job(first.id))?.output?.format === 'Gif' && (await job(first.id))?.span?.kind === 'custom')
  const set = await job(first.id)
  h.step('the job renders frames 0-4 as a GIF', set?.output?.format === 'Gif' && set?.span?.kind === 'custom' && Math.abs(set.span.end - 5 / comp.frameRate) < 1e-3, JSON.stringify(set?.span))
  h.step('at a quarter of the size', set?.width === Math.round(comp.width / 4), String(set?.width))
  h.step('into renders/短片.gif', set?.output?.output === 'renders/短片.gif', set?.output?.output)
  await h.until(async () => (await kit.getByTestId(`motion-queue-item-${first.id}-detail`).textContent()).includes('renders/短片.gif'))
  h.step('the panel shows the file, format and size', (await kit.getByTestId(`motion-queue-item-${first.id}-detail`).textContent()).includes('GIF'), await kit.getByTestId(`motion-queue-item-${first.id}-detail`).textContent())
  await h.undo()
  await h.until(async () => (await job(first.id))?.output?.format !== 'Gif')
  h.step('undo puts the output settings back', (await job(first.id))?.output?.format !== 'Gif' && (await job(first.id))?.span?.kind === 'custom')
  await h.undo()
  await h.until(async () => (await job(first.id))?.span?.kind !== 'custom')
  h.step('undo again the range', (await job(first.id))?.span?.kind !== 'custom')
  // Two seconds long: the run is still going while a layer is deleted below.
  await settings(first.id, async () => {
    await short('短片', '0:00:02:00')
    await kit.getByTestId('motion-job-size').selectOption('full')
  })
  await h.until(async () => (await job(first.id))?.output?.format === 'Gif')

  // 3. A preset: saved from one job's dialog, offered in the next.
  await kit.getByTestId(`motion-queue-item-${first.id}-settings`).click()
  await kit.getByTestId('motion-job-preset-name').fill('小 GIF')
  await kit.getByTestId('motion-job-preset-save').click()
  await kit.getByTestId('motion-job-dialog-cancel').click()

  // 4. Duplicate, reorder, remove; each undone.
  await kit.getByTestId(`motion-queue-item-${first.id}-duplicate`).click()
  await h.until(async () => (await jobs()).length === 2)
  const second = (await jobs()).find((each) => each.id !== first.id)
  h.step('Duplicate adds a copy below', (await jobs())[1]?.id === second?.id && second?.output?.format === 'Gif')
  await settings(second.id, async () => {
    const options = await kit.getByTestId('motion-job-preset').locator('option').allTextContents()
    h.step('the saved preset is offered', options.includes('小 GIF'), options.join(','))
    await kit.getByTestId('motion-job-preset').selectOption('小 GIF')
    await kit.getByTestId('motion-job-name').fill('第二')
  })
  await h.until(async () => (await job(second.id))?.output?.output === 'renders/第二.gif')
  h.step('the preset and a new name give renders/第二.gif', (await job(second.id))?.output?.output === 'renders/第二.gif', (await job(second.id))?.output?.output)
  await kit.getByTestId(`motion-queue-item-${second.id}-up`).click()
  await h.until(async () => (await jobs())[0]?.id === second.id)
  h.step('Up moves a job before the other', (await jobs())[0]?.id === second.id)
  await h.undo()
  await h.until(async () => (await jobs())[0]?.id === first.id)
  h.step('undo moves it back', (await jobs())[0]?.id === first.id)
  await kit.getByTestId(`motion-queue-item-${second.id}-remove`).click()
  await h.until(async () => (await jobs()).length === 1)
  h.step('Remove takes a job out', (await jobs()).length === 1)
  await h.undo()
  await h.until(async () => (await jobs()).length === 2)
  h.step('undo puts it back', (await jobs()).length === 2)

  // 5. Render both, one after another; a layer deleted by hand meanwhile stays deleted, and the jobs render the project as it was.
  const layersBefore = await h.layers()
  const victim = layersBefore[0]
  await kit.getByTestId('motion-queue-render').click()
  await h.until(async () => (await state(first.id)) === 'rendering', 100)
  // Its name (the row's middle may be a column's menu while the panels narrow the timeline).
  await kit.getByTestId(`motion-layer-${victim.id}`).getByText(victim.name, { exact: true }).first().click()
  await h.until(async () => (await h.layer(victim.id))?.selected)
  await page.keyboard.press('Delete')
  await h.until(async () => (await h.layers()).length === layersBefore.length - 1)
  const during = await state(first.id)
  h.step('the editor takes an edit while it renders', during === 'rendering' && (await h.layers()).length === layersBefore.length - 1, JSON.stringify({ before: layersBefore.length, now: (await h.layers()).length, victim, state: await state(first.id) }))
  await h.until(async () => (await state(second.id)) === 'done' || (await state(second.id)) === 'failed', 600)
  h.step('the first job is written', (await state(first.id)) === 'done', await status(first.id).textContent())
  h.step('then the second', (await state(second.id)) === 'done', await status(second.id).textContent())
  h.step('each says the file it wrote', (await status(second.id).textContent()).includes('renders/第二.gif'), await status(second.id).textContent())
  h.step('the files are GIFs beside the project', isGif(await file('renders/短片.gif')) && isGif(await file('renders/第二.gif')))
  h.step('the deleted layer stays deleted', (await h.layers()).length === layersBefore.length - 1)
  await h.undo()
  await h.until(async () => (await h.layers()).length === layersBefore.length)
  h.step('and undo brings it back', (await h.layers()).length === layersBefore.length)

  // 6. Stop: the long first job stops, the second still renders.
  await settings(first.id, async () => {
    await kit.getByTestId('motion-job-span').selectOption('comp')
    await kit.getByTestId('motion-job-size').selectOption('full')
    await kit.getByTestId('motion-job-name').fill('长片')
  })
  await h.until(async () => (await job(first.id))?.span?.kind === 'comp')
  await kit.getByTestId('motion-queue-render').click()
  await h.until(async () => (await state(first.id)) === 'rendering' && Number(await status(first.id).getAttribute('data-done')) > 0, 300)
  h.step('the panel shows frames done, time taken and time left', /第 \d+\/\d+ 帧 · 已用 \d+:\d\d · 还要/.test(await status(first.id).textContent()), await status(first.id).textContent())
  await kit.getByTestId('motion-queue-stop').click()
  await h.until(async () => (await state(second.id)) === 'done' && (await kit.getByTestId('motion-queue-render').count()) === 1, 600)
  h.step('Stop stops the job rendering', (await state(first.id)) === 'stopped' && (await file('renders/长片.gif')) == null, await status(first.id).textContent())
  h.step('and the next job still renders', (await state(second.id)) === 'done')

  // 7. Stop All: nothing after the job rendering runs.
  await kit.getByTestId('motion-queue-render').click()
  await h.until(async () => (await state(first.id)) === 'rendering', 300)
  await kit.getByTestId('motion-queue-stop-all').click()
  await h.until(async () => (await kit.getByTestId('motion-queue-render').count()) === 1, 600)
  h.step('Stop All stops the run', (await state(first.id)) === 'stopped' && (await state(second.id)) !== 'rendering', `${await state(first.id)} ${await state(second.id)}`)
  h.step('the panel’s buttons are back', (await kit.getByTestId(`motion-queue-item-${first.id}-remove`).isEnabled()))

  // 8. From the Project panel: a composition's menu adds a job.
  await kit.getByTestId('motion-project-toggle').click()
  await kit.getByTestId(`motion-item-${comp.id}`).click({ button: 'right' })
  await kit.getByTestId('motion-item-menu-queue').click()
  await h.until(async () => (await jobs()).length === 3)
  h.step('the Project panel adds a composition to the queue', (await jobs()).length === 3 && (await jobs())[2].comp === comp.id)
  const third = (await jobs())[2]
  h.step('starting from the settings used last (GIF)', third.output?.format === 'Gif', third.output?.format)
  await kit.getByTestId('motion-queue-close').click()
  h.step('the panel closes', (await kit.getByTestId('motion-queue-panel').count()) === 0)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
