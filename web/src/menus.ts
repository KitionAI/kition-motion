import type { MenuRow } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'

/**
 * The editor's menus: the commands each menu shows, in order, with separators and submenus. A
 * command is listed only when clicking it, where it is enabled, visibly does its job in this
 * editor; the engine keeps every other command for agents (`menu-removed.md` in the kit's docs
 * lists what was left out and why). Labels are English and translated by `PHRASES`.
 */

const SEPARATOR = '---'

/** A menu's rows from `[label, command, shortcut?]` entries; a submenu is `[name, entries]`; `'---'` separates. */
type Entry = typeof SEPARATOR | readonly [label: string, command: string, shortcut?: string] | readonly [submenu: string, entries: readonly Entry[]]

function rowsOf(path: readonly string[], entries: readonly Entry[]): MenuRow[] {
  return entries.flatMap((entry): MenuRow[] => {
    if (entry === SEPARATOR) return [{ path, label: SEPARATOR, command: SEPARATOR }]
    const [label, second, shortcut] = entry
    if (typeof second !== 'string') return rowsOf([...path, label], second)
    return [{ path, label, command: second, ...(shortcut ? { shortcut } : {}) }]
  })
}

/** The kit's own rows (`kit:` commands): the editor runs them, not the engine. */
export const KIT = {
  zoomFit: 'kit:zoom-fit',
  zoom100: 'kit:zoom-100',
  arrangeFront: 'kit:arrange-front',
  arrangeForward: 'kit:arrange-forward',
  arrangeBackward: 'kit:arrange-backward',
  arrangeBack: 'kit:arrange-back',
} as const

/** Layer ▸ Arrange's rows and where each moves the selected layers (`layer.arrange`'s `to`). */
export const ARRANGE: Record<string, 'front' | 'forward' | 'backward' | 'back'> = {
  [KIT.arrangeFront]: 'front',
  [KIT.arrangeForward]: 'forward',
  [KIT.arrangeBackward]: 'backward',
  [KIT.arrangeBack]: 'back',
}

const MENUS: ReadonlyArray<readonly [string, readonly Entry[]]> = [
  ['File', [['Render Movie (MP4)', 'file.exportMovie']]],
  [
    'Edit',
    [
      ['Undo', 'edit.undo'],
      ['Redo', 'edit.redo'],
      SEPARATOR,
      ['Cut', 'edit.cut'],
      ['Copy', 'edit.copy'],
      ['Paste', 'edit.paste'],
      ['Clear', 'edit.clear'],
      SEPARATOR,
      ['Duplicate', 'edit.duplicate'],
      ['Split Layer', 'edit.splitLayer'],
      ['Lift Work Area', 'edit.liftWorkArea'],
      ['Extract Work Area', 'edit.extractWorkArea'],
      SEPARATOR,
      ['Select All', 'edit.selectAll'],
      ['Deselect All', 'edit.deselectAll'],
    ],
  ],
  [
    'Composition',
    [
      ['New Composition…', 'comp.new'],
      ['Composition Settings…', 'comp.settings'],
      SEPARATOR,
      ['Play / Pause', 'playback.toggle', 'Space'],
      SEPARATOR,
      ['Trim Comp to Work Area', 'comp.trimToWorkArea'],
      ['Crop Comp to Selected Layer(s) Bounds', 'comp.cropToLayerBounds'],
    ],
  ],
  [
    'Layer',
    [
      [
        'New',
        [
          ['Text', 'layer.newText'],
          ['Shape Layer', 'layer.newShape'],
          ['Solid…', 'layer.newSolid'],
          ['Null Object', 'layer.newNull'],
          ['Adjustment Layer', 'layer.newAdjustment'],
          ['Camera', 'layer.newCamera'],
          ['Light', 'layer.newLight'],
        ],
      ],
      SEPARATOR,
      ['Solid Settings…', 'layer.settings'],
      ['Rename…', 'layer.rename'],
      SEPARATOR,
      ['Pre-compose', 'layer.precompose'],
      ['Mask', [['New Mask', 'layer.addMask']]],
      [
        'Layer Styles',
        [
          ['Drop Shadow', 'layer.style.dropShadow'],
          ['Inner Shadow', 'layer.style.innerShadow'],
          ['Outer Glow', 'layer.style.outerGlow'],
          ['Inner Glow', 'layer.style.innerGlow'],
          ['Bevel and Emboss', 'layer.style.bevelEmboss'],
          ['Satin', 'layer.style.satin'],
          ['Color Overlay', 'layer.style.colorOverlay'],
          ['Gradient Overlay', 'layer.style.gradientOverlay'],
          ['Stroke', 'layer.style.stroke'],
        ],
      ],
      ['Time', [['Time-Reverse Layer', 'layer.timeReverse']]],
      [
        'Arrange',
        [
          ['Bring Layer to Front', KIT.arrangeFront, 'Cmd+Shift+]'],
          ['Bring Layer Forward', KIT.arrangeForward, 'Cmd+]'],
          ['Send Layer Backward', KIT.arrangeBackward, 'Cmd+['],
          ['Send Layer to Back', KIT.arrangeBack, 'Cmd+Shift+['],
        ],
      ],
      [
        'Switches',
        [
          ['Hide Other Video', 'layer.hideOtherVideo'],
          ['Show All Video', 'layer.showAllVideo'],
          ['Unlock All Layers', 'layer.unlockAll'],
        ],
      ],
    ],
  ],
  [
    'Animation',
    [
      ['Easy Ease', 'keys.easyEase'],
      ['Easy Ease In', 'keys.easyEaseIn'],
      ['Easy Ease Out', 'keys.easyEaseOut'],
      ['Toggle Hold Keyframe', 'keys.toggleHold'],
      ['Time-Reverse Keyframes', 'keys.timeReverse'],
      SEPARATOR,
      ['Separate Dimensions', 'prop.separateDimensions'],
    ],
  ],
  [
    'View',
    [
      ['Zoom In', 'view.zoomIn', '.'],
      ['Zoom Out', 'view.zoomOut', ','],
      ['Fit in Window', KIT.zoomFit, 'Cmd+0'],
      ['Zoom 100%', KIT.zoom100],
      SEPARATOR,
      ['Go to Time…', 'time.set'],
      ['Go to Start', 'time.start', 'Home'],
      ['Go to End', 'time.end', 'End'],
      ['Previous Frame', 'time.previousFrame', 'PageUp'],
      ['Next Frame', 'time.nextFrame', 'PageDown'],
      ['Previous Keyframe', 'time.previousKey', 'J'],
      ['Next Keyframe', 'time.nextKey', 'K'],
    ],
  ],
]

