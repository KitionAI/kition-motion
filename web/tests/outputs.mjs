// Output formats by hand, in a real browser through the development host: node tests/outputs.mjs [chromium|webkit]
// Export… writes a PNG sequence with transparency into a folder (a numbered file per frame), a
// JPEG sequence, the sound alone as WAV and the current frame as one PNG; a Render Queue job
// writes a PNG sequence too. Results are read from the host's files. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
/** The host's files under `prefix`, sorted, with their first bytes. */
const files = (prefix) =>
  page.evaluate((folder) => {
    const out = []
    for (const [path, entry] of window.devHost.files.entries()) if (path.startsWith(folder)) out.push([path, [...new Uint8Array(entry.bytes).slice(0, 32)]])
    return out.sort((a, b) => (a[0] < b[0] ? -1 : 1))
  }, prefix)
const dialog = () => kit.getByTestId('motion-export-dialog')

/** Export… with `format`, the size and the rest set by `fill`, rendered; waits for the dialog to close. */
async function exportAs(format, fill = async () => {}) {
  await kit.getByTestId('motion-render').click()
  await dialog().waitFor()
  await kit.getByTestId('motion-export-format').selectOption(format)
  await fill()
  await kit.getByTestId('motion-export-dialog-ok').click()
  await h.until(async () => (await dialog().count()) === 0, 1200)
}

try {
  await h.open()
  const comp = (await h.details()).comp
  const frames = Math.round(comp.duration * comp.frameRate)

  // 1. A PNG sequence with transparency, into a folder of its own.
  await kit.getByTestId('motion-render').click()
  await dialog().waitFor()
  await kit.getByTestId('motion-export-format').selectOption('png')
  const options = await kit.getByTestId('motion-export-format').locator('option').allTextContents()
  h.step('Export offers PNG and JPEG sequences, WAV and the current frame, in Chinese', ['PNG 序列', 'JPEG 序列', 'WAV（只有声音）', '当前帧（PNG）'].every((label) => options.includes(label)), options.join(' | '))
  h.step('a PNG sequence keeps transparency unless told not to', (await kit.getByTestId('motion-export-alpha').locator('input').isChecked().catch(() => kit.getByTestId('motion-export-alpha').isChecked())) === true)
  await kit.getByTestId('motion-export-folder').fill('帧')
  await kit.getByTestId('motion-export-size').selectOption('half')
  await kit.getByTestId('motion-export-dialog-ok').click()
  await h.until(async () => (await dialog().count()) === 0, 1200)
  let pngs = await files('帧/')
  h.step('one numbered PNG per frame in the folder', pngs.length === frames && pngs[0][0] === `帧/${comp.name}_00000.png`, `${pngs.length} of ${frames}: ${pngs[0]?.[0]}`)
  h.step('with an alpha channel (RGBA)', pngs.length > 0 && pngs.every(([, bytes]) => String.fromCharCode(...bytes.slice(1, 4)) === 'PNG' && bytes[25] === 6))

  // 2. A JPEG sequence.
  await exportAs('jpeg', async () => {
    h.step('a JPEG sequence has no transparency switch', (await kit.getByTestId('motion-export-alpha').count()) === 0)
    await kit.getByTestId('motion-export-folder').fill('jpg')
    await kit.getByTestId('motion-export-size').selectOption('half')
  })
  const jpegs = await files('jpg/')
  h.step('one JPEG per frame', jpegs.length === frames && jpegs.every(([, bytes]) => bytes[0] === 0xff && bytes[1] === 0xd8), String(jpegs.length))

  // 3. The sound alone.
  await exportAs('wav', async () => {
    h.step('WAV asks no picture size or frame rate', (await kit.getByTestId('motion-export-size').count()) === 0 && (await kit.getByTestId('motion-export-rate').count()) === 0)
  })
  const wav = await files(`${comp.name}.wav`)
  h.step('WAV writes the sound beside the project', wav.length === 1 && String.fromCharCode(...wav[0][1].slice(0, 4)) === 'RIFF', JSON.stringify(wav.map(([path]) => path)))

  // 4. The current frame as one PNG.
  const before = (await files('')).length
  await exportAs('frame')
  await h.until(async () => (await files('')).length === before + 1)
  const frame = (await files('')).find(([path]) => path.startsWith(`${comp.name} `) && path.endsWith('.png'))
  h.step('the current frame is saved as a PNG', frame != null && String.fromCharCode(...frame[1].slice(1, 4)) === 'PNG', frame?.[0])

  // 5. A Render Queue job writes a PNG sequence (five frames, a quarter of the size).
  await h.setUp([{ command: 'renderQueue.add', params: { comp: comp.id, timeSpan: 'custom', start: 0, end: 5 / comp.frameRate, format: 'gif', output: 'x.gif' } }])
  const job = (await h.query('renderQueue.list', {})).items[0]
  await kit.getByTestId('motion-menu-File').click()
  await kit.locator('[role=menu] [data-testid="motion-control-render-queue"]').first().click()
  await kit.getByTestId('motion-queue-panel').waitFor()
  await kit.getByTestId(`motion-queue-item-${job.id}-settings`).click()
  await kit.getByTestId('motion-job-dialog').waitFor()
  await kit.getByTestId('motion-job-format').selectOption('png')
  h.step('the job’s PNG keeps transparency', (await kit.getByTestId('motion-job-alpha').count()) === 1)
  await kit.getByTestId('motion-job-size').selectOption('quarter')
  await kit.getByTestId('motion-job-name').fill('镜头')
  await kit.getByTestId('motion-job-folder').fill('队列')
  await kit.getByTestId('motion-job-dialog-ok').click()
  await h.until(async () => (await h.query('renderQueue.list', {})).items[0]?.output?.format === 'PngSequence')
  h.step('the job is a PNG sequence named with the frame number', (await h.query('renderQueue.list', {})).items[0]?.output?.output === '队列/镜头_[#####].png', (await h.query('renderQueue.list', {})).items[0]?.output?.output)
  await kit.getByTestId('motion-queue-render').click()
  const status = kit.getByTestId(`motion-queue-item-${job.id}-status`)
  await h.until(async () => (await status.count()) === 1 && (await status.getAttribute('data-state')) === 'done', 600)
  pngs = await files('队列/')
  h.step('Render writes a PNG per frame into its folder', pngs.length === 5 && pngs[4][0] === '队列/镜头_00004.png', pngs.map(([path]) => path).join(' '))
  h.step('and the job says how many files', (await status.textContent()).includes('5 个文件'), await status.textContent())
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
