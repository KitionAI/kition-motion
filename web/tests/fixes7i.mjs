// The fixes after the round 7i hand test, by hand, in a real browser through the development host: node tests/fixes7i.mjs [chromium|webkit]
// Every layer setting's source choice, wherever it sits in an effect (inside a group, or declared
// by the effect itself), offers Source / Masks / Effects & Masks and reads "<setting> 来源"; no
// effect setting anywhere offers the bare digits 0 to 9; every group heading inside an effect card
// reads in Chinese; the misleading setting and option names say what they do; a 3D layer's fourth
// rotation row reads Z 轴旋转; a new Render Queue job that is an image sequence writes into a folder
// of its own named after the composition (and its frames land there); and a full preview cache
// keeps its actions behind one ▾, so the status bar's zoom controls stay in view at 1280 × 800 in
// English and 1440 × 900 in Chinese, with the English tooltips worded plainly. Results are read
// from the engine (`prop.get`, `layer.tree`, `effect.list`, `renderQueue.list`), the host's files
// and the elements' own attributes and boxes. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const SOURCE_ZH = ['源', '蒙版', '效果和蒙版']

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
/** Opens the group at `path` (`effects/#2/blurMap`) in the Properties panel, by a click on its heading. */
async function openGroup(path) {
  const toggle = kit.getByTestId(`motion-group-toggle-${path.replaceAll('/', '.')}`)
  await toggle.waitFor()
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click()
  await h.until(async () => (await toggle.getAttribute('aria-expanded')) === 'true')
  return ((await toggle.textContent()) ?? '').trim()
}
/** A drop-down's option texts and the chosen one, read again if the card was redrawn while being read. */
async function readSelect(select) {
  let out = { options: [], chosen: '' }
  await h.until(async () => {
    out = await select.evaluate((element) => ({ options: [...element.options].map((option) => option.textContent ?? ''), chosen: element.selectedOptions[0]?.textContent ?? '' }))
    return out.options.length > 0
  })
  return out
}
/** A setting's row in the Properties panel: its name and (for a drop-down) its options. */
async function setting(path) {
  const id = path.replaceAll('/', '.')
  const name = kit.getByTestId(`motion-prop-name-${id}`)
  await name.waitFor()
  const select = kit.getByTestId(`motion-value-${id}`)
  const isSelect = (await select.count()) === 1 && (await select.evaluate((element) => element.tagName === 'SELECT'))
  return { name: ((await name.textContent()) ?? '').trim(), ...(isSelect ? await readSelect(select) : { options: [], chosen: '' }) }
}
const same = (a, b) => a.length === b.length && a.every((each, at) => each === b[at])
const digitsOnly = (options) => options.length > 0 && options.every((option) => /^\d+$/.test(option))

