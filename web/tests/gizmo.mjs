// The 3D handles by hand, in a real browser through the development host: node tests/gizmo.mjs [chromium|webkit]
// A selected 3D layer shows an arrow per axis, a square per plane and a ring per rotation axis on
// the canvas: dragging the X arrow moves it along X only, the XY square in X and Y (Shift: its main
// direction only), the Z ring turns its Z Rotation (Shift: 15° steps); in the Top view the Z arrow
// changes its depth; a camera and a light have the handles too. Every drag is one undo step. Where
// the handles are is the engine's (`gizmoHandles`, on the workspace's data-gizmo); the results
// are read with `prop.get`. Prints PASS or FAIL per step.
import { start } from './harness.mjs'

const h = await start(process.argv[2] ?? 'chromium')
const { page, kit } = h
const SHAPE = 27
const near = (a, b, eps) => Math.abs(a - b) <= eps
const value = async (layer, path) => (await h.query('prop.get', { layer, path })).value
const handles = async () => JSON.parse((await kit.getByTestId('motion-workspace').getAttribute('data-gizmo')) || '{}')
const zoom = async () => Number((await kit.getByTestId('motion-workspace').getAttribute('data-view')).split(' ')[0])
/** Waits for the handles to be shown for the selection now, two reads apart the same (settled). */
async function settled(part) {
  let last = ''
  await h.until(async () => {
    const now = JSON.stringify(await handles())
    const ok = now === last && (await handles())[part] != null
    last = now
    return ok
  })
  return handles()
}
/** A press on a part of the handles, moved by (dx, dy) page pixels in steps, released; Shift held while it moves when `shift`. */
async function dragPart(part, dx, dy, shift = false) {
  const spot = (await settled(part))[part]
  const from = await h.at(spot[0], spot[1])
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  if (shift) await page.keyboard.down('Shift')
  for (let i = 1; i <= 10; i += 1) {
    await page.mouse.move(from.x + (dx * i) / 10, from.y + (dy * i) / 10)
    await page.waitForTimeout(16)
  }
  await page.mouse.up()
  if (shift) await page.keyboard.up('Shift')
}

