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
  /**
   * A drop-down's options (English, translated by the caller); `key`, when set, is the editor's
   * own string for an option whose words read differently here (a wave's Square is 方波, not 正方形).
   */
  options?: Array<{ label: string; value: number; key?: string }>
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

/** The drop-downs that choose a wave's shape (Wave Warp, Tone, Modulator): their Sine, Square… are waves. */
const WAVE_PARAMS = new Set(['waveType', 'waveformOptions', 'modulationType'])
const WAVES = new Set(['Sine', 'Square', 'Triangle', 'Saw', 'Sawtooth'])

/** A drop-down's options, wave shapes named as waves. */
function optionsOf(param: EffectParam) {
  const options = param.options
  if (!options || !WAVE_PARAMS.has(param.id) || !options.some((option) => option.label === 'Sine')) return options
  return options.map((option) => (WAVES.has(option.label) ? { ...option, key: `wave.${option.label}` } : option))
}

/** The meta of an effect's parameter from `effect.list`. */
export function effectMeta(param: EffectParam | undefined): PropMeta | null {
  if (!param) return null
  const slider = param.sliderMin !== undefined && param.sliderMax !== undefined && param.sliderMax > param.sliderMin ? ([param.sliderMin, param.sliderMax] as [number, number]) : undefined
  return { min: param.min, max: param.max, slider, decimals: param.decimals, unit: param.unit === 'px' ? 'px' : undefined, options: optionsOf(param), fallback: param.default }
}

/** A number of the engine's range, or undefined for an open end. */
const bound = (value: number | null | undefined) => (typeof value === 'number' && Number.isFinite(value) ? value : undefined)

/**
 * A property's meta: its effect's (`params` of the effect it belongs to), else what the engine's
 * tree declares (a number's range and slider, a drop-down's labels) with the units the editor
 * shows. `parent` is the group's match name.
 */
export function propMeta(node: TreeNode, parent: TreeNode | undefined, params: readonly EffectParam[] | undefined): PropMeta | null {
  if (params) return effectMeta(params.find((param) => param.id === node.match)) ?? layerSourceMeta(node, params)
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

/**
 * What of a chosen layer an effect sees, the three values of a layer setting's source companion
 * (`<layer setting>Source`, which `effect.list` does not list): the layer itself, its masks only,
 * or its effects and masks (the engine reads every value from 2 up as the last).
 */
export const LAYER_SOURCE_OPTIONS = [
  { label: 'Source', value: 0 },
  { label: 'Masks', value: 1 },
  { label: 'Effects & Masks', value: 2 },
]

/** The meta of a layer setting's source companion, or null for any other property. */
function layerSourceMeta(node: TreeNode, params: readonly EffectParam[]): PropMeta | null {
  const match = node.match ?? ''
  if (node.type !== 'enum' || !match.endsWith('Source')) return null
  const base = match.slice(0, -'Source'.length)
  const layer = params.some((param) => param.type === 'layer' && (param.id === base || param.id.endsWith(`/${base}`)))
  return layer ? { options: LAYER_SOURCE_OPTIONS, fallback: 2 } : null
}
