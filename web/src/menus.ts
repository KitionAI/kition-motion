import type { MenuRow } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'
import { EFFECT_PREFIX, effectRows, type MotionEffect } from './effects'
import { INTERPOLATIONS } from './timeline/interp'
import type { Overlays } from './CanvasGuides'
import { LABELS, labelRow } from './labels'

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
    // An empty override: no key (not the engine command's).
    return [{ path, label, command: second, ...(shortcut !== undefined ? { shortcut } : {}) }]
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
  'keyframe.toggleHold': 'Cmd+Alt+Shift+H',
  // Cmd+R reloads the page in a browser before the editor could take it.
  'view.rulers': 'Cmd+Alt+Shift+R',
  // The engine's Cmd+Alt+Shift+N is New Mask's here: New Folder has no key.
  'project.newFolder': '',
}

/**
 * Engine commands outside the menus whose own shortcuts work here, each with a visible result:
 * Alt+Left and Right (with Shift, ten frames) nudge the selected keyframes, Cmd+Up and Down select
 * the layer above or below, Cmd+Alt+A selects every keyframe of the selected layers.
 */
export const EXTRA_SHORTCUTS = ['keyframe.nudgeForward', 'keyframe.nudgeBackward', 'keyframe.nudgeForward10', 'keyframe.nudgeBackward10', 'layer.selectPrevious', 'layer.selectNext', 'keyframe.selectAll']

/**
 * The engine commands whose shortcuts the frame runs: the menus' (bar the ones given another
 * shortcut) and `EXTRA_SHORTCUTS`. Every other engine shortcut does nothing here, since its
 * command shows no result in this editor or replaces the project (New Project, Cmd+Alt+N).
 */
export function shortcutCommands(rows: readonly MenuRow[]): ReadonlySet<string> {
  return new Set([...rows.map((row) => row.command).filter((command) => !command.startsWith('kit:') && command !== SEPARATOR && !(command in SHORTCUT_OVERRIDES) && !BROWSER_KEYS.has(command)), ...EXTRA_SHORTCUTS])
}

/**
 * Menu commands whose engine shortcut is the browser's own: Custom View 1's F11 is full screen, so
 * the frame does not run it (the key stays the browser's); the editor's Alt+Shift+2 switches to it
 * (`EDITOR_KEYS.view3dCustom`).
 */
const BROWSER_KEYS: ReadonlySet<string> = new Set(['view.threeD.custom1'])

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
  safe: 'kit:safe-areas',
  alignTo: 'kit:align-to',
  gridSettings: 'kit:grid-settings',
  newGuide: 'kit:new-guide',
  roi: 'kit:region-of-interest',
  clearRoi: 'kit:clear-region-of-interest',
  skipFrames: 'kit:skip-frames',
  purgeCache: 'kit:purge-cache',
  addToQueue: 'kit:add-to-render-queue',
  rove: 'kit:rove-across-time',
  convertExpression: 'kit:convert-expression',
  renderQueue: 'kit:render-queue',
  cacheAhead: 'kit:cache-ahead',
  wiggler: 'kit:wiggler',
  smoother: 'kit:smoother',
  collectFiles: 'kit:collect-files',
  twoViews: 'kit:two-views',
} as const

/** View's switches (the engine's own rows, answered by the editor) and the overlay each turns on and off. */
export const OVERLAY_ROWS: Readonly<Record<string, keyof Overlays>> = {
  'view.rulers': 'rulers',
  'view.guides': 'guides',
  'view.lockGuides': 'lockGuides',
  'view.snapToGuides': 'snapGuides',
  'view.grid': 'grid',
  'view.snapToGrid': 'snapGrid',
  [KIT.safe]: 'safe',
}

/** View ▸ Resolution's rows and the divisor each renders at (every n-th pixel). */
export const RESOLUTIONS: ReadonlyArray<readonly [command: string, label: string, divisor: number]> = [
  ['view.res.full', 'Full', 1],
  ['view.res.half', 'Half', 2],
  ['view.res.third', 'Third', 3],
  ['view.res.quarter', 'Quarter', 4],
]

