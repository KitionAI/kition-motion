import { DiamondIcon as Diamond } from '@kition/tessera/icons/react'
import { useKitEditor } from '@kition/sdk/editor'

/** Whether `times` (composition seconds) has a keyframe at `time`, within half a frame. */
export function keyAt(times: readonly number[], time: number, frame: number) {
  return times.some((at) => Math.abs(at - time) < frame / 2)
}

/**
 * A property's keyframe diamond (the Properties panel's and the timeline's): hollow when the
 * property is not animated, outlined in the accent colour when it is, filled when it has a
 * keyframe at the current time. A click adds a keyframe here, or removes the one here.
 */
export function KeyDiamond({ layer, path, name, times, time, frame, testId, readOnly }: { layer: number; path: string; name: string; times: readonly number[]; time: number; frame: number; testId: string; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  const here = keyAt(times, time, frame)
  const label = t(here ? 'removeKey' : 'addKey', { name })
  return (
    <button
      type="button"
      className="motion-key-button"
      data-animated={times.length > 0}
      data-key-here={here || undefined}
      aria-label={label}
      title={label}
      disabled={readOnly}
      onClick={() => void (here ? run('prop.toggleKey', { layer, path }) : run('prop.addKey', { layer, path, time, timeBase: 'comp' }))}
      data-testid={testId}
    >
      <Diamond size={12} aria-hidden="true" />
    </button>
  )
}
