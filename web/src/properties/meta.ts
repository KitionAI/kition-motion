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
  /** A layer setting's source companion: the layer setting it belongs to (its name reads "<setting> Source"). */
  source?: { id: string; name: string }
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

/** A setting's id inside its effect (`blurMap/placement`), from its path in the layer's tree (`effects/#1/blurMap/placement`). */
export function effectParamId(path: string) {
  return path.replace(/^effects\/[^/]+\//, '')
}

/**
 * The effect a property belongs to, as `effect.list` lists it: its id and settings.
 */
export type EffectEntry = { id: string; params?: readonly EffectParam[] }

/**
 * A property's meta: its effect's (from `effect`, the effect it belongs to: a layer setting's
 * source companion, or the setting `effect.list` lists at the same id), else what the engine's
 * tree declares (a number's range and slider, a drop-down's labels) with the units the editor
 * shows. `parent` is the group's match name.
 */
export function propMeta(node: TreeNode, parent: TreeNode | undefined, effect: EffectEntry | undefined): PropMeta | null {
  if (effect) {
    const params = effect.params ?? []
    const id = effectParamId(node.path)
    const source = node.type === 'enum' ? layerSourceMeta(id, params) : null
    if (source) return source
    const meta = inEffect(effect.id, id, effectMeta(params.find((param) => param.id === id)))
    if (meta?.options || node.type !== 'enum') return meta ?? treeMeta(node, parent)
    const hidden = HIDDEN_CHOICES[`${effect.id}/${id}`]
    return hidden ? { ...meta, options: hidden.map((label, value) => ({ label, value })) } : (meta ?? treeMeta(node, parent))
  }
  return treeMeta(node, parent)
}

/** What the engine's tree declares about a property outside an effect's list. */
function treeMeta(node: TreeNode, parent: TreeNode | undefined): PropMeta | null {
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
 * The choices of the drop-downs an effect keeps out of its own list (hidden settings the editor
 * still shows), by `<effect>/<setting id>`: Curves' Channel picks which curve its graph edits.
 */
const HIDDEN_CHOICES: Record<string, readonly string[]> = {
  'ec.color.curves/channel': ['RGB', 'Red', 'Green', 'Blue', 'Alpha'],
}

/**
 * What of a chosen layer an effect sees, the three values of a layer setting's source companion
 * (the engine's `layer_source_id`: `<layer setting id>Source`, beside it in its group, either
 * added by the engine or declared by the effect with its own default): the layer itself, its
 * masks only, or its effects and masks (the engine reads every value from 2 up as the last).
 */
export const LAYER_SOURCE_OPTIONS = [
  { label: 'Source', value: 0 },
  { label: 'Masks', value: 1 },
  { label: 'Effects & Masks', value: 2 },
]

/** The layer setting whose source companion has id `id` (inside its effect), if it is one. */
export function layerSettingOf(id: string, params: readonly EffectParam[]): EffectParam | undefined {
  return params.find((param) => param.type === 'layer' && `${param.id}Source` === id)
}

/** The meta of a layer setting's source companion (`id` inside its effect), or null for any other setting. */
function layerSourceMeta(id: string, params: readonly EffectParam[]): PropMeta | null {
  const layer = layerSettingOf(id, params)
  if (!layer) return null
  const declared = params.find((param) => param.id === id)
  return { options: LAYER_SOURCE_OPTIONS, fallback: typeof declared?.default === 'number' ? declared.default : 2, source: { id: layer.id, name: layer.name } }
}

/**
 * Settings, groups and options whose words read differently in one effect than the same English
 * elsewhere, by `<effect>/<id inside it>` (an option: `<effect>/<id>=<English option>`): the
 * editor's own string (`strings.ts`). Color's Tone group is its tones (light and dark), not the
 * audio effect Tone, and its HSL Secondary's Key is a selection, not a keyframe; Radio Waves'
 * Orientation turns the wave and Direction is where it travels.
 */
const IN_EFFECT: Record<string, string> = {
  'ec.color.grade/basicCorrection/tone': 'fx.grade.tone',
  'ec.color.grade/creative/look': 'fx.grade.look',
  'ec.color.grade/hslSecondary/key': 'fx.grade.key',
  'ec.generate.radiowaves/waveMotion/orientation': 'fx.radioWaves.orientation',
  'ec.generate.radiowaves/waveMotion/direction': 'fx.radioWaves.direction',
  'ec.blur.cameralens/blurMap/placement=Center': 'fx.lensBlur.center',
  'ec.blur.cameralens/blurMap/placement=Stretch': 'fx.lensBlur.stretch',
  'ec.distort.ccbendit/renderPrestart=None': 'fx.bendIt.none',
  'ec.distort.ccbendit/renderPrestart=Static': 'fx.bendIt.static',
  'ec.generate.advancedlightning/lightningType=Direction': 'fx.lightning.direction',
  // Particle Playground's property mappers hold each colour's Min and Max (and Operator) in one
  // group: each named with its colour (Red Min, Green Min, …), not three alike.
  ...Object.fromEntries(
    ['persistentPropertyMapper', 'ephemeralPropertyMapper'].flatMap((mapper) =>
      ['red', 'green', 'blue'].flatMap((colour) =>
        ['Min', 'Max', ...(mapper === 'ephemeralPropertyMapper' ? ['Operator'] : [])].map((what) => [`ec.sim.particleplayground/${mapper}/${colour}${what}`, `fx.playground.${colour}${what}`]),
      ),
    ),
  ),
}

/** An effect setting's meta with the options that read differently in this effect keyed to the editor's own strings. */
function inEffect(effect: string, id: string, meta: PropMeta | null): PropMeta | null {
  const options = meta?.options
  if (!meta || !options?.some((option) => IN_EFFECT[`${effect}/${id}=${option.label}`])) return meta
  return { ...meta, options: options.map((option) => ({ ...option, key: IN_EFFECT[`${effect}/${id}=${option.label}`] ?? option.key })) }
}

type Phrase = (text: string) => string
type Strings = (key: string, vars?: Record<string, string | number>) => string

/**
 * The name a property or group shows (Properties and the timeline): the engine's English name
 * translated, but a layer setting's source companion reads "<setting> Source" from its layer
 * setting's own name, a name that reads differently in its effect takes that effect's words, and
 * a 3D layer's Rotation (beside its X and Y Rotation) reads Z Rotation.
 */
export function nodeName(node: TreeNode, effect: EffectEntry | undefined, meta: PropMeta | null, threeD: boolean, phrase: Phrase, t: Strings): string {
  if (meta?.source) return t('effect.layerSource', { name: nodeName({ name: meta.source.name, path: `effects/#/${meta.source.id}` }, effect, null, threeD, phrase, t) })
  if (effect) {
    const key = IN_EFFECT[`${effect.id}/${effectParamId(node.path)}`]
    if (key) return t(key)
  }
  if (threeD && node.path === 'transform/rotation') return phrase('Z Rotation')
  return phrase(node.name)
}
