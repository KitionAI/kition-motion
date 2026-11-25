import { useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { snapIn, type TimelineFrame } from './context'
import { keyId, keysOf, type KeyRef } from './keys'
import { secondsFor } from './view'

/** How far (CSS pixels) a press may wander and still be a click. */
const CLICK_SLOP = 3

type KeyPress = {
  kind: 'key'
  pointerId: number
  x: number
  width: number
  /** The pressed keyframe's time: the drag snaps it. */
  anchor: number
  /** The keyframes the drag moves, and their marks. */
  elements: HTMLElement[]
  /** The selection command the press sent; the move waits for it. */
  selecting: Promise<unknown>
  /** A click on one of several selected keyframes selects it alone. */
  alone: KeyRef[] | null
  moved: boolean
  delta: number
}

type MarqueePress = { kind: 'marquee'; pointerId: number; x: number; y: number; add: boolean; moved: boolean }

/** The marquee in the rows box's own coordinates (scrolled content). */
export type Marquee = { left: number; top: number; width: number; height: number }

const SELECT = { timeBase: 'comp', selectProperties: true } as const

/**
 * The keyframe gestures over the timeline's rows: a click on a keyframe selects it (Shift or Cmd
 * adds or removes it), a drag moves the selected keyframes together (snapped to frames and the
 * playhead; one `keyframe.move`, one undo step, on release), a drag on empty track draws a marquee
 * that selects the keyframes in it, and a click on empty timeline deselects everything.
 */
export function useKeyGestures(rows: RefObject<HTMLDivElement | null>, frame: TimelineFrame, selected: ReadonlySet<string>) {
  const { run } = useKitEditor()
  const press = useRef<KeyPress | MarqueePress | null>(null)
  const [marquee, setMarquee] = useState<Marquee | null>(null)

  const marqueeBox = (current: MarqueePress, x: number, y: number): Marquee | null => {
    const box = rows.current
    if (!box) return null
    const outer = box.getBoundingClientRect()
    return { left: Math.min(current.x, x) - outer.left + box.scrollLeft, top: Math.min(current.y, y) - outer.top + box.scrollTop, width: Math.abs(x - current.x), height: Math.abs(y - current.y) }
  }

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !rows.current) return
    const target = event.target as HTMLElement
    const add = event.shiftKey || event.metaKey || event.ctrlKey
    const mark = target.closest<HTMLElement>('.motion-key')
    if (mark) {
      const keys = keysOf(mark)
      const ids = keys.map(keyId)
      const all = ids.every((id) => selected.has(id))
      let moving: Set<string>
      let selecting: Promise<unknown> = Promise.resolve()
      if (add) {
        selecting = run('keyframe.select', { ...SELECT, keys, toggle: true })
        moving = all ? new Set([...selected].filter((id) => !ids.includes(id))) : new Set([...selected, ...ids])
      } else if (!all) {
        selecting = run('keyframe.select', { ...SELECT, keys })
        moving = new Set(ids)
      } else {
        // It looks selected, but the editor's copy may be behind the engine (a click elsewhere
        // just before, not answered yet): it is selected again, the others kept, so the drag
        // moves it and Delete removes it.
        selecting = run('keyframe.select', { ...SELECT, keys, add: true })
        moving = new Set(selected)
      }
      const track = mark.closest('.motion-row-track')
      const elements = [...rows.current.querySelectorAll<HTMLElement>('.motion-key')].filter((element) => {
        const its = keysOf(element)
        return its.length > 0 && its.every((key) => moving.has(keyId(key)))
      })
      event.currentTarget.setPointerCapture(event.pointerId)
      press.current = {
        kind: 'key',
        pointerId: event.pointerId,
        x: event.clientX,
        width: track?.getBoundingClientRect().width ?? frame.width,
        anchor: keys[0]?.time ?? 0,
        elements: frame.readOnly ? [] : elements,
        selecting,
        alone: !add && all && selected.size > ids.length ? keys : null,
        moved: false,
        delta: 0,
      }
      return
    }
    // Empty timeline: below the rows, or a track beside its bar and keyframes.
    if (target !== event.currentTarget && !target.classList.contains('motion-row-track')) return
    event.currentTarget.setPointerCapture(event.pointerId)
    press.current = { kind: 'marquee', pointerId: event.pointerId, x: event.clientX, y: event.clientY, add, moved: false }
  }

  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = press.current
    if (!current || current.pointerId !== event.pointerId) return
    if (!current.moved && Math.hypot(event.clientX - current.x, current.kind === 'marquee' ? event.clientY - current.y : 0) <= CLICK_SLOP) return
    current.moved = true
    if (current.kind === 'marquee') {
      setMarquee(marqueeBox(current, event.clientX, event.clientY))
      return
    }
    if (!current.elements.length) return
    current.delta = snapIn(frame, current.anchor + secondsFor(frame.view, event.clientX - current.x, current.width)) - current.anchor
    const px = (current.delta / frame.view.span) * current.width
    for (const element of current.elements) element.style.translate = `${px}px 0`
  }

  const finish = (event: ReactPointerEvent<HTMLDivElement>, cancelled: boolean) => {
    const current = press.current
    if (!current || current.pointerId !== event.pointerId) return
    press.current = null
    event.currentTarget.releasePointerCapture(event.pointerId)
    if (current.kind === 'key') {
      for (const element of current.elements) element.style.translate = ''
      if (cancelled) return
      if (current.moved && Math.abs(current.delta) > 1e-9) {
        const delta = current.delta
        void current.selecting.then(() => run('keyframe.move', { delta }))
      } else if (!current.moved && current.alone) void run('keyframe.select', { ...SELECT, keys: current.alone })
      return
    }
    setMarquee(null)
    if (cancelled) return
    if (!current.moved) {
      void run('edit.deselectAll', {})
      return
    }
    const left = Math.min(current.x, event.clientX)
    const right = Math.max(current.x, event.clientX)
    const top = Math.min(current.y, event.clientY)
    const bottom = Math.max(current.y, event.clientY)
    const keys = [...(rows.current?.querySelectorAll<HTMLElement>('.motion-key') ?? [])]
      .filter((element) => {
        const box = element.getBoundingClientRect()
        const x = box.left + box.width / 2
        const y = box.top + box.height / 2
        return x >= left && x <= right && y >= top && y <= bottom
      })
      .flatMap(keysOf)
    if (keys.length) void run('keyframe.select', { ...SELECT, keys, add: current.add })
    else if (!current.add) void run('edit.deselectAll', {})
  }

  return {
    marquee,
    handlers: {
      onPointerDown,
      onPointerMove,
      onPointerUp: (event: ReactPointerEvent<HTMLDivElement>) => finish(event, false),
      onPointerCancel: (event: ReactPointerEvent<HTMLDivElement>) => finish(event, true),
    },
  }
}
