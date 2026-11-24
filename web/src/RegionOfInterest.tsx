import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import type { ViewStore } from './CanvasScrollbars'
import { useView } from './CanvasGuides'

/** A region of interest in composition pixels: `[x, y, width, height]`. */
export type Region = [number, number, number, number]

/** The smallest region a drag draws (composition pixels each way); a shorter drag draws none. */
const SMALLEST = 4

/**
 * View ▸ Region of Interest: while `drawing`, a drag over the composition draws the region the
 * viewer renders (only the pixels inside it, the rest grey), set with `view.setRegionOfInterest`
 * when the pointer is let go; Escape stops drawing. The region set is outlined over the picture.
 */
export function RegionOfInterest({ views, comp, region, drawing, onDrawn, onCancel }: { views: ViewStore; comp: { width: number; height: number }; region: Region | null; drawing: boolean; onDrawn: (region: Region) => void; onCancel: () => void }) {
  const { t } = useKitEditor()
  const view = useView(views)
  const box = useRef<HTMLDivElement>(null)
  const [drag, setDrag] = useState<{ pointerId: number; from: [number, number]; to: [number, number] } | null>(null)
  if (!view || !(view.zoom > 0) || (!drawing && !region)) return null
  const sx = (x: number) => (x - view.x) * view.zoom
  const sy = (y: number) => (y - view.y) * view.zoom
  /** The composition point under the pointer, kept inside the composition. */
  const at = (event: { clientX: number; clientY: number }): [number, number] => {
    const rect = box.current?.getBoundingClientRect()
    const x = view.x + (event.clientX - (rect?.left ?? 0)) / view.zoom
    const y = view.y + (event.clientY - (rect?.top ?? 0)) / view.zoom
    return [Math.round(Math.min(comp.width, Math.max(0, x))), Math.round(Math.min(comp.height, Math.max(0, y)))]
  }
  const regionOf = (from: [number, number], to: [number, number]): Region => [Math.min(from[0], to[0]), Math.min(from[1], to[1]), Math.abs(to[0] - from[0]), Math.abs(to[1] - from[1])]
  const shown = drag ? regionOf(drag.from, drag.to) : region
  const end = (event: ReactPointerEvent<HTMLDivElement>, cancelled: boolean) => {
    if (!drag || drag.pointerId !== event.pointerId) return
    setDrag(null)
    if (cancelled) return
    // Where the pointer is let go (WebKit may not report the last moves before the release).
    const drawn = regionOf(drag.from, at(event))
    if (drawn[2] >= SMALLEST && drawn[3] >= SMALLEST) onDrawn(drawn)
  }
  return (
    <div
      ref={box}
      className="motion-roi"
      data-drawing={drawing || undefined}
      data-testid="motion-roi-layer"
      tabIndex={drawing ? -1 : undefined}
      onPointerDown={(event) => {
        if (!drawing || event.button !== 0) return
        event.preventDefault()
        event.stopPropagation()
        event.currentTarget.setPointerCapture(event.pointerId)
        const from = at(event)
        setDrag({ pointerId: event.pointerId, from, to: from })
      }}
      onPointerMove={(event) => {
        if (drag && drag.pointerId === event.pointerId) setDrag({ ...drag, to: at(event) })
      }}
      onPointerUp={(event) => end(event, false)}
      onPointerCancel={(event) => end(event, true)}
      onKeyDown={(event) => {
        if (event.key !== 'Escape') return
        event.stopPropagation()
        setDrag(null)
        onCancel()
      }}
      title={drawing ? t('roi.hint') : undefined}
    >
      {shown ? (
        <svg width={view.width} height={view.height} aria-hidden="true">
          <rect className="motion-roi-box" x={sx(shown[0])} y={sy(shown[1])} width={shown[2] * view.zoom} height={shown[3] * view.zoom} data-testid="motion-roi-box" />
        </svg>
      ) : null}
    </div>
  )
}
