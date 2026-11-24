import { useEffect, useMemo, useRef, useState, type CSSProperties, type MutableRefObject, type PointerEvent as ReactPointerEvent } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { snapIn, useTimeline } from './context'
import { sideOf, type PropRead, type Side } from './interp'
import { keyId } from './keys'
import { pathId } from './PropertyRows'
import { useSampler } from './samples'

/** A property the graph shows: an animated property of a selected layer. */
export type GraphTarget = { layer: MotionLayer; path: string; name: string }

/** What the graph draws: the properties' values over time, or how fast they change. */
export type GraphMode = 'value' | 'speed'

/** One side's temporal ease as `keys.info` answers it, per dimension (null where it has none). */
type Ease = { speed: number; influence: number } | null
type KeyInfo = { time: number; inEase?: Ease[]; outEase?: Ease[] }

/** One key of a curve: its times, its value per dimension, its sides, and its eases. */
type GraphKey = { compTime: number; layerTime: number; value: number[]; in: Side; out: Side; inEase: Ease[]; outEase: Ease[] }

/** A property's curves: one per dimension, sampled over the stretch in view. */
type Curve = { target: GraphTarget; uid: number; dims: number; keys: GraphKey[]; samples: number[][]; from: number; to: number; spatial: boolean }

/**
 * One line the graph draws for a curve: a dimension's values, a dimension's speed, or (for
 * Position and Anchor Point in the speed graph) the speed along the path. `ease` is the index of
 * the keys' eases it shows handles for.
 */
type Line = { dim: number; kind: string; points: Array<[number, number]>; ease: number }

/** The colours of the dimensions (x, y, z), and of a one-number property. */
const DIM_CLASS = ['x', 'y', 'z']

/** One-line curves (a number, or a speed along a path) take a colour each, by their place in the list. */
const ONE_COLOURS = ['var(--kit-primary)', '#d9822b', '#2b9e8f', '#c2417a', '#6b7fd7']
const lineStyle = (kind: string, index: number) => (kind === 'one' ? ({ '--motion-dim': ONE_COLOURS[index % ONE_COLOURS.length] } as CSSProperties) : undefined)

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

/** The time of a curve's sample `index`. */
const sampleTime = (curve: Curve, index: number) => curve.from + (curve.samples.length > 1 ? ((curve.to - curve.from) * index) / (curve.samples.length - 1) : 0)

/**
 * The lines a curve draws in `mode`: in the value graph one per dimension; in the speed graph the
 * change per second of each dimension, or for a spatial property its speed along the path (the
 * length of the change), which is what its ease sets.
 */
function linesOf(curve: Curve, mode: GraphMode): Line[] {
  const kindOf = (dim: number) => (curve.dims === 1 ? 'one' : DIM_CLASS[dim])
  if (mode === 'value') {
    return Array.from({ length: curve.dims }, (_, dim) => ({ dim, kind: kindOf(dim), ease: dim, points: curve.samples.map((value, index): [number, number] => [sampleTime(curve, index), value[dim] ?? 0]) }))
  }
  const n = curve.samples.length
  const rate = (index: number, dim: number) => {
    const [a, b] = [Math.max(0, index - 1), Math.min(n - 1, index + 1)]
    const dt = sampleTime(curve, b) - sampleTime(curve, a)
    return dt > 0 ? ((curve.samples[b][dim] ?? 0) - (curve.samples[a][dim] ?? 0)) / dt : 0
  }
  if (curve.spatial) {
    const points = curve.samples.map((_, index): [number, number] => [sampleTime(curve, index), Math.hypot(...Array.from({ length: curve.dims }, (__, dim) => rate(index, dim)))])
    return [{ dim: 0, kind: 'one', ease: 0, points }]
  }
  return Array.from({ length: curve.dims }, (_, dim) => ({ dim, kind: kindOf(dim), ease: dim, points: curve.samples.map((__, index): [number, number] => [sampleTime(curve, index), rate(index, dim)]) }))
}

