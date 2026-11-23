import { type PointerEvent as ReactPointerEvent } from 'react'
import { useTimeline } from './context'
import { leftOf, rulerLabel, rulerStep, timeAtX } from './view'

/**
 * The time ruler over the tracks: labelled ticks at a step that suits the zoom (frames when zoomed
 * in), and the playhead. A press or drag scrubs the current time.
 */
export function Ruler({ onSeek, rulerRef }: { onSeek: (seconds: number) => void; rulerRef: (element: HTMLDivElement | null) => void }) {
  const { info, view, width } = useTimeline()
  const fps = info.comp?.fps ?? 30
  const duration = info.comp?.duration ?? 0
  const step = rulerStep(width / view.span, fps)
  const first = Math.ceil(view.start / step.seconds - 1e-9)
  const last = Math.floor(Math.min(duration, view.start + view.span) / step.seconds + 1e-9)
  const marks = last >= first ? Array.from({ length: Math.min(400, last - first + 1) }, (_, index) => (first + index) * step.seconds) : []
  const seek = (event: ReactPointerEvent<HTMLDivElement>) => {
    const time = timeAtX(view, event.clientX, event.currentTarget.getBoundingClientRect())
    onSeek(Math.min(duration, Math.max(0, time)))
  }
  return (
    <div
      ref={rulerRef}
      className="motion-ruler"
      onPointerDown={(event) => {
        if (event.button !== 0) return
        event.currentTarget.setPointerCapture(event.pointerId)
        seek(event)
      }}
      onPointerMove={(event) => {
        if (event.buttons & 1 && event.currentTarget.hasPointerCapture(event.pointerId)) seek(event)
      }}
      data-testid="motion-ruler"
      data-step={step.frames ? 'frames' : 'seconds'}
    >
      {marks.map((mark) => (
        <span key={mark} className="motion-ruler-mark" style={{ left: leftOf(view, mark) }}>
          {rulerLabel(mark, fps)}
        </span>
      ))}
      <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
    </div>
  )
}
