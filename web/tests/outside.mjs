// A project changed outside Kition, opened with its stored replica (Kit UI protocol UI-R1,
// section 7), in a real browser through the development host with a stored replica:
// node tests/outside.mjs [chromium|webkit]
// The person's edit is saved (the host keeps the replica); another program renames a layer in the
// .kmotion; the editor reopens it with the stored snapshot: the rename shows, the rest is the
// project the person saved, every layer keeps its engine id, the replica keeps its document id,
// and every object of the stored replica keeps its object id. Prints PASS or FAIL per step.
import { readFileSync } from 'node:fs'
import { parseSchema, Replica } from '@kition/sdk/kdm'
import { start } from './harness.mjs'

const PATH = 'Sample.kmotion'
const SCHEMA = parseSchema(readFileSync(new URL('../../crates/kdm/kdm-model/model/schema.json', import.meta.url), 'utf8'))

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit, step, until } = h

const version = () => page.evaluate((path) => window.devHost.files.get(path).version, PATH)
const fileText = async () => Buffer.from(await page.evaluate((path) => Array.from(new Uint8Array(window.devHost.files.get(path).bytes)), PATH)).toString('utf8')
/** The objects of the replica the host stores for the document (object id → object), or null. */
const storedObjects = async () => {
  const bytes = await page.evaluate((path) => {
    const stored = window.devHost.history.replica(path)
    return stored ? Array.from(new Uint8Array(stored.snapshot)) : null
  }, PATH)
  return bytes ? Replica.load(SCHEMA, new Uint8Array(bytes)).stateJson().objects : null
}
const docId = () => page.evaluate(() => window.devHost.connection().replica.info())
async function save() {
  const before = await version()
  await kit.getByTestId('motion-save').click()
  return until(async () => (await version()) > before, 300)
}

try {
  await h.open('&replica=1')
  // The person's edit, saved: the host keeps the replica with the file.
  await h.setUp([{ command: 'layer.newNull', params: {} }])
  await until(async () => (await h.layers()).length > 0)
  step('the save writes the file', await save())
  step('the host keeps the replica', await until(async () => (await storedObjects()) != null, 100))
  const before = await storedObjects()
  const id = await docId()
  step('the document has a replica id', typeof id === 'string' && id.length > 0, String(id))
  const layers = await h.layers()
  const renamed = layers[0]

  // Another program renames the top layer in the file; the editor reopens it with its replica.
  const project = JSON.parse(await fileText())
  const comp = Object.values(project.items).find((item) => item.kind?.type === 'Comp' && item.kind.layers.some((layer) => layer.id === renamed.id))
  step('the saved file holds the layer', comp != null, JSON.stringify(renamed))
  comp.kind.layers.find((layer) => layer.id === renamed.id).name = 'Changed outside'
  const changed = Array.from(Buffer.from(JSON.stringify(project, null, 2), 'utf8'))
  await page.evaluate((list) => window.devHost.reopenChanged(new Uint8Array(list).buffer), changed)
  await kit.getByTestId('motion-timeline').waitFor({ timeout: 60000 })
  const shows = await until(async () => (await h.layers()).some((layer) => layer.name === 'Changed outside'), 300)
  step('the outside change shows', shows, JSON.stringify((await h.layers()).map((layer) => layer.name)))
  const after = await h.layers()
  step(
    'every layer keeps its engine id, and only the renamed one changed',
    JSON.stringify(after.map((layer) => [layer.id, layer.id === renamed.id ? renamed.name : layer.name])) === JSON.stringify(layers.map((layer) => [layer.id, layer.name])),
    JSON.stringify({ before: layers.map((layer) => [layer.id, layer.name]), after: after.map((layer) => [layer.id, layer.name]) }),
  )
  step('the document keeps its replica id', (await docId()) === id, `${await docId()} vs ${id}`)

  // The replica the next save stores holds every object it held, under the same object ids.
  step('the next save writes the file', await save())
  const kept = await until(async () => {
    const now = await storedObjects()
    return now != null && Object.values(now).some((object) => object.name === 'Changed outside')
  }, 100)
  step('the host keeps the replica with the outside change', kept)
  const now = (await storedObjects()) ?? {}
  const lost = Object.keys(before).filter((object) => !(object in now))
  step('every object keeps its object id', lost.length === 0 && Object.keys(now).length === Object.keys(before).length, `lost ${lost.length} of ${Object.keys(before).length}; now ${Object.keys(now).length}`)
  const object = Object.keys(before).find((key) => before[key].$type === 'Layer' && before[key].name === renamed.name && before[key].id === renamed.id)
  step('the renamed layer is the same object', object != null && now[object]?.name === 'Changed outside', String(object))
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
