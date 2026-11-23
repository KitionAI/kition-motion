import { useRef, useState } from 'react'
import { useKitEditor } from '@kition/sdk/editor'

const KEY = 'kition-motion.properties-width'
const DEFAULT_WIDTH = 300
const MIN_WIDTH = 240
const MAX_WIDTH = 640
/** The composition keeps at least this much width beside the panel. */
const MIN_CANVAS = 360

const clampWidth = (width: number) => Math.round(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, (globalThis.innerWidth ?? 1440) - MIN_CANVAS), width)))

/** The Properties panel's width, kept for the session (sessionStorage). */
export function usePanelWidth() {
  const [width, setWidth] = useState(() => {
    const stored = Number(globalThis.sessionStorage?.getItem(KEY))
    return Number.isFinite(stored) && stored >= MIN_WIDTH ? clampWidth(stored) : DEFAULT_WIDTH
  })
  const set = (next: number) => {
    const width = clampWidth(next)
    setWidth(width)
    try {
      sessionStorage.setItem(KEY, String(width))
    } catch {
      // Storage refused (private mode): the width holds until the page closes.
    }
  }
  return [width, set] as const
}

/** The panel's left edge: dragged left or right (or Left and Right when focused) it makes the panel wider or narrower. */
export function PanelEdge({ width, onWidth }: { width: number; onWidth: (width: number) => void }) {
  const { t } = useKitEditor()
  const drag = useRef<{ pointerId: number; x: number; width: number } | null>(null)
  const [dragging, setDragging] = useState(false)
  return (
    <div
      className="motion-panel-edge"
      role="separator"
      aria-orientation="vertical"
      aria-label={t('props.resize')}
      aria-valuenow={width}
      tabIndex={0}
      data-dragging={dragging || undefined}
      onPointerDown={(event) => {
        if (event.button !== 0) return
        event.preventDefault()
        event.currentTarget.setPointerCapture(event.pointerId)
        drag.current = { pointerId: event.pointerId, x: event.clientX, width }
        setDragging(true)
      }}
      onPointerMove={(event) => {
        const current = drag.current
        if (current?.pointerId === event.pointerId) onWidth(current.width + current.x - event.clientX)
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
        if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
        event.preventDefault()
        event.stopPropagation()
        onWidth(width + (event.key === 'ArrowLeft' ? 16 : -16))
      }}
      data-testid="motion-properties-edge"
    />
  )
}