/** The menu bar's menus, in order. */
export const MENU_ORDER = MENUS.map(([name]) => name)

/** Every menu row, in menu order. */
export const MENU_ROWS: readonly MenuRow[] = MENUS.flatMap(([name, entries]) => rowsOf([name], entries))

/** Menus kept in a read-only document. */
export const VIEW_MENUS = ['View']

/** The rows each menu tab shows in Single line's row, by menu. */
export const MOTION_PRIMARY: Record<string, readonly string[]> = {
  File: ['file.exportMovie'],
  Edit: ['edit.undo', 'edit.redo', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.duplicate', 'edit.splitLayer', 'edit.clear'],
  Composition: ['comp.new', 'comp.settings', 'playback.toggle', 'comp.trimToWorkArea'],
  Layer: ['layer.newText', 'layer.newSolid', 'layer.newShape', 'layer.newNull', 'layer.newAdjustment', 'layer.precompose', 'layer.addMask'],
  Animation: ['keys.easyEase', 'keys.toggleHold'],
  View: ['view.zoomIn', 'view.zoomOut', KIT.zoomFit, 'time.set', 'time.start', 'time.end', 'time.previousKey', 'time.nextKey'],
}

/**
 * Rows hidden where they would have nothing to act on (so a shown row always does something):
 * Arrange and Deselect All without a selection, Select All with every layer selected, Show All
 * Video with none hidden, Unlock All with none locked, Trim to Work Area while the work area is
 * the whole composition, Solid Settings unless the selected layer is a solid.
 */
export function hiddenRow(info: MotionInfo | null) {
  const layers = info?.layers ?? []
  const selected = layers.some((layer) => layer.selected)
  const comp = info?.comp
  const hidden = new Set<string>()
  if (!selected) for (const command of [...Object.keys(ARRANGE), 'edit.deselectAll']) hidden.add(command)
  if (layers.length && layers.every((layer) => layer.selected)) hidden.add('edit.selectAll')
  if (!layers.some((layer) => !layer.switches.video)) hidden.add('layer.showAllVideo')
  if (!layers.some((layer) => layer.switches.locked)) hidden.add('layer.unlockAll')
  if (layers.find((layer) => layer.selected)?.type !== 'Solid') hidden.add('layer.settings')
  if (!comp || (comp.workArea[0] <= 1e-6 && comp.workArea[1] >= comp.duration - 1e-6)) hidden.add('comp.trimToWorkArea')
  const key = [...hidden].sort().join(' ')
  return { key, hidden: (row: MenuRow) => hidden.has(row.command) }
}
