import type { MenuRow } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'
import { EFFECT_PREFIX, effectRows, type MotionEffect } from './effects'
import { INTERPOLATIONS } from './timeline/interp'

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
    const [label, second, given] = entry
    if (typeof second !== 'string') return rowsOf([...path, label], second)
    const shortcut = given ?? SHORTCUT_OVERRIDES[second]
    return [{ path, label, command: second, ...(shortcut ? { shortcut } : {}) }]
  })
}

/**
 * Engine commands whose own shortcut a browser or the system takes before the page sees it, with
 * the one this editor gives them instead (its menus show it, `App` runs it): Cmd+N opens a window
 * and is Kition's (RESERVED_SHORTCUTS), Cmd+Shift+N a private window, Cmd+Alt+H hides the other
 * applications on a Mac.
 */
export const SHORTCUT_OVERRIDES: Readonly<Record<string, string>> = {
  'comp.new': 'Cmd+Alt+N',
  'layer.addMask': 'Cmd+Alt+Shift+N',
  'keys.toggleHold': 'Cmd+Alt+Shift+H',
}

/**
 * Engine commands outside the menus whose own shortcuts work here, each with a visible result:
 * Alt+Left and Right (with Shift, ten frames) nudge the selected keyframes, Cmd+Up and Down select
 * the layer above or below, Cmd+Alt+A selects every keyframe of the selected layers.
 */
const EXTRA_SHORTCUTS = ['keys.nudgeForward', 'keys.nudgeBackward', 'keys.nudgeForward10', 'keys.nudgeBackward10', 'layer.selectPrevious', 'layer.selectNext', 'keys.selectAll']

/**
 * The engine commands whose shortcuts the frame runs: the menus' (bar the ones given another
 * shortcut) and `EXTRA_SHORTCUTS`. Every other engine shortcut does nothing here, since its
 * command shows no result in this editor or replaces the project (New Project, Cmd+Alt+N).
 */
export function shortcutCommands(rows: readonly MenuRow[]): ReadonlySet<string> {
  return new Set([...rows.map((row) => row.command).filter((command) => !command.startsWith('kit:') && command !== SEPARATOR && !SHORTCUT_OVERRIDES[command]), ...EXTRA_SHORTCUTS])
}

/** The kit's own rows (`kit:` commands): the editor runs them, not the engine. */
export const KIT = {
  zoomFit: 'kit:zoom-fit',
  zoom100: 'kit:zoom-100',
  arrangeFront: 'kit:arrange-front',
  arrangeForward: 'kit:arrange-forward',
  arrangeBackward: 'kit:arrange-backward',
  arrangeBack: 'kit:arrange-back',
  saveFrame: 'kit:save-frame',
  addMarker: 'kit:add-marker',
  projectPanel: 'kit:project-panel',
} as const

/** Layer ▸ Arrange's rows and where each moves the selected layers (`layer.arrange`'s `to`). */
export const ARRANGE: Record<string, 'front' | 'forward' | 'backward' | 'back'> = {
  [KIT.arrangeFront]: 'front',
  [KIT.arrangeForward]: 'forward',
  [KIT.arrangeBackward]: 'backward',
  [KIT.arrangeBack]: 'back',
}

const MENUS: ReadonlyArray<readonly [string, readonly Entry[]]> = [
  [
    'File',
    [
      ['Import…', 'file.import', 'Cmd+I'],
      SEPARATOR,
      ['Export…', 'file.exportMovie'],
      ['Save Frame As PNG…', KIT.saveFrame],
    ],
  ],
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
      ['Add Marker', KIT.addMarker, '*'],
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
      ['Keyframe Interpolation', [...INTERPOLATIONS.map((row) => [row.label, row.command] as const), SEPARATOR, ['Keyframe Interpolation…', 'keys.interpolation']]],
      ['Keyframe Velocity…', 'keys.velocity'],
      SEPARATOR,
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
      ['Project Panel', KIT.projectPanel],
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

/** Every menu row, in menu order: the Effect menu (every effect the engine has, by category) after Layer. */
export function menuRows(effects: readonly MotionEffect[]): MenuRow[] {
  return MENUS.flatMap(([name, entries]) => [...rowsOf([name], entries), ...(name === 'Layer' ? effectRows(effects) : [])])
}

/** Menus kept in a read-only document. */
export const VIEW_MENUS = ['View']

/** The rows each menu tab shows in Single line's row, by menu. */
export const MOTION_PRIMARY: Record<string, readonly string[]> = {
  File: ['file.import', 'file.exportMovie', KIT.saveFrame],
  Edit: ['edit.undo', 'edit.redo', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.duplicate', 'edit.splitLayer', 'edit.clear'],
  Composition: ['comp.new', 'comp.settings', 'playback.toggle', KIT.addMarker, 'comp.trimToWorkArea'],
  Layer: ['layer.newText', 'layer.newSolid', 'layer.newShape', 'layer.newNull', 'layer.newAdjustment', 'layer.precompose', 'layer.addMask'],
  Effect: ['kit:find-effect', 'effect.applyLast'],
  Animation: ['keys.easyEase', 'keys.toggleHold'],
  View: ['view.zoomIn', 'view.zoomOut', KIT.zoomFit, 'time.set', 'time.start', 'time.end', 'time.previousKey', 'time.nextKey'],
}

/**
 * Rows hidden where they would have nothing to act on (so a shown row always does something):
 * Arrange and Deselect All without a selection, Select All with every layer selected, Show All
 * Video with none hidden, Unlock All with none locked, Trim to Work Area while the work area is
 * the whole composition, Solid Settings unless the selected layer is a solid, effects without a
 * selection, Last Effect until one was applied here, Fit in Window while the view is fitted and
 * Zoom 100% at 100%, and the Keyframe Interpolation rows without selected keyframes.
 */
export function hiddenRow(info: MotionInfo | null, lastEffect: boolean, view: { fitted: boolean; actual: boolean }, keysSelected: boolean) {
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
  if (!selected || !lastEffect) hidden.add('effect.applyLast')
  if (view.fitted) hidden.add(KIT.zoomFit)
  if (view.actual) hidden.add(KIT.zoom100)
  if (!keysSelected) for (const row of INTERPOLATIONS) hidden.add(row.command)
  const key = [...hidden].sort().join(' ') + (selected ? '' : ' effects')
  return { key, hidden: (row: MenuRow) => hidden.has(row.command) || (!selected && row.command.startsWith(EFFECT_PREFIX)) }
}
