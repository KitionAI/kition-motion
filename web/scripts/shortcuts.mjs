// Prints every shortcut the editor binds, as Markdown grouped by area, with the zh-CN names:
// node scripts/shortcuts.mjs > shortcuts.md   (needs the engine built: pnpm engine)
// Read from the code's own tables, so it cannot drift from what the keys do: the menus
// (src/menus.ts, each row's own shortcut or else its engine command's, SHORTCUT_OVERRIDES, and
// EXTRA_SHORTCUTS), the canvas tools (the engine's tools()), the timeline's and the editor's own
// keys and the property gestures (src/keys.ts). Names are translated by PHRASES (src/strings.ts).
import { readFileSync } from 'node:fs'
import { createServer } from 'vite'

const web = new URL('..', import.meta.url).pathname
const server = await createServer({ configFile: `${web}vite.config.ts`, root: web, server: { middlewareMode: true, watch: null }, appType: 'custom', logLevel: 'error' })
try {
  const menus = await server.ssrLoadModule('/src/menus.ts')
  const keys = await server.ssrLoadModule('/src/keys.ts')
  const { PHRASES } = await server.ssrLoadModule('/src/strings.ts')
  const engine = await import(`${web}public/engine/kition_motion_kit.js`)
  engine.initSync({ module: readFileSync(`${web}public/engine/kition_motion_kit_bg.wasm`) })
  const doc = engine.MotionDocument.create('Shortcuts.kmotion', 64, 36)
  const commands = new Map(JSON.parse(doc.commands()).map((command) => [command.id, command]))
  const tools = JSON.parse(doc.tools())
  const zh = PHRASES['zh-CN']
  const phrase = (text) => zh[text] ?? zh[text.replace(/…$/, '')] ?? text
  const cell = (text) => String(text).replace(/\|/g, '\\|')
  const keysCell = (list) => list.map((key) => `\`${cell(key)}\``).join(' / ')
  const out = ['# Kition Motion 快捷键', '', '> 由 `kition-motion/web/scripts/shortcuts.mjs` 从编辑器代码的快捷键表生成，请勿手改。Mac 上 Cmd 是 ⌘、Alt 是 ⌥；Windows 和 Linux 上 Cmd 是 Ctrl。', '']
  const table = (title, rows) => {
    if (!rows.length) return
    out.push(`## ${title}`, '', '| 快捷键 | 作用 |', '|---|---|', ...rows.map(([shortcut, action]) => `| ${shortcut} | ${cell(action)} |`), '')
  }

  // The menus, menu by menu (the Effect menu's effects have no shortcuts).
  const byMenu = new Map()
  for (const row of menus.menuRows([])) {
    if (row.command === '---') continue
    const shortcut = row.shortcut ?? (row.command.startsWith('kit:') ? undefined : commands.get(row.command)?.shortcut)
    if (!shortcut) continue
    const top = row.path[0]
    const list = byMenu.get(top) ?? []
    list.push([keysCell([shortcut]), [...row.path.slice(1), row.label].map(phrase).join(' ▸ ')])
    byMenu.set(top, list)
  }
  for (const [top, rows] of byMenu) table(`菜单：${phrase(top)}`, rows)
  table(
    '不在菜单里的命令',
    menus.EXTRA_SHORTCUTS.flatMap((id) => {
      const command = commands.get(id)
      return command?.shortcut ? [[keysCell([command.shortcut]), phrase(command.title)]] : []
    }),
  )
  table('画布工具', tools.filter((tool) => tool.shortcut).map((tool) => [keysCell([tool.shortcut]), phrase(tool.label)]))
  table('时间轴（焦点在时间轴上时）', Object.values(keys.TIMELINE_KEYS).map((binding) => [keysCell(binding.keys), binding.zh]))
  table('编辑器', Object.values(keys.EDITOR_KEYS).map((binding) => [keysCell(binding.keys), binding.zh]))
  table('属性面板与属性行', Object.values(keys.PROPERTY_KEYS).map((binding) => [keysCell(binding.keys), binding.zh]))
  process.stdout.write(out.join('\n'))
} finally {
  await server.close()
}
