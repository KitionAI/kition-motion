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
  // How many times the person zoomed or scrolled (not `fit`, not `restore`): the graph editor fits
  // itself to the keys only until the person zooms.
  const [moved, setMoved] = useState(0)
  const view = stored ? clampView(stored, duration, frame) : { start: 0, span: duration }
  const latest = useRef({ view, duration, frame, width })
  latest.current = { view, duration, frame, width }

  /**
   * `next`, or the whole composition when it shows all of it. The next key press or wheel notch
   * zooms from `next` even when it comes before React draws it (two quick presses are two steps).
   */
  const settle = useCallback((next: TimeView, byHand = true) => {
    latest.current = { ...latest.current, view: next }
    setStored(next.span >= latest.current.duration - 1e-9 ? null : next)
    if (byHand) setMoved((count) => count + 1)
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

  /** Shows from `start` to `end` seconds (the graph editor's 适合全部, and its fit when it opens). */
  const fit = useCallback((start: number, end: number) => settle(clampView({ start, span: Math.max(latest.current.frame * 2, end - start) }, latest.current.duration, latest.current.frame), false), [settle])

  /** The stretch shown now, to put back with `restore` (null: the whole composition). */
  const snapshot = useCallback(() => {
    const { view: current, duration: length } = latest.current
    return current.span >= length - 1e-9 ? null : current
  }, [])
  /** Shows again what `snapshot` gave (the graph editor closing puts back the layer bars' zoom). */
  const restore = useCallback(
    (saved: TimeView | null) => {
      settle(saved ? clampView(saved, latest.current.duration, latest.current.frame) : { start: 0, span: latest.current.duration }, false)
    },
    [settle],
  )

  return { view, zoomed: stored !== null, zoomIn: () => zoomBy(ZOOM_STEP), zoomOut: () => zoomBy(1 / ZOOM_STEP), scrollTo, fit, snapshot, restore, moved }
}
