import { useMemo } from 'react'
import { ChevronDownIcon as Open, ChevronRightIcon as Closed } from '@kition/tessera/icons/react'
import { useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { useEffectParams } from '../properties/catalog'
import { KeyDiamond } from '../properties/KeyDiamond'
import { propMeta } from '../properties/meta'
import { PropertyValue } from '../properties/PropertyValue'
import { useTimeline } from './context'
import { KeyMark } from './KeyMark'
import { keyTimes } from './keys'
import { effectOf, parentsOf, propertyRows, type TreeNode, type Twirl } from './tree'
import { leftOf } from './view'

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
  const { phrase } = useKitEditor()
  const { info, view, readOnly } = useTimeline()
  const paramsOf = useEffectParams()
  const tree = useEngineQuery<{ properties?: TreeNode }>('layer.tree', { layer: layer.id, depth: TREE_DEPTH, time: info.time })
  const parents = useMemo(() => (tree?.properties ? parentsOf(tree.properties) : new Map<string, TreeNode>()), [tree])
  if (!tree?.properties) return null
  const rows = propertyRows(tree.properties, layer, twirl, toggled)
  return (
    <>
      {rows.map((row) => {
        const { node, depth } = row
        // Group and property names are the engine's English ones, translated where the editor knows them.
        const name = phrase(node.name)
        const id = `${layer.id}-${pathId(node.path)}`
        if (row.kind === 'group') {
          return (
            <div key={node.path} className="motion-row motion-prop-row" data-testid={`motion-group-${id}`}>
              <div className="motion-timeline-names motion-prop-names" style={{ paddingLeft: 36 + depth * 14 }}>
                <button type="button" className="motion-twirl" aria-expanded={row.open} aria-label={name} onClick={() => onToggle(node.path)} data-testid={`motion-twirl-${id}`}>
                  {row.open ? <Open size={12} aria-hidden="true" /> : <Closed size={12} aria-hidden="true" />}
                </button>
                <span className="motion-row-name">{name}</span>
              </div>
              <div className="motion-row-track" />
            </div>
          )
        }
        const times = keyTimes(layer, node.path)
        const meta = propMeta(node, parents.get(node.path), paramsOf(effectOf(node.path, parents)), [])
        return (
          <div key={node.path} className="motion-row motion-prop-row" data-testid={`motion-prop-row-${id}`}>
            <div className="motion-timeline-names motion-prop-names" style={{ paddingLeft: 36 + depth * 14 }}>
              <KeyDiamond layer={layer.id} path={node.path} name={name} times={times} time={info.time} frame={info.frame} testId={`motion-row-add-key-${id}`} readOnly={readOnly} />
              <span className="motion-row-name motion-prop-label">{name}</span>
              <PropertyValue layer={layer} node={node} meta={meta} time={info.time} name={name} testId={`motion-row-value-${id}`} readOnly={readOnly} compact />
            </div>
            <div className="motion-row-track">
              {times.map((time) => (
                <KeyMark key={time} time={time} keys={[{ layer: layer.id, path: node.path, time }]} selected={selected} label={name} row={id} />
              ))}
              <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
            </div>
          </div>
        )
      })}
    </>
  )
}
