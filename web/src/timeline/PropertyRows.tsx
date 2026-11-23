import { ChevronDownIcon as Open, ChevronRightIcon as Closed, DiamondIcon as Diamond } from '@kition/tessera/icons/react'
import { useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { NumberField } from '../ValueField'
import { useTimeline } from './context'
import { KeyMark } from './KeyMark'
import { keyTimes } from './keys'
import { editableNumbers, propertyRows, type TreeNode, type Twirl } from './tree'
import { leftOf } from './view'

/** How deep the timeline reads a layer's property tree (shape groups nest). */
const TREE_DEPTH = 8

/** A path as a test id part (`transform/position` → `transform.position`). */
export const pathId = (path: string) => path.replaceAll('/', '.')

/**
 * A twirled-open layer's rows under it (`layer.tree`): groups that open and close, and per
 * property the add-keyframe diamond, its value at the current time (typed as in the Properties
 * panel; on an animated property that sets a keyframe here), and its keyframes on the track.
 */
export function PropertyRows({ layer, twirl, toggled, onToggle, selected }: { layer: MotionLayer; twirl: Twirl; toggled: ReadonlySet<string>; onToggle: (path: string) => void; selected: ReadonlySet<string> }) {
  const { run, t } = useKitEditor()
  const { info, view, readOnly } = useTimeline()
  const tree = useEngineQuery<{ properties?: TreeNode }>('layer.tree', { layer: layer.id, depth: TREE_DEPTH, time: info.time })
  if (!tree?.properties) return null
  const rows = propertyRows(tree.properties, layer, twirl, toggled)
  const set = (node: TreeNode, index: number, next: number) => {
    const value = Array.isArray(node.value) ? (node.value as number[]).map((part, at) => (at === index ? next : part)) : next
    void run('prop.set', { layer: layer.id, path: node.path, value, time: info.time, timeBase: 'comp' })
  }
  return (
    <>
      {rows.map((row) => {
        const { node, depth } = row
        const id = `${layer.id}-${pathId(node.path)}`
        if (row.kind === 'group') {
          return (
            <div key={node.path} className="motion-row motion-prop-row" data-testid={`motion-group-${id}`}>
              <div className="motion-timeline-names motion-prop-names" style={{ paddingLeft: 36 + depth * 14 }}>
                <button type="button" className="motion-twirl" aria-expanded={row.open} aria-label={node.name} onClick={() => onToggle(node.path)} data-testid={`motion-twirl-${id}`}>
                  {row.open ? <Open size={12} aria-hidden="true" /> : <Closed size={12} aria-hidden="true" />}
                </button>
                <span className="motion-row-name">{node.name}</span>
              </div>
              <div className="motion-row-track" />
            </div>
          )
        }
        const numbers = editableNumbers(node, layer)
        const times = keyTimes(layer, node.path)
        return (
          <div key={node.path} className="motion-row motion-prop-row" data-testid={`motion-prop-row-${id}`}>
            <div className="motion-timeline-names motion-prop-names" style={{ paddingLeft: 36 + depth * 14 }}>
              <button
                type="button"
                className="motion-key-button"
                data-animated={times.length > 0}
                aria-label={t('addKey', { name: node.name })}
                title={t('addKey', { name: node.name })}
                disabled={readOnly}
                onClick={() => void run('prop.addKey', { layer: layer.id, path: node.path, time: info.time, timeBase: 'comp' })}
                data-testid={`motion-row-add-key-${id}`}
              >
                <Diamond size={12} aria-hidden="true" />
              </button>
              <span className="motion-row-name motion-prop-label">{node.name}</span>
              {numbers ? (
                <span className="motion-prop-values">
                  {numbers.map((value, index) => (
                    <NumberField key={index} value={value} label={`${node.name} ${index + 1}`} onCommit={(next) => set(node, index, next)} testId={`motion-row-value-${id}-${index}`} readOnly={readOnly} />
                  ))}
                </span>
              ) : null}
            </div>
            <div className="motion-row-track">
              {times.map((time) => (
                <KeyMark key={time} time={time} keys={[{ layer: layer.id, path: node.path, time }]} selected={selected} label={node.name} row={id} />
              ))}
              <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
            </div>
          </div>
        )
      })}
    </>
  )
}
