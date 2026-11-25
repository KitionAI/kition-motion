import { shortcutMatches } from '@kition/sdk/editor'

/** The parts of a key event a binding reads. */
type KeyLike = { key: string; code: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean; altKey: boolean }

/**
 * A key the editor itself handles (not a menu row's): the keys as kits write them (`Cmd+C`,
 * `Shift+F3`), what it does in Chinese and English, and, where the keys alone cannot say it, how
 * an event matches; `command` names the menu row a key also runs (the same action, not a clash). The handlers ask `pressed`, and `scripts/shortcuts.mjs` prints these tables
 * for the help docs, so the two cannot drift apart.
 */
export type Binding = { keys: readonly string[]; zh: string; en: string; match?: (event: KeyLike) => boolean; command?: string }

/** Whether `event` is one of `binding`'s keys. */
export function pressed(binding: Binding, event: KeyLike) {
  return binding.match ? binding.match(event) : binding.keys.some((keys) => shortcutMatches(keys, event, undefined, true))
}

/** Alt+Shift and a digit key, by its place on the keyboard (Option changes the character on a Mac). */
const altShiftDigit = (event: KeyLike, digit: number) => event.altKey && event.shiftKey && !event.metaKey && !event.ctrlKey && event.code === `Digit${digit}`

/**
 * The key that types `*` on the keyboard in use: `*` on any layout, the full-width `＊` a Chinese or
 * Japanese input method types, the keypad's key, and Shift+8 while an input method holds the key
 * (`Process`).
 */
const isStar = (event: KeyLike) => event.key === '*' || event.key === '＊' || event.code === 'NumpadMultiply' || (event.key === 'Process' && event.shiftKey && event.code === 'Digit8')

const plain = (event: KeyLike) => !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey

/** The timeline's keys, asked before the editor's (Timeline.tsx). */
export const TIMELINE_KEYS = {
  delete: { keys: ['Delete', 'Backspace'], zh: '删除选中的标记、关键帧或图层', en: 'Delete the selected marker, keyframes or layers' },
  copyKeys: { keys: ['Cmd+C'], zh: '复制选中的关键帧（没有选中关键帧时复制图层）', en: 'Copy the selected keyframes (the layers when none is selected)' },
  pasteKeys: { keys: ['Cmd+V'], zh: '在当前时间粘贴刚复制的关键帧', en: 'Paste the copied keyframes at the current time' },
  previousKey: { keys: ['J'], zh: '转到上一个关键帧', en: 'Go to the previous keyframe' },
  nextKey: { keys: ['K'], zh: '转到下一个关键帧', en: 'Go to the next keyframe' },
  graph: { keys: ['Shift+F3'], zh: '切换图表编辑器', en: 'Show or hide the Graph Editor' },
  graphFit: { keys: ['F'], zh: '图表编辑器里：适合全部（时间轴显示从第一个到最后一个关键帧）', en: 'In the Graph Editor: fit all (show every keyframe’s time)', match: (event: KeyLike) => plain(event) && event.code === 'KeyF' },
  zoomIn: { keys: ['='], zh: '放大时间轴', en: 'Zoom the timeline in', match: (event: KeyLike) => !event.metaKey && !event.ctrlKey && !event.altKey && event.code === 'Equal' },
  zoomOut: { keys: ['-'], zh: '缩小时间轴', en: 'Zoom the timeline out', match: (event: KeyLike) => !event.metaKey && !event.ctrlKey && !event.altKey && event.code === 'Minus' },
  levels: { keys: ['L'], zh: '只显示“音频电平”（连按两下 L 再加上波形）', en: 'Show only Audio Levels (L twice: the waveform too)' },
  animated: { keys: ['U'], zh: '只显示有关键帧的属性（再按一次收起）', en: 'Show only the animated properties (again: close)' },
  anchor: { keys: ['A'], zh: '只显示“锚点”', en: 'Show only Anchor Point' },
  position: { keys: ['P'], zh: '只显示“位置”', en: 'Show only Position' },
  scale: { keys: ['S'], zh: '只显示“缩放”', en: 'Show only Scale' },
  rotation: { keys: ['R'], zh: '只显示“旋转”（3D 图层上连同“方向”和 X、Y、Z 轴旋转）', en: 'Show only Rotation (on a 3D layer with Orientation and X, Y and Z Rotation)' },
  opacity: { keys: ['T'], zh: '只显示“不透明度”——上一次点的是时间轴时；上一次点的是画面时，T 是文字工具', en: 'Show only Opacity, when the last press was on the timeline; after a press on the composition T is the Text tool' },
  duplicate: { keys: ['Cmd+D'], zh: '复制选中的图层', en: 'Duplicate the selected layers' },
} satisfies Record<string, Binding>

