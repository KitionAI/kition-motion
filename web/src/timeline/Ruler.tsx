import { useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { snapIn, useTimeline } from './context'
import { leftOf, rulerLabel, rulerStep, timeAtX } from './view'

type Edge = 'start' | 'end'

/**
 * The time ruler over the tracks: labelled ticks at a step that suits the zoom (frames when zoomed
 * in), the work area as a band whose ends drag (snapped like a bar's, set on release), and the
 * playhead. A press or drag elsewhere scrubs the current time.
 */
export function Ruler({ onSeek, onWorkArea, rulerRef, children }: { onSeek: (seconds: number) => void; onWorkArea: (start: number, end: number) => void; rulerRef: (element: HTMLDivElement | null) => void; children?: ReactNode }) {
  const frame = useTimeline()
  const { info, view, width, readOnly } = frame
  const { t } = useKitEditor()
  const fps = info.comp?.fps ?? 30
  const duration = info.comp?.duration ?? 0
  const step = rulerStep(width / view.span, fps)
  const first = Math.ceil(view.start / step.seconds - 1e-9)
  const last = Math.floor(Math.min(duration, view.start + view.span) / step.seconds + 1e-9)
  const marks = last >= first ? Array.from({ length: Math.min(400, last - first + 1) }, (_, index) => (first + index) * step.seconds) : []
  const [work, setWork] = useState<[number, number] | null>(null)
  const grip = useRef<{ pointerId: number; edge: Edge } | null>(null)
  const area = work ?? info.comp?.workArea ?? [0, duration]

  const timeAt = (event: { clientX: number }, element: Element) => Math.min(duration, Math.max(0, timeAtX(view, event.clientX, element.getBoundingClientRect())))
  const seek = (event: ReactPointerEvent<HTMLDivElement>) => onSeek(timeAt(event, event.currentTarget))

  /** The work area with `edge` dragged to the pointer: snapped, at least a frame long. */
  const dragged = (edge: Edge, event: { clientX: number }, ruler: Element): [number, number] => {
    const at = snapIn(frame, timeAt(event, ruler))
    const [start, end] = info.comp?.workArea ?? [0, duration]
    return edge === 'start' ? [Math.min(at, end - info.frame), end] : [start, Math.max(at, start + info.frame)]
  }
  const handle = (edge: Edge) => ({
    onPointerDown: (event: ReactPointerEvent<HTMLSpanElement>) => {
      if (event.button !== 0 || readOnly) return
      event.stopPropagation()
      // No text selection or native drag from the press: WebKit starts one on the move (selectstart)
      // and cancels the pointer when a drag begins, which would drop the work area edit.
      event.preventDefault()
      event.currentTarget.setPointerCapture(event.pointerId)
      grip.current = { pointerId: event.pointerId, edge }
    },
    onPointerMove: (event: ReactPointerEvent<HTMLSpanElement>) => {
      const ruler = event.currentTarget.parentElement
      if (grip.current?.pointerId !== event.pointerId || !ruler) return
      event.stopPropagation()
      setWork(dragged(edge, event, ruler))
    },
    onPointerUp: (event: ReactPointerEvent<HTMLSpanElement>) => {
      const ruler = event.currentTarget.parentElement
      if (grip.current?.pointerId !== event.pointerId || !ruler) return
      event.stopPropagation()
      grip.current = null
      setWork(null)
      const [start, end] = dragged(edge, event, ruler)
      const [oldStart, oldEnd] = info.comp?.workArea ?? [0, duration]
      if (Math.abs(start - oldStart) > 1e-9 || Math.abs(end - oldEnd) > 1e-9) onWorkArea(start, end)
    },
    onPointerCancel: () => {
      grip.current = null
      setWork(null)
    },
  })

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
      data-view-start={view.start}
      data-view-span={view.span}
      data-step={step.frames ? 'frames' : 'seconds'}
      data-step-seconds={step.seconds}
    >
      <span className="motion-work-area" style={{ left: leftOf(view, area[0]), width: `${((area[1] - area[0]) / view.span) * 100}%` }} aria-hidden="true" />
      {marks.map((mark) => (
        <span key={mark} className="motion-ruler-mark" style={{ left: leftOf(view, mark) }}>
          {rulerLabel(mark, fps)}
        </span>
      ))}
      <span className="motion-work-handle" data-edge="start" style={{ left: leftOf(view, area[0]) }} title={t('workAreaStart')} {...handle('start')} data-testid="motion-work-start" />
      <span className="motion-work-handle" data-edge="end" style={{ left: leftOf(view, area[1]) }} title={t('workAreaEnd')} {...handle('end')} data-testid="motion-work-end" />
      {/* The composition's markers. */}
      {children}
      <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
    </div>
  )
}

/**
 * The frames the preview has kept (its frame cache): a thin bar along the ruler's foot over each
 * stretch of kept frames, so the person sees what plays back in real time. `frames` are frame
 * numbers from the composition's start.
 */
export function CacheBar({ frames }: { frames: readonly number[] }) {
  const { info, view } = useTimeline()
  const { t } = useKitEditor()
  const fps = info.comp?.fps ?? 30
  const runs: Array<[number, number]> = []
  for (const frame of frames) {
    const last = runs[runs.length - 1]
    if (last && frame === last[1] + 1) last[1] = frame
    else runs.push([frame, frame])
  }
  return (
    <span className="motion-cache-bar" data-testid="motion-cache-bar" data-frames={frames.length} title={t('cache.title', { count: frames.length })} aria-hidden="true">
      {runs.map(([from, to]) => (
        <span key={from} className="motion-cache-run" style={{ left: leftOf(view, from / fps), width: `${((to - from + 1) / fps / view.span) * 100}%` }} />
      ))}
    </span>
  )
}