/** View ▸ 3D View's rows (`view.threeD.*`) and the view each shows (`view.get3D`'s `view`). */
export const THREE_D_VIEWS: ReadonlyArray<readonly [command: string, label: string, view: string]> = [
  ['view.threeD.activeCamera', 'Active Camera', 'activeCamera'],
  ['view.threeD.front', 'Front', 'front'],
  ['view.threeD.left', 'Left', 'left'],
  ['view.threeD.top', 'Top', 'top'],
  ['view.threeD.back', 'Back', 'back'],
  ['view.threeD.right', 'Right', 'right'],
  ['view.threeD.bottom', 'Bottom', 'bottom'],
  ['view.threeD.custom1', 'Custom View 1', 'custom1'],
  ['view.threeD.custom2', 'Custom View 2', 'custom2'],
  ['view.threeD.custom3', 'Custom View 3', 'custom3'],
]

/** The 3D views' keys that work in every browser (the editor's own, keys.ts), shown in the menu. */
const VIEW_KEYS: Readonly<Record<string, string>> = { 'view.threeD.front': 'Alt+Shift+1', 'view.threeD.custom1': 'Alt+Shift+2', 'view.threeD.activeCamera': 'Alt+Shift+3' }

/** What View ▸ 3D View needs to know: whether the composition has 3D layers, the view shown, and whether it has a camera layer. */
export type ThreeDView = { has3D: boolean; view: string; camera: boolean }

/** A switch's label: what a click on it does now. */
const OVERLAY_LABELS: Record<keyof Overlays, readonly [on: string, off: string]> = {
  rulers: ['Hide Rulers', 'Show Rulers'],
  guides: ['Hide Guides', 'Show Guides'],
  lockGuides: ['Unlock Guides', 'Lock Guides'],
  snapGuides: ['Stop Snapping to Guides', 'Snap to Guides'],
  grid: ['Hide Grid', 'Show Grid'],
  snapGrid: ['Stop Snapping to Grid', 'Snap to Grid'],
  safe: ['Hide Title/Action Safe', 'Show Title/Action Safe'],
}

/**
 * Rows the editor runs as one engine command with fixed parameters (Layer ▸ Transform, Time and
 * Align, Animation ▸ Sequence Layers). Each is shown only where it would change something
 * (`wouldChange`), so a shown row always does.
 */
export const KIT_RUNS: Readonly<Record<string, readonly [command: string, params: Record<string, unknown>]>> = {
  'kit:transform-reset': ['layer.transform', { op: 'reset' }],
  'kit:fit': ['layer.transform', { op: 'fit' }],
  'kit:fit-width': ['layer.transform', { op: 'fitWidth' }],
  'kit:fit-height': ['layer.transform', { op: 'fitHeight' }],
  'kit:flip-h': ['layer.transform', { op: 'flipH' }],
  'kit:flip-v': ['layer.transform', { op: 'flipV' }],
  'kit:move-in': ['layer.timing', { op: 'moveInToTime' }],
  'kit:move-out': ['layer.timing', { op: 'moveOutToTime' }],
  'kit:trim-in': ['layer.timing', { op: 'trimInToTime' }],
  'kit:trim-out': ['layer.timing', { op: 'trimOutToTime' }],
  'kit:align-left': ['layer.align', { edge: 'left' }],
  'kit:align-hcenter': ['layer.align', { edge: 'hcenter' }],
  'kit:align-right': ['layer.align', { edge: 'right' }],
  'kit:align-top': ['layer.align', { edge: 'top' }],
  'kit:align-vcenter': ['layer.align', { edge: 'vcenter' }],
  'kit:align-bottom': ['layer.align', { edge: 'bottom' }],
  'kit:distribute-left': ['layer.distribute', { mode: 'left' }],
  'kit:distribute-hcenter': ['layer.distribute', { mode: 'hcenter' }],
  'kit:distribute-right': ['layer.distribute', { mode: 'right' }],
  'kit:distribute-top': ['layer.distribute', { mode: 'top' }],
  'kit:distribute-vcenter': ['layer.distribute', { mode: 'vcenter' }],
  'kit:distribute-bottom': ['layer.distribute', { mode: 'bottom' }],
  'kit:sequence': ['layer.sequence', {}],
}

