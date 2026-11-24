import { PlusIcon as Plus } from '@kition/tessera/icons/react'
import { Button, useKitEditor } from '@kition/sdk/editor'
import type { TreeNode } from '../timeline/tree'
import { PropertyChildren, PropertyGroup, PropertyRow, usePanel } from './PropertyTree'
import { ShapeAddMenu } from './ShapeItems'

/** The shapes a simple shape layer draws. */
const SHAPES = new Set(['rect', 'ellipse', 'star'])

/** Contents items that make a shape layer more than one shape with a fill and a stroke. */
const PATHS = new Set(['rect', 'ellipse', 'star', 'path', 'group'])

/** A shape layer drawn as one rectangle, ellipse, polygon or star group: the parts the Shape section shows. */
type Simple = { group: TreeNode; shape: TreeNode; fill?: TreeNode; stroke?: TreeNode }

/** The one group of a simple shape layer, or null when its contents are more than that. */
export function simpleShape(contents: TreeNode): Simple | null {
  const items = contents.children ?? []
  if (items.length !== 1 || items[0].match !== 'group') return null
  return simpleGroup(items[0])
}

/** A shape group drawing one rectangle, ellipse, polygon or star: its parts, or null. */
function simpleGroup(group: TreeNode): Simple | null {
  if (group.match !== 'group') return null
  const inner = (group.children ?? []).find((child) => child.match === 'contents')?.children ?? []
  const shapes = inner.filter((child) => PATHS.has(child.match ?? ''))
  if (shapes.length !== 1 || !SHAPES.has(shapes[0].match ?? '')) return null
  return { group, shape: shapes[0], fill: inner.find((child) => child.match === 'fill'), stroke: inner.find((child) => child.match === 'stroke') }
}

const child = (node: TreeNode | undefined, match: string) => node?.children?.find((item) => item.match === match)

/** The quick settings of a shape group drawing one shape: (property, its parent, label). */
function quickRows({ shape, fill, stroke }: Simple, t: (key: string) => string): Array<[TreeNode | undefined, TreeNode | undefined, string]> {
  return [
    [child(shape, 'size'), shape, t('shape.size')],
    [child(shape, 'roundness'), shape, t('shape.roundness')],
    [child(shape, 'points'), shape, t('shape.points')],
    [child(shape, 'outerRadius'), shape, t('shape.outerRadius')],
    [child(shape, 'innerRadius'), shape, t('shape.innerRadius')],
    [child(fill, 'color'), fill, t('shape.fillColor')],
    [child(fill, 'opacity'), fill, t('shape.fillOpacity')],
    [child(stroke, 'color'), stroke, t('shape.strokeColor')],
    [child(stroke, 'width'), stroke, t('shape.strokeWidth')],
  ]
}

/** A group's own items (what is in its Contents). */
const itemsOf = (group: TreeNode) => (group.children ?? []).find((item) => item.match === 'contents')?.children

/**
 * A shape layer's Contents section: for one rectangle, ellipse, polygon or star, its size (or
 * radius and points), roundness, fill colour and opacity, and stroke colour and width, with Add
 * fill and Add stroke when it has none; with several items, the same settings for each group that
 * draws one shape, under its name; then every item of its contents in a group.
 */
export function ShapeControls({ contents, scope }: { contents: TreeNode; scope: string }) {
  const { run, t } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const simple = simpleShape(contents)
  if (!simple) {
    const groups = (contents.children ?? []).flatMap((group) => {
      const parts = simpleGroup(group)
      return parts ? [[group, parts] as const] : []
    })
    return (
      <div className="motion-shape" data-testid="motion-shape-controls">
        {groups.map(([group, parts]) => (
          <div key={group.path} className="motion-shape-quick" data-testid={`motion-shape-quick-${group.uid}`}>
            <span className="motion-shape-quick-name kui-muted">{group.name}</span>
            <div className="motion-props">{quickRows(parts, t).map(([node, parent, label]) => (node ? <PropertyRow key={node.path} node={node} parent={parent} label={label} /> : null))}</div>
          </div>
        ))}
        <div className="motion-shape-add">
          <ShapeAddMenu layer={layer.id} items={contents.children} testId="motion-shape-add" readOnly={readOnly} />
        </div>
        <PropertyChildren node={contents} scope={scope} depth={0} />
      </div>
    )
  }
  const { group, fill, stroke } = simple
  const add = (kind: 'fill' | 'stroke') => void run('layer.addShapeItem', { layer: layer.id, kind, group: group.uid })
  return (
    <div className="motion-shape" data-testid="motion-shape-controls">
      <div className="motion-props">{quickRows(simple, t).map(([node, parent, label]) => (node ? <PropertyRow key={node.path} node={node} parent={parent} label={label} /> : null))}</div>
      <div className="motion-shape-add">
        {/* Into the shape's group, so an operator added here changes the shape. */}
        <ShapeAddMenu layer={layer.id} group={group.uid} items={itemsOf(group)} testId="motion-shape-add" readOnly={readOnly} />
      </div>
      {!fill || !stroke ? (
        <div className="motion-shape-add">
          {!fill ? (
            <Button disabled={readOnly} onClick={() => add('fill')} data-testid="motion-shape-add-fill">
              <Plus size={14} aria-hidden="true" />
              {t('shape.addFill')}
            </Button>
          ) : null}
          {!stroke ? (
            <Button disabled={readOnly} onClick={() => add('stroke')} data-testid="motion-shape-add-stroke">
              <Plus size={14} aria-hidden="true" />
              {t('shape.addStroke')}
            </Button>
          ) : null}
        </div>
      ) : null}
      <PropertyGroup node={contents} label={t('shape.all')} scope={scope} depth={0} />
    </div>
  )
}