try {
  await h.open()

  // 1. Source drop-downs in the Properties panel: one the effect declares itself (Displacement Map,
  // default 源), and ones inside groups (Camera Lens Blur ▸ 模糊图, Caustics ▸ 底部).
  await h.setUp([
    { command: 'effect.apply', params: { effect: 'ec.distort.displacementmap', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.blur.cameralens', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.sim.caustics', layers: [SHAPE] } },
    { command: 'edit.deselectAll', params: {} },
  ])
  await selectOnly(SHAPE)
  await card(1)
  const displacement = await setting('effects/#1/displacementMapLayerSource')
  h.step('Displacement Map ▸ 置换图层 来源 offers 源 / 蒙版 / 效果和蒙版 (its own default 源)', displacement.name === '置换图层 来源' && same(displacement.options, SOURCE_ZH) && displacement.chosen === '源', JSON.stringify(displacement))
  await kit.getByTestId('motion-value-effects.#1.displacementMapLayerSource').selectOption({ label: '蒙版' })
  const sourceValue = async () => (await h.query('prop.get', { layer: SHAPE, path: 'effects/#1/displacementMapLayerSource' })).value
  await h.until(async () => (await sourceValue()) === 1)
  h.step('choosing 蒙版 there stores the value 1', (await sourceValue()) === 1, JSON.stringify(await sourceValue()))
  await card(2)
  const blurMap = await openGroup('effects/#2/blurMap')
  const lensSource = await setting('effects/#2/blurMap/blurMapLayerSource')
  h.step('Camera Lens Blur ▸ 模糊图 (a group) ▸ 图层 来源 offers the three, 效果和蒙版 chosen', blurMap === '模糊图' && lensSource.name === '图层 来源' && same(lensSource.options, SOURCE_ZH) && lensSource.chosen === '效果和蒙版', `${blurMap}: ${JSON.stringify(lensSource)}`)
  const placement = await setting('effects/#2/blurMap/placement')
  h.step('and 模糊图位置 reads 居中 / 伸缩以适合', placement.name === '模糊图位置' && same(placement.options, ['居中', '伸缩以适合']), JSON.stringify(placement))
  await card(3)
  const bottom = await openGroup('effects/#3/bottom')
  const bottomSource = await setting('effects/#3/bottom/bottomSource')
  h.step('Caustics ▸ 底部 ▸ 底部 来源 offers the three', bottom === '底部' && bottomSource.name === '底部 来源' && same(bottomSource.options, SOURCE_ZH), `${bottom}: ${JSON.stringify(bottomSource)}`)
  const anyDigits = await kit.locator('.motion-effect-card select').evaluateAll((selects) => selects.filter((select) => select.options.length > 0 && [...select.options].every((option) => /^\d+$/.test(option.textContent ?? ''))).map((select) => select.getAttribute('aria-label')))
  h.step('no drop-down in these cards lists bare digits', anyDigits.length === 0, anyDigits.join(' | '))

  // 2. Every effect, scanned: each drop-down (wherever it sits) has named options, each group
  // heading reads in Chinese (but for abbreviations), each layer setting's source reads
  // "<setting> 来源" / "<setting> Source". The effects go on a hidden layer in batches; the names
  // and options come from the editor's own `propMeta` and `nodeName` over the engine's tree.
  const list = await h.query('effect.list', {})
  await h.setUp([
    { command: 'effect.removeAll', params: { layers: [SHAPE] } },
    { command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'video', value: false } },
  ])
  const trees = []
  for (let at = 0; at < list.length; at += 25) {
    await h.setUp(list.slice(at, at + 25).map((effect) => ({ command: 'effect.apply', params: { effect: effect.id, layers: [SHAPE] } })))
    const tree = await h.query('layer.tree', { layer: SHAPE, depth: 12 })
    const effects = (tree?.properties?.children ?? []).find((child) => child.path === 'effects')
    trees.push(...(effects?.children ?? []))
    await h.setUp([{ command: 'effect.removeAll', params: { layers: [SHAPE] } }])
  }
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'video', value: true } }])
  h.step(`(set-up) every effect applied once (${list.length})`, trees.length === list.length, `${trees.length} of ${list.length}`)
  const scan = await kit.locator('body').evaluate(
    async (_, [effects, list]) => {
      const { PHRASES, CATALOGS } = await import('/src/strings.ts')
      const { LEARNED_NAMES_ZH, shapedZh } = await import('/src/effectNames.ts')
      const { propMeta, nodeName } = await import('/src/properties/meta.ts')
      const zh = PHRASES['zh-CN']
      const shown = (text) => LEARNED_NAMES_ZH.get(text) ?? (zh[text] && zh[text] !== text ? zh[text] : (shapedZh(text) ?? text))
      // A numbered name the phrases lack reads as its translated base and the number (`main.tsx`).
      const named = (text) => {
        const whole = shown(text)
        const match = whole === text ? /^(.*\S) (\d+)$/.exec(text) : null
        return match && shown(match[1]) !== match[1] ? `${shown(match[1])} ${match[2]}` : whole
      }
      const strings = (locale) => (key, values = {}) => (CATALOGS[locale][key] ?? key).replace(/\{(\w+)\}/g, (all, name) => String(values[name] ?? all))
      const [tZh, tEn] = [strings('zh-CN'), strings('en-US')]
      const byId = new Map(list.map((effect) => [effect.id, effect]))
      const out = { enums: 0, digits: [], groups: new Set(), headings: [], sources: 0, badSources: [], names: {} }
      const walk = (node, effect, parent) => {
        for (const child of node.children ?? []) {
          if (child.children) {
            out.groups.add(child.name)
            const name = nodeName(child, effect, null, false, named, tZh)
            if (name === child.name) out.headings.push(child.name)
            out.names[`${effect.id}/${child.path.replace(/^effects\/[^/]+\//, '')}`] = name
            walk(child, effect, child)
            continue
          }
          const meta = propMeta(child, parent, effect)
          out.names[`${effect.id}/${child.path.replace(/^effects\/[^/]+\//, '')}`] = nodeName(child, effect, meta, false, named, tZh)
          for (const option of meta?.options ?? []) out.names[`${effect.id}/${child.path.replace(/^effects\/[^/]+\//, '')}=${option.label}`] = option.key ? tZh(option.key) : named(option.label)
          if (child.type !== 'enum') continue
          out.enums += 1
          // What the drop-down would list: its options, else the digits PropertyValue falls back to.
          if (!meta?.options?.length) out.digits.push(`${effect.id}/${child.path}`)
          if (meta?.source) {
            out.sources += 1
            const [inZh, inEn] = [nodeName(child, effect, meta, false, named, tZh), nodeName(child, effect, meta, false, (text) => text, tEn)]
            if (!inZh.endsWith(' 来源') || /的源|图层源/.test(inZh) || !inEn.endsWith(' Source')) out.badSources.push(`${effect.id}/${child.path}: ${inZh} / ${inEn}`)
          }
        }
      }
      for (const node of effects) {
        const effect = byId.get(node.effect)
        if (effect) walk(node, effect, node)
      }
      return { ...out, groups: out.groups.size }
    },
    [trees, list],
  )
  const HEADINGS_ALLOWED = new Set(['Alpha', 'RGB'])
  const headingsLeft = [...new Set(scan.headings)].filter((name) => !HEADINGS_ALLOWED.has(name))
  h.step(`no drop-down of any effect lists bare digits (${scan.enums} drop-downs in ${trees.length} effects)`, scan.digits.length === 0 && scan.enums > 500, scan.digits.slice(0, 20).join(' | '))
  h.step(`every layer setting’s source reads "<setting> 来源" / "<setting> Source" (${scan.sources})`, scan.sources >= 70 && scan.badSources.length === 0, scan.badSources.slice(0, 10).join(' | '))
  h.step(`every group heading inside an effect card reads in Chinese, but for abbreviations (${scan.groups} headings)`, scan.groups > 150 && headingsLeft.length === 0, headingsLeft.slice(0, 20).join(' | '))
  // The reworded names, as the editor names them in their effects.
  const WORDS = {
    'ec.color.grade/basicCorrection/tone': '影调',
    'ec.color.grade/creative/look': '调色风格',
    'ec.color.ociolook/look': '外观',
    'ec.color.grade/hslSecondary': 'HSL 辅助',
    'ec.3d.depthoffield/focalBias': '焦外过渡',
    'ec.3d.fog3d/layerContribution': '渐变图层影响',
    'ec.blur.cameralens/blurMap/placement': '模糊图位置',
    'ec.blur.cameralens/blurMap/placement=Center': '居中',
    'ec.blur.cameralens/blurMap/placement=Stretch': '伸缩以适合',
    'ec.distort.ccbendit/renderPrestart': '起点之前',
    'ec.distort.ccbendit/renderPrestart=None': '不显示',
    'ec.distort.ccbendit/renderPrestart=Static': '保持原样',
    'ec.generate.cclightsweep/lightReceptionMode': '光带合成方式',
    'ec.generate.advancedlightning/expertSettings/coreDrain': '主干衰减',
    'ec.generate.advancedlightning/conductivityState': '传导状态（随机形状）',
    'ec.generate.advancedlightning/lightningType=Direction': '定向',
    'ec.generate.advancedlightning/direction': '方向',
    'ec.sim.foam/universeSize': '模拟区域大小',
    'ec.sim.foam/flowMap/flowMapFits=Universe': '模拟区域',
    'ec.sim.particleplayground/gravity/affects/olderYoungerThan': '年龄大于/小于（秒）',
    'ec.utility.colorprofileconverter/intent': '渲染意图',
    'ec.distort.bezierwarp/topLeftTangent': '上边左切点',
    'ec.distort.bezierwarp/topRightTangent': '上边右切点',
    'ec.distort.bezierwarp/rightTopTangent': '右边上切点',
    'ec.distort.bezierwarp/rightBottomTangent': '右边下切点',
    'ec.distort.bezierwarp/bottomRightTangent': '下边右切点',
    'ec.distort.bezierwarp/bottomLeftTangent': '下边左切点',
    'ec.distort.bezierwarp/leftBottomTangent': '左边下切点',
    'ec.distort.bezierwarp/leftTopTangent': '左边上切点',
    'ec.generate.radiowaves/waveMotion/orientation': '朝向',
    'ec.generate.radiowaves/waveMotion/direction': '移动方向',
    'ec.channel.cccomposite/compositeOriginal=In Front': '在上面',
    'ec.stylize.glow/operation=On Top': '在上面',
    'ec.blur.directional/direction': '方向',
  }
  const wrong = Object.entries(WORDS).filter(([key, words]) => scan.names[key] !== words).map(([key, words]) => `${key}: ${scan.names[key]} (not ${words})`)
  h.step(`the reworded names read as meant in their effects (${Object.keys(WORDS).length}), others keep theirs`, wrong.length === 0, wrong.join(' | '))

  // 3. The reworded names in the cards themselves.
  await h.setUp([
    { command: 'effect.apply', params: { effect: 'ec.color.grade', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.generate.radiowaves', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.generate.advancedlightning', layers: [SHAPE] } },
    { command: 'effect.apply', params: { effect: 'ec.color.curves', layers: [SHAPE] } },
    { command: 'edit.deselectAll', params: {} },
  ])
  await selectOnly(SHAPE)
  const color = await card(1)
  const headings = await color.locator('.motion-prop-group-toggle').allTextContents()
  h.step('Color’s groups read 基本校正 / 创意 / 曲线 / 色轮 / HSL 辅助 / 晕影', ['基本校正', '创意', '曲线', '色轮', 'HSL 辅助', '晕影'].every((name) => headings.includes(name)) && !headings.some((name) => /[A-Za-z]{3,}/.test(name.replace('HSL', ''))), headings.join(' | '))
  await openGroup('effects/#1/basicCorrection')
  const inner = await color.locator('.motion-prop-group-toggle').allTextContents()
  h.step('and inside 基本校正: 白平衡 and 影调 (not 音调)', inner.includes('白平衡') && inner.includes('影调') && !inner.includes('音调'), inner.join(' | '))
  await openGroup('effects/#1/creative')
  h.step('创意 ▸ 调色风格', (await setting('effects/#1/creative/look')).name === '调色风格')
  await card(2)
  const waveMotion = await openGroup('effects/#2/waveMotion')
  const [orientation, direction] = [await setting('effects/#2/waveMotion/orientation'), await setting('effects/#2/waveMotion/direction')]
  h.step('Radio Waves ▸ 波动: 朝向 and 移动方向', waveMotion === '波动' && orientation.name === '朝向' && direction.name === '移动方向', `${waveMotion}: ${orientation.name} / ${direction.name}`)
  await card(3)
  const lightningType = await setting('effects/#3/lightningType')
  h.step('Advanced Lightning ▸ 闪电类型 starts with 定向, beside the setting 方向', lightningType.options[0] === '定向' && (await setting('effects/#3/direction')).name === '方向', lightningType.options.join(' | '))
  const expert = await openGroup('effects/#3/expertSettings')
  h.step('its 专家设置 ▸ 主干衰减; 传导状态（随机形状）', expert === '专家设置' && (await setting('effects/#3/expertSettings/coreDrain')).name === '主干衰减' && (await setting('effects/#3/conductivityState')).name === '传导状态（随机形状）')
  await card(4)
  const channel = await setting('effects/#4/channel')
  h.step('Curves ▸ 通道 names its curves, not digits', !digitsOnly(channel.options) && channel.options.length === 5 && channel.options[0] === 'RGB', channel.options.join(' | '))

  // 4. R on a 3D layer: the fourth row reads Z 轴旋转 in the timeline and in Properties; 旋转 on a 2D layer.
  await h.setUp([{ command: 'effect.removeAll', params: { layers: [SHAPE] } }, { command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await selectOnly(SHAPE)
  await h.focusEditor()
  await page.keyboard.press('KeyR')
  const timelineRotation = kit.getByTestId(`motion-prop-row-${SHAPE}-transform.rotation`).locator('.motion-row-name')
  const panelRotation = kit.getByTestId('motion-prop-name-rotation')
  await h.until(async () => (await timelineRotation.count()) === 1 && (await timelineRotation.textContent()) === 'Z 轴旋转')
  h.step('R on a 3D layer: the timeline’s fourth row reads Z 轴旋转', (await timelineRotation.textContent()) === 'Z 轴旋转', await timelineRotation.textContent())
  await h.until(async () => (await panelRotation.textContent()) === 'Z 轴旋转')
  h.step('and Properties too', (await panelRotation.textContent()) === 'Z 轴旋转', await panelRotation.textContent())
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: false } }])
  await h.until(async () => (await timelineRotation.textContent()) === '旋转' && (await panelRotation.textContent()) === '旋转')
  h.step('a 2D layer keeps 旋转', (await timelineRotation.textContent()) === '旋转' && (await panelRotation.textContent()) === '旋转', `${await timelineRotation.textContent()} / ${await panelRotation.textContent()}`)

  // 5. The Render Queue: a new image-sequence job writes into its own folder, named after the
  // composition (numbered for a second one); rendered, its frames land there and nowhere else.
  await reopen('zh-CN', 1440, 900)
  const jobs = async () => (await h.query('renderQueue.list', {})).items ?? []
  const addJob = async () => {
    await kit.getByTestId('motion-menu-File').click()
    await kit.locator('[data-kit-ui-id="menu:File/Export"]').hover()
    await kit.locator('[role=menu] [data-testid="motion-control-add-to-render-queue"]').first().click()
  }
  const folderOf = (id) => kit.getByTestId(`motion-queue-item-${id}-folder`)
  const dialog = async (id, fill) => {
    await kit.getByTestId(`motion-queue-item-${id}-settings`).click()
    await kit.getByTestId('motion-job-dialog').waitFor()
    await fill()
    await kit.getByTestId('motion-job-dialog-ok').click()
    await h.until(async () => (await kit.getByTestId('motion-job-dialog').count()) === 0)
  }
  await addJob()
  await h.until(async () => (await jobs()).length === 1)
  const first = (await jobs())[0]
  await dialog(first.id, async () => {
    await kit.getByTestId('motion-job-format').selectOption('png')
    await kit.getByTestId('motion-job-folder').fill('job-png')
  })
  await h.until(async () => (await folderOf(first.id).getAttribute('data-folder')) === 'job-png')
  const png = (await jobs())[0].output?.format
  await addJob()
  await h.until(async () => (await jobs()).length === 2)
  const second = (await jobs())[1]
  const own = `${second.compName}_序列`
  await h.until(async () => (await folderOf(second.id).getAttribute('data-folder')) === own)
  h.step(`the next job keeps the PNG sequence and writes into ${own}/ (not the project’s folder, not job-png/)`, second.output?.format === png && (await folderOf(second.id).getAttribute('data-folder')) === own && ((await folderOf(second.id).textContent()) ?? '').includes(`${own}/`), `${second.output?.format} · ${await folderOf(second.id).textContent()}`)
  await addJob()
  await h.until(async () => (await jobs()).length === 3)
  const third = (await jobs())[2]
  await h.until(async () => (await folderOf(third.id).count()) === 1)
  h.step(`a third one gets a folder of its own too (${own} 2)`, (await folderOf(third.id).getAttribute('data-folder')) === `${own} 2`, await folderOf(third.id).textContent())
  // Render the second only, three frames at a quarter of the size, to its default folder.
  await dialog(second.id, async () => {
    await kit.getByTestId('motion-job-span').selectOption('custom')
    await kit.getByTestId('motion-job-start').fill('0:00:00:00')
    await kit.getByTestId('motion-job-end').fill('0:00:00:03')
    await kit.getByTestId('motion-job-size').selectOption('quarter')
  })
  for (const job of [first, third]) {
    await kit.getByTestId(`motion-queue-item-${job.id}-on`).click()
    await h.until(async () => (await jobs()).find((each) => each.id === job.id)?.render === false)
  }
  const filesBefore = new Set(await page.evaluate(() => [...window.devHost.files.keys()]))
  await kit.getByTestId('motion-queue-render').click()
  const state = (id) => kit.getByTestId(`motion-queue-item-${id}-status`).getAttribute('data-state')
  await h.until(async () => ['done', 'failed'].includes(await state(second.id)), 900)
  const written = (await page.evaluate(() => [...window.devHost.files.keys()])).filter((path) => !filesBefore.has(path))
  const inside = written.filter((path) => path.startsWith(`${own}/`) && path.endsWith('.png'))
  h.step(`rendered, its frames are written into ${own}/ and nowhere else`, (await state(second.id)) === 'done' && inside.length >= 3 && inside.length === written.length, written.join(' | '))

  // 6. The status bar with a full cache: one ▾ after 缓存已满 N/M holds the actions; the zoom
  // controls stay whole inside the bar, overlapping nothing, at 1280 × 800 in English and
  // 1440 × 900 in Chinese, before caching, while it caches, full, and full with the ▾ open.
  await page.context().addInitScript(() => Object.defineProperty(Navigator.prototype, 'deviceMemory', { get: () => undefined, configurable: true }))
  /** The status bar's own children and the zoom controls' parts, laid out once two reads agree. */
  const layout = async () => {
    const read = () =>
      kit.locator('.kui-status-bar').evaluate((bar) => {
        const box = (element) => {
          const rect = element.getBoundingClientRect()
          return { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom, width: rect.width }
        }
        const zoom = bar.querySelector('.kui-zoom')
        const summary = bar.querySelector('.motion-summary')
        return {
          bar: box(bar),
          zoom: zoom ? box(zoom) : null,
          zoomParts: zoom ? [...zoom.children].map((part) => ({ name: part.className || part.tagName, ...box(part) })) : [],
          others: [...bar.children].filter((child) => child !== zoom && !child.classList.contains('kui-status-spacer') && child.getBoundingClientRect().width > 0).map((child) => ({ name: child.getAttribute('data-testid') || child.className, ...box(child) })),
          summaryCut: summary ? summary.scrollWidth > summary.clientWidth + 1 : false,
        }
      })
    let last = await read()
    for (let i = 0; i < 20; i += 1) {
      await page.waitForTimeout(100)
      const next = await read()
      if (JSON.stringify(next) === JSON.stringify(last)) return next
      last = next
    }
    return last
  }
  const zoomFits = (look) =>
    look.zoom !== null &&
    look.zoomParts.length >= 3 &&
    look.zoomParts.every((part) => part.width > 0 && part.left >= look.bar.left - 0.5 && part.right <= look.bar.right + 0.5) &&
    look.others.every((other) => other.right <= look.zoom.left + 0.5 && other.right <= look.bar.right + 0.5)
  const describe = (look) => `bar ${look.bar.left.toFixed(0)}–${look.bar.right.toFixed(0)}; zoom ${look.zoom ? `${look.zoom.left.toFixed(0)}–${look.zoom.right.toFixed(0)}` : 'none'}; ${look.others.map((other) => `${other.name} ${other.left.toFixed(0)}–${other.right.toFixed(0)}`).join(', ')}`
  for (const [locale, width, height] of [['en', 1280, 800], ['zh-CN', 1440, 900]]) {
    const at = `${locale} ${width} × ${height}`
    await reopen(locale, width, height)
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
    await page.mouse.move(width / 2, 5)
    const off = await layout()
    h.step(`${at}: before caching, the zoom controls are whole and overlap nothing`, zoomFits(off), describe(off))
    const cacheButton = kit.getByTestId('motion-cache-ahead')
    await cacheButton.click()
    // While it caches (when this machine is slow enough to see it).
    if ((await cacheButton.getAttribute('data-full')) !== 'true' && /\d+\/\d+/.test((await cacheButton.textContent()) ?? '')) {
      const caching = await layout()
      h.step(`${at}: while it caches, the same`, zoomFits(caching), describe(caching))
    }
    await h.until(async () => (await cacheButton.getAttribute('data-full')) === 'true', 3000)
    const menuButton = kit.getByTestId('motion-cache-full-menu')
    await h.until(async () => (await menuButton.count()) === 1)
    const full = await layout()
    const visibleActions = await kit.locator('[data-testid="motion-cache-lower"], [data-testid="motion-cache-more-memory"], [data-testid="motion-cache-loop-cached"]').count()
    h.step(`${at}: full, the bar holds 缓存已满 N/M and one ▾, the actions out of it`, (await cacheButton.getAttribute('data-full')) === 'true' && visibleActions === 0 && full.others.some((other) => other.name === 'motion-cache-full-menu'), `${await cacheButton.textContent()} · ${visibleActions} actions in the bar`)
    h.step(`${at}: full, the zoom controls are whole and overlap nothing`, zoomFits(full), describe(full))
    if (locale === 'zh-CN') h.step(`${at}: and the composition’s summary is not cut short`, !full.summaryCut, describe(full))
    await menuButton.click()
    const popover = kit.getByTestId('motion-cache-full-popover')
    await popover.waitFor()
    const lower = kit.getByTestId('motion-cache-lower')
    const more = kit.getByTestId('motion-cache-more-memory')
    const loopCached = kit.getByTestId('motion-cache-loop-cached')
    await h.until(async () => (await lower.count()) === 1 && (await more.count()) === 1 && (await loopCached.count()) === 1)
    const texts = [(await lower.textContent()) ?? '', (await more.textContent()) ?? '', (await loopCached.textContent()) ?? '']
    const expected = locale === 'en' ? ['Set preview to Half to cache more', 'Use more memory (1 GB)', 'Loop only the cached part'] : ['把预览分辨率改为二分之一可缓存更多', '使用更多内存（1 GB）', '只循环已缓存的部分']
    const explained = ((await kit.getByTestId('motion-cache-full-text').textContent()) ?? '')
    h.step(`${at}: the ▾ opens the explanation, then ${expected.join(' / ')}`, same(texts, expected) && /\d+/.test(explained), `${texts.join(' / ')} · ${explained}`)
    const opened = await layout()
    h.step(`${at}: with it open, the zoom controls are whole and overlap nothing`, zoomFits(opened), describe(opened))
    // English tooltips: "51 of 300 frames cached (the first 1.7 s)", no brackets inside brackets.
    const held = Number(await cacheButton.getAttribute('data-done'))
    const seconds = (Math.round((held / 30) * 10) / 10).toFixed(1)
    const tips = [(await cacheButton.getAttribute('title')) ?? '', (await loopCached.getAttribute('title')) ?? '', explained]
    const nested = (text) => /\([^()]*\(/.test(text) || /（[^（）]*（/.test(text)
    const said = locale === 'en' ? `${held} of 300 frames cached (the first ${seconds} s)` : `已缓存前 ${held} 帧（${seconds} 秒）`
    h.step(`${at}: the tooltips say ${said}, no brackets inside brackets`, tips.every((tip) => tip.includes(said)) && !tips.some(nested), tips.join(' || '))
    await loopCached.click()
    await h.until(async () => (await loopCached.getAttribute('aria-pressed')) === 'true')
    h.step(`${at}: its loop toggle works from there`, (await loopCached.getAttribute('aria-pressed')) === 'true' && (await loopCached.getAttribute('data-area')) === `0.000 ${(held / 30).toFixed(3)}`, await loopCached.getAttribute('data-area'))
    await loopCached.click()
    await h.until(async () => (await loopCached.getAttribute('aria-pressed')) === 'false')
    await page.keyboard.press('Escape')
    await h.until(async () => (await popover.count()) === 0)
    // The cache button itself still turns caching off.
    await cacheButton.click()
    await h.until(async () => (await cacheButton.getAttribute('aria-pressed')) === 'false')
    h.step(`${at}: a click on 缓存已满 N/M still turns caching off (the ▾ goes)`, (await cacheButton.getAttribute('aria-pressed')) === 'false' && (await menuButton.count()) === 0)
    if (locale === 'en') {
      // 7. English: a 3D layer's fourth rotation row reads Z Rotation.
      await h.setUp([{ command: 'layer.setSwitch', params: { layers: ['#1'], switch: 'threeD', value: true } }])
      const ground = (await h.layers())[0].id
      await selectOnly(ground)
      await h.focusEditor()
      await page.keyboard.press('KeyR')
      const rotation = kit.getByTestId(`motion-prop-row-${ground}-transform.rotation`).locator('.motion-row-name')
      await h.until(async () => (await rotation.count()) === 1 && (await rotation.textContent()) === 'Z Rotation')
      h.step('en: R on a 3D layer reads Z Rotation in the timeline and Properties', (await rotation.textContent()) === 'Z Rotation' && (await kit.getByTestId('motion-prop-name-rotation').textContent()) === 'Z Rotation', `${await rotation.textContent()} / ${await kit.getByTestId('motion-prop-name-rotation').textContent()}`)
    }
  }
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