/** Engine rows that are hidden, like `KIT_RUNS`, where they would change nothing. */
export const TRIED_ROWS: Readonly<Record<string, Record<string, unknown>>> = {
  'layer.centerAnchor': {},
  // Add Expression acts on the properties selected in the timeline: hidden until one is.
  'prop.setExpression': {},
  'layer.freezeFrame': {},
  // Layer Styles ▸ Remove All: hidden unless a selected layer has a style.
  'layer.style.removeAll': {},
  // Enable Time Remapping: hidden unless a footage or composition layer is selected.
  'layer.enableTimeRemap': {},
  // Convert to Bezier Path: hidden unless a selected shape layer has a Rectangle, Ellipse or Polystar Path.
  'path.convertToBezier': {},
  // Reset 3D View on the active camera: hidden unless the camera layer would change (the other views always reset).
  'view.reset3DView': {},
  // Remove Unused Footage: hidden while every footage item and solid is used.
  'file.removeUnusedFootage': {},
  // Exponential Scale: hidden unless two Scale keyframes are selected.
  'keyframe.exponentialScale': {},
}

/** Layer ▸ Arrange's rows and where each moves the selected layers (`layer.arrange`'s `to`). */
export const ARRANGE: Record<string, 'front' | 'forward' | 'backward' | 'back'> = {
  [KIT.arrangeFront]: 'front',
  [KIT.arrangeForward]: 'forward',
  [KIT.arrangeBackward]: 'backward',
  [KIT.arrangeBack]: 'back',
}

/** The View menu's switch rows, labelled for what they do with `overlays` as they are. */
function overlayEntries(overlays: Overlays): Entry[] {
  const row = (command: string): Entry => {
    const key = OVERLAY_ROWS[command]
    return [OVERLAY_LABELS[key][overlays[key] ? 0 : 1], command]
  }
  return [
    row('view.rulers'),
    SEPARATOR,
    row('view.guides'),
    ['New Guide…', KIT.newGuide],
    row('view.snapToGuides'),
    row('view.lockGuides'),
    ['Clear Guides', 'view.clearGuides'],
    SEPARATOR,
    row('view.grid'),
    row('view.snapToGrid'),
    ['Grid Settings…', KIT.gridSettings],
    row(KIT.safe),
  ]
}

/** What Layer ▸ Align lines layers up with: the composition's frame, or the selected layers' bounds. */
export type AlignTo = 'composition' | 'selection'

/** No layer styles on the selected layer. */
const NO_STYLES: ReadonlySet<string> = new Set()

/** A Layer ▸ Layer Styles row: marked (Added) when the selected layer has the style (choosing it again turns it on). */
const styleLabel = (label: string, id: string, styled: ReadonlySet<string>) => (styled.has(id) ? `${label} (Added)` : label)

/** What the Animation menu knows of the selected keyframes: whether any moves through space (Rove Across Time applies) and whether they all rove. */
export type SelectedKeys = { spatial: boolean; roving: boolean; /** Whether turning roving on would change any (not on a first or last keyframe). */ rovable?: boolean; /** How many keyframes are selected (Wiggler takes two, Smoother three). */ count?: number }
const NO_KEYS: SelectedKeys = { spatial: false, roving: false }

/** The Keyframe Assistant rows, saying in the label what they wait for when they cannot run yet (choosing one then says it again). */
export const ASSISTANT_NEEDS = {
  wiggler: 'Wiggler… (Select Two or More Keyframes)',
  smoother: 'Smoother… (Select Three or More Keyframes)',
  convert: 'Convert Expression to Keyframes (Select a Property with an Expression)',
  convertOff: 'Convert Expression to Keyframes (Turn the Expression On First)',
} as const

