import type { TreeNode } from '../timeline/tree'
import { enumOptions } from './enums'

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
  'transform/opacity': { min: 0, max: 100, slider: [0, 100], unit: '%', decimals: 0 },
  'transform/rotation': { unit: '°', decimals: 1 },
  'transform/rotationX': { unit: '°', decimals: 1 },
  'transform/rotationY': { unit: '°', decimals: 1 },
  'transform/scale': { unit: '%', decimals: 1 },
  'fill/opacity': { min: 0, max: 100, slider: [0, 100], unit: '%', decimals: 0 },
  'stroke/opacity': { min: 0, max: 100, slider: [0, 100], unit: '%', decimals: 0 },
  'stroke/width': { min: 0, unit: 'px', decimals: 1 },
  'rect/roundness': { min: 0, unit: 'px', decimals: 1 },
  'rect/size': { min: 0, unit: 'px', decimals: 1 },
  'ellipse/size': { min: 0, unit: 'px', decimals: 1 },
  'trim/start': { min: 0, max: 100, slider: [0, 100], unit: '%', decimals: 1 },
  'trim/end': { min: 0, max: 100, slider: [0, 100], unit: '%', decimals: 1 },
  'trim/offset': { unit: '°', decimals: 1 },
  'masks/opacity': { min: 0, max: 100, slider: [0, 100], unit: '%', decimals: 0 },
}

/** The meta of an effect's parameter from `effect.list`. */
export function effectMeta(param: EffectParam | undefined): PropMeta | null {
  if (!param) return null
  const slider = param.sliderMin !== undefined && param.sliderMax !== undefined && param.sliderMax > param.sliderMin ? ([param.sliderMin, param.sliderMax] as [number, number]) : undefined
  return { min: param.min, max: param.max, slider, decimals: param.decimals, unit: param.unit === 'px' ? 'px' : undefined, options: param.options, fallback: param.default }
}

/**
 * A property's meta: its effect's (`params` of the effect it belongs to), else what the editor
 * knows (ranges and units of Transform and shape properties, drop-down labels of the engine's
 * own properties). `parent` is the group's match name.
 */
export function propMeta(node: TreeNode, parent: TreeNode | undefined, params: readonly EffectParam[] | undefined, masks: readonly string[]): PropMeta | null {
  if (params) return effectMeta(params.find((param) => param.id === node.match))
  const known = KNOWN[`${parent?.match ?? ''}/${node.match ?? ''}`] ?? (node.path.startsWith('masks/') ? KNOWN[`masks/${node.match}`] : undefined)
  if (node.type !== 'enum') return known ?? null
  const labels = enumOptions(node.match, parent?.match, node.path, masks)
  return labels ? { options: labels.map((label, value) => ({ label, value })) } : null
}
