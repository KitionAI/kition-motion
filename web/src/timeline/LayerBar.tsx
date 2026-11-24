import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { timecode, type MotionLayer } from '../motion'
import { snapIn, useTimeline } from './context'
import { leftOf, secondsFor } from './view'

/** What a press on a bar does: move the layer, or trim its in or out point. */
type Grip = 'move' | 'in' | 'out'

/** A press on a bar: what it grips, where it started, and the layers a move takes along. */
type Press = { pointerId: number; grip: Grip; x: number; width: number; layers: number[]; moved: boolean }

/** How far (CSS pixels) a press may wander and still be a click. */
const CLICK_SLOP = 3

/**
 * A layer's bar from its in to its out point. Dragged, it moves the layer in time (with the other
 * selected layers when it is one of them); its ends trim the in and out points. Times snap to
 * frames, and to the playhead within a few pixels; the readout shows the new in and out while
 * dragging, and the change is one command (one undo step) on release.
 */
export function LayerBar({ layer, shift, onShift }: { layer: MotionLayer; shift: { layers: number[]; delta: number } | null; onShift: (shift: { layers: number[]; delta: number } | null) => void }) {
  const frame = useTimeline()
  const { info, view, readOnly } = frame
  const { run } = useKitEditor()
  const press = useRef<Press | null>(null)
  const [trim, setTrim] = useState<{ in: number; out: number } | null>(null)
  const moving = shift && shift.layers.includes(layer.id) ? shift.delta : 0
  const shown = trim ?? { in: layer.in + moving, out: layer.out + moving }
  const locked = layer.switches.locked || readOnly
  const duration = info.comp?.duration ?? 0

  /** The move or trim a drag `dx` pixels long gives, snapped. */
  const dragged = (current: Press, dx: number) => {
    const seconds = secondsFor(view, dx, current.width)
    if (current.grip === 'move') {
      // The in point snaps; when it does not move, the out point may snap to the playhead instead.
      const atIn = snapIn(frame, layer.in + seconds) - layer.in
      const atOut = snapIn(frame, layer.out + seconds) - layer.out
      const delta = Math.abs(layer.out + atOut - info.time) < 1e-9 ? atOut : atIn
      return { in: layer.in + delta, out: layer.out + delta }
    }
    if (current.grip === 'in') return { in: Math.min(layer.out - info.frame, Math.max(0, snapIn(frame, layer.in + seconds))), out: layer.out }
    return { in: layer.in, out: Math.max(layer.in + info.frame, Math.min(Math.max(duration, layer.out), snapIn(frame, layer.out + seconds))) }
  }

  const down = (event: ReactPointerEvent<HTMLDivElement>, grip: Grip) => {
    if (event.button !== 0) return
    event.stopPropagation()
    const add = event.shiftKey || event.metaKey || event.ctrlKey
    // A press selects the layer (Shift or Cmd adds it); a move takes the selected layers along.
    // A layer shown selected is still asked for (added, which keeps the other selected layers for
    // the drag): a click on empty timeline just before may have deselected it in the engine
    // without the editor knowing yet, and the layer would end up unselected.
    void run('layer.select', add ? { layers: [layer.id], add, toggle: layer.selected } : { layers: [layer.id], add: layer.selected })
    const track = event.currentTarget.closest('.motion-row-track')
    if (locked || !track) return
    const others = layer.selected && !add ? info.layers.filter((each) => each.selected && !each.switches.locked).map((each) => each.id) : []
    event.currentTarget.setPointerCapture(event.pointerId)
    press.current = { pointerId: event.pointerId, grip, x: event.clientX, width: track.getBoundingClientRect().width, layers: others.length ? others : [layer.id], moved: false }
  }

  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = press.current
    if (!current || current.pointerId !== event.pointerId) return
    const dx = event.clientX - current.x
    if (!current.moved && Math.abs(dx) <= CLICK_SLOP) return
    current.moved = true
    const next = dragged(current, dx)
    if (current.grip === 'move') onShift({ layers: current.layers, delta: next.in - layer.in })
    else setTrim(next)
  }

  const up = (event: ReactPointerEvent<HTMLDivElement>, cancelled: boolean) => {
    const current = press.current
    if (!current || current.pointerId !== event.pointerId) return
    press.current = null
    event.currentTarget.releasePointerCapture(event.pointerId)
    onShift(null)
    setTrim(null)
    if (cancelled || !current.moved) return
    const next = dragged(current, event.clientX - current.x)
    if (current.grip === 'move') {
      const delta = next.in - layer.in
      if (Math.abs(delta) > 1e-9) void run('layer.timing', { layers: current.layers, delta })
    } else if (current.grip === 'in' && Math.abs(next.in - layer.in) > 1e-9) void run('layer.timing', { layers: [layer.id], in: next.in })
    else if (current.grip === 'out' && Math.abs(next.out - layer.out) > 1e-9) void run('layer.timing', { layers: [layer.id], out: next.out })
  }

  const fps = info.comp?.fps ?? 30
  const dragging = trim !== null || moving !== 0
  const handlers = (grip: Grip) => ({
    onPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => down(event, grip),
    onPointerMove: move,
    onPointerUp: (event: ReactPointerEvent<HTMLDivElement>) => up(event, false),
    onPointerCancel: (event: ReactPointerEvent<HTMLDivElement>) => up(event, true),
  })
  return (
    <div
      className="motion-bar"
      data-locked={locked || undefined}
      data-dragging={dragging || undefined}
      style={{ left: leftOf(view, shown.in), width: `${((shown.out - shown.in) / view.span) * 100}%` }}
      {...handlers('move')}
      data-testid={`motion-bar-${layer.id}`}
    >
      {locked ? null : (
        <>
          <div className="motion-bar-end" data-edge="in" {...handlers('in')} data-testid={`motion-bar-in-${layer.id}`} />
          <div className="motion-bar-end" data-edge="out" {...handlers('out')} data-testid={`motion-bar-out-${layer.id}`} />
        </>
      )}
      {dragging ? (
        <span className="motion-bar-readout" data-testid="motion-bar-readout">
          {timecode(shown.in, fps)} – {timecode(shown.out, fps)}
        </span>
      ) : null}
    </div>
  )
}