/** Whether Convert Expression to Keyframes can run: yes, not until the selected expression is turned on, or not without a selected property with an expression. */
export type Convertible = boolean | 'off'

/**
 * The Animation menu's rows that cannot run now (their labels say why): the SDK has no disabled
 * state for a kit's own rows, so `waitingRowStyles` greys them; choosing one still says why.
 */
export function waitingRows(keys: SelectedKeys, convertible: Convertible): string[] {
  const rows: string[] = []
  if (convertible !== true) rows.push(KIT.convertExpression)
  if ((keys.count ?? 0) < 2) rows.push(KIT.wiggler)
  if ((keys.count ?? 0) < 3) rows.push(KIT.smoother)
  if (!keys.roving && keys.rovable === false) rows.push(KIT.rove)
  return rows
}

/** A style sheet greying `rows` (`kit:…` ids) in the menu bar, keyed by their Kit UI ids (`control:…`). */
export function waitingRowStyles(rows: readonly string[]): string {
  // The SDK's look of a disabled row (`.kui-menu-item:disabled`: faded, no hand cursor, no hover).
  return rows
    .map((row) => `.kui-menu-item[data-kit-ui-id="control:${row.replace(/^kit:/, '')}"] { opacity: 0.4; cursor: default; }\n.kui-menu-item[data-kit-ui-id="control:${row.replace(/^kit:/, '')}"]:hover { background: transparent; }`)
    .join('\n')
}


