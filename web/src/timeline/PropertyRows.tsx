import { useEffect, useMemo, useState } from 'react'
import { ChevronDownIcon as Open, ChevronRightIcon as Closed } from '@kition/tessera/icons/react'
import { useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { useEffectParams } from '../properties/catalog'
import { KeyDiamond } from '../properties/KeyDiamond'
import { propMeta } from '../properties/meta'
import { PropertyValue } from '../properties/PropertyValue'
import { PARAMETRIC_SHAPES, ShapeAddMenu, ShapeItemMenu } from '../properties/ShapeItems'
import type { ContextMenuPoint } from '@kition/sdk/editor'
import { useTimeline } from './context'
import { KeyMark } from './KeyMark'
import { sideOf, type PropRead } from './interp'
import { keyTimes } from './keys'
import { effectOf, parentsOf, propertyRows, type TreeNode, type Twirl } from './tree'
import { ExpressionRow, explainError, type ExprError } from './ExpressionRow'
import { leftOf } from './view'
import { useWaveformReader, type Waveform } from './samples'

/** How many peaks a waveform is drawn with. */
const PEAKS = 800

/**
 * A layer's sound drawn on its track from its in point to its out point (the engine's
 * `layerWaveform`, heard on its own with its levels, muted or not), asked again a moment after
 * each change. `data-peaks` and `data-loudest` say what it drew.
 */
function WaveformTrack({ layer, version }: { layer: MotionLayer; version: unknown }) {
  const read = useWaveformReader()
  const { view } = useTimeline()
  const [wave, setWave] = useState<Waveform | null>(null)
  useEffect(() => {
    let current = true
    const timer = window.setTimeout(() => {
      void read(layer.id, PEAKS).then((answer) => current && setWave(answer))
    }, 150)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [read, layer.id, version])
  if (!wave) return null
  const left = leftOf(view, wave.from)
  const right = leftOf(view, wave.to)
  const count = wave.peaks.length
  const points = count ? wave.peaks.map((peak, at) => `${(at / Math.max(1, count - 1)) * 100},${50 - peak * 48}`).join(' ') + ' ' + [...wave.peaks].reverse().map((peak, at) => `${((count - 1 - at) / Math.max(1, count - 1)) * 100},${50 + peak * 48}`).join(' ') : ''
  return (
    <svg
      className="motion-waveform"
      style={{ left, width: `calc(${right} - ${left})` }}
      viewBox="0 0 100 100"
      preserveAspectRatio="none"
      aria-hidden="true"
      data-testid={`motion-waveform-${layer.id}`}
      data-peaks={count}
      data-loudest={count ? Math.max(...wave.peaks).toFixed(3) : '0'}
    >
      <line x1="0" y1="50" x2="100" y2="50" />
      {count ? <polygon points={points} /> : null}
    </svg>
  )
}

/** An animated property's keyframes on its track, each shaped by its interpolation (`prop.get`). */
function PropertyKeys({ layer, path, times, selected, name, row }: { layer: number; path: string; times: number[]; selected: ReadonlySet<string>; name: string; row: string }) {
  const read = useEngineQuery<PropRead>('prop.get', { layer, path })
  const interps = new Map((read?.keys ?? []).map((key) => [Math.round(key.compTime * 1e6), { in: sideOf(key.in), out: sideOf(key.out) }]))
  return (
    <>
      {times.map((time) => (
        <KeyMark key={time} time={time} keys={[{ layer, path, time }]} selected={selected} label={name} row={row} interp={interps.get(Math.round(time * 1e6))} />
      ))}
    </>
  )
}

/** How deep the timeline reads a layer's property tree (shape groups nest). */
const TREE_DEPTH = 8

/** A path as a test id part (`transform/position` → `transform.position`). */
export const pathId = (path: string) => path.replaceAll('/', '.')

/**
 * A twirled-open layer's rows under it (`layer.tree`): groups that open and close, and per
 * property the keyframe diamond, its value at the current time (the Properties panel's control;
 * on an animated property an edit sets a keyframe here), and its keyframes on the track.
 */
export function PropertyRows({ layer, twirl, toggled, onToggle, selected }: { layer: MotionLayer; twirl: Twirl; toggled: ReadonlySet<string>; onToggle: (path: string) => void; selected: ReadonlySet<string> }) {
  const { phrase, t, run } = useKitEditor()
  const { info, view, readOnly } = useTimeline()
  // Expressions that fail now (the engine evaluates them at the current time).
  const failing = useEngineQuery<{ errors?: ExprError[] }>('expr.errors', { time: info.time })
  const paramsOf = useEffectParams()
  const tree = useEngineQuery<{ properties?: TreeNode }>('layer.tree', { layer: layer.id, depth: TREE_DEPTH, time: info.time })
  const parents = useMemo(() => (tree?.properties ? parentsOf(tree.properties) : new Map<string, TreeNode>()), [tree])
  // A Rectangle, Ellipse or Polystar Path row's right-click menu (Convert to Bezier Path).
  const [shapeMenu, setShapeMenu] = useState<{ uid: number; name: string; id: string; at: ContextMenuPoint } | null>(null)
  if (!tree?.properties) return null
  const rows = propertyRows(tree.properties, layer, twirl, toggled)
  return (
    <>
      {rows.map((row) => {
        const { node, depth } = row
        // Group and property names are the engine's English ones, translated where the editor knows them.
        const name = phrase(node.name)
        const id = `${layer.id}-${pathId(node.path)}`
        if (row.kind === 'waveform') {
          return (
            <div key={`${node.path}~wave`} className="motion-row motion-prop-row motion-wave-row" data-testid={`motion-waveform-row-${layer.id}`}>
              <div className="motion-timeline-names motion-prop-names" style={{ paddingLeft: 36 + depth * 14 }}>
                <span className="motion-row-name">{t('timeline.waveform')}</span>
              </div>
              <div className="motion-row-track">
                <WaveformTrack layer={layer} version={`${layer.in} ${layer.out} ${layer.start} ${JSON.stringify(node.children?.map((child) => child.value))} ${JSON.stringify(layer.animated.find((property) => property.path === 'audio/levels')?.times)}`} />
                <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
              </div>
            </div>
          )
        }
        if (row.kind === 'group') {
          return (
            <div
              key={node.path}
              className="motion-row motion-prop-row"
              data-testid={`motion-group-${id}`}
              onContextMenu={
                layer.type === 'Shape' && node.uid != null && PARAMETRIC_SHAPES.has(node.match ?? '') && !readOnly
                  ? (event) => {
                      event.preventDefault()
                      event.stopPropagation()
                      setShapeMenu({ uid: node.uid ?? 0, name, id, at: { x: event.clientX, y: event.clientY } })
                    }
                  : undefined
              }
            >
              <div className="motion-timeline-names motion-prop-names" style={{ paddingLeft: 36 + depth * 14 }}>
                <button type="button" className="motion-twirl" aria-expanded={row.open} aria-label={name} onClick={() => onToggle(node.path)} data-testid={`motion-twirl-${id}`}>
                  {row.open ? <Open size={12} aria-hidden="true" /> : <Closed size={12} aria-hidden="true" />}
                </button>
                <span className="motion-row-name">{name}</span>
                {/* A shape layer's Contents (and each shape group's): Add puts a shape, paint or operator at its end. */}
                {layer.type === 'Shape' && node.uid != null && (node.path === 'contents' || node.match === 'group') ? (
                  <ShapeAddMenu layer={layer.id} group={node.path === 'contents' ? undefined : node.uid} items={node.path === 'contents' ? node.children : node.children?.find((child) => child.match === 'contents')?.children} testId={`motion-row-shape-add-${id}`} readOnly={readOnly} compact />
                ) : null}
              </div>
              <div className="motion-row-track" />
            </div>
          )
        }
        const times = keyTimes(layer, node.path)
        const meta = propMeta(node, parents.get(node.path), paramsOf(effectOf(node.path, parents)))
        const failure = (failing?.errors ?? []).find((error) => error.layer === layer.id && error.path === node.path)
        const expression =
          node.expression !== undefined ? (
            <ExpressionRow
              key={`${node.path}=`}
              layer={layer.id}
              path={node.path}
              name={name}
              text={node.expression}
              enabled={node.expressionEnabled !== false}
              error={failure ? explainError(failure.message, t, node.expression) : null}
              depth={depth}
              time={info.time}
              testId={`motion-expr-${id}`}
              readOnly={readOnly}
            />
          ) : null
        return [
          <div key={node.path} className="motion-row motion-prop-row" data-selected={layer.selectedProps.includes(node.path) || undefined} data-testid={`motion-prop-row-${id}`}>
            <div className="motion-timeline-names motion-prop-names" style={{ paddingLeft: 36 + depth * 14 }}>
              <KeyDiamond layer={layer.id} path={node.path} name={name} times={times} time={info.time} frame={info.frame} testId={`motion-row-add-key-${id}`} readOnly={readOnly} expression={node.expression !== undefined} />
              {/* A click on the name selects the property (Shift or Cmd adds it): what Add Expression, Copy and the keyframe commands act on. */}
              <span
                className="motion-row-name motion-prop-label"
                role="button"
                tabIndex={-1}
                aria-pressed={layer.selectedProps.includes(node.path)}
                onPointerDown={(event) => event.stopPropagation()}
                onClick={(event) => void run('prop.select', { layer: layer.id, path: node.path, add: event.shiftKey || event.metaKey || event.ctrlKey })}
                data-testid={`motion-prop-name-${id}`}
              >
                {name}
              </span>
              <PropertyValue layer={layer} node={node} meta={meta} time={info.time} name={name} testId={`motion-row-value-${id}`} readOnly={readOnly} compact />
            </div>
            <div className="motion-row-track">
              {times.length ? <PropertyKeys layer={layer.id} path={node.path} times={times} selected={selected} name={name} row={id} /> : null}
              <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
            </div>
          </div>,
          expression,
        ]
      })}
      {shapeMenu ? <ShapeItemMenu layer={layer.id} uid={shapeMenu.uid} name={shapeMenu.name} at={shapeMenu.at} onClose={() => setShapeMenu(null)} testId={`motion-row-item-menu-${shapeMenu.id}`} /> : null}
    </>
  )
}
