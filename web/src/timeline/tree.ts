import type { MotionLayer } from '../motion'

/** A group or a property of `layer.tree`: groups have `children`, properties a `type` and a `value`. */
export type TreeNode = {
  name: string
  path: string
  uid?: number
  match?: string
  type?: string
  value?: unknown
  keys?: Array<{ time: number }>
  children?: TreeNode[]
  /** An applied effect's id (`ec.blur.gaussian`), on its group. */
  effect?: string
  /** Whether a group (an effect, a mask, a shape group) is switched on. */
  enabled?: boolean
  /** A drop-down property's labels, in the order of its values (English, from the engine). */
  options?: string[]
  /** A number's range, slider range and decimals, when the engine declares them. */
  range?: { min: number | null; max: number | null; sliderMin: number | null; sliderMax: number | null; decimals: number }
  /** A text property's character and paragraph settings at the asked time. */
  style?: TextStyle
  /** A mask group's mode and whether it is inverted. */
  mask?: { mode: string; inverted: boolean }
  /** The property's expression, when it has one, and whether it is on. */
  expression?: string
  expressionEnabled?: boolean
}

/** A text layer's character and paragraph settings, as `layer.tree` reports them on its Source Text. */
export type TextStyle = {
  family: string
  style: string
  size: number
  fill: number[]
  stroke: number[]
  strokeWidth: number
  applyFill: boolean
  applyStroke: boolean
  tracking: number
  leading: number
  autoLeading: boolean
  justify: 'left' | 'center' | 'right' | 'other'
}

/** What a layer's twirl shows: its whole property tree, or only some properties (After Effects' U, P, S, R, T). */
export type Twirl = { kind: 'tree' } | { kind: 'only'; key: string; paths: readonly string[] | 'animated' }

/** One row under a layer: a group that opens and closes, or a property. */
export type PropertyRow = { kind: 'group'; node: TreeNode; depth: number; open: boolean } | { kind: 'property'; node: TreeNode; depth: number }

/** The properties the solo keys show (P, S, R, T), as After Effects shows them for a 2D layer. */
export const SOLO_PATHS: Record<string, readonly string[]> = {
  KeyP: ['transform/position'],
  KeyS: ['transform/scale'],
  KeyR: ['transform/rotation'],
  KeyT: ['transform/opacity'],
}

/** Groups and properties only a 3D layer has: hidden on a 2D layer, as After Effects hides them. */
const THREE_D_ONLY = new Set(['materialOptions', 'geometryOptions', 'transform/orientation', 'transform/rotationX', 'transform/rotationY'])

/** The groups open until the person closes them. */
const OPEN_BY_DEFAULT = new Set(['transform'])

/** Whether the group at `path` is open: its default, flipped when the person toggled it. */
export function groupOpen(path: string, toggled: boolean) {
  return OPEN_BY_DEFAULT.has(path) !== toggled
}

/** Whether a layer is in 3D: its 3D switch is on, or it is a camera or a light (always 3D). */
export function isThreeD(layer: MotionLayer) {
  return layer.switches.threeD || layer.type === 'Camera' || layer.type === 'Light'
}

/** Whether a 2D layer shows the node (a 3D layer shows everything). */
export function shown(node: TreeNode, layer: MotionLayer) {
  return isThreeD(layer) || !THREE_D_ONLY.has(node.path)
}

/** Whether a group has anything to show (an empty Masks or Effects group is not drawn). */
function hasContent(node: TreeNode, layer: MotionLayer): boolean {
  return (node.children ?? []).some((child) => shown(child, layer) && (child.children ? hasContent(child, layer) : true))
}

/** The rows a twirled layer shows, top to bottom. `toggled` holds the group paths the person flipped. */
export function propertyRows(root: TreeNode, layer: MotionLayer, twirl: Twirl, toggled: ReadonlySet<string>): PropertyRow[] {
  const rows: PropertyRow[] = []
  if (twirl.kind === 'only') {
    const wanted = new Set(twirl.paths === 'animated' ? layer.animated.map((property) => property.path) : twirl.paths)
    const visit = (node: TreeNode) => {
      for (const child of node.children ?? []) {
        if (!shown(child, layer)) continue
        if (child.children) visit(child)
        else if (wanted.has(child.path)) rows.push({ kind: 'property', node: child, depth: 0 })
      }
    }
    visit(root)
    return rows
  }
  const visit = (node: TreeNode, depth: number) => {
    for (const child of node.children ?? []) {
      if (!shown(child, layer)) continue
      if (!child.children) {
        rows.push({ kind: 'property', node: child, depth })
        continue
      }
      if (!hasContent(child, layer)) continue
      const open = groupOpen(child.path, toggled.has(child.path))
      rows.push({ kind: 'group', node: child, depth, open })
      if (open) visit(child, depth + 1)
    }
  }
  visit(root, 0)
  return rows
}

/** The numbers of a property the timeline edits: a 2D layer shows x and y of a 3D value. */
export function editableNumbers(node: TreeNode, layer: MotionLayer): number[] | null {
  if (node.type === 'scalar' && typeof node.value === 'number') return [node.value]
  if ((node.type === 'vec2' || node.type === 'vec3') && Array.isArray(node.value) && node.value.every((part) => typeof part === 'number')) {
    const parts = node.value as number[]
    return isThreeD(layer) ? parts : parts.slice(0, 2)
  }
  return null
}

/** Each node's parent group in `root`, by path. */
export function parentsOf(root: TreeNode): Map<string, TreeNode> {
  const parents = new Map<string, TreeNode>()
  const visit = (node: TreeNode, depth: number) => {
    if (depth > 32) return
    for (const child of node.children ?? []) {
      parents.set(child.path, node)
      visit(child, depth + 1)
    }
  }
  visit(root, 0)
  return parents
}

/** The effect a node belongs to: its nearest group with an effect id. */
export function effectOf(path: string, parents: ReadonlyMap<string, TreeNode>): string | undefined {
  let node = parents.get(path)
  for (let depth = 0; node && depth < 32; depth += 1) {
    if (node.effect) return node.effect
    node = parents.get(node.path)
  }
  return undefined
}
