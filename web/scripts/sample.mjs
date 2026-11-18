// Makes public/sample/Sample.kmotion with the engine itself: a 3-second composition with a solid,
// a title fading in, and a shape that moves across.
import fs from 'node:fs/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'

const web = fileURLToPath(new URL('..', import.meta.url))
const engine = await import(pathToFileURL(`${web}public/engine/kition_motion_kit.js`).href)
await engine.default({ module_or_path: await fs.readFile(`${web}public/engine/kition_motion_kit_bg.wasm`) })
const doc = engine.MotionDocument.create('Sample.kmotion', 1280, 720)
const run = (command, params = {}) => doc.execute(command, JSON.stringify(params))
const top = () => JSON.parse(doc.inspect()).info.layers[0].id
run('comp.settings', { duration: 3 })
run('layer.newSolid', { name: 'Night', color: '#1d2b64' })
run('layer.newShape', { kind: 'rect' })
const shape = top()
run('prop.addKey', { layer: shape, path: 'transform/position', time: 0, value: [200, 360] })
run('prop.addKey', { layer: shape, path: 'transform/position', time: 2.5, value: [1080, 360] })
run('layer.newText', { text: 'Northern Lights' })
const title = top()
run('prop.addKey', { layer: title, path: 'transform/opacity', time: 0, value: 0 })
run('prop.addKey', { layer: title, path: 'transform/opacity', time: 1, value: 100 })
run('layer.select', { layers: [] })
await fs.writeFile(`${web}public/sample/Sample.kmotion`, doc.save())
console.log('wrote public/sample/Sample.kmotion')
