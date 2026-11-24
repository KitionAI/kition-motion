// The Project panel by hand, in a real browser through the development host: node tests/project.mjs [chromium|webkit]
// Folders (new, renamed, items dragged in and out, opened and closed, deleted), the search box,
// selecting several items (Cmd or Ctrl, Shift, the arrows), the selected item's details, Interpret
// Footage, Replace Footage through the host's picker, Reveal Layer Source in Project, New
// Composition from Selection, Remove Unused Footage, and the keys (Enter renames, Delete deletes),
// each undone. Results are read from the engine (`project.summary`). Prints PASS or FAIL per step.
import { png, start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const item = async (id) => (await h.items()).find((each) => each.id === id)
const byName = async (name) => (await h.items()).find((each) => each.name === name)
const row = (id) => kit.getByTestId(`motion-item-${id}`)
const visible = async (id) => (await row(id).count()) === 1
const selection = async () => ((await kit.getByTestId('motion-project-list').getAttribute('data-selection')) ?? '').split(' ').filter(Boolean).map(Number)
const engineSelection = async () => (await h.query('project.summary', {})).selection ?? []
const same = (a, b) => a.length === b.length && a.every((each) => b.includes(each))
/** Right-clicks a row and picks a row of its menu. */
async function menu(id, entry) {
  await row(id).scrollIntoViewIfNeeded()
  await row(id).click({ button: 'right' })
  await kit.getByTestId(`motion-item-menu-${entry}`).click()
}
/** A drag from one element to another, waiting for both to stay put first. */
async function dragOnto(from, to) {
  await from.scrollIntoViewIfNeeded()
  let a = await h.centre(from)
  let b = await h.centre(to)
  for (let i = 0; i < 10; i += 1) {
    await page.waitForTimeout(80)
    const [a2, b2] = [await h.centre(from), await h.centre(to)]
    if (a2 && b2 && a && b && a2.y === a.y && b2.y === b.y) break
    ;[a, b] = [a2, b2]
  }
  await page.mouse.move(a.x, a.y)
  await page.mouse.down()
  for (let i = 1; i <= 10; i += 1) {
    await page.mouse.move(a.x + ((b.x - a.x) * i) / 10, a.y + ((b.y - a.y) * i) / 10)
    await page.waitForTimeout(16)
  }
  await page.mouse.move(b.x + 1, b.y + 1)
  await page.waitForTimeout(16)
  await page.mouse.move(b.x, b.y)
  await page.waitForTimeout(100)
  await page.mouse.up()
  await page.waitForTimeout(150)
}

try {
  await h.open()
  await h.putFile('red.png', png(64, 48, [255, 0, 0]))
  await h.putFile('green.png', png(40, 40, [0, 200, 0]))
  await h.setUp([{ command: 'file.import', params: { paths: ['red.png', 'green.png'] } }])
  await h.until(async () => (await byName('green.png')) != null)
  const red = (await byName('red.png')).id
  const green = (await byName('green.png')).id
  await kit.getByTestId('motion-project-toggle').click()
  await kit.getByTestId('motion-project-panel').waitFor()

  // 1. New Folder from the panel's header: named for renaming at once; Enter keeps the name typed.
  const was = await h.items()
  const before = was.length
  const isNew = (each) => each.type === 'Folder' && !was.some((old) => old.id === each.id)
  await kit.getByTestId('motion-project-new-folder').click()
  await h.until(async () => (await h.items()).some(isNew))
  const folder = (await h.items()).find(isNew)
  const field = kit.getByTestId(`motion-item-rename-${folder?.id}`)
  await field.waitFor()
  step('New Folder makes a folder named in Chinese, ready to rename', folder?.name === '文件夹 1' && (await h.items()).length === before + 1, folder?.name)
  await field.fill('素材')
  await page.keyboard.press('Enter')
  await h.until(async () => (await item(folder.id))?.name === '素材')
  step('the typed name is kept', (await item(folder.id))?.name === '素材')
  await h.undo()
  await h.until(async () => (await item(folder.id))?.name === '文件夹 1')
  step('undo gives back its first name', (await item(folder.id))?.name === '文件夹 1')
  await h.undo()
  await h.until(async () => (await item(folder.id)) == null)
  step('undo again removes the folder', (await item(folder.id)) == null)
  await page.keyboard.press(`${h.primary}+Shift+KeyZ`)
  await h.until(async () => (await item(folder.id)) != null)
  step('redo brings it back', (await item(folder.id)) != null)

  // 2. A picture dragged onto the folder goes into it (one undo step); the folder opens to show it.
  await dragOnto(row(red), row(folder.id))
  await h.until(async () => (await item(red))?.parent === folder.id)
  step('dragging a picture onto a folder moves it in', (await item(red))?.parent === folder.id)
  step('the folder opens to show it', (await row(folder.id).getAttribute('aria-expanded')) === 'true' && (await visible(red)))
  await h.undo()
  await h.until(async () => (await item(red))?.parent == null)
  step('undo takes it back out', (await item(red))?.parent == null)
  await dragOnto(row(red), row(folder.id))
  await h.until(async () => (await item(red))?.parent === folder.id)
  // Its twirl closes and opens the folder; the arrows do it from the keyboard.
  await kit.getByTestId(`motion-item-twirl-${folder.id}`).click()
  await h.until(async () => !(await visible(red)))
  step('the twirl closes the folder', !(await visible(red)) && (await row(folder.id).getAttribute('aria-expanded')) === 'false')
  await row(folder.id).click()
  await page.keyboard.press('ArrowRight')
  await h.until(async () => visible(red))
  step('Right arrow opens it again', await visible(red))
  await page.keyboard.press('ArrowLeft')
  await h.until(async () => !(await visible(red)))
  step('Left arrow closes it', !(await visible(red)))
  await page.keyboard.press('ArrowRight')
  await h.until(async () => visible(red))
  // Out again: dragged onto the list's empty space.
  const listBox = await kit.getByTestId('motion-project-list').boundingBox()
  await dragOnto(row(red), kit.getByTestId('motion-project-list'))
  const emptyDrop = async () => (await item(red))?.parent == null
  if (!(await h.until(emptyDrop, 20))) {
    // The list's centre was a row: drop below the last row instead.
    await page.mouse.move((await h.centre(row(red))).x, (await h.centre(row(red))).y)
    await dragOnto(row(red), kit.getByTestId('motion-project-list'))
  }
  step('dragged onto the empty list, it leaves the folder', await h.until(emptyDrop), JSON.stringify(listBox))
  await h.undo()
  await h.until(async () => (await item(red))?.parent === folder.id)
  step('undo puts it back in the folder', (await item(red))?.parent === folder.id)

  // 3. The search box filters by name; Escape clears it and gives the list the keys.
  await kit.getByTestId('motion-project-search').click()
  await page.keyboard.type('gre')
  await h.until(async () => !(await visible(red)))
  step('searching “gre” shows only the green picture', (await visible(green)) && !(await visible(red)) && !(await visible(folder.id)))
  await page.keyboard.press('Escape')
  await h.until(async () => visible(red))
  step('Escape clears the search', (await visible(red)) && (await kit.getByTestId('motion-project-search').inputValue()) === '')
  step('and the focus goes back to the list, not a text field', await kit.getByTestId('motion-project-list').evaluate((element) => document.activeElement === element))

  // 4. Several selected: Cmd or Ctrl adds, Shift and the arrows extend; the engine has the same selection.
  await row(green).click()
  await row(red).click({ modifiers: [h.primary] })
  await h.until(async () => same(await engineSelection(), [green, red]))
  step('Cmd or Ctrl-click adds an item to the selection', same(await selection(), [green, red]) && same(await engineSelection(), [green, red]))
  step('the details say how many are selected', (await kit.getByTestId('motion-project-info').textContent()).includes('选中了 2 项'))
  await row(green).click()
  await page.keyboard.press('Shift+ArrowUp')
  await h.until(async () => (await selection()).length === 2)
  step('Shift and an arrow extend the selection', (await selection()).length === 2 && (await selection()).includes(green), String(await selection()))

  // 5. One selected: its details, and how many layers use it.
  await row(green).click()
  await kit.getByTestId('motion-project-info').waitFor()
  const info = async () => kit.getByTestId('motion-project-info').textContent()
  step('the details show its type and size', (await info()).includes('图片') && (await info()).includes('40 × 40'), await info())
  step('and that no layer uses it', (await kit.getByTestId('motion-project-info-uses').textContent()).includes('0 个图层'))
  await row(green).dblclick()
  await h.until(async () => (await item(green))?.uses === 1)
  await h.until(async () => (await kit.getByTestId('motion-project-info-uses').textContent()).includes('1 个图层'))
  step('placed once, it says one layer uses it', (await kit.getByTestId('motion-project-info-uses').textContent()).includes('1 个图层'))
  const layer = (await h.layers()).find((each) => each.name === 'green.png')

  // 6. Interpret Footage: loop and alpha, one undo step.
  const original = (await item(green))?.footage
  await menu(green, 'interpret')
  await kit.getByTestId('motion-interpret-dialog').waitFor()
  step('Interpret Footage opens in Chinese', (await kit.getByTestId('motion-interpret-dialog').textContent()).includes('解释素材'))
  await kit.getByTestId('motion-interpret-loop').fill('3')
  await kit.getByTestId('motion-interpret-alpha').selectOption('premultiplied')
  await kit.getByTestId('motion-interpret-dialog-ok').click()
  await h.until(async () => (await item(green))?.footage?.loop === 3)
  const interpreted = (await item(green))?.footage
  step('it loops three times, read as premultiplied', interpreted?.loop === 3 && interpreted?.alpha === 'premultiplied', JSON.stringify(interpreted))
  await h.until(async () => (await kit.getByTestId('motion-project-info').textContent()).includes('3 次'))
  step('the details show the loop', (await kit.getByTestId('motion-project-info').textContent()).includes('3 次'))
  await h.undo()
  await h.until(async () => (await item(green))?.footage?.loop === 1)
  step('undo interprets it as before', (await item(green))?.footage?.loop === 1 && (await item(green))?.footage?.alpha === original?.alpha, JSON.stringify((await item(green))?.footage))

  // 7. Replace Footage: the host's picker; the layer keeps its place, now showing the new file.
  // The development host's picker answers the first picture in the workspace: only blue.png is there while it picks.
  await page.evaluate(() => ['red.png', 'green.png'].forEach((path) => window.devHost.files.delete(path)))
  await h.putFile('blue.png', png(40, 40, [0, 0, 255]))
  await menu(green, 'replace')
  await h.until(async () => (await item(green))?.name === 'blue.png')
  await h.putFile('red.png', png(64, 48, [255, 0, 0]))
  await h.putFile('green.png', png(40, 40, [0, 200, 0]))
  await h.until(async () => (await item(green))?.name === 'blue.png')
  step('Replace Footage takes the picked file', (await item(green))?.name === 'blue.png', (await item(green))?.name)
  step('the layer still uses the item', (await h.layers()).some((each) => each.id === layer?.id))
  await h.undo()
  await h.until(async () => (await item(green))?.name === 'green.png')
  step('undo gives the item its file back', (await item(green))?.name === 'green.png')

  // 8. Reveal Layer Source in Project: the item's folder opens and the item is selected.
  await dragOnto(row(green), row(folder.id))
  await h.until(async () => (await item(green))?.parent === folder.id)
  await kit.getByTestId(`motion-item-twirl-${folder.id}`).click()
  await h.until(async () => !(await visible(green)))
  await row(folder.id).click()
  await kit.getByTestId(`motion-layer-${layer.id}`).click({ button: 'right' })
  await kit.getByTestId('motion-layer-menu-reveal-source').click()
  await h.until(async () => (await visible(green)) && same(await selection(), [green]))
  step('Reveal Layer Source in Project opens its folder and selects it', (await visible(green)) && same(await selection(), [green]), String(await selection()))
  await h.undo()
  await h.until(async () => (await item(green))?.parent == null)

  // 9. New Composition from Selection: one composition holding both pictures; undo.
  const comps = async () => (await h.items()).filter((each) => each.type === 'Composition').length
  const compsBefore = await comps()
  const active = (await h.details()).comp.id
  await row(green).click()
  await row(red).click({ modifiers: [h.primary] })
  await menu(red, 'new-comp')
  await h.until(async () => (await comps()) === compsBefore + 1)
  step('New Composition from Selection makes one composition', (await comps()) === compsBefore + 1)
  await h.until(async () => (await h.layers()).length === 2)
  step('holding both pictures', (await h.layers()).length === 2, String((await h.layers()).length))
  await h.undo()
  await h.until(async () => (await comps()) === compsBefore)
  step('undo removes it', (await comps()) === compsBefore)
  await h.until(async () => (await h.details()).comp?.id === active)

  // 10. Remove Unused Footage: the red picture (no layer uses it) goes, the green one stays.
  await menu(green, 'remove-unused')
  await h.until(async () => (await item(red)) == null)
  step('Remove Unused Footage removes what no layer uses', (await item(red)) == null && (await item(green)) != null)
  await h.undo()
  await h.until(async () => (await item(red)) != null)
  step('undo brings it back', (await item(red)) != null)

  // 11. Keys: Enter renames, Delete deletes the selection (asking first when a layer uses one).
  const named = (await item(folder.id))?.name
  await row(folder.id).click()
  await page.keyboard.press('Enter')
  await kit.getByTestId(`motion-item-rename-${folder.id}`).waitFor()
  await kit.getByTestId(`motion-item-rename-${folder.id}`).fill('图片')
  await page.keyboard.press('Enter')
  await h.until(async () => (await item(folder.id))?.name === '图片')
  step('Enter renames the selected item', (await item(folder.id))?.name === '图片')
  await h.undo()
  await h.until(async () => (await item(folder.id))?.name === named)
  step('undo restores its name', (await item(folder.id))?.name === named)
  await row(folder.id).click()
  await page.keyboard.press('Delete')
  await h.until(async () => (await item(folder.id)) == null && (await item(red)) == null)
  step('Delete removes a folder with what is in it', (await item(folder.id)) == null && (await item(red)) == null)
  await h.undo()
  await h.until(async () => (await item(folder.id)) != null && (await item(red))?.parent === folder.id)
  step('undo brings the folder and its picture back', (await item(folder.id)) != null && (await item(red))?.parent === folder.id)
  await row(green).click()
  await row(folder.id).click({ modifiers: [h.primary] })
  await page.keyboard.press('Delete')
  await kit.getByTestId('motion-delete-dialog').waitFor()
  step('deleting two items one layer uses asks first', (await kit.getByTestId('motion-delete-dialog').textContent()).includes('2 项'), await kit.getByTestId('motion-delete-dialog').textContent())
  await kit.getByTestId('motion-delete-dialog-cancel').click()
  step('cancelled, nothing goes', (await item(green)) != null && (await item(folder.id)) != null)
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}

function step(name, ok, detail) {
  h.step(name, ok, detail)
}
