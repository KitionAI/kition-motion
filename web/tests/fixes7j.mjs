// The fixes after the round 7j hand test, by hand, in a real browser through the development host: node tests/fixes7j.mjs [chromium|webkit]
// A Render Queue job whose folder is still the one the editor gave it takes the folder of its new
// format when the format changes (a movie switched to a PNG sequence gets its own `<comp>_序列`
// folder, unique among the queued jobs, and its frames land there; a sequence switched to GIF
// leaves that folder), while a folder the person typed never changes; Colorama's channel pickers
// read 相位取自 / 添加相位取自, apart from 添加相位 来源; the reworded group headings read as
// meant; with only 3D layers selected the Properties panel names the fourth rotation Z 轴旋转;
// the English status bar says "1 layer"; and the cache-full popover's Loop only the cached part is
// a check row whose tick shows while it is on. Results are read from the engine (`renderQueue.list`,
// `layer.tree`), the host's files and the elements' own attributes. Prints PASS or FAIL per step.
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
/** Opens effect card `index` (1-based) of the selected layer and answers it once its rows are drawn. */
async function card(index) {
  const toggle = kit.getByTestId(`motion-effect-toggle-${index}`)
  await toggle.waitFor()
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click()
  const box = kit.getByTestId(`motion-effect-card-${index}`)
  await h.until(async () => (await box.locator('.motion-prop').count()) > 0)
  return box
}
/** Opens the group at `path` (`effects/#2/tuning`) in the Properties panel, by a click on its heading; answers the heading. */
async function openGroup(path) {
  const toggle = kit.getByTestId(`motion-group-toggle-${path.replaceAll('/', '.')}`)
  await toggle.waitFor()
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click()
  await h.until(async () => (await toggle.getAttribute('aria-expanded')) === 'true')
  return ((await toggle.textContent()) ?? '').trim()
}
/** A setting's name in the Properties panel. */
async function settingName(path) {
  const name = kit.getByTestId(`motion-prop-name-${path.replaceAll('/', '.')}`)
  await name.waitFor()
  return ((await name.textContent()) ?? '').trim()
}
/** The paths of the groups under effect `index` (1-based) of `layer`, as the engine names them (`effects/#1/…`). */
async function groupPaths(layer, index) {
  const tree = await h.query('layer.tree', { layer, depth: 12 })
  const effects = (tree?.properties?.children ?? []).find((child) => child.path === 'effects')
  const out = []
  const walk = (node) => {
    for (const child of node.children ?? []) {
      if (!child.children) continue
      out.push(child.path)
      walk(child)
    }
  }
  const node = effects?.children?.[index - 1]
  if (node) walk(node)
  return out
}

