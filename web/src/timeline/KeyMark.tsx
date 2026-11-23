import { timecode } from '../motion'
import { useTimeline } from './context'
import { keyId, type KeyRef } from './keys'
import { leftOf } from './view'

/**
 * A keyframe drawn on a track (the SDK's keyframe diamond): it stands for one property's keyframe,
 * or on a collapsed layer row for every property's keyframe at that time. Selected when all of
 * them are; the track's gestures read its keys from `data-keys`. `row` names the row in its test id
 * (`27` for the layer's own row, `27-transform.position` for a property's).
 */
export function KeyMark({ time, keys, selected, label, row }: { time: number; keys: KeyRef[]; selected: ReadonlySet<string>; label: string; row: string }) {
  const { info, view } = useTimeline()
  const on = keys.length > 0 && keys.every((key) => selected.has(keyId(key)))
  return (
    <span
      className="kui-keyframe motion-key"
      role="option"
      aria-selected={on}
      aria-label={`${label} ${timecode(time, info.comp?.fps ?? 30)}`}
      data-shape="diamond"
      data-keys={JSON.stringify(keys)}
      style={{ left: leftOf(view, time) }}
      data-testid={`motion-key-${row}-${Math.round(time * 1000)}`}
    />
  )
}
