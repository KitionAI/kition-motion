import type { TreeNode } from '../timeline/tree'

/** One parameter of an effect as `effect.list` describes it. */
export type EffectParam = {
  id: string
  name: string
  type: string
  default?: unknown
  options?: Array<{ label: string; value: number }>
  min?: number
  max?: number
  sliderMin?: number
  sliderMax?: number
  decimals?: number
  unit?: string
}

/** What a property's control needs beyond its value: range, slider, decimals, unit, and a drop-down's labels. */
export type PropMeta = {
  min?: number
  max?: number
  /** The slider's range, when the property has one. */
  slider?: [number, number]
  decimals?: number
  unit?: string
  /** A drop-down's options (English, translated by the caller). */
  options?: Array<{ label: string; value: number }>
  /** A default to go back to. */
  fallback?: unknown
}

/** Ranges and units the editor knows for properties outside effects, by `parent/match` or `match`. */
const KNOWN: Record<string, PropMeta> = {
  'transform/opacity': { unit: '%', decimals: 0 },
  'transform/rotation': { unit: '°', decimals: 1 },
  'transform/rotationX': { unit: '°', decimals: 1 },
  'transform/rotationY': { unit: '°', decimals: 1 },
  'transform/scale': { unit: '%', decimals: 1 },
  'fill/opacity': { unit: '%', decimals: 0 },
  'stroke/opacity': { unit: '%', decimals: 0 },
  'stroke/width': { min: 0, unit: 'px', decimals: 1 },
  'rect/roundness': { min: 0, unit: 'px', decimals: 1 },
  'rect/size': { min: 0, unit: 'px', decimals: 1 },
  'ellipse/size': { min: 0, unit: 'px', decimals: 1 },
  'trim/start': { unit: '%', decimals: 1 },
  'trim/end': { unit: '%', decimals: 1 },
  'trim/offset': { unit: '°', decimals: 1 },
  'masks/opacity': { unit: '%', decimals: 0 },
}

/** The meta of an effect's parameter from `effect.list`. */
export function effectMeta(param: EffectParam | undefined): PropMeta | null {
  if (!param) return null
  const slider = param.sliderMin !== undefined && param.sliderMax !== undefined && param.sliderMax > param.sliderMin ? ([param.sliderMin, param.sliderMax] as [number, number]) : undefined
  return { min: param.min, max: param.max, slider, decimals: param.decimals, unit: param.unit === 'px' ? 'px' : undefined, options: param.options, fallback: param.default }
}

/** A number of the engine's range, or undefined for an open end. */
const bound = (value: number | null | undefined) => (typeof value === 'number' && Number.isFinite(value) ? value : undefined)

/**
 * A property's meta: its effect's (`params` of the effect it belongs to), else what the engine's
 * tree declares (a number's range and slider, a drop-down's labels) with the units the editor
 * shows. `parent` is the group's match name.
 */
export function propMeta(node: TreeNode, parent: TreeNode | undefined, params: readonly EffectParam[] | undefined): PropMeta | null {
  if (params) return effectMeta(params.find((param) => param.id === node.match))
  if (node.type === 'enum') return node.options ? { options: node.options.map((label, value) => ({ label, value })) } : null
  const known = KNOWN[`${parent?.match ?? ''}/${node.match ?? ''}`] ?? (node.path.startsWith('masks/') ? KNOWN[`masks/${node.match}`] : undefined)
  const range = node.range
  if (!range) return known ?? null
  const [low, high] = [bound(range.sliderMin), bound(range.sliderMax)]
  return {
    ...known,
    min: bound(range.min),
    max: bound(range.max),
    slider: low !== undefined && high !== undefined && high > low ? [low, high] : undefined,
    decimals: known?.decimals ?? range.decimals,
  }
}
