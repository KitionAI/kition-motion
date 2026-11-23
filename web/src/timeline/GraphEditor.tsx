import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { snapIn, useTimeline } from './context'
import { sideOf, type PropRead, type Side } from './interp'
import { keyId } from './keys'
import { pathId } from './PropertyRows'
import { useSampler } from './samples'

/** A property the graph shows: an animated property of a selected layer. */
export type GraphTarget = { layer: MotionLayer; path: string; name: string }

/** One side's temporal ease as `keys.info` answers it, per dimension (null where it has none). */
type Ease = { speed: number; influence: number } | null
type KeyInfo = { time: number; inEase?: Ease[]; outEase?: Ease[] }

/** One key of a curve: its times, its value per dimension, its sides, and its eases. */
type GraphKey = { compTime: number; layerTime: number; value: number[]; in: Side; out: Side; inEase: Ease[]; outEase: Ease[] }

/** A property's curves: one per dimension, sampled over the stretch in view. */
type Curve = { target: GraphTarget; uid: number; dims: number; keys: GraphKey[]; samples: number[][]; from: number; to: number; spatial: boolean }

/** The colours of the dimensions (x, y, z), and of a one-number property. */
const DIM_CLASS = ['x', 'y', 'z']

/** Space (CSS pixels) kept above and below the curves. */
const PAD = 18

/** How far (CSS pixels) a press may wander and still be a click. */
const CLICK_SLOP = 3

/** The numbers of a value the graph draws: a number, or the first `dims` of an array of numbers. */
function numbersOf(value: unknown, dims: number): number[] | null {
  if (typeof value === 'number') return [value]
  if (Array.isArray(value) && value.length >= dims && value.slice(0, dims).every((part) => typeof part === 'number')) return value.slice(0, dims) as number[]
  return null
}

/** How many dimensions of a property the graph draws (a 2D layer's x and y of a 3D value), or 0 for a value it cannot draw. */
function dimsOf(type: string | undefined, layer: MotionLayer) {
  if (type === 'scalar') return 1
  if (type === 'vec2') return 2
  if (type === 'vec3') return layer.switches.threeD ? 3 : 2
  return 0
}

/** The curves of `targets` over the stretch in view, read again when `revision` (the layers' keys and the selected keys) or the view changes. */
function useCurves(targets: readonly GraphTarget[], from: number, to: number, count: number, revision: unknown) {
  const { query } = useKitEditor()
  const sample = useSampler()
  const [curves, setCurves] = useState<Curve[]>([])
  const asked = useRef(0)
  useEffect(() => {
    const ask = ++asked.current
    void (async () => {
      const read: Curve[] = []
      for (const target of targets) {
        const prop = await query<PropRead>('prop.get', { layer: target.layer.id, path: target.path }).catch(() => null)
        const dims = dimsOf(prop?.type, target.layer)
        if (!prop || prop.uid == null || !dims || !prop.keys?.length) continue
        const info = await query<KeyInfo[]>('keys.info', { keys: prop.keys.map((key) => ({ layer: target.layer.id, prop: prop.uid, time: key.layerTime })) }).catch(() => null)
        const eases = new Map((Array.isArray(info) ? info : []).map((key) => [Math.round(key.time * 1e6), key]))
        const keys = prop.keys.flatMap((key): GraphKey[] => {
          const value = numbersOf(key.value, dims)
          if (!value) return []
          const ease = eases.get(Math.round(key.layerTime * 1e6))
          return [{ compTime: key.compTime, layerTime: key.layerTime, value, in: sideOf(key.in), out: sideOf(key.out), inEase: ease?.inEase ?? [], outEase: ease?.outEase ?? [] }]
        })
        const values = (await sample(target.layer.id, target.path, from, to, count).catch(() => null)) ?? []
        const samples = values.map((value) => numbersOf(value, dims)).filter((value): value is number[] => value !== null)
        // A spatial property (Position, Anchor Point) eases its speed along its path: one ease for all its dimensions.
        const spatial = dims > 1 && keys.some((key) => key.outEase.length === 1 || key.inEase.length === 1)
        read.push({ target, uid: prop.uid, dims, keys, samples, from, to, spatial })
      }
      if (ask === asked.current) setCurves(read)
    })()
  }, [targets, from, to, count, revision, query, sample])
  return curves
}