try {
  await h.open()

  // 1. The Render Queue: a change of format chooses the folder again while it is the editor's.
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
  /** Picks `format` in the open job dialog and answers the folder box once it has settled. */
  const pickFormat = async (format) => {
    await kit.getByTestId('motion-job-format').selectOption(format)
    await h.until(async () => (await kit.getByTestId('motion-job-format').inputValue()) === format)
    let value = await folderField().inputValue()
    await h.until(async () => {
      const again = await folderField().inputValue()
      const same = again === value
      value = again
      return same
    })
    return value
  }

  await addJob()
  await h.until(async () => (await jobs()).length === 1)
  const movie = (await jobs())[0]
  const own = `${movie.compName}_序列`
  await h.until(async () => (await folderOf(movie.id).count()) === 1)
  h.step('(set-up) a new job is a movie (H.264) into the project’s folder', movie.output?.format === 'H264' && (await folderOf(movie.id).getAttribute('data-folder')) === '' && ((await folderOf(movie.id).textContent()) ?? '').includes('项目文件夹'), `${movie.output?.format} · ${await folderOf(movie.id).textContent()}`)
  await openSettings(movie.id)
  const toPng = await pickFormat('png')
  h.step(`switched to PNG sequence, the movie job’s folder box becomes ${own}`, toPng === own, JSON.stringify(toPng))
  // Two frames, at a quarter of the size.
  await kit.getByTestId('motion-job-span').selectOption('custom')
  await kit.getByTestId('motion-job-start').fill('0:00:00:00')
  await kit.getByTestId('motion-job-end').fill('0:00:00:02')
  await kit.getByTestId('motion-job-size').selectOption('quarter')
  await closeWith('motion-job-dialog-ok')
  await h.until(async () => (await folderOf(movie.id).getAttribute('data-folder')) === own && (await job(movie.id))?.output?.format === 'PngSequence')
  h.step(`OK: the job is a PNG sequence into ${own}/`, (await job(movie.id))?.output?.format === 'PngSequence' && (await folderOf(movie.id).getAttribute('data-folder')) === own && ((await folderOf(movie.id).textContent()) ?? '').includes(`${own}/`), await folderOf(movie.id).textContent())
  const filesBefore = new Set(await page.evaluate(() => [...window.devHost.files.keys()]))
  await kit.getByTestId('motion-queue-render').click()
  const state = (id) => kit.getByTestId(`motion-queue-item-${id}-status`).getAttribute('data-state')
  await h.until(async () => ['done', 'failed'].includes(await state(movie.id)), 900)
  const written = (await page.evaluate(() => [...window.devHost.files.keys()])).filter((path) => !filesBefore.has(path)).sort()
  const inside = written.filter((path) => path.startsWith(`${own}/`) && path.endsWith('.png'))
  h.step(`rendered (2 frames), the frames land in ${own}/ and nowhere else`, (await state(movie.id)) === 'done' && inside.length === 2 && written.length === 2, written.join(' | '))

  // The next new job is a PNG sequence again (the format used last), in its own numbered folder;
  // switched to GIF it leaves the sequence folder for the project's folder.
  await addJob()
  await h.until(async () => (await jobs()).length === 2)
  const sequence = (await jobs())[1]
  await h.until(async () => (await folderOf(sequence.id).getAttribute('data-folder')) === `${own} 2`)
  h.step(`(set-up) the next job is a PNG sequence into ${own} 2/`, sequence.output?.format === 'PngSequence' && (await folderOf(sequence.id).getAttribute('data-folder')) === `${own} 2`, await folderOf(sequence.id).textContent())
  await openSettings(sequence.id)
  const toGif = await pickFormat('gif')
  h.step('switched to GIF, the sequence job’s folder box empties (the project’s folder)', toGif === '', JSON.stringify(toGif))
  await closeWith('motion-job-dialog-ok')
  await h.until(async () => (await job(sequence.id))?.output?.format === 'Gif' && (await folderOf(sequence.id).getAttribute('data-folder')) === '')
  const gifOut = (await job(sequence.id))?.output?.output ?? ''
  h.step(`OK: the GIF is written as ${sequence.compName}.gif in the project’s folder, not in a _序列 folder`, (await folderOf(sequence.id).getAttribute('data-folder')) === '' && ((await folderOf(sequence.id).textContent()) ?? '').includes('项目文件夹') && !gifOut.includes('/') && gifOut.endsWith('.gif'), `${gifOut} · ${await folderOf(sequence.id).textContent()}`)
  // Back to a sequence (not saved): a folder of its own that no other job writes to.
  await openSettings(sequence.id)
  const back = await pickFormat('png')
  h.step(`switched back to PNG sequence it gets ${own} 2 (${own} is the first job’s)`, back === `${own} 2`, JSON.stringify(back))
  await closeWith('motion-job-dialog-cancel')

  // A folder the person typed is never changed by a change of format.
  await openSettings(sequence.id)
  await folderField().fill('mine')
  const typedPng = await pickFormat('png')
  const typedMovie = await pickFormat('h264')
  h.step('a typed folder (mine) stays through PNG sequence and H.264', typedPng === 'mine' && typedMovie === 'mine', `${typedPng} / ${typedMovie}`)
  await pickFormat('gif')
  await closeWith('motion-job-dialog-ok')
  await h.until(async () => (await folderOf(sequence.id).getAttribute('data-folder')) === 'mine')
  await openSettings(sequence.id)
  const keptPng = await pickFormat('png')
  h.step('opened again, the job’s own folder (mine) stays when it becomes a sequence', keptPng === 'mine', JSON.stringify(keptPng))
  await closeWith('motion-job-dialog-cancel')
  h.step('Cancel changes nothing', (await folderOf(sequence.id).getAttribute('data-folder')) === 'mine' && (await job(sequence.id))?.output?.format === 'Gif', `${await folderOf(sequence.id).getAttribute('data-folder')} · ${(await job(sequence.id))?.output?.format}`)

  // 2. Colorama's names: the channel pickers apart from the layer's source; the reworded headings.
  await reopen('zh-CN', 1440, 900)
  const list = await h.query('effect.list', {})
  const idOf = (name) => list.find((effect) => effect.name === name)?.id
  const applied = [
    ['ec.color.colorama', null],
    [idOf('Time Warp') ?? 'ec.time.timewarp', ['tuning', '微调']],
    ['ec.sim.cchair', ['hairfallMap', '毛发下垂映射']],
    ['ec.distort.warpstabilizer', ['edgeCropping', '生成边缘前裁剪']],
    ['ec.sim.ccparticleworld', ['extras', '其他设置']],
    ['ec.color.grade', ['refine', '选区优化']],
  ]
  await h.setUp([...applied.map(([effect]) => ({ command: 'effect.apply', params: { effect, layers: [SHAPE] } })), { command: 'edit.deselectAll', params: {} }])
  await selectOnly(SHAPE)
  await card(1)
  const inputPhase = await openGroup('effects/#1/inputPhase')
  const names = []
  for (const id of ['mode', 'addPhase', 'addPhaseFrom', 'addPhaseSource']) names.push(await settingName(`effects/#1/inputPhase/${id}`))
  h.step('Colorama ▸ 输入相位: 相位取自 / 添加相位 / 添加相位取自 / 添加相位 来源', inputPhase === '输入相位' && names.join(' / ') === '相位取自 / 添加相位 / 添加相位取自 / 添加相位 来源', `${inputPhase}: ${names.join(' / ')}`)
  const bare = names.map((name) => name.replace(/\s+/g, ''))
  h.step('no two of them are the same without their spaces', new Set(bare).size === bare.length, bare.join(' | '))
  for (let index = 2; index <= applied.length; index += 1) {
    const [effect, [group, expected]] = applied[index - 1]
    const paths = await groupPaths(SHAPE, index)
    const target = paths.find((path) => path.endsWith(`/${group}`))
    await card(index)
    let heading = '(no such group)'
    if (target) {
      // Its parents first (Warp Stabilizer ▸ 高级, Color ▸ HSL 辅助).
      const parts = target.split('/')
      for (let end = 3; end < parts.length; end += 1) await openGroup(parts.slice(0, end).join('/'))
      const toggle = kit.getByTestId(`motion-group-toggle-${target.replaceAll('/', '.')}`)
      await toggle.waitFor()
      heading = ((await toggle.textContent()) ?? '').trim()
    }
    h.step(`${effect}: the ${group} heading reads ${expected}`, heading === expected, `${target}: ${heading}`)
  }

  // 3. Two 3D layers selected (by clicks): Properties names the fourth rotation Z 轴旋转; with one
  // of them 2D again, 旋转 (and no X / Y rotation or orientation rows).
  await h.setUp([
    { command: 'effect.removeAll', params: { layers: [SHAPE] } },
    { command: 'layer.setSwitch', params: { layers: [SHAPE, NIGHT], switch: 'threeD', value: true } },
    { command: 'edit.deselectAll', params: {} },
  ])
  await selectOnly(SHAPE)
  await page.keyboard.down(h.primary)
  await rowName(NIGHT).click()
  await page.keyboard.up(h.primary)
  await h.until(async () => (await h.layer(SHAPE))?.selected && (await h.layer(NIGHT))?.selected)
  const multiName = (id) => kit.getByTestId(`motion-multi-${id}`).locator('.motion-prop-name')
  await h.until(async () => (await multiName('rotation').count()) === 1 && (await multiName('rotationX').count()) === 1)
  const rows3d = await kit.locator('[data-testid^="motion-multi-"].motion-prop .motion-prop-name').allTextContents()
  h.step('two 3D layers: 方向 / X 轴旋转 / Y 轴旋转 / Z 轴旋转 in the multi-layer Transform', (await multiName('rotation').textContent()) === 'Z 轴旋转' && ['方向', 'X 轴旋转', 'Y 轴旋转', 'Z 轴旋转'].every((name) => rows3d.includes(name)), rows3d.join(' | '))
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [NIGHT], switch: 'threeD', value: false } }])
  await h.until(async () => (await multiName('rotationX').count()) === 0 && (await multiName('rotation').textContent()) === '旋转')
  h.step('one of them 2D again: 旋转, without the 3D rows', (await multiName('rotation').textContent()) === '旋转' && (await multiName('rotationX').count()) === 0 && (await multiName('orientation').count()) === 0, await multiName('rotation').textContent())

  // 4. English: the status bar counts "1 layer", "2 layers"; the multi-layer rotation reads Z Rotation.
  await reopen('en', 1280, 800)
  await h.setUp([
    { command: 'layer.setSwitch', params: { layers: [SHAPE, NIGHT], switch: 'threeD', value: true } },
    { command: 'edit.deselectAll', params: {} },
  ])
  await selectOnly(SHAPE)
  await page.keyboard.down(h.primary)
  await rowName(NIGHT).click()
  await page.keyboard.up(h.primary)
  await h.until(async () => (await multiName('rotation').count()) === 1 && (await multiName('rotation').textContent()) === 'Z Rotation')
  h.step('en: two 3D layers selected, the rotation row reads Z Rotation', (await multiName('rotation').textContent()) === 'Z Rotation', await multiName('rotation').textContent())
  await kit.getByTestId('motion-menu-Composition').click()
  await kit.locator('[role=menu] [data-testid="motion-cmd-comp.new"]').first().click()
  await page.waitForTimeout(400)
  await page.keyboard.press('Enter')
  await h.until(async () => (await h.details()).comp?.width === 1920 && (await h.layers()).length === 0)
  const summary = kit.locator('.motion-summary')
  await h.setUp([{ command: 'layer.newSolid', params: { name: 'One', color: '#2a6fdb' } }])
  await h.until(async () => ((await summary.textContent()) ?? '').endsWith(', 1 layer'))
  h.step('en: a composition with one layer says "1 layer"', ((await summary.textContent()) ?? '').endsWith(', 1 layer'), await summary.textContent())
  await h.setUp([{ command: 'layer.newSolid', params: { name: 'Two', color: '#db6f2a' } }])
  await h.until(async () => ((await summary.textContent()) ?? '').endsWith(', 2 layers'))
  h.step('and with two, "2 layers"', ((await summary.textContent()) ?? '').endsWith(', 2 layers'), await summary.textContent())

  // 5. A full cache's ▾: Loop only the cached part is a check row, ticked while on.
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
  await cacheButton.click()
  await h.until(async () => (await cacheButton.getAttribute('data-full')) === 'true', 3000)
  h.step('(set-up) the cache fills up', (await cacheButton.getAttribute('data-full')) === 'true', await cacheButton.textContent())
  await kit.getByTestId('motion-cache-full-menu').click()
  const loopCached = kit.getByTestId('motion-cache-loop-cached')
  const loopRow = kit.getByTestId('motion-cache-loop-cached-row')
  await h.until(async () => (await loopCached.count()) === 1)
  /** The row's state, its tick and its pixels, with the pointer away from it (no hover). */
  const look = async () => {
    await page.mouse.move(5, 5)
    await page.waitForTimeout(200)
    const shot = async () => (await loopCached.screenshot()).toString('base64')
    let pixels = await shot()
    // Read again until two pictures agree (the popover settled).
    await h.until(async () => {
      const again = await shot()
      const same = again === pixels
      pixels = again
      return same
    })
    return {
      role: await loopCached.getAttribute('role'),
      checked: await loopCached.getAttribute('aria-checked'),
      ticks: await loopCached.locator('svg').count(),
      text: ((await loopCached.textContent()) ?? '').trim(),
      pixels,
    }
  }
  const off = await look()
  h.step('off: a check row (menuitemcheckbox), aria-checked false, no tick', off.role === 'menuitemcheckbox' && off.checked === 'false' && off.ticks === 0 && off.text === '只循环已缓存的部分', JSON.stringify({ ...off, pixels: undefined }))
  await loopCached.click()
  await h.until(async () => (await loopCached.getAttribute('aria-checked')) === 'true' && (await loopRow.getAttribute('data-area')) !== null)
  const on = await look()
  h.step('on (a click): aria-checked true, a tick shows, the loop is the cached frames', on.checked === 'true' && on.ticks === 1 && /^0\.000 \d+\.\d{3}$/.test((await loopRow.getAttribute('data-area')) ?? ''), `${JSON.stringify({ ...on, pixels: undefined })} · ${await loopRow.getAttribute('data-area')}`)
  h.step('on and off look different (the row’s pixels differ)', on.pixels !== off.pixels)
  await loopCached.click()
  await h.until(async () => (await loopCached.getAttribute('aria-checked')) === 'false')
  const offAgain = await look()
  h.step('off again (a click): no tick, the same picture as before', offAgain.checked === 'false' && offAgain.ticks === 0 && offAgain.pixels === off.pixels && (await loopRow.getAttribute('data-area')) === null, JSON.stringify({ ...offAgain, pixels: undefined }))
  await page.keyboard.press('Escape')
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
