import { createContext, useContext } from 'react'
import { ChevronDownIcon as Open, ChevronRightIcon as Closed } from '@kition/tessera/icons/react'
import { useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { keyTimes } from '../timeline/keys'
import { shown, type TreeNode } from '../timeline/tree'
import { pathId } from '../timeline/PropertyRows'
import { KeyDiamond } from './KeyDiamond'
import { propMeta, type EffectParam } from './meta'
import { PropertyValue } from './PropertyValue'
import { useOpen } from './Section'

/** What every row of the panel reads: the layer shown, the time, and the layer's mask names. */
export type PanelFrame = { layer: MotionLayer; time: number; frame: number; readOnly: boolean; masks: readonly string[] }

export const PanelContext = createContext<PanelFrame | null>(null)

export function usePanel(): PanelFrame {
  const frame = useContext(PanelContext)
  if (!frame) throw new Error('usePanel outside the Properties panel')
  return frame
}

/** Properties the engine keeps out of its own UI (a shape item's blend mode and path direction). */
const HIDDEN = new Set(['blend', 'direction'])

/** Whether the panel shows `node` on `layer`. */
export function visible(node: TreeNode, layer: MotionLayer) {
  if (!shown(node, layer)) return false
  if (!node.children && HIDDEN.has(node.match ?? '') && node.path.startsWith('contents/')) return false
  return true
}

/** A property's test id part: Transform's by its name alone (`position`), the rest by their path (`contents.group.contents.rect.size`). */
export function propId(path: string) {
  return path.startsWith('transform/') && path.split('/').length === 2 ? path.slice('transform/'.length) : pathId(path)
}

/**
 * One property: its keyframe diamond, its name (translated), and its value control. `params`
 * are its effect's parameters (from `effect.list`), when it belongs to one.
 */
export function PropertyRow({ node, parent, params, label }: { node: TreeNode; parent?: TreeNode; params?: readonly EffectParam[]; label?: string }) {
  const { phrase } = useKitEditor()
  const { layer, time, frame, readOnly, masks } = usePanel()
  const name = label ?? phrase(node.name)
  const id = propId(node.path)
  const meta = propMeta(node, parent, params, masks)
  // Text keeps its own field below its name; the rest sit beside it.
  const wide = node.type === 'text' || (node.type === 'scalar' && !!meta?.slider)
  return (
    <div className="motion-prop" data-wide={wide || undefined} data-testid={`motion-prop-${id}`}>
      <KeyDiamond layer={layer.id} path={node.path} name={name} times={keyTimes(layer, node.path)} time={time} frame={frame} testId={`motion-add-key-${id}`} readOnly={readOnly} />
      <span className="motion-prop-name kui-muted" title={name}>
        {name}
      </span>
      <PropertyValue layer={layer} node={node} meta={meta} time={time} name={name} testId={`motion-value-${id}`} readOnly={readOnly} />
    </div>
  )
}

/** A group inside a section: a twirl that opens and closes it (remembered per layer type), then its rows. */
export function PropertyGroup({ node, scope, depth, byDefault = false, label }: { node: TreeNode; scope: string; depth: number; byDefault?: boolean; label?: string }) {
  const { phrase } = useKitEditor()
  const [open, setOpen] = useOpen(scope, node.path, byDefault)
  const name = label ?? phrase(node.name)
  return (
    <div className="motion-prop-group" data-depth={depth} data-testid={`motion-group-${pathId(node.path)}`}>
      <button type="button" className="motion-prop-group-toggle" aria-expanded={open} onClick={() => setOpen(!open)} data-testid={`motion-group-toggle-${pathId(node.path)}`}>
        {open ? <Open size={12} aria-hidden="true" /> : <Closed size={12} aria-hidden="true" />}
        <span>{name}</span>
      </button>
      {open ? <PropertyChildren node={node} scope={scope} depth={depth + 1} /> : null}
    </div>
  )
}

/** A group's children: properties as rows, groups as nested groups (empty ones left out). */
export function PropertyChildren({ node, scope, depth, params }: { node: TreeNode; scope: string; depth: number; params?: readonly EffectParam[] }) {
  const { layer } = usePanel()
  return (
    <div className="motion-props">
      {(node.children ?? [])
        .filter((child) => visible(child, layer) && (!child.children || hasRows(child, layer)))
        .map((child) => (child.children ? <PropertyGroup key={child.path} node={child} scope={scope} depth={depth} /> : <PropertyRow key={child.path} node={child} parent={node} params={params} />))}
    </div>
  )
}

/** Whether a group has a property to show somewhere under it. */
export function hasRows(node: TreeNode, layer: MotionLayer): boolean {
  return (node.children ?? []).some((child) => visible(child, layer) && (!child.children || hasRows(child, layer)))
}
