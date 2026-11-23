/**
 * The option labels of drop-down properties outside effects (`layer.tree` reports their value as
 * an index only), as the engine builds them (crates/engine/project/src/build.rs). Effect
 * parameters take theirs from `effect.list`. English, translated through `PHRASES`.
 */

/** Blending modes as `layer.setBlendMode` takes them, in the order After Effects lists them. */
export const BLEND_MODES = [
  'Normal',
  'Dissolve',
  'Dancing Dissolve',
  '---',
  'Darken',
  'Multiply',
  'Color Burn',
  'Classic Color Burn',
  'Linear Burn',
  'Darker Color',
  '---',
  'Add',
  'Lighten',
  'Screen',
  'Color Dodge',
  'Classic Color Dodge',
  'Linear Dodge',
  'Lighter Color',
  '---',
  'Overlay',
  'Soft Light',
  'Hard Light',
  'Linear Light',
  'Vivid Light',
  'Pin Light',
  'Hard Mix',
  '---',
  'Difference',
  'Classic Difference',
  'Exclusion',
  'Subtract',
  'Divide',
  '---',
  'Hue',
  'Saturation',
  'Color',
  'Luminosity',
  '---',
  'Stencil Alpha',
  'Stencil Luma',
  'Silhouette Alpha',
  'Silhouette Luma',
  'Alpha Add',
  'Luminescent Premul',
] as const

const INTER_CHAR_BLEND = ['Normal', 'Darken', 'Multiply', 'Color Burn', 'Linear Burn', 'Darker Color', 'Add', 'Lighten', 'Screen', 'Color Dodge', 'Lighter Color', 'Overlay', 'Soft Light', 'Hard Light', 'Linear Light', 'Vivid Light', 'Pin Light', 'Hard Mix', 'Difference', 'Exclusion', 'Subtract', 'Divide', 'Hue', 'Saturation', 'Color', 'Luminosity']
const SHADOWS = ['Off', 'On', 'Only']
const BASED_ON = ['Characters', 'Characters Excluding Spaces', 'Words', 'Lines']
const SEL_MODES = ['Add', 'Subtract', 'Intersect', 'Min', 'Max', 'Difference']
const CAPS = ['Butt Cap', 'Round Cap', 'Projecting Cap']
const JOINS = ['Miter Join', 'Round Join', 'Bevel Join']
const COMPOSITE = ['Below Previous in Same Group', 'Above Previous in Same Group']

/**
 * Options by the property's match name, or by `parent/match` where the same name means different
 * things (a Polystar's Type, a gradient's Type).
 */
const OPTIONS: Record<string, readonly string[]> = {
  anchorGrouping: ['Character', 'Word', 'Line', 'All'],
  fillStroke: ['Per Character Palette', 'All Fills Over All Strokes', 'All Strokes Over All Fills'],
  interCharBlend: INTER_CHAR_BLEND,
  basedOn: BASED_ON,
  shape: ['Square', 'Ramp Up', 'Ramp Down', 'Triangle', 'Round', 'Smooth'],
  trackingType: ['Before & After', 'Before', 'After'],
  characterAlignment: ['Left or Top', 'Center', 'Right or Bottom', 'Adjust Kerning'],
  characterRange: ['Preserve Case & Digits', 'Full Unicode'],
  'star/type': ['Star', 'Polygon'],
  'gfill/type': ['Linear', 'Radial'],
  'gstroke/type': ['Linear', 'Radial'],
  composite: COMPOSITE,
  'repeater/composite': ['Below', 'Above'],
  rule: ['Non-Zero Winding', 'Even-Odd'],
  cap: CAPS,
  join: JOINS,
  'taper/units': ['Pixels', 'Percent'],
  'wave/units': ['Pixels', 'Cycles'],
  'advanced/units': ['Percentage', 'Index'],
  'trim/mode': ['Simultaneously', 'Individually'],
  'merge/mode': ['Merge', 'Add', 'Subtract', 'Intersect', 'Exclude Intersections'],
  'rangeSelector/mode': SEL_MODES,
  'wigglySelector/mode': SEL_MODES,
  'expressionSelector/mode': SEL_MODES,
  points: ['Corner', 'Smooth'],
  falloff: ['None', 'Smooth', 'Inverse Square Clamped'],
  castsShadows: SHADOWS,
  acceptsShadows: SHADOWS,
  bevelStyle: ['None', 'Angular', 'Concave', 'Convex'],
  irisShape: ['Fast Rectangle', 'Triangle', 'Square', 'Pentagon', 'Hexagon', 'Heptagon', 'Octagon', 'Nonagon', 'Decagon'],
}

/**
 * The labels of a drop-down property, or null when the editor does not know them (the value is
 * then shown as a number). `match` is the property's match name, `parent` its group's, `path`
 * its tree path, `masks` the layer's mask names.
 */
export function enumOptions(match: string | undefined, parent: string | undefined, path: string, masks: readonly string[]): readonly string[] | null {
  if (!match) return null
  // A text layer's Path Options ▸ Path: None, then the layer's masks.
  if (path === 'text/pathOptions/path') return ['None', ...masks]
  return OPTIONS[`${parent ?? ''}/${match}`] ?? OPTIONS[match] ?? null
}
