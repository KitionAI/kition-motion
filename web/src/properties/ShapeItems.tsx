import { ArrowDownIcon, ArrowUpIcon, CopyIcon, EyeIcon, EyeOffIcon, PlusIcon, TrashIcon } from '@kition/tessera/icons/react'
import type { TreeNode } from '../timeline/tree'
import { reveal } from './Section'
import { useState } from 'react'
import { Button, ContextMenu, IconButton, MenuButton, MenuItem, MenuSeparator, SubMenu, TextInput, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'

/**
 * What a shape layer's Add menu offers (`layer.addShapeItem`'s kinds), in three submenus so the
 * menu fits any window: the shapes, the paints and the path operations (`id` names the submenu in
 * test ids), each kind named by the engine's English (translated by PHRASES).
 */
export const SHAPE_MENUS: ReadonlyArray<{ id: string; label: string; kinds: ReadonlyArray<readonly [kind: string, label: string]> }> = [
  {
    id: 'shapes',
    label: 'Shapes: Rectangle, Ellipse, Star, Path',
    kinds: [
      ['group', 'Group (empty)'],
      ['rect', 'Rectangle'],
      ['ellipse', 'Ellipse'],
      ['star', 'Polystar'],
      ['path', 'Path'],
    ],
  },
  {
    id: 'paints',
    label: 'Fill & Stroke (Gradients Too)',
    kinds: [
      ['fill', 'Fill'],
      ['stroke', 'Stroke'],
      ['gfill', 'Gradient Fill'],
      ['gstroke', 'Gradient Stroke'],
    ],
  },
  { id: 'operations', label: 'Path Operations: Merge, Trim, Repeater…', kinds: [] },
]

/** Every kind of the Add menu, in its order. */
const OPERATIONS: ReadonlyArray<readonly [kind: string, label: string]> = [
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
const MENUS = SHAPE_MENUS.map((menu) => (menu.id === 'operations' ? { ...menu, kinds: OPERATIONS } : menu))

/**
 * The Add menu of a shape layer's contents (or of one of its groups, `group` its uid): a new
 * shape, paint or path operator at the end of that group, one undo step (`layer.addShapeItem`).
 */
/** The paints: one added goes above the group's existing paints, so it shows in front of them. */
const PAINTS = new Set(['fill', 'stroke', 'gfill', 'gstroke'])

export function ShapeAddMenu({ layer, group, items, testId, readOnly, compact = false }: { layer: number; group?: number; items?: readonly TreeNode[]; testId: string; readOnly: boolean; compact?: boolean }) {
  const { run, t, phrase } = useKitEditor()
  const [search, setSearch] = useState('')
  const query = search.trim().toLowerCase()
  const found = query ? MENUS.flatMap((menu) => menu.kinds).filter(([kind, label]) => label.toLowerCase().includes(query) || phrase(label).toLowerCase().includes(query) || kind.includes(query)) : []
  const add = async (kind: string) => {
    const first = PAINTS.has(kind) ? (items ?? []).findIndex((item) => PAINTS.has(item.match ?? '')) : -1
    const answer = await run('layer.addShapeItem', { layer, kind, ...(group != null ? { group } : {}), ...(first >= 0 ? { index: first + 1 } : {}) })
    // The new item opens in the panel and scrolls into view.
    try {
      const path = (JSON.parse(answer ?? 'null') as { path?: string } | null)?.path
      if (path) reveal('Shape', path)
    } catch {
      // No path in the answer: nothing to reveal.
    }
  }
  return (
    <MenuButton placement="auto"
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
      {/* A search over every kind (in either language): what matches is listed flat, Enter adds the first. */}
      <div className="motion-add-search" role="none">
        <TextInput
          type="search"
          value={search}
          placeholder={t('shape.add.search')}
          aria-label={t('shape.add.search')}
          onChange={(event) => setSearch(event.target.value)}
          onKeyDown={(event) => {
            // The field's own keys (letters, arrows within the text), not the menu's; Escape still closes it.
            if (event.key !== 'Escape') event.stopPropagation()
            if (event.key === 'Enter' && found[0]) {
              event.preventDefault()
              const first = found[0][0]
              setSearch('')
              void add(first)
              ;(event.currentTarget.closest('[role="menu"]') as HTMLElement | null)?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
            }
          }}
          data-testid={`${testId}-search`}
        />
      </div>
      {query ? (
        found.length ? (
          found.map(([kind, label]) => <MenuItem key={kind} label={phrase(label)} onSelect={() => { setSearch(''); void add(kind) }} testId={`${testId}-${kind}`} />)
        ) : (
          <p className="kui-muted motion-add-none">{t('shape.add.none')}</p>
        )
      ) : (
        <>
          <MenuSeparator />
          {MENUS.map((menu) => (
            <SubMenu key={menu.id} label={phrase(menu.label)} path={`${testId}/${menu.id}`} uiId={`${testId}-${menu.id}`} testId={`${testId}-menu-${menu.id}`}>
              {menu.kinds.map(([kind, label]) => (
                <MenuItem key={kind} label={phrase(label)} onSelect={() => void add(kind)} testId={`${testId}-${kind}`} />
              ))}
            </SubMenu>
          ))}
        </>
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

/** The shape items Convert to Bezier Path turns into a Path item (`path.convertToBezier` with `shape`). */
export const PARAMETRIC_SHAPES = new Set(['rect', 'ellipse', 'star'])

/**
 * The right-click menu of a Rectangle, Ellipse or Polystar Path item (its group header in the
 * Properties panel or its row in the timeline): Convert to Bezier Path, which makes it a Path item
 * with the same outline whose vertices the Pen and Selection tools edit (one undo step).
 */
export function ShapeItemMenu({ layer, uid, name, at, onClose, testId }: { layer: number; uid: number; name: string; at: ContextMenuPoint | null; onClose: () => void; testId: string }) {
  const { run, phrase } = useKitEditor()
  return (
    <ContextMenu at={at} onClose={onClose} label={name} testId={testId}>
      <MenuItem label={phrase('Convert to Bezier Path')} onSelect={() => void run('path.convertToBezier', { layer, shape: uid })} testId={`${testId}-convert`} />
    </ContextMenu>
  )
}
