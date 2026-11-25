import { useRef, useState, type RefObject } from 'react'
import { useKitEditor } from '@kition/sdk/editor'

const KEY = 'kition-motion.timeline-height'
/** The timeline's share of the window's height when nothing is kept (about 342 px at 900 px). */
const DEFAULT_SHARE = 0.38
const MIN_HEIGHT = 120
/** The composition keeps at least this much height above the transport. */
const MIN_CANVAS = 180

/** The timeline's height, kept for the session (sessionStorage); 38 % of the window's height at first. */
export function useTimelineHeight() {
  const [height, setHeight] = useState(() => {
    const stored = Number(globalThis.sessionStorage?.getItem(KEY))
    return Number.isFinite(stored) && stored >= MIN_HEIGHT ? stored : Math.max(MIN_HEIGHT, Math.round((globalThis.innerHeight || 700) * DEFAULT_SHARE))
  })
  const set = (next: number) => {
    setHeight(next)
    try {
      sessionStorage.setItem(KEY, String(Math.round(next)))
    } catch {
      // Storage refused (private mode): the height holds until the page closes.
    }
  }
  return [height, set] as const
}

/**
 * The bar between the composition and the transport: dragged up or down (or Up and Down when
 * focused) it makes the timeline taller or shorter, between a few rows and what leaves the
 * composition room.
 */
export function Splitter({ height, onHeight, main }: { height: number; onHeight: (height: number) => void; main: RefObject<HTMLDivElement | null> }) {
  const { t } = useKitEditor()
  const drag = useRef<{ pointerId: number; y: number; height: number } | null>(null)
  const [dragging, setDragging] = useState(false)
  const clamp = (next: number) => {
    // The transport sits between the splitter and the timeline; the rest of the column is the composition's.
    const room = (main.current?.clientHeight ?? 800) - MIN_CANVAS - 40
    return Math.round(Math.max(MIN_HEIGHT, Math.min(Math.max(MIN_HEIGHT, room), next)))
  }
  return (
    <div
      className="motion-splitter"
      role="separator"
      aria-orientation="horizontal"
      aria-label={t('splitter')}
      aria-valuenow={height}
      tabIndex={0}
      data-dragging={dragging || undefined}
      onPointerDown={(event) => {
        if (event.button !== 0) return
        event.currentTarget.setPointerCapture(event.pointerId)
        drag.current = { pointerId: event.pointerId, y: event.clientY, height }
        setDragging(true)
      }}
      onPointerMove={(event) => {
        const current = drag.current
        if (current?.pointerId === event.pointerId) onHeight(clamp(current.height + current.y - event.clientY))
      }}
      onPointerUp={(event) => {
        if (drag.current?.pointerId !== event.pointerId) return
        drag.current = null
        setDragging(false)
      }}
      onPointerCancel={() => {
        drag.current = null
        setDragging(false)
      }}
      onKeyDown={(event) => {
        if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return
        event.preventDefault()
        event.stopPropagation()
        onHeight(clamp(height + (event.key === 'ArrowUp' ? 16 : -16)))
      }}
      data-testid="motion-splitter"
    />
  )
}