/** The value a line has at `time` (its nearest sample). */
function lineAt(line: Line, time: number) {
  let best = line.points[0]
  for (const point of line.points) if (Math.abs(point[0] - time) < Math.abs((best?.[0] ?? Infinity) - time)) best = point
  return best?.[1] ?? 0
}

/** The values a curve's lines span, with its keys' values (value graph) or its eases' speeds (speed graph), with room around them. */
function rangeOf(curve: Curve, lines: readonly Line[], mode: GraphMode): [number, number] {
  let low = mode === 'speed' && curve.spatial ? 0 : Infinity
  let high = -Infinity
  const take = (value: number) => {
    if (!Number.isFinite(value)) return
    low = Math.min(low, value)
    high = Math.max(high, value)
  }
  for (const line of lines) for (const [, value] of line.points) take(value)
  for (const key of curve.keys) {
    if (mode === 'value') key.value.forEach(take)
    else for (const ease of [...key.inEase, ...key.outEase]) if (ease) take(ease.speed)
  }
  if (!Number.isFinite(low) || !Number.isFinite(high)) return [0, 1]
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

/** A curve's id in test ids and keys: its layer and its property's path. */
const curveId = (curve: Curve) => `${curve.target.layer.id}-${pathId(curve.target.path)}`

type Drag =
  | { kind: 'key'; pointerId: number; curve: Curve; key: GraphKey; dim: number; x: number; y: number; moved: boolean; time: number; value: number }
  | { kind: 'handle'; pointerId: number; curve: Curve; key: GraphKey; dim: number; side: 'in' | 'out'; span: number; time: number; value: number }
  | { kind: 'box'; pointerId: number; x0: number; y0: number; x1: number; y1: number; add: boolean }

/**
 * The graph editor (Shift+F3 or the timeline's 图表 button): the selected layers' animated
 * properties over the stretch of time the timeline shows, as their values or (速度) how fast they
 * change. Each property is drawn on its own vertical scale, fitted to its own values, so an
 * Opacity from 0 to 100 and a Position in the thousands both fill the height; the value labels
 * belong to the property marked in the list (a click on another's name moves them there). A key
 * clicks to select it (Shift adds or removes it), a drag on empty space selects the keys in its
 * box, and 适合全部 (F) shows every key's time. In the value graph a key drags in time and value;
 * in the speed graph in time. A selected eased key shows its handles, which drag its influence
 * (sideways) and speed: in the value graph as the curve's slope, in the speed graph as the height,
 * which is also where Position and Anchor Point, whose ease is their speed along the path, show
 * theirs. Each drag is one step (`keys.set`, `keys.setEase`, `keys.select`), sent on release.
 */
export function GraphEditor({
  targets,
  selected,
  revision,
  onFit,
  fitRef,
}: {
  targets: readonly GraphTarget[]
  selected: ReadonlySet<string>
  revision: string
  onFit: (start: number, end: number) => void
  fitRef: MutableRefObject<(() => void) | null>
}) {
  const { run, t } = useKitEditor()
  const frame = useTimeline()
  const { info, view, width, readOnly } = frame
  const box = useRef<HTMLDivElement>(null)
  const [height, setHeight] = useState(0)
  const [mode, setMode] = useState<GraphMode>('value')
  const [scaleOf, setScaleOf] = useState<string | null>(null)
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
  const [drag, showDrag] = useState<Drag | null>(null)
  // The drag as the pointer has left it: a busy page takes several moves and the release in one go,
  // before the moves' render, so the handlers read this and not the drag of the last render.
  const dragNow = useRef<Drag | null>(null)
  const setDrag = (next: Drag | null) => {
    dragNow.current = next
    showDrag(next)
  }
  const lines = useMemo(() => new Map(curves.map((curve) => [curve, linesOf(curve, mode)])), [curves, mode])
  // Each curve's own value range; it stays put while something is dragged, so the drag does not rescale under the pointer.
  const live = useMemo(() => new Map(curves.map((curve) => [curveId(curve), rangeOf(curve, lines.get(curve) ?? [], mode)])), [curves, lines, mode])
  const held = useRef<Map<string, [number, number]> | null>(null)
  const ranges = drag && drag.kind !== 'box' && held.current ? held.current : live
  const tall = Math.max(1, height - 2 * PAD)
  const rangeFor = (curve: Curve) => ranges.get(curveId(curve)) ?? [0, 1]
  const x = (seconds: number) => ((seconds - view.start) / view.span) * width
  const y = (curve: Curve, value: number) => {
    const [low, high] = rangeFor(curve)
    return PAD + ((high - value) / (high - low)) * tall
  }
  const valueAt = (curve: Curve, py: number) => {
    const [low, high] = rangeFor(curve)
    return high - ((py - PAD) / tall) * (high - low)
  }
  const secondsAt = (px: number) => view.start + (px / Math.max(1, width)) * view.span

  const keyRef = (curve: Curve, key: GraphKey) => ({ layer: curve.target.layer.id, path: curve.target.path, time: key.compTime })
  const isSelected = (curve: Curve, key: GraphKey) => selected.has(keyId(keyRef(curve, key)))
  // The scale's owner: the one picked in the list, else the first curve with a selected key, else the first.
  const owner = curves.find((curve) => curveId(curve) === scaleOf) ?? curves.find((curve) => curve.keys.some((key) => isSelected(curve, key))) ?? curves[0]

  /** Where a key's mark sits on one of its lines: its value, or the line's speed at its time. */
  const markOf = (curve: Curve, line: Line, key: GraphKey) => (mode === 'value' ? (key.value[line.dim] ?? 0) : lineAt(line, key.compTime))

  // 适合全部 (F): the timeline shows from the first key to the last, with a little room.
  const fit = () => {
    const times = curves.flatMap((curve) => curve.keys.map((key) => key.compTime))
    if (!times.length) return
    const [first, last] = [Math.min(...times), Math.max(...times)]
    const room = Math.max(info.frame * 4, (last - first) * 0.08)
    onFit(Math.max(0, first - room), last + room)
  }
  fitRef.current = fit

  const local = (event: { clientX: number; clientY: number }) => {
    const rect = box.current?.querySelector('svg')?.getBoundingClientRect()
    return rect ? { px: event.clientX - rect.left, py: event.clientY - rect.top } : { px: 0, py: 0 }
  }

  // A press on the graph starts no text selection or native drag: WebKit starts one on the move and
  // cancels the pointer when a drag begins, which would drop the box, key or handle being dragged.
  // The press then no longer takes the focus from a text field, so that is done here.
  const own = (event: ReactPointerEvent<SVGElement>) => {
    event.preventDefault()
    const active = document.activeElement
    if (active instanceof HTMLElement && active.matches('input, textarea, [contenteditable]')) active.blur()
  }

  const pressKey = (event: ReactPointerEvent<SVGElement>, curve: Curve, key: GraphKey, dim: number, mark: number) => {
    if (event.button !== 0) return
    event.stopPropagation()
    own(event)
    const add = event.shiftKey || event.metaKey || event.ctrlKey
    if (add || !isSelected(curve, key)) void run('keys.select', { keys: [keyRef(curve, key)], timeBase: 'comp', selectProperties: true, ...(add ? { toggle: true } : {}) })
    if (readOnly) return
    event.currentTarget.setPointerCapture(event.pointerId)
    held.current = live
    setDrag({ kind: 'key', pointerId: event.pointerId, curve, key, dim, x: event.clientX, y: event.clientY, moved: false, time: key.compTime, value: mark })
  }

  const pressHandle = (event: ReactPointerEvent<SVGElement>, curve: Curve, key: GraphKey, dim: number, side: 'in' | 'out', span: number) => {
    if (event.button !== 0 || readOnly) return
    event.stopPropagation()
    own(event)
    event.currentTarget.setPointerCapture(event.pointerId)
    held.current = live
    const { px, py } = local(event)
    setDrag({ kind: 'handle', pointerId: event.pointerId, curve, key, dim, side, span, time: secondsAt(px), value: valueAt(curve, py) })
  }

  // A press on empty space starts a selection box.
  const pressSpace = (event: ReactPointerEvent<SVGElement>) => {
    if (event.button !== 0) return
    own(event)
    event.currentTarget.setPointerCapture(event.pointerId)
    const { px, py } = local(event)
    setDrag({ kind: 'box', pointerId: event.pointerId, x0: px, y0: py, x1: px, y1: py, add: event.shiftKey || event.metaKey || event.ctrlKey })
  }

  /** `drag` taken to where the pointer of `event` is. */
  const movedTo = (drag: Drag, event: ReactPointerEvent<SVGElement>): Drag => {
    const { px, py } = local(event)
    if (drag.kind === 'box') return { ...drag, x1: px, y1: py }
    if (drag.kind === 'key') {
      const moved = drag.moved || Math.hypot(event.clientX - drag.x, event.clientY - drag.y) > CLICK_SLOP
      if (!moved) return drag
      return { ...drag, moved, time: Math.max(0, snapIn(frame, secondsAt(px))), value: mode === 'value' ? valueAt(drag.curve, py) : drag.value }
    }
    return { ...drag, time: secondsAt(px), value: valueAt(drag.curve, py) }
  }

  const move = (event: ReactPointerEvent<SVGElement>) => {
    const current = dragNow.current
    if (!current || current.pointerId !== event.pointerId) return
    const next = movedTo(current, event)
    if (next !== current) setDrag(next)
  }

  const release = (event: ReactPointerEvent<SVGElement>, cancelled: boolean) => {
    const current = dragNow.current
    if (!current || current.pointerId !== event.pointerId) return
    setDrag(null)
    if (cancelled) return
    // Where the pointer was let go, whether or not its last moves have been drawn.
    const drag = movedTo(current, event)
    if (drag.kind === 'box') {
      const [left, right] = [Math.min(drag.x0, drag.x1), Math.max(drag.x0, drag.x1)]
      const [top, bottom] = [Math.min(drag.y0, drag.y1), Math.max(drag.y0, drag.y1)]
      const inside = curves.flatMap((curve) =>
        curve.keys
          .filter((key) =>
            (lines.get(curve) ?? []).some((line) => {
              const [kx, ky] = [x(key.compTime), y(curve, markOf(curve, line, key))]
              return kx >= left && kx <= right && ky >= top && ky <= bottom
            }),
          )
          .map((key) => keyRef(curve, key)),
      )
      // A click on empty space (no box) without Shift clears the selected keys.
      if (right - left <= CLICK_SLOP && bottom - top <= CLICK_SLOP) {
        if (!drag.add) void run('keys.select', { keys: [], timeBase: 'comp' })
        return
      }
      void run('keys.select', { keys: inside, timeBase: 'comp', ...(drag.add ? { add: true } : {}) })
      return
    }
    const { curve, key, dim } = drag
    const { layer, path } = curve.target
    if (drag.kind === 'key') {
      if (!drag.moved) return
      const params: Record<string, unknown> = { layer: layer.id, path, time: key.compTime, timeBase: 'comp' }
      if (mode === 'value') {
        const value = key.value.map((part, index) => (index === dim ? drag.value : part))
        params.value = curve.dims === 1 ? value[0] : value
      }
      if (Math.abs(drag.time - key.compTime) > 1e-9) params.newTime = drag.time
      if (params.value !== undefined || params.newTime !== undefined) void run('keys.set', params)
      return
    }
    const ease = handleEase(drag, key, mode)
    if (ease) void run('keys.setEase', { layer: layer.id, path, time: key.layerTime, side: drag.side, dim, influence: ease.influence, speed: ease.speed })
  }

  const head = (
    <div className="motion-graph-tools">
      <div className="motion-graph-modes" role="group" aria-label={t('graph.mode')}>
        {(['value', 'speed'] as const).map((each) => (
          <button key={each} type="button" aria-pressed={mode === each} onClick={() => setMode(each)} data-testid={`motion-graph-mode-${each}`}>
            {t(`graph.mode.${each}`)}
          </button>
        ))}
      </div>
      <button type="button" title={t('graph.fitHint')} onClick={fit} disabled={!curves.length} data-testid="motion-graph-fit">
        {t('graph.fit')}
      </button>
    </div>
  )

  if (!targets.length) {
    return (
      <div className="motion-graph" ref={box} data-testid="motion-graph">
        <div className="motion-timeline-names motion-graph-names">{head}</div>
        <p className="kui-muted motion-graph-empty">{t('graph.empty')}</p>
      </div>
    )
  }

  const [ownLow, ownHigh] = owner ? rangeFor(owner) : [0, 1]
  return (
    <div className="motion-graph" ref={box} data-testid="motion-graph" data-mode={mode} data-scale={owner ? curveId(owner) : undefined}>
      <div className="motion-timeline-names motion-graph-names">
        {head}
        <ul className="motion-graph-legend">
          {curves.map((curve, curveIndex) => (
            <li key={curveId(curve)} data-scale={curve === owner || undefined} data-testid={`motion-graph-legend-${curveId(curve)}`}>
              <button type="button" className="motion-graph-legend-name" title={t('graph.scaleHint', { name: `${curve.target.layer.name} › ${curve.target.name}` })} onClick={() => setScaleOf(curveId(curve))}>
                {curve.target.layer.name} › {curve.target.name}
              </button>
              {(lines.get(curve) ?? []).map((line) => (
                <span key={line.dim} className="motion-graph-chip" data-dim={line.kind} style={lineStyle(line.kind, curveIndex)}>
                  {line.kind === 'one' ? '' : line.kind.toUpperCase()}
                </span>
              ))}
            </li>
          ))}
        </ul>
      </div>
      <div className="motion-graph-plot">
        <svg
          width={width}
          height={height}
          role="img"
          aria-label={t(mode === 'value' ? 'graph.title' : 'graph.titleSpeed')}
          onPointerDown={pressSpace}
          onPointerMove={move}
          onPointerUp={(event) => release(event, false)}
          onPointerCancel={(event) => release(event, true)}
          data-testid="motion-graph-svg"
        >
          {owner
            ? gridOf(ownLow, ownHigh, Math.max(2, Math.floor(tall / 40))).map((value) => (
                <g key={value} className="motion-graph-grid">
                  <line x1={0} x2={width} y1={y(owner, value)} y2={y(owner, value)} />
                  <text x={4} y={y(owner, value) - 2}>
                    {Math.abs(value) >= 1000 ? value.toFixed(0) : Number(value.toFixed(3))}
                  </text>
                </g>
              ))
            : null}
          <line className="motion-graph-playhead" x1={x(info.time)} x2={x(info.time)} y1={0} y2={height} />
          {curves.map((curve, curveIndex) =>
            (lines.get(curve) ?? []).map((line) => {
              const id = `${curveId(curve)}-${line.dim}`
              const points = line.points.map(([time, value]) => `${x(time).toFixed(1)},${y(curve, value).toFixed(1)}`).join(' ')
              return (
                <g key={id} className="motion-graph-curve" data-dim={line.kind} style={lineStyle(line.kind, curveIndex)}>
                  <polyline points={points} data-testid={`motion-graph-curve-${id}`} />
                  {curve.keys.map((key, index) => {
                    const mark = markOf(curve, line, key)
                    const dragging = drag?.kind === 'key' && drag.moved && drag.curve === curve && drag.key === key && drag.dim === line.dim
                    const at = dragging ? { time: drag.time, value: drag.value } : { time: key.compTime, value: mark }
                    const on = isSelected(curve, key)
                    // Spatial properties ease along their path: their handles are in the speed graph.
                    const handles = on && (mode === 'speed' || !curve.spatial) ? handlesOf(curve, index, line, mode, mark, drag) : []
                    return (
                      <g key={key.compTime}>
                        {handles.map((handle) => (
                          <g key={handle.side} className="motion-graph-handle">
                            <line x1={x(key.compTime)} y1={y(curve, handle.from)} x2={x(handle.time)} y2={y(curve, handle.value)} />
                            <circle
                              cx={x(handle.time)}
                              cy={y(curve, handle.value)}
                              r={4}
                              onPointerDown={(event) => pressHandle(event, curve, key, line.ease, handle.side, handle.span)}
                              data-testid={`motion-graph-handle-${id}-${Math.round(key.compTime * 1000)}-${handle.side}`}
                            />
                          </g>
                        ))}
                        <rect
                          className="motion-graph-key"
                          aria-selected={on}
                          x={x(at.time) - 4}
                          y={y(curve, at.value) - 4}
                          width={8}
                          height={8}
                          onPointerDown={(event) => pressKey(event, curve, key, line.dim, mark)}
                          data-testid={`motion-graph-key-${id}-${Math.round(key.compTime * 1000)}`}
                        />
                      </g>
                    )
                  })}
                </g>
              )
            }),
          )}
          {drag?.kind === 'box' ? (
            <rect
              className="motion-graph-box"
              x={Math.min(drag.x0, drag.x1)}
              y={Math.min(drag.y0, drag.y1)}
              width={Math.abs(drag.x1 - drag.x0)}
              height={Math.abs(drag.y1 - drag.y0)}
              data-testid="motion-graph-box"
            />
          ) : null}
        </svg>
      </div>
    </div>
  )
}

/** A Bezier handle of a key: which side, where it starts (the key's mark) and ends (seconds, value), and the length of its segment. */
type Handle = { side: 'in' | 'out'; from: number; time: number; value: number; span: number }

/**
 * The handles a selected key shows on `line`: one per eased (Bezier) side that has a neighbouring
 * key, at its influence along the segment; in the value graph at its speed's slope from the key's
 * value, in the speed graph at its speed's height. Or where a handle of it is being dragged.
 */
function handlesOf(curve: Curve, index: number, line: Line, mode: GraphMode, mark: number, drag: Drag | null): Handle[] {
  const key = curve.keys[index]
  const handles: Handle[] = []
  const sides: Array<['in' | 'out', GraphKey | undefined, Ease | undefined, Side]> = [
    ['in', curve.keys[index - 1], key.inEase[line.ease], key.in],
    ['out', curve.keys[index + 1], key.outEase[line.ease], key.out],
  ]
  for (const [side, other, ease, kind] of sides) {
    if (!other || !ease || kind !== 'bezier') continue
    const span = Math.abs(other.compTime - key.compTime)
    const sign = side === 'out' ? 1 : -1
    const from = mode === 'value' ? mark : ease.speed
    if (drag?.kind === 'handle' && drag.key === key && drag.dim === line.ease && drag.side === side) {
      handles.push({ side, from: mode === 'value' ? mark : drag.value, time: drag.time, value: drag.value, span })
      continue
    }
    const reach = (ease.influence / 100) * span
    handles.push({ side, from, time: key.compTime + sign * reach, value: mode === 'value' ? mark + sign * ease.speed * reach : ease.speed, span })
  }
  return handles
}

/**
 * The influence (%) and speed a handle dragged to (`time`, `value`) gives its side of the key:
 * the influence from how far along the segment it is; the speed from the slope back to the key
 * (value graph) or its height (speed graph).
 */
function handleEase(drag: Extract<Drag, { kind: 'handle' }>, key: GraphKey, mode: GraphMode) {
  const sign = drag.side === 'out' ? 1 : -1
  const reach = sign * (drag.time - key.compTime)
  if (!(drag.span > 0)) return null
  const clamped = Math.min(drag.span, Math.max(drag.span * 0.001, reach))
  const influence = Math.min(100, Math.max(0.1, (clamped / drag.span) * 100))
  const speed = mode === 'value' ? (sign * (drag.value - (key.value[drag.dim] ?? 0))) / clamped : drag.value
  return Number.isFinite(speed) ? { influence, speed } : null
}
