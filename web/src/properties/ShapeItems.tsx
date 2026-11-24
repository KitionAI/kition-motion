import { ArrowDownIcon, ArrowUpIcon, CopyIcon, EyeIcon, EyeOffIcon, PlusIcon, TrashIcon } from '@kition/tessera/icons/react'
import { Button, IconButton, MenuButton, MenuItem, MenuSeparator, useKitEditor } from '@kition/sdk/editor'

/**
 * What a shape layer's Add menu offers (`layer.addShapeItem`'s kinds), in its order: the shapes,
 * the paints, then the path operators, each named by the engine's English (translated by PHRASES).
 */
export const SHAPE_KINDS: ReadonlyArray<readonly [kind: string, label: string] | '---'> = [
  ['group', 'Group (empty)'],
  ['rect', 'Rectangle'],
  ['ellipse', 'Ellipse'],
  ['star', 'Polystar'],
  ['path', 'Path'],
  '---',
  ['fill', 'Fill'],
  ['stroke', 'Stroke'],
  ['gfill', 'Gradient Fill'],
  ['gstroke', 'Gradient Stroke'],
  '---',
  ['merge', 'Merge Paths'],
  ['offset', 'Offset Paths'],
  ['pucker', 'Pucker & Bloat'],
  ['repeater', 'Repeater'],
  ['round', 'Round Corners'],
  ['trim', 'Trim Paths'],
  ['twist', 'Twist'],
  ['wiggle', 'Wiggle Paths'],
  ['zigzag', 'Zig Zag'],
]

/**
 * The Add menu of a shape layer's contents (or of one of its groups, `group` its uid): a new
 * shape, paint or path operator at the end of that group, one undo step (`layer.addShapeItem`).
 */
export function ShapeAddMenu({ layer, group, testId, readOnly, compact = false }: { layer: number; group?: number; testId: string; readOnly: boolean; compact?: boolean }) {
  const { run, t, phrase } = useKitEditor()
  return (
    <MenuButton
      align="start"
      trigger={({ open, toggle }) =>
        compact ? (
          <IconButton icon={PlusIcon} label={t('shape.add')} pressed={open} disabled={readOnly} onClick={toggle} data-testid={testId} />
        ) : (
          <Button disabled={readOnly} aria-expanded={open} onClick={toggle} data-testid={testId}>
            <PlusIcon size={14} aria-hidden="true" />
            {t('shape.add')}
          </Button>
        )
      }
    >
      {SHAPE_KINDS.map((entry, at) =>
        entry === '---' ? (
          <MenuSeparator key={`-${at}`} />
        ) : (
          <MenuItem key={entry[0]} label={phrase(entry[1])} onSelect={() => void run('layer.addShapeItem', { layer, kind: entry[0], ...(group != null ? { group } : {}) })} testId={`${testId}-${entry[0]}`} />
        ),
      )}
    </MenuButton>
  )
}

/**
 * A shape item's own buttons in its group header: on or off, move up and down among its
 * siblings (the order the shapes, paints and operators apply in), duplicate, delete; each one
 * undo step (`prop.setGroupEnabled`, `prop.moveGroup`, `prop.duplicateGroup`, `prop.removeGroup`).
 */
export function ShapeItemActions({ layer, uid, enabled, index, count, name, testId, readOnly }: { layer: number; uid: number; enabled: boolean; index: number; count: number; name: string; testId: string; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  return (
    <span className="motion-item-actions" onClick={(event) => event.stopPropagation()}>
      <IconButton icon={enabled ? EyeIcon : EyeOffIcon} label={t(enabled ? 'shape.item.off' : 'shape.item.on', { name })} pressed={enabled} disabled={readOnly} onClick={() => void run('prop.setGroupEnabled', { layer, prop: uid, value: !enabled })} data-testid={`${testId}-enabled`} />
      <IconButton icon={ArrowUpIcon} label={t('shape.item.up', { name })} disabled={readOnly || index === 0} onClick={() => void run('prop.moveGroup', { layer, prop: uid, index })} data-testid={`${testId}-up`} />
      <IconButton icon={ArrowDownIcon} label={t('shape.item.down', { name })} disabled={readOnly || index >= count - 1} onClick={() => void run('prop.moveGroup', { layer, prop: uid, index: index + 2 })} data-testid={`${testId}-down`} />
      <IconButton icon={CopyIcon} label={t('shape.item.duplicate', { name })} disabled={readOnly} onClick={() => void run('prop.duplicateGroup', { layer, prop: uid })} data-testid={`${testId}-duplicate`} />
      <IconButton icon={TrashIcon} label={t('shape.item.remove', { name })} disabled={readOnly} onClick={() => void run('prop.removeGroup', { layer, prop: uid })} data-testid={`${testId}-remove`} />
    </span>
  )
}
