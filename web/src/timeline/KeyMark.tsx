import { timecode } from '../motion'
import { useTimeline } from './context'
import type { KeyInterp } from './interp'
import { keyId, type KeyRef } from './keys'
import { leftOf } from './view'

/**
 * A keyframe drawn on a track (the SDK's keyframe diamond): it stands for one property's keyframe,
 * or on a collapsed layer row for every property's keyframe at that time. Selected when all of
 * them are; the track's gestures read its keys from `data-keys`. `row` names the row in its test id
 * (`27` for the layer's own row, `27-transform.position` for a property's).
 *
 * A property's keyframe shows its interpolation by its shape, one half per side: a diamond's half
 * for linear, an hourglass's half for Bezier (eased), a square's half for hold. A layer row's mark,
 * standing for several, is a diamond.
 */
export function KeyMark({ time, keys, selected, label, row, interp }: { time: number; keys: KeyRef[]; selected: ReadonlySet<string>; label: string; row: string; interp?: KeyInterp }) {
  const { info, view } = useTimeline()
  const on = keys.length > 0 && keys.every((key) => selected.has(keyId(key)))
  return (
    <span
      className="kui-keyframe motion-key"
      role="option"
      aria-selected={on}
      aria-label={`${label} ${timecode(time, info.comp?.fps ?? 30)}`}
      data-shape={interp ? 'halves' : 'diamond'}
      data-in={interp?.in}
      data-out={interp?.out}
      data-keys={JSON.stringify(keys)}
      style={{ left: leftOf(view, time) }}
      data-testid={`motion-key-${row}-${Math.round((keys[0]?.time ?? time) * 1000)}`}
    />
  )
}
