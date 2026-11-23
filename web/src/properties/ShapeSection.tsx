import { PlusIcon as Plus } from '@kition/tessera/icons/react'
import { Button, useKitEditor } from '@kition/sdk/editor'
import type { TreeNode } from '../timeline/tree'
import { PropertyChildren, PropertyGroup, PropertyRow, usePanel } from './PropertyTree'

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
  const group = items[0]
  const inner = (group.children ?? []).find((child) => child.match === 'contents')?.children ?? []
  const shapes = inner.filter((child) => PATHS.has(child.match ?? ''))
  if (shapes.length !== 1 || !SHAPES.has(shapes[0].match ?? '')) return null
  return { group, shape: shapes[0], fill: inner.find((child) => child.match === 'fill'), stroke: inner.find((child) => child.match === 'stroke') }
}

const child = (node: TreeNode | undefined, match: string) => node?.children?.find((item) => item.match === match)

/**
 * A shape layer's Contents section: for one rectangle, ellipse, polygon or star, its size (or
 * radius and points), roundness, fill colour and opacity, and stroke colour and width, with Add
 * fill and Add stroke when it has none; then every item of its contents in a closed group.
 */
export function ShapeControls({ contents, scope }: { contents: TreeNode; scope: string }) {
  const { run, t } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const simple = simpleShape(contents)
  if (!simple) return <PropertyChildren node={contents} scope={scope} depth={0} />
  const { group, shape, fill, stroke } = simple
  const add = (kind: 'fill' | 'stroke') => void run('layer.addShapeItem', { layer: layer.id, kind, group: group.uid })
  const rows: Array<[TreeNode | undefined, TreeNode | undefined, string]> = [
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
  return (
    <div className="motion-shape" data-testid="motion-shape-controls">
      <div className="motion-props">
        {rows.map(([node, parent, label]) => (node ? <PropertyRow key={node.path} node={node} parent={parent} label={label} /> : null))}
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