/** The values the graph spans: every sample's and key's, with room around them. */
function rangeOf(curves: readonly Curve[]): [number, number] {
  let low = Infinity
  let high = -Infinity
  for (const curve of curves) {
    for (const value of [...curve.samples, ...curve.keys.map((key) => key.value)]) {
      for (const part of value) {
        low = Math.min(low, part)
        high = Math.max(high, part)
      }
    }
  }
  if (!Number.isFinite(low)) return [0, 1]
  if (high - low < 1e-6) return [low - 1 - Math.abs(low) * 0.1, high + 1 + Math.abs(high) * 0.1]
  return [low, high]
}

/** Round numbers between `low` and `high` for the value grid, about `count` of them. */
function gridOf(low: number, high: number, count: number) {
  const raw = (high - low) / Math.max(1, count)
  const power = 10 ** Math.floor(Math.log10(raw))
  const step = [1, 2, 5, 10].map((factor) => factor * power).find((candidate) => candidate >= raw) ?? raw
  const lines: number[] = []
  for (let value = Math.ceil(low / step) * step; value <= high + 1e-9 && lines.length < 40; value += step) lines.push(Math.round(value / step) * step)
  return lines
}

type Drag =
  | { kind: 'key'; pointerId: number; curve: Curve; key: GraphKey; dim: number; x: number; y: number; moved: boolean; time: number; value: number }
  | { kind: 'handle'; pointerId: number; curve: Curve; key: GraphKey; dim: number; side: 'in' | 'out'; span: number; time: number; value: number }

/**
 * The graph editor (Shift+F3 or the timeline's 图表 button): the value curves of the selected
 * layers' animated properties over the stretch of time the timeline shows, one per dimension. A
 * key clicks to select it (Shift adds or removes it) and drags in time (snapped like the
 * timeline's) and value; a selected eased key shows its Bezier handles, which drag to change its
 * speed and influence. Each drag is one step (`keys.set`, `keys.setEase`), sent on release.
 * Position and Anchor Point ease their speed along their path, so their keys drag but show no handles.
 */
