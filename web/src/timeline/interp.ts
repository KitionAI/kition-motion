/** A keyframe side's interpolation as the engine names it (`prop.get`'s `in` and `out`). */
export type Side = 'linear' | 'bezier' | 'hold'

/** A keyframe's interpolation on its incoming and outgoing sides. */
export type KeyInterp = { in: Side; out: Side }

/** `Linear`, `Bezier`, `Hold` (any case) as a side; anything else reads as linear. */
export function sideOf(label: unknown): Side {
  const name = String(label ?? '').toLowerCase()
  return name === 'bezier' || name === 'hold' ? name : 'linear'
}

/** One keyframe of a property as `prop.get` answers it (the fields the editor reads). */
export type PropKey = { compTime: number; layerTime: number; value: unknown; in?: string; out?: string }

/** What `prop.get` answers for a property (the fields the editor reads). */
export type PropRead = { uid?: number; type?: string; keys?: PropKey[] }

/**
 * The Animation menu's Keyframe Interpolation rows and the key menu's: each sets the selected
 * keyframes' interpolation (`keys.interpolation`), both sides.
 */
export const INTERPOLATIONS = [
  { command: 'kit:interp-linear', label: 'Linear', interpolation: 'linear' },
  { command: 'kit:interp-bezier', label: 'Bezier', interpolation: 'bezier' },
  { command: 'kit:interp-auto', label: 'Auto Bezier', interpolation: 'autoBezier' },
  { command: 'kit:interp-hold', label: 'Hold', interpolation: 'hold' },
] as const

/** The `keys.interpolation` value a Keyframe Interpolation row sets, by its command. */
export const INTERPOLATION_OF: Record<string, string> = Object.fromEntries(INTERPOLATIONS.map((row) => [row.command, row.interpolation]))
