import { createContext, useContext, useEffect, useRef, useState } from 'react'
import { ChevronDownIcon as Open, ChevronRightIcon as Closed } from '@kition/tessera/icons/react'
import { Checkbox, Select, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { keyTimes } from '../timeline/keys'
import { shown, type TreeNode } from '../timeline/tree'
import { pathId } from '../timeline/PropertyRows'
import { KeyDiamond } from './KeyDiamond'
import { propMeta, type EffectParam } from './meta'
import { PropertyValue } from './PropertyValue'
import { takeReveal, useOpen } from './Section'
import { PARAMETRIC_SHAPES, ShapeAddMenu, ShapeItemActions, ShapeItemMenu } from './ShapeItems'
import type { ContextMenuPoint } from '@kition/sdk/editor'

/** What every row of the panel reads: the layer shown, the time, and the layer's mask names. */
export type PanelFrame = { layer: MotionLayer; time: number; frame: number; readOnly: boolean }

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
  const { phrase, run } = useKitEditor()
  const { layer, time, frame, readOnly } = usePanel()
  const name = label ?? phrase(node.name)
  const id = propId(node.path)
  const chosen = layer.selectedProps.includes(node.path)
  const meta = propMeta(node, parent, params)
  // Text keeps its own field below its name; the rest sit beside it.
  const wide = node.type === 'text' || node.type === 'gradient' || (node.type === 'scalar' && !!meta?.slider)
  return (
    <div className="motion-prop" data-wide={wide || undefined} data-testid={`motion-prop-${id}`}>
      <KeyDiamond layer={layer.id} path={node.path} name={name} times={keyTimes(layer, node.path)} time={time} frame={frame} testId={`motion-add-key-${id}`} readOnly={readOnly} expression={node.expression !== undefined} />
      {/* A click on the name selects the property (Shift or Cmd adds it), as on the timeline. */}
      <span
        className="motion-prop-name kui-muted"
        title={name}
        role="button"
        tabIndex={-1}
        aria-pressed={chosen}
        data-selected={chosen || undefined}
        onClick={(event) => void run('prop.select', { layer: layer.id, path: node.path, add: event.shiftKey || event.metaKey || event.ctrlKey })}
        data-testid={`motion-prop-name-${id}`}
      >
        {name}
      </span>
      <PropertyValue layer={layer} node={node} meta={meta} time={time} name={name} testId={`motion-value-${id}`} readOnly={readOnly} />
    </div>
  )
}

/** A group inside a section: a twirl that opens and closes it (remembered per layer type), then its rows. */
/**
 * `item`: the group is a shape item (a group, shape, paint or operator in a shape layer's
 * contents) at `index` of `count` siblings: its header has its own buttons (on/off, up, down,
 * duplicate, delete), and a shape group an Add menu for what goes inside it.
 */
export function PropertyGroup({ node, scope, depth, byDefault = false, label, item }: { node: TreeNode; scope: string; depth: number; byDefault?: boolean; label?: string; item?: { index: number; count: number } }) {
  const { phrase } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const [open, setOpen] = useOpen(scope, node.path, byDefault)
  const name = label ?? phrase(node.name)
  const id = pathId(node.path)
  // A Rectangle, Ellipse or Polystar Path's header: a right-click converts it to a Bezier path.
  const [menu, setMenu] = useState<ContextMenuPoint | null>(null)
  // An item just added from an Add menu scrolls into view.
  const box = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (takeReveal(node.path)) box.current?.scrollIntoView({ block: 'nearest' })
  })
  const convertible = item != null && node.uid != null && PARAMETRIC_SHAPES.has(node.match ?? '') && !readOnly
  return (
    <div ref={box} className="motion-prop-group" data-depth={depth} data-testid={`motion-group-${id}`} data-off={node.enabled === false || undefined}>
      <span
        className="motion-prop-group-head"
        onContextMenu={
          convertible
            ? (event) => {
                event.preventDefault()
                setMenu({ x: event.clientX, y: event.clientY })
              }
            : undefined
        }
      >
        <button type="button" className="motion-prop-group-toggle" aria-expanded={open} onClick={() => setOpen(!open)} data-testid={`motion-group-toggle-${id}`}>
          {open ? <Open size={12} aria-hidden="true" /> : <Closed size={12} aria-hidden="true" />}
          <span>{name}</span>
        </button>
        {item && node.uid != null ? (
          <>
            {node.match === 'group' ? <ShapeAddMenu layer={layer.id} group={node.uid} items={node.children?.find((child) => child.match === 'contents')?.children} testId={`motion-shape-add-${id}`} readOnly={readOnly} compact /> : null}
            <ShapeItemActions layer={layer.id} uid={node.uid} enabled={node.enabled !== false} index={item.index} count={item.count} name={name} testId={`motion-item-${id}`} readOnly={readOnly} />
          </>
        ) : null}
      </span>
      {convertible && node.uid != null ? <ShapeItemMenu layer={layer.id} uid={node.uid} name={name} at={menu} onClose={() => setMenu(null)} testId={`motion-item-menu-${id}`} /> : null}
      {open && node.mask ? <MaskSettings node={node} mask={node.mask} /> : null}
      {open ? <PropertyChildren node={node} scope={scope} depth={depth + 1} /> : null}
    </div>
  )
}