export function GraphEditor({ targets, selected, revision }: { targets: readonly GraphTarget[]; selected: ReadonlySet<string>; revision: string }) {
  const { run, t } = useKitEditor()
  const frame = useTimeline()
  const { info, view, width, readOnly } = frame
  const box = useRef<HTMLDivElement>(null)
  const [height, setHeight] = useState(0)
  useEffect(() => {
    const element = box.current
    if (!element) return undefined
    const observer = new ResizeObserver(() => setHeight(element.getBoundingClientRect().height))
    observer.observe(element)
    setHeight(element.getBoundingClientRect().height)
    return () => observer.disconnect()
  }, [])
  const count = Math.round(Math.min(600, Math.max(48, width / 3)))
  const curves = useCurves(targets, view.start, view.start + view.span, count, revision)
  const [drag, setDrag] = useState<Drag | null>(null)
  // The value range stays put while something is dragged, so the drag does not rescale under the pointer.
  const live = useMemo(() => rangeOf(curves), [curves])
  const held = useRef<[number, number] | null>(null)
  const [low, high] = drag && held.current ? held.current : live
  const tall = Math.max(1, height - 2 * PAD)
  const x = (seconds: number) => ((seconds - view.start) / view.span) * width
  const y = (value: number) => PAD + ((high - value) / (high - low)) * tall
  const valueAt = (py: number) => high - ((py - PAD) / tall) * (high - low)
  const secondsAt = (px: number) => view.start + (px / Math.max(1, width)) * view.span

  const local = (event: { clientX: number; clientY: number }) => {
    const rect = box.current?.querySelector('svg')?.getBoundingClientRect()
    return rect ? { px: event.clientX - rect.left, py: event.clientY - rect.top } : { px: 0, py: 0 }
  }

  const keyRef = (curve: Curve, key: GraphKey) => ({ layer: curve.target.layer.id, path: curve.target.path, time: key.compTime })
  const isSelected = (curve: Curve, key: GraphKey) => selected.has(keyId(keyRef(curve, key)))

  const pressKey = (event: ReactPointerEvent<SVGElement>, curve: Curve, key: GraphKey, dim: number) => {
    if (event.button !== 0) return
    event.stopPropagation()
    const add = event.shiftKey || event.metaKey || event.ctrlKey
    if (add || !isSelected(curve, key)) void run('keys.select', { keys: [keyRef(curve, key)], timeBase: 'comp', selectProperties: true, ...(add ? { toggle: true } : {}) })
    if (readOnly) return
    event.currentTarget.setPointerCapture(event.pointerId)
    held.current = live
    setDrag({ kind: 'key', pointerId: event.pointerId, curve, key, dim, x: event.clientX, y: event.clientY, moved: false, time: key.compTime, value: key.value[dim] ?? 0 })
  }

  const pressHandle = (event: ReactPointerEvent<SVGElement>, curve: Curve, key: GraphKey, dim: number, side: 'in' | 'out', span: number) => {
    if (event.button !== 0 || readOnly) return
    event.stopPropagation()
    event.currentTarget.setPointerCapture(event.pointerId)
    held.current = live
    const { px, py } = local(event)
    setDrag({ kind: 'handle', pointerId: event.pointerId, curve, key, dim, side, span, time: secondsAt(px), value: valueAt(py) })
  }

  const move = (event: ReactPointerEvent<SVGElement>) => {
    if (!drag || drag.pointerId !== event.pointerId) return
    const { px, py } = local(event)
    if (drag.kind === 'key') {
      const moved = drag.moved || Math.hypot(event.clientX - drag.x, event.clientY - drag.y) > CLICK_SLOP
      if (!moved) return
      setDrag({ ...drag, moved, time: Math.max(0, snapIn(frame, secondsAt(px))), value: valueAt(py) })
      return
    }
    setDrag({ ...drag, time: secondsAt(px), value: valueAt(py) })
  }

  const release = (event: ReactPointerEvent<SVGElement>, cancelled: boolean) => {
    if (!drag || drag.pointerId !== event.pointerId) return
    setDrag(null)
    if (cancelled) return
    const { curve, key, dim } = drag
    const { layer, path } = curve.target
    if (drag.kind === 'key') {
      if (!drag.moved) return
      const value = key.value.map((part, index) => (index === dim ? drag.value : part))
      const whole = curve.dims === 1 ? value[0] : value
      const params: Record<string, unknown> = { layer: layer.id, path, time: key.compTime, timeBase: 'comp', value: whole }
      if (Math.abs(drag.time - key.compTime) > 1e-9) params.newTime = drag.time
      void run('keys.set', params)
      return
    }
    const ease = handleEase(drag, key)
    if (ease) void run('keys.setEase', { layer: layer.id, path, time: key.layerTime, side: drag.side, dim, influence: ease.influence, speed: ease.speed })
  }

  if (!targets.length) {
    return (
      <div className="motion-graph" ref={box} data-testid="motion-graph">
        <div className="motion-timeline-names motion-graph-names" />
        <p className="kui-muted motion-graph-empty">{t('graph.empty')}</p>
      </div>
    )
  }

  return (
    <div className="motion-graph" ref={box} data-testid="motion-graph" data-range={`${low} ${high}`}>
      <div className="motion-timeline-names motion-graph-names">
        <ul className="motion-graph-legend">
          {curves.map((curve) => (
            <li key={`${curve.target.layer.id}-${curve.target.path}`} data-testid={`motion-graph-legend-${curve.target.layer.id}-${pathId(curve.target.path)}`}>
              <span className="motion-graph-legend-name" title={`${curve.target.layer.name} › ${curve.target.name}`}>
                {curve.target.layer.name} › {curve.target.name}
              </span>
              {Array.from({ length: curve.dims }, (_, dim) => (
                <span key={dim} className="motion-graph-chip" data-dim={curve.dims === 1 ? 'one' : DIM_CLASS[dim]}>
                  {curve.dims === 1 ? '' : DIM_CLASS[dim].toUpperCase()}
                </span>
              ))}
            </li>
          ))}
        </ul>
      </div>
      <div className="motion-graph-plot">
        <svg width={width} height={height} role="img" aria-label={t('graph.title')} onPointerMove={move} onPointerUp={(event) => release(event, false)} onPointerCancel={(event) => release(event, true)}>
          {gridOf(low, high, Math.max(2, Math.floor(tall / 40))).map((value) => (
            <g key={value} className="motion-graph-grid">
              <line x1={0} x2={width} y1={y(value)} y2={y(value)} />
              <text x={4} y={y(value) - 2}>
                {Math.abs(value) >= 1000 ? value.toFixed(0) : Number(value.toFixed(3))}
              </text>
            </g>
          ))}
          <line className="motion-graph-playhead" x1={x(info.time)} x2={x(info.time)} y1={0} y2={height} />
          {curves.map((curve) =>
            Array.from({ length: curve.dims }, (_, dim) => {
              const id = `${curve.target.layer.id}-${pathId(curve.target.path)}-${dim}`
              const step = curve.samples.length > 1 ? (curve.to - curve.from) / (curve.samples.length - 1) : 0
              const points = curve.samples.map((value, index) => `${x(curve.from + step * index).toFixed(1)},${y(value[dim] ?? 0).toFixed(1)}`).join(' ')
              const kind = curve.dims === 1 ? 'one' : DIM_CLASS[dim]
              return (
                <g key={id} className="motion-graph-curve" data-dim={kind}>
                  <polyline points={points} data-testid={`motion-graph-curve-${id}`} />
                  {curve.keys.map((key, index) => {
                    const dragging = drag?.kind === 'key' && drag.moved && drag.curve === curve && drag.key === key
                    const at = dragging ? { time: drag.time, value: key.value.map((part, which) => (which === dim ? drag.value : part))[dim] } : { time: key.compTime, value: key.value[dim] ?? 0 }
                    const on = isSelected(curve, key)
                    const handles = on && !curve.spatial ? handlesOf(curve, index, dim, drag) : []
                    return (
                      <g key={key.compTime}>
                        {handles.map((handle) => (
                          <g key={handle.side} className="motion-graph-handle">
                            <line x1={x(key.compTime)} y1={y(key.value[dim] ?? 0)} x2={x(handle.time)} y2={y(handle.value)} />
                            <circle
                              cx={x(handle.time)}
                              cy={y(handle.value)}
                              r={4}
                              onPointerDown={(event) => pressHandle(event, curve, key, dim, handle.side, handle.span)}
                              data-testid={`motion-graph-handle-${id}-${Math.round(key.compTime * 1000)}-${handle.side}`}
                            />
                          </g>
                        ))}
                        <rect
                          className="motion-graph-key"
                          aria-selected={on}
                          x={x(at.time) - 4}
                          y={y(at.value) - 4}
                          width={8}
                          height={8}
                          onPointerDown={(event) => pressKey(event, curve, key, dim)}
                          data-testid={`motion-graph-key-${id}-${Math.round(key.compTime * 1000)}`}
                        />
                      </g>
                    )
                  })}
                </g>
              )
            }),
          )}
        </svg>
      </div>
    </div>
  )
}