const MENUS = (overlays: Overlays, alignTo: AlignTo, skipFrames: boolean, remapOn: boolean, styled: ReadonlySet<string>, keys: SelectedKeys, cacheAhead: boolean, twoViews: boolean, convertible: Convertible, labelNow: string | null): ReadonlyArray<readonly [string, readonly Entry[]]> => [
  [
    'File',
    [
      ['Import…', 'file.import', 'Cmd+I'],
      ['New', [['New Folder', 'project.newFolder']]],
      ['Dependencies', [['Collect Files…', KIT.collectFiles], ['Remove Unused Footage', 'file.removeUnusedFootage']]],
      SEPARATOR,
      ['Export…', 'file.exportMovie'],
      ['Export', [['Lottie JSON…', 'file.exportLottie'], ['Add to Render Queue', KIT.addToQueue]]],
      ['Render Queue', KIT.renderQueue],
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
      ['Clear', 'edit.delete'],
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
      // The selected layers' colour label (as on the timeline's right-click menu).
      // The current one (every selected layer has it) says so; the menu also ticks it.
      ['Label', [...LABELS.map(([name]) => [name === labelNow ? `${name} (Current)` : name, labelRow(name)] as const), SEPARATOR, ['Select Label Group', 'edit.selectLabelGroup']]],
      SEPARATOR,
      ['Pre-compose', 'layer.precompose'],
      ['Reveal Layer Source in Project', 'layer.revealSource'],
      ['Mask', [['New Mask', 'layer.addMask']]],
      ['Mask and Shape Path', [['Convert to Bezier Path', 'path.convertToBezier']]],
      [
        'Layer Styles',
        [
          [styleLabel('Drop Shadow', 'dropShadow', styled), 'layer.style.dropShadow'],
          [styleLabel('Inner Shadow', 'innerShadow', styled), 'layer.style.innerShadow'],
          [styleLabel('Outer Glow', 'outerGlow', styled), 'layer.style.outerGlow'],
          [styleLabel('Inner Glow', 'innerGlow', styled), 'layer.style.innerGlow'],
          [styleLabel('Bevel and Emboss', 'bevelEmboss', styled), 'layer.style.bevelEmboss'],
          [styleLabel('Satin', 'satin', styled), 'layer.style.satin'],
          [styleLabel('Color Overlay', 'colorOverlay', styled), 'layer.style.colorOverlay'],
          [styleLabel('Gradient Overlay', 'gradientOverlay', styled), 'layer.style.gradientOverlay'],
          [styleLabel('Stroke', 'stroke', styled), 'layer.style.stroke'],
          SEPARATOR,
          ['Remove All', 'layer.style.removeAll'],
        ],
      ],
      [
        'Transform',
        [
          ['Reset', 'kit:transform-reset'],
          ['Center Anchor Point in Layer Content', 'layer.centerAnchor'],
          SEPARATOR,
          ['Fit to Comp', 'kit:fit', 'Cmd+Alt+F'],
          ['Fit to Comp Width', 'kit:fit-width'],
          ['Fit to Comp Height', 'kit:fit-height'],
          SEPARATOR,
          ['Flip Horizontal', 'kit:flip-h'],
          ['Flip Vertical', 'kit:flip-v'],
        ],
      ],
      [
        'Time',
        [
          [remapOn ? 'Disable Time Remapping' : 'Enable Time Remapping', 'layer.enableTimeRemap', 'Cmd+Alt+T'],
          ['Time Stretch…', 'layer.timeStretch'],
          ['Time-Reverse Layer', 'layer.timeReverse'],
          ['Freeze Frame', 'layer.freezeFrame'],
          SEPARATOR,
          ['Move In Point to Current Time', 'kit:move-in', '['],
          ['Move Out Point to Current Time', 'kit:move-out', ']'],
          ['Trim In Point to Current Time', 'kit:trim-in', 'Alt+['],
          ['Trim Out Point to Current Time', 'kit:trim-out', 'Alt+]'],
        ],
      ],
      [
        'Align',
        [
          [alignTo === 'composition' ? 'Align to Selection Instead' : 'Align to Composition Instead', KIT.alignTo],
          SEPARATOR,
          ['Align Left', 'kit:align-left'],
          ['Align Horizontal Centers', 'kit:align-hcenter'],
          ['Align Right', 'kit:align-right'],
          ['Align Top', 'kit:align-top'],
          ['Align Vertical Centers', 'kit:align-vcenter'],
          ['Align Bottom', 'kit:align-bottom'],
          SEPARATOR,
          ['Distribute Left Edges', 'kit:distribute-left'],
          ['Distribute Horizontal Centers', 'kit:distribute-hcenter'],
          ['Distribute Right Edges', 'kit:distribute-right'],
          ['Distribute Top Edges', 'kit:distribute-top'],
          ['Distribute Vertical Centers', 'kit:distribute-vcenter'],
          ['Distribute Bottom Edges', 'kit:distribute-bottom'],
        ],
      ],
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
      ['Keyframe Interpolation', [...INTERPOLATIONS.map((row) => [row.label, row.command] as const), SEPARATOR, ['Keyframe Interpolation…', 'keyframe.interpolation']]],
      ['Keyframe Velocity…', 'keyframe.velocity'],
      SEPARATOR,
      ['Easy Ease', 'keyframe.easyEase'],
      ['Easy Ease In', 'keyframe.easyEaseIn'],
      ['Easy Ease Out', 'keyframe.easyEaseOut'],
      ['Toggle Hold Keyframe', 'keyframe.toggleHold'],
      [keys.roving ? 'Rove Across Time (On)' : keys.rovable === false ? 'Rove Across Time (Not on the First or Last Keyframe)' : 'Rove Across Time', KIT.rove],
      ['Time-Reverse Keyframes', 'keyframe.timeReverse'],
      ['Sequence Layers…', 'kit:sequence'],
      [
        'Keyframe Assistant',
        [
          ['Exponential Scale', 'keyframe.exponentialScale'],
          [convertible === true ? 'Convert Expression to Keyframes' : convertible === 'off' ? ASSISTANT_NEEDS.convertOff : ASSISTANT_NEEDS.convert, KIT.convertExpression],
          [(keys.count ?? 0) >= 2 ? 'Wiggler…' : ASSISTANT_NEEDS.wiggler, KIT.wiggler],
          [(keys.count ?? 0) >= 3 ? 'Smoother…' : ASSISTANT_NEEDS.smoother, KIT.smoother],
        ],
      ],
      SEPARATOR,
      ['Select Equal Keyframes', 'keyframe.selectEqual'],
      ['Select Previous Keyframes', 'keyframe.selectPrevious'],
      ['Select Following Keyframes', 'keyframe.selectFollowing'],
      SEPARATOR,
      ['Separate Dimensions', 'prop.separateDimensions'],
      ['Add Expression', 'prop.setExpression'],
      SEPARATOR,
      ['Apply Animation Preset…', 'animation.applyPreset'],
      ['Save Animation Preset…', 'animation.savePreset'],
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
      ['Resolution', RESOLUTIONS.map(([command, label]) => [label, command] as const)],
      // The menu shows the keys that work in a browser (Alt+Shift+1, 2, 3; F10 to F12 are often the browser's or the system's).
      ['3D View', [...THREE_D_VIEWS.map(([command, label]) => (VIEW_KEYS[command] ? ([label, command, VIEW_KEYS[command]] as const) : ([label, command] as const))), SEPARATOR, ['Reset 3D View', 'view.reset3DView']]],
      [twoViews ? 'One View' : 'Two Views', KIT.twoViews],
      ['Region of Interest', KIT.roi],
      ['Clear Region of Interest', KIT.clearRoi],
      SEPARATOR,
      [skipFrames ? 'Play Every Frame' : 'Skip Frames to Keep Real Time', KIT.skipFrames],
      [cacheAhead ? 'Stop Caching in Background' : 'Cache Work Area in Background', KIT.cacheAhead],
      ['Purge Preview Cache', KIT.purgeCache],
      SEPARATOR,
      ...overlayEntries(overlays),
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
export function menuRows(effects: readonly MotionEffect[], overlays: Overlays, alignTo: AlignTo, skipFrames = true, remapOn = false, styled: ReadonlySet<string> = NO_STYLES, keys: SelectedKeys = NO_KEYS, cacheAhead = false, twoViews = false, convertible: Convertible = true, labelNow: string | null = null): MenuRow[] {
  return MENUS(overlays, alignTo, skipFrames, remapOn, styled, keys, cacheAhead, twoViews, convertible, labelNow).flatMap(([name, entries]) => [...rowsOf([name], entries), ...(name === 'Layer' ? effectRows(effects) : [])])
}

/** Menus kept in a read-only document. */
export const VIEW_MENUS = ['View']

/** The rows each menu tab shows in Single line's row, by menu. */
export const MOTION_PRIMARY: Record<string, readonly string[]> = {
  File: ['file.import', 'file.exportMovie', KIT.saveFrame],
  Edit: ['edit.undo', 'edit.redo', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.duplicate', 'edit.splitLayer', 'edit.delete'],
  Composition: ['comp.new', 'comp.settings', 'playback.toggle', KIT.addMarker, 'comp.trimToWorkArea'],
  Layer: ['layer.newText', 'layer.newSolid', 'layer.newShape', 'layer.newNull', 'layer.newAdjustment', 'layer.precompose', 'layer.addMask'],
  Effect: ['kit:find-effect', 'effect.applyLast'],
  Animation: ['keyframe.easyEase', 'keyframe.toggleHold'],
  View: ['view.zoomIn', 'view.zoomOut', KIT.zoomFit, 'time.set', 'time.start', 'time.end', 'time.previousKey', 'time.nextKey'],
}

/**
 * Rows hidden where they would have nothing to act on (so a shown row always does something):
 * Arrange and Deselect All without a selection, Select All with every layer selected, Show All
 * Video with none hidden, Unlock All with none locked, Trim to Work Area while the work area is
 * the whole composition, Solid Settings unless the selected layer is a solid, effects without a
 * selection, Last Effect until one was applied here, Fit in Window while the view is fitted and
 * Zoom 100% at 100%, the resolution the canvas shows, View ▸ 3D View without 3D layers (and the view shown), the Keyframe Interpolation rows without selected keyframes, and the
 * `KIT_RUNS` and `TRIED_ROWS` rows that would change nothing now (`idle`).
 */
export function hiddenRow(info: MotionInfo | null, lastEffect: boolean, view: { fitted: boolean; actual: boolean; preview: number; alignTo: AlignTo; threeD: ThreeDView; region: boolean; cached: boolean; keys?: SelectedKeys }, keysSelected: boolean, idle: ReadonlySet<string>) {
  const layers = info?.layers ?? []
  const selected = layers.some((layer) => layer.selected)
  const comp = info?.comp
  const hidden = new Set<string>()
  if (!selected) for (const command of [...Object.keys(ARRANGE), 'edit.deselectAll', 'edit.selectLabelGroup', ...LABELS.map(([name]) => labelRow(name))]) hidden.add(command)
  // Layer ▸ Label lists every colour (the current one ticked, `currentLabel`); Select Label Group
  // only when it would select another layer.
  const chosen = layers.filter((layer) => layer.selected)
  if (!layers.some((layer) => !layer.selected && chosen.some((each) => each.label === layer.label))) hidden.add('edit.selectLabelGroup')
  if (layers.length && layers.every((layer) => layer.selected)) hidden.add('edit.selectAll')
  if (!layers.some((layer) => !layer.switches.video)) hidden.add('layer.showAllVideo')
  if (!layers.some((layer) => layer.switches.locked)) hidden.add('layer.unlockAll')
  if (layers.find((layer) => layer.selected)?.type !== 'Solid') hidden.add('layer.settings')
  if (!['Footage', 'Composition', 'Solid'].includes(layers.find((layer) => layer.selected)?.type ?? '')) hidden.add('layer.revealSource')
  if (!comp) hidden.add(KIT.addToQueue)
  if (!comp || (comp.workArea[0] <= 1e-6 && comp.workArea[1] >= comp.duration - 1e-6)) hidden.add('comp.trimToWorkArea')
  if (!selected || !lastEffect) hidden.add('effect.applyLast')
  if (view.fitted) hidden.add(KIT.zoomFit)
  if (view.actual) hidden.add(KIT.zoom100)
  // The resolution the canvas shows now.
  for (const [command, , divisor] of RESOLUTIONS) if (divisor === view.preview) hidden.add(command)
  // View ▸ 3D View: only with 3D layers, and not the view shown now; Reset 3D View resets the
  // view shown (the active camera's needs a camera layer).
  for (const [command, , shown] of THREE_D_VIEWS) if (!view.threeD?.has3D || shown === view.threeD.view) hidden.add(command)
  // Two Views: only with 3D layers.
  if (!view.threeD?.has3D) hidden.add(KIT.twoViews)
  // On the active camera, Reset 3D View resets the camera layer: shown only when that would change it.
  if (!view.threeD?.has3D || (view.threeD.view === 'activeCamera' && (!view.threeD.camera || idle.has('view.reset3DView')))) hidden.add('view.reset3DView')
  // Clear Region of Interest with none set; Purge Preview Cache with no frame kept.
  if (!view.region) hidden.add(KIT.clearRoi)
  if (!view.cached) hidden.add(KIT.purgeCache)
  if (!keysSelected) for (const row of INTERPOLATIONS) hidden.add(row.command)
  // Rove Across Time: only for keyframes that move a layer through space.
  if (!keysSelected || !view.keys?.spatial) hidden.add(KIT.rove)
  // Wiggler (two or more selected keyframes), Smoother (three or more) and Convert Expression to
  // Keyframes (a selected property with an expression turned on) stay in the menu: their labels
  // say what they wait for (`ASSISTANT_NEEDS`).
  for (const command of idle) if (command !== 'view.reset3DView') hidden.add(command)
  // Aligning to the selection needs two or more selected layers.
  if (view.alignTo === 'selection' && layers.filter((layer) => layer.selected).length < 2) for (const command of Object.keys(KIT_RUNS)) if (command.startsWith('kit:align-')) hidden.add(command)
  const key = [...hidden].sort().join(' ') + (selected ? '' : ' effects')
  return { key, hidden: (row: MenuRow) => hidden.has(row.command) || (!selected && row.command.startsWith(EFFECT_PREFIX)) }
}