/** The modes `layer.mask.mode` takes, from its parameter schema, in the engine's order. */
function maskModes(schema: Record<string, unknown> | null | undefined): string[] {
  const mode = (schema as Record<string, { enum?: unknown }> | null | undefined)?.mode
  return Array.isArray(mode?.enum) ? mode.enum.filter((value): value is string => typeof value === 'string') : []
}

/** A mask's mode and Inverted, above its feather, opacity and expansion; each change one undo step. */
function MaskSettings({ node, mask }: { node: TreeNode; mask: { mode: string; inverted: boolean } }) {
  const { run, t, phrase, byId } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const id = pathId(node.path)
  return (
    <div className="motion-prop motion-mask-settings" data-testid={`motion-mask-${id}`}>
      <span className="motion-prop-name kui-muted">{t('mask.mode')}</span>
      <Select
        value={mask.mode}
        disabled={readOnly}
        aria-label={t('mask.mode')}
        onChange={(event) => void run('layer.mask.mode', { layer: layer.id, mask: node.uid, mode: event.target.value })}
        data-testid={`motion-mask-mode-${id}`}
      >
        {maskModes(byId.get('layer.mask.mode')?.schema).map((mode) => (
          <option key={mode} value={mode}>
            {phrase(mode)}
          </option>
        ))}
      </Select>
      <Checkbox label={t('mask.inverted')} checked={mask.inverted} disabled={readOnly} onChange={(value) => void run('layer.mask.invert', { layer: layer.id, mask: node.uid, value })} testId={`motion-mask-invert-${id}`} />
    </div>
  )
}

/** A group's children: properties as rows, groups as nested groups (empty ones left out). */
export function PropertyChildren({ node, scope, depth, params }: { node: TreeNode; scope: string; depth: number; params?: readonly EffectParam[] }) {
  const { layer } = usePanel()
  // A shape layer's contents (or a shape group's): every group in it is a shape item, ordered.
  const items = layer.type === 'Shape' && node.match === 'contents' && node.path.startsWith('contents') ? (node.children ?? []).filter((child) => child.children) : null
  return (
    <div className="motion-props">
      {(node.children ?? [])
        .filter((child) => visible(child, layer) && (!child.children || hasRows(child, layer) || items?.includes(child)))
        .map((child) =>
          child.children ? (
            <PropertyGroup key={child.path} node={child} scope={scope} depth={depth} item={items?.includes(child) ? { index: items.indexOf(child), count: items.length } : undefined} />
          ) : (
            <PropertyRow key={child.path} node={child} parent={node} params={params} />
          ),
        )}
    </div>
  )
}

/** Whether a group has a property to show somewhere under it. */
export function hasRows(node: TreeNode, layer: MotionLayer): boolean {
  return (node.children ?? []).some((child) => visible(child, layer) && (!child.children || hasRows(child, layer)))
}