/** A Bezier handle of a key: which side, where it is (seconds, value), and the length of its segment. */
type Handle = { side: 'in' | 'out'; time: number; value: number; span: number }

/**
 * The handles a selected key shows on dimension `dim`: one per eased (Bezier) side that has a
 * neighbouring key, at its influence along the segment and its speed's slope, or where a handle
 * of it is being dragged.
 */
function handlesOf(curve: Curve, index: number, dim: number, drag: Drag | null): Handle[] {
  const key = curve.keys[index]
  const handles: Handle[] = []
  const sides: Array<['in' | 'out', GraphKey | undefined, Ease | undefined, Side]> = [
    ['in', curve.keys[index - 1], key.inEase[dim], key.in],
    ['out', curve.keys[index + 1], key.outEase[dim], key.out],
  ]
  for (const [side, other, ease, kind] of sides) {
    if (!other || !ease || kind !== 'bezier') continue
    const span = Math.abs(other.compTime - key.compTime)
    if (drag?.kind === 'handle' && drag.key === key && drag.dim === dim && drag.side === side) {
      handles.push({ side, time: drag.time, value: drag.value, span })
      continue
    }
    const reach = (ease.influence / 100) * span
    const sign = side === 'out' ? 1 : -1
    handles.push({ side, time: key.compTime + sign * reach, value: (key.value[dim] ?? 0) + sign * ease.speed * reach, span })
  }
  return handles
}

/** The influence (%) and speed a handle dragged to (`time`, `value`) gives its side of the key. */
function handleEase(drag: Extract<Drag, { kind: 'handle' }>, key: GraphKey) {
  const sign = drag.side === 'out' ? 1 : -1
  const reach = sign * (drag.time - key.compTime)
  if (!(drag.span > 0)) return null
  const clamped = Math.min(drag.span, Math.max(drag.span * 0.001, reach))
  const influence = Math.min(100, Math.max(0.1, (clamped / drag.span) * 100))
  const speed = (sign * (drag.value - (key.value[drag.dim] ?? 0))) / clamped
  return Number.isFinite(speed) ? { influence, speed } : null
}
