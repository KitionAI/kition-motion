// An agent's draft and a restored version in the editor (Kit UI protocol sections 7d and 7m),
// through the development host with the replica and checkpoints kept (`&replica=1&checkpoints=1`)
// and the sample project (case motion.collab.drafts/editor): node tests/drafts.mjs [chromium|webkit]
// a. the person adds a null layer (Layer ▸ New ▸ Null Object); an agent's draft adds a solid; the
//    draft's comparison lists it and the project does not hold it; the person renames the null
//    layer meanwhile (a double-click on its name in the timeline); the draft is accepted: the
//    timeline shows the solid's row and the new name, and the inspect answer holds both;
// b. one Undo (the editor's Undo button) takes back the solid only; Redo brings it back;
// d. the project saved, a checkpoint recorded with a label, a layer added after it, the checkpoint
//    restored through the host: the timeline has the checkpoint's layers; one Undo brings the later
//    layer back.
import { start } from './harness.mjs'

const PATH = 'Sample.kmotion'
const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step } = h
let at = 'opening'

/** Waits (up to `tries` × 100 ms) for `check`; a failure names what did not happen. */
const must = async (check, message, tries = 200) => {
  if (!(await h.until(check, tries))) throw new Error(`${message} (at ${at})`)
}
/** A host call that fails with `what` instead of hanging. */
const host = (what, work, arg) => Promise.race([page.evaluate(work, arg), new Promise((_, reject) => setTimeout(() => reject(new Error(`${what} did not answer in 20 s (at ${at})`)), 20000))])
const names = async () => (await h.layers()).map((layer) => layer.name)
const ids = async () => (await h.layers()).map((layer) => layer.id)
const version = () => page.evaluate((path) => window.devHost.files.get(path)?.version ?? 0, PATH)
/** The person adds a null layer with Layer ▸ New ▸ Null Object; answers its id. */
const newNull = async () => {
  const before = await ids()
  await kit.getByTestId('motion-menu-Layer').click()
  await kit.locator('[data-kit-ui-id="menu:Layer/New"]').hover()
  await kit.locator('[role=menu] [data-testid="motion-cmd-layer.newNull"]').first().click()
  await must(async () => (await ids()).length === before.length + 1, 'Layer ▸ New ▸ Null Object adds a layer')
  const id = (await ids()).find((each) => !before.includes(each))
  await kit.getByTestId(`motion-layer-${id}`).waitFor({ state: 'visible' })
  return id
}
/** The editor's Undo or Redo button (`motion-undo`, `motion-redo`), once it can run. */
const press = async (testId) => {
  const button = kit.getByTestId(testId)
  await must(() => button.isEnabled(), `${testId} is enabled`)
  await button.click()
}

try {
  await h.open('&replica=1&checkpoints=1')
  page.context().setDefaultTimeout(10000)
  await must(async () => (await page.evaluate(() => window.devHost.connection() != null)) && (await ids()).length > 0, 'the sample project opens')

  at = '(a) the draft accepted'
  const mine = await newNull()
  const ran = await host('the draft run', async () => window.devHost.connection().ai.run([{ command: 'layer.newSolid', params: { name: 'From the draft', color: '#00ff00' } }], false, { draft: true }))
  const drafted = ran.completed?.[0]?.result?.layer
  if (drafted == null) throw new Error(`the draft answers the solid's id: ${JSON.stringify(ran)}`)
  const compared = await host('draft/compare', () => window.devHost.connection().draft.compare({ maxSize: 256 }))
  step('the comparison lists the draft\'s solid', compared.summary.created.length >= 1, JSON.stringify(compared.summary))
  step('comparing changes nothing in the project', !(await ids()).includes(drafted) && !(await names()).includes('From the draft'))
  // The person renames the null layer meanwhile.
  await kit.getByTestId(`motion-name-${mine}`).dblclick()
  await kit.getByTestId(`motion-rename-${mine}`).waitFor()
  await page.keyboard.press(`${h.primary}+KeyA`)
  await page.keyboard.type('Meanwhile')
  await page.keyboard.press('Enter')
  await must(async () => (await h.layer(mine))?.name === 'Meanwhile', 'the person renames the layer')
  const count = (await ids()).length
  const merged = await host('draft/merge', () => window.devHost.connection().draft.merge({ kind: 'agent', principal: 'agent:test' }))
  step('the draft merged', merged?.merged === true, JSON.stringify(merged))
  await must(async () => (await ids()).includes(drafted), 'the project holds the merged solid')
  await kit.getByTestId(`motion-layer-${drafted}`).waitFor({ state: 'visible' })
  step('the timeline shows the draft\'s solid and the person\'s rename', ((await kit.getByTestId(`motion-name-${drafted}`).textContent()) ?? '').includes('From the draft') && ((await kit.getByTestId(`motion-name-${mine}`).textContent()) ?? '').includes('Meanwhile'))
  step('both edits are in the project', (await ids()).length === count + 1 && (await h.layer(mine))?.name === 'Meanwhile', JSON.stringify(await names()))
  const all = await ids()

  at = '(b) Undo and Redo'
  await press('motion-undo')
  await must(async () => !(await ids()).includes(drafted), 'Undo takes the solid back')
  await kit.getByTestId(`motion-layer-${drafted}`).waitFor({ state: 'detached' })
  step('one Undo takes back the draft only', (await h.layer(mine))?.name === 'Meanwhile' && (await ids()).length === count, JSON.stringify(await names()))
  await press('motion-redo')
  await must(async () => (await ids()).includes(drafted), 'Redo brings the solid back')
  await kit.getByTestId(`motion-layer-${drafted}`).waitFor({ state: 'visible' })
  step('Redo brings the draft back', JSON.stringify(await ids()) === JSON.stringify(all), JSON.stringify(await names()))

  at = '(d) the checkpoint restored'
  const saved = await version()
  await kit.getByTestId('motion-save').click()
  await must(async () => (await version()) > saved, 'saved through the host')
  await must(() => page.evaluate((path) => window.devHost.history.replica(path) != null, PATH), 'the replica is stored with the save')
  const checkpoint = await host('recordCheckpoint', () => window.devHost.recordCheckpoint({ origin: 'save', label: 'Before the later layer' }))
  if (!checkpoint?.id) throw new Error('a checkpoint is recorded')
  const listed = await host('the checkpoint list', (path) => window.devHost.history.checkpoints(path), PATH)
  step('the checkpoint is listed with its label', listed.some((entry) => entry.id === checkpoint.id && entry.label === 'Before the later layer'), JSON.stringify(listed))
  const later = await newNull()
  await host('restoreCheckpoint', (id) => window.devHost.restoreCheckpoint(id), checkpoint.id)
  await must(async () => JSON.stringify(await ids()) === JSON.stringify(all), 'the restore gives back the checkpoint\'s layers')
  await kit.getByTestId(`motion-layer-${later}`).waitFor({ state: 'detached' })
  step('the restore gives back the checkpoint\'s layers', (await h.layer(mine))?.name === 'Meanwhile' && (await ids()).includes(drafted))
  await press('motion-undo')
  await must(async () => (await ids()).includes(later), 'Undo brings the later layer back')
  await kit.getByTestId(`motion-layer-${later}`).waitFor({ state: 'visible' })
  step('one Undo takes the restore back', (await ids()).length === all.length + 1)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
