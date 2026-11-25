import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import { snapIn, useTimeline } from './context'
import { leftOf, secondsFor } from './view'

/** A marker: the composition's (`layer` null) or a layer's, by its index in `marker.list`. */
export type MarkerRef = { layer: number | null; index: number }

type Marker = { index: number; time: number; comment: string; duration: number }

/** How far (CSS pixels) a press on a marker may wander and still be a click. */
const CLICK_SLOP = 3

export const sameMarker = (a: MarkerRef | null, b: MarkerRef | null) => a != null && b != null && a.layer === b.layer && a.index === b.index

/** The parameters naming `marker` in the `marker.*` commands. */
export const markerParams = (marker: MarkerRef) => (marker.layer == null ? { index: marker.index } : { layer: marker.layer, index: marker.index })

/**
 * The markers of the composition (on the ruler) or of one layer (on its bar's row), each a small
 * flag with its name. A click selects one (Delete then removes it), a drag moves it in time
 * (snapped like a bar, one undo step on release), and a double-click renames it.
 */
export function MarkerFlags({ layer, selected, onSelect, onRename }: { layer: number | null; selected: MarkerRef | null; onSelect: (marker: MarkerRef) => void; onRename: (marker: MarkerRef, comment: string) => void }) {
  const frame = useTimeline()
  const { info, view, readOnly } = frame
  const { run, t } = useKitEditor()
  const answer = useEngineQuery<Marker[]>('marker.list', layer == null ? {} : { layer })
  const markers = Array.isArray(answer) ? answer : []
  const press = useRef<{ pointerId: number; x: number; width: number; index: number; moved: boolean } | null>(null)
  const [moving, setMoving] = useState<{ index: number; time: number } | null>(null)
  const duration = info.comp?.duration ?? 0

  const at = (marker: Marker, current: NonNullable<typeof press.current>, clientX: number) =>
    Math.min(duration, Math.max(0, snapIn(frame, marker.time + secondsFor(view, clientX - current.x, current.width))))

  return (
    <>
      {markers.map((marker) => {
        const ref = { layer, index: marker.index }
        const time = moving?.index === marker.index ? moving.time : marker.time
        return (
          <span
            key={marker.index}
            className="motion-marker"
            data-kind={layer == null ? 'comp' : 'layer'}
            aria-selected={sameMarker(selected, ref)}
            style={{ left: leftOf(view, time) }}
            title={marker.comment || t('marker.untitled')}
            onPointerDown={(event: ReactPointerEvent<HTMLSpanElement>) => {
              if (event.button !== 0) return
              event.stopPropagation()
              onSelect(ref)
              if (readOnly) return
              const track = event.currentTarget.parentElement
              event.currentTarget.setPointerCapture(event.pointerId)
              press.current = { pointerId: event.pointerId, x: event.clientX, width: track?.getBoundingClientRect().width ?? 1, index: marker.index, moved: false }
            }}
            onPointerMove={(event) => {
              const current = press.current
              if (!current || current.pointerId !== event.pointerId) return
              event.stopPropagation()
              if (!current.moved && Math.abs(event.clientX - current.x) <= CLICK_SLOP) return
              current.moved = true
              setMoving({ index: marker.index, time: at(marker, current, event.clientX) })
            }}
            onPointerUp={(event) => {
              const current = press.current
              if (!current || current.pointerId !== event.pointerId) return
              event.stopPropagation()
              press.current = null
              setMoving(null)
              if (!current.moved) return
              const next = at(marker, current, event.clientX)
              if (Math.abs(next - marker.time) > 1e-9) void run('marker.set', { ...markerParams(ref), time: next })
            }}
            onPointerCancel={() => {
              press.current = null
              setMoving(null)
            }}
            onDoubleClick={(event) => {
              event.stopPropagation()
              if (!readOnly) onRename(ref, marker.comment)
            }}
            data-testid={`motion-marker-${layer ?? 'comp'}-${marker.index}`}
            data-time={marker.time}
          >
            {marker.comment ? <span className="motion-marker-label">{marker.comment}</span> : null}
          </span>
        )
      })}
    </>
  )
}