try {
  await h.open()
  await h.setUp([{ command: 'layer.setSwitch', params: { layers: [SHAPE], switch: 'threeD', value: true } }])
  await kit.getByTestId(`motion-layer-${SHAPE}`).locator('.motion-row-name').click()
  await h.until(async () => (await h.layer(SHAPE))?.selected)
  const shown = await settled('x')
  h.step('a selected 3D layer shows its arrows, squares and rings', ['x', 'y', 'xy', 'rx', 'ry', 'rz'].every((part) => shown[part] != null), Object.keys(shown).join(' '))
  const start = await value(SHAPE, 'transform/position')
  const k = await zoom()

  // 1. The X arrow: along X only, one undo step.
  await dragPart('x', 80, 30)
  await h.until(async () => !near((await value(SHAPE, 'transform/position'))[0], start[0], 0.5))
  let now = await value(SHAPE, 'transform/position')
  h.step('dragging the X arrow moves the layer along X only', near(now[0] - start[0], 80 / k, 6) && near(now[1], start[1], 1e-6) && near(now[2] ?? 0, start[2] ?? 0, 1e-6), JSON.stringify({ start, now, expected: 80 / k }))
  await h.undo()
  await h.until(async () => near((await value(SHAPE, 'transform/position'))[0], start[0], 1e-6))
  h.step('one undo puts it back', near((await value(SHAPE, 'transform/position'))[0], start[0], 1e-6))

  // 2. The XY square: in X and Y; with Shift along its main direction only.
  await dragPart('xy', 50, 30)
  await h.until(async () => !near((await value(SHAPE, 'transform/position'))[1], start[1], 0.5))
  now = await value(SHAPE, 'transform/position')
  h.step('the XY square moves it in X and Y', near(now[0] - start[0], 50 / k, 6) && near(now[1] - start[1], 30 / k, 6) && near(now[2] ?? 0, start[2] ?? 0, 1e-6), JSON.stringify(now))
  await h.undo()
  await h.until(async () => near((await value(SHAPE, 'transform/position'))[1], start[1], 1e-6))
  await dragPart('xy', 60, 15, true)
  await h.until(async () => !near((await value(SHAPE, 'transform/position'))[0], start[0], 0.5))
  now = await value(SHAPE, 'transform/position')
  h.step('with Shift, along its main direction only', near(now[0] - start[0], 60 / k, 6) && near(now[1], start[1], 1e-6), JSON.stringify(now))
  await h.undo()
  await h.until(async () => near((await value(SHAPE, 'transform/position'))[0], start[0], 1e-6))
  h.step('undone', near((await value(SHAPE, 'transform/position'))[0], start[0], 1e-6) && near((await value(SHAPE, 'transform/position'))[1], start[1], 1e-6))

  // 3. The Z ring: a quarter of the way round it turns Z Rotation; with Shift in 15° steps.
  const ringDrag = async (shift) => {
    const { origin, rz } = await settled('rz')
    // 30° along the ring, about the handles' origin.
    const [dx, dy] = [rz[0] - origin[0], rz[1] - origin[1]]
    const [s, c] = [Math.sin(Math.PI / 6), Math.cos(Math.PI / 6)]
    const to = [origin[0] + dx * c - dy * s, origin[1] + dx * s + dy * c]
    await dragPart('rz', (to[0] - rz[0]) * k, (to[1] - rz[1]) * k, shift)
  }
  await ringDrag(false)
  await h.until(async () => Math.abs(await value(SHAPE, 'transform/rotation')) > 1)
  const turned = await value(SHAPE, 'transform/rotation')
  h.step('dragging along the Z ring turns the layer about Z', turned > 15 && turned < 45, String(turned))
  await h.undo()
  await h.until(async () => near(await value(SHAPE, 'transform/rotation'), 0, 1e-6))
  h.step('one undo turns it back', near(await value(SHAPE, 'transform/rotation'), 0, 1e-6))
  await ringDrag(true)
  await h.until(async () => Math.abs(await value(SHAPE, 'transform/rotation')) > 1)
  const stepped = await value(SHAPE, 'transform/rotation')
  h.step('with Shift in 15° steps', Math.abs(stepped) >= 15 && near(stepped / 15, Math.round(stepped / 15), 1e-6), String(stepped))
  await h.undo()
  await h.until(async () => near(await value(SHAPE, 'transform/rotation'), 0, 1e-6))

  // 4. The Top view: the Z arrow changes the depth only.
  await kit.getByTestId('motion-3d-view').selectOption('top')
  await h.until(async () => (await h.query('view.get3D', {}))?.view === 'top')
  const top = await settled('z')
  h.step('the Top view shows the Z arrow', top.z != null)
  const away = [top.z[0] - top.origin[0], top.z[1] - top.origin[1]]
  const length = Math.hypot(...away)
  await dragPart('z', (away[0] / length) * 60, (away[1] / length) * 60)
  await h.until(async () => !near((await value(SHAPE, 'transform/position'))[2] ?? 0, start[2] ?? 0, 0.5))
  now = await value(SHAPE, 'transform/position')
  h.step('dragging it changes Z only', Math.abs((now[2] ?? 0) - (start[2] ?? 0)) > 5 && near(now[0], start[0], 1e-6) && near(now[1], start[1], 1e-6), JSON.stringify(now))
  await h.undo()
  await h.until(async () => near((await value(SHAPE, 'transform/position'))[2] ?? 0, start[2] ?? 0, 1e-6))
  h.step('one undo', near((await value(SHAPE, 'transform/position'))[2] ?? 0, start[2] ?? 0, 1e-6))

  // 5. A camera and a light, from Layer ▸ New, have the handles in the Top view.
  for (const [command, kind] of [['layer.newCamera', 'camera'], ['layer.newLight', 'light']]) {
    const before = await h.layers()
    await kit.getByTestId('motion-menu-Layer').click()
    await kit.locator('[data-kit-ui-id="menu:Layer/New"]').hover()
    await kit.locator(`[role=menu] [data-testid="motion-cmd-${command}"]`).first().click()
    await h.until(async () => (await h.layers()).length === before.length + 1)
    const made = (await h.layers()).find((layer) => !before.some((old) => old.id === layer.id))
    await h.until(async () => (await h.layer(made.id))?.selected)
    // Near the composition, where the Top view shows it (a new camera stands far in front).
    await h.setUp([{ command: 'prop.set', params: { layer: made.id, path: 'transform/position', value: [640, 300, -400] } }])
    await h.until(async () => (await h.query('view.get3D', {}))?.view === 'top' || (await kit.getByTestId('motion-3d-view').selectOption('top').then(() => false)))
    const at = await value(made.id, 'transform/position')
    h.step(`a ${kind} shows the handles`, (await settled('x')).x != null, JSON.stringify(await handles()))
    await dragPart('x', 50, 0)
    await h.until(async () => !near((await value(made.id, 'transform/position'))[0], at[0], 0.5))
    const moved = await value(made.id, 'transform/position')
    h.step(`a ${kind}’s X arrow moves it along X`, moved[0] > at[0] + 5 && near(moved[1], at[1], 1e-6) && near(moved[2], at[2], 1e-6), JSON.stringify({ at, moved }))
    await h.undo()
    await h.until(async () => near((await value(made.id, 'transform/position'))[0], at[0], 1e-6))
    h.step(`one undo puts the ${kind} back`, near((await value(made.id, 'transform/position'))[0], at[0], 1e-6))
  }
} catch (error) {
  h.crashed(error)
} finally {
  await h.finish()
}