/** The editor's own keys (App.tsx), after the timeline's. */
export const EDITOR_KEYS = {
  nudge: {
    keys: ['←', '→', '↑', '↓'],
    zh: '选取工具下把选中的图层移动 1 像素（加 Shift 移动 10 像素）',
    en: 'With the Selection tool, move the selected layers 1 pixel (10 with Shift)',
    match: (event: KeyLike) => !event.metaKey && !event.ctrlKey && !event.altKey && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key),
  },
  marker: { command: 'kit:add-marker', keys: ['*'], zh: '在当前时间添加标记（小键盘 * 或任何键盘布局上打出 * 的键，如 Shift+8）', en: 'Add a marker at the current time (the keypad’s *, or whatever key types * on the layout, such as Shift+8)', match: (event: KeyLike) => isStar(event) && !event.metaKey && !event.ctrlKey && !event.altKey },
  play: { command: 'playback.toggle', keys: ['Space'], zh: '播放或暂停（按住空格拖动则平移画面）', en: 'Play or pause (held while dragging: pan)', match: (event: KeyLike) => event.key === ' ' && !event.metaKey && !event.ctrlKey },
  zoomIn: { command: 'view.zoomIn', keys: ['.'], zh: '放大合成', en: 'Zoom the composition in', match: (event: KeyLike) => event.key === '.' && !event.metaKey && !event.ctrlKey && !event.altKey },
  zoomOut: { command: 'view.zoomOut', keys: [','], zh: '缩小合成', en: 'Zoom the composition out', match: (event: KeyLike) => event.key === ',' && !event.metaKey && !event.ctrlKey && !event.altKey },
  nextFrame: { command: 'time.nextFrame', keys: ['PageDown', 'Cmd+→'], zh: '下一帧（加 Shift 前进 10 帧）', en: 'Next frame (10 with Shift)', match: (event: KeyLike) => event.key === 'PageDown' || (event.key === 'ArrowRight' && (event.metaKey || event.ctrlKey)) },
  previousFrame: { command: 'time.previousFrame', keys: ['PageUp', 'Cmd+←'], zh: '上一帧（加 Shift 后退 10 帧）', en: 'Previous frame (10 with Shift)', match: (event: KeyLike) => event.key === 'PageUp' || (event.key === 'ArrowLeft' && (event.metaKey || event.ctrlKey)) },
  start: { command: 'time.start', keys: ['Home'], zh: '转到开头', en: 'Go to the start', match: (event: KeyLike) => event.key === 'Home' && plain(event) },
  end: { command: 'time.end', keys: ['End'], zh: '转到结尾', en: 'Go to the end', match: (event: KeyLike) => event.key === 'End' && plain(event) },
  // Alt+Shift with 1, 2 or 3 work in every browser; F10 and F12 too (F11 is the browser's full
  // screen almost everywhere, so it is not offered).
  view3dFront: { command: 'view.threeD.front', keys: ['Alt+Shift+1', 'F10'], zh: '3D 视图：正面', en: '3D view: Front', match: (event: KeyLike) => altShiftDigit(event, 1) || (plain(event) && event.key === 'F10') },
  view3dCustom: { command: 'view.threeD.custom1', keys: ['Alt+Shift+2'], zh: '3D 视图：自定义视图 1', en: '3D view: Custom View 1', match: (event: KeyLike) => altShiftDigit(event, 2) },
  view3dCamera: { command: 'view.threeD.activeCamera', keys: ['Alt+Shift+3', 'F12'], zh: '3D 视图：活动摄像机', en: '3D view: Active Camera', match: (event: KeyLike) => altShiftDigit(event, 3) || (plain(event) && event.key === 'F12') },
} satisfies Record<string, Binding>

/** Mouse gestures with keys in the Properties panel and on the timeline's property rows (KeyDiamond.tsx). */
export const PROPERTY_KEYS = {
  stopwatch: { keys: ['单击 ◇'], zh: '开始动画、在两个关键帧之间加关键帧，或删除当前时间的关键帧', en: 'Start animating, add a keyframe between keyframes, or remove the one here' },
  clearAnimation: { keys: ['Alt+单击 ◇'], zh: '清除这个属性的全部关键帧（右键菜单里也有“清除动画”）', en: 'Remove every keyframe of the property (also 清除动画 in its menu)' },
} satisfies Record<string, Binding>
