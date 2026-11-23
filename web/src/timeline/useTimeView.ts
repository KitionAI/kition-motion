import { useCallback, useEffect, useRef, useState, type RefObject } from 'react'
import { clampView, timeAtX, zoomView, type TimeView } from './view'

/** How much one notch of zoom (`=` or `-`) changes the timeline's scale. */
const ZOOM_STEP = 1.5

/** How fast Cmd or Ctrl and the wheel zoom: the scale changes by e^(pixels × this). */
const WHEEL_ZOOM = 0.002

/**
 * The stretch of the composition the timeline shows: the whole of it until zoomed. `zoomBy`
 * zooms around a time (the playhead when it is in view, else the middle); over the rows, Cmd or
 * Ctrl and the wheel zoom around the pointer, and a sideways wheel (or Shift and the wheel)
 * scrolls a zoomed timeline.
 */
export function useTimeView(rows: RefObject<HTMLDivElement | null>, duration: number, frame: number, playhead: number, width: number) {
  const [stored, setStored] = useState<TimeView | null>(null)
  const view = stored ? clampView(stored, duration, frame) : { start: 0, span: duration }
  const latest = useRef({ view, duration, frame, width })
  latest.current = { view, duration, frame, width }

  /**
   * `next`, or the whole composition when it shows all of it. The next key press or wheel notch
   * zooms from `next` even when it comes before React draws it (two quick presses are two steps).
   */
  const settle = useCallback((next: TimeView) => {
    latest.current = { ...latest.current, view: next }
    setStored(next.span >= latest.current.duration - 1e-9 ? null : next)
  }, [])

  const zoomBy = useCallback(
    (factor: number, anchor?: number) => {
      const { view: current, duration: length, frame: step } = latest.current
      const inView = playhead >= current.start && playhead <= current.start + current.span
      settle(zoomView(current, factor, anchor ?? (inView ? playhead : current.start + current.span / 2), length, step))
    },
    [playhead, settle],
  )

  const scrollTo = useCallback((start: number) => settle(clampView({ ...latest.current.view, start }, latest.current.duration, latest.current.frame)), [settle])

  useEffect(() => {
    const element = rows.current
    if (!element) return undefined
    const wheel = (event: WheelEvent) => {
      const { view: current, duration: length, frame: step, width: across } = latest.current
      if (event.metaKey || event.ctrlKey) {
        event.preventDefault()
        const ruler = element.querySelector('.motion-ruler')?.getBoundingClientRect()
        const anchor = ruler ? Math.min(length, Math.max(0, timeAtX(current, event.clientX, ruler))) : current.start + current.span / 2
        settle(zoomView(current, Math.exp(-event.deltaY * WHEEL_ZOOM), anchor, length, step))
        return
      }
      const sideways = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.shiftKey ? event.deltaY : 0
      if (!sideways || current.span >= length - 1e-9) return
      event.preventDefault()
      settle(clampView({ start: current.start + (sideways * current.span) / Math.max(1, across), span: current.span }, length, step))
    }
    element.addEventListener('wheel', wheel, { passive: false })
    return () => element.removeEventListener('wheel', wheel)
  }, [rows, settle])

  return { view, zoomed: stored !== null, zoomIn: () => zoomBy(ZOOM_STEP), zoomOut: () => zoomBy(1 / ZOOM_STEP), scrollTo }
}
