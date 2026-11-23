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
const THREE_D_ONLY = new Set(['materialOptions', 'transform/orientation', 'transform/rotationX', 'transform/rotationY'])

/** The groups open until the person closes them. */
const OPEN_BY_DEFAULT = new Set(['transform'])

/** Whether the group at `path` is open: its default, flipped when the person toggled it. */
export function groupOpen(path: string, toggled: boolean) {
  return OPEN_BY_DEFAULT.has(path) !== toggled
}

function shown(node: TreeNode, layer: MotionLayer) {
  return layer.switches.threeD || !THREE_D_ONLY.has(node.path)
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
    return layer.switches.threeD ? parts : parts.slice(0, 2)
  }
  return null
}
