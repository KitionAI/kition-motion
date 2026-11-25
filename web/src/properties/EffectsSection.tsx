import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react'
import { ChevronRightIcon as Chevron, EllipsisIcon as More, GripVerticalIcon as Grip, PlusIcon as Plus, RotateCcwIcon as Reset } from '@kition/tessera/icons/react'
import { Button, IconButton, MenuButton, MenuItem, MenuSeparator, Switch, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import type { TreeNode } from '../timeline/tree'
import { useEffectOf } from './catalog'
import { PropertyChildren, usePanel } from './PropertyTree'
import { Section, remember, useOpen } from './Section'

/** A card's open state, remembered per layer type and effect. */
const cardKey = (effect: TreeNode) => `effect:${effect.uid ?? effect.path}`

/** One applied effect: its header (drag handle, open, on/off, name, reset, a menu) and its parameters. */
function EffectCard({ effect, index, count, scope, selected, onSelect, onGrip }: { effect: TreeNode; index: number; count: number; scope: string; selected: boolean; onSelect: () => void; onGrip: (event: ReactPointerEvent<HTMLElement>) => void }) {
  const { run, t, phrase } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const effectOf = useEffectOf()
  const [open, setOpen] = useOpen(scope, cardKey(effect), false)
  const name = phrase(effect.name)
  const uid = effect.uid
  const move = (to: number) => void run('prop.moveGroup', { layer: layer.id, prop: uid, index: to + 1 })
  const remove = () => void run('prop.removeGroup', { layer: layer.id, prop: uid })
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // Delete on the card or its buttons (not in one of its fields) removes the effect, not the layer.
    if (readOnly || (event.target instanceof Element && event.target.closest('input, textarea, select, [contenteditable="true"], [role="menu"]'))) return
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault()
      event.stopPropagation()
      remove()
    }
  }
  return (
    <div
      className="motion-effect-card"
      tabIndex={0}
      role="group"
      aria-label={name}
      data-selected={selected || undefined}
      data-enabled={effect.enabled !== false}
      data-uid={uid}
      onFocus={(event) => event.target === event.currentTarget && onSelect()}
      onPointerDown={onSelect}
      onKeyDown={keyDown}
      data-testid={`motion-effect-card-${index + 1}`}
    >
      <div className="motion-effect-header">
        <span className="motion-effect-grip" aria-label={t('effect.drag')} title={t('effect.drag')} onPointerDown={readOnly ? undefined : onGrip} data-testid={`motion-effect-grip-${index + 1}`}>
          <Grip size={14} aria-hidden="true" />
        </span>
        <button type="button" className="motion-effect-toggle" aria-expanded={open} onClick={() => setOpen(!open)} data-testid={`motion-effect-toggle-${index + 1}`}>
          <Chevron size={13} aria-hidden="true" className="motion-effect-chevron" />
          <span className="motion-effect-title">{name}</span>
        </button>
        <Switch
          className="motion-effect-switch"
          checked={effect.enabled !== false}
          disabled={readOnly}
          aria-label={t('effect.enabled', { name })}
          onCheckedChange={(value) => void run('prop.setGroupEnabled', { layer: layer.id, prop: uid, value })}
          data-testid={`motion-effect-enabled-${index + 1}`}
        />
        <IconButton icon={Reset} label={t('effect.reset')} disabled={readOnly} onClick={() => void run('effect.reset', { layer: layer.id, effect: index + 1 })} data-testid={`motion-effect-reset-${index + 1}`} />
        <MenuButton placement="auto" align="end" trigger={({ open: shown, toggle }) => <IconButton icon={More} label={t('effect.menu')} pressed={shown} disabled={readOnly} onClick={toggle} data-testid={`motion-effect-menu-${index + 1}`} />}>
          <MenuItem label={t('effect.duplicate')} onSelect={() => void run('prop.duplicateGroup', { layer: layer.id, prop: uid })} testId={`motion-effect-duplicate-${index + 1}`} />
          <MenuItem label={t('effect.moveUp')} disabled={index === 0} onSelect={() => move(index - 1)} testId={`motion-effect-up-${index + 1}`} />
          <MenuItem label={t('effect.moveDown')} disabled={index === count - 1} onSelect={() => move(index + 1)} testId={`motion-effect-down-${index + 1}`} />
          <MenuSeparator />
          <MenuItem label={t('effect.remove')} onSelect={remove} testId={`motion-effect-remove-${index + 1}`} />
        </MenuButton>
      </div>
      {open ? (
        <div className="motion-effect-body">
          <PropertyChildren node={effect} scope={scope} depth={0} effect={effectOf(effect.effect)} />
        </div>
      ) : null}
    </div>
  )
}

/** A card being dragged by its handle: where it started and the slot it would land in. */
type Drag = { pointerId: number; from: number; to: number; y: number }

/**
 * The layer's effects as cards, top to bottom in render order: each opens to its parameters
 * (keyframable, with the ranges and drop-down labels `effect.list` gives), switches on and off,
 * resets, duplicates, moves up or down (or drags by its handle), and is removed from its menu or
 * with Delete while selected. Add effect opens the effect search. An effect applied from anywhere
 * opens here and scrolls into view.
 */
export function EffectsSection({ layer, group, onAdd }: { layer: MotionLayer; group: TreeNode | undefined; onAdd: () => void }) {
  const { run, t } = useKitEditor()
  const { readOnly } = usePanel()
  const applied = group?.children ?? []
  const scope = layer.type
  const [selected, setSelected] = useState<number | null>(null)
  const list = useRef<HTMLDivElement>(null)
  const [drag, setDrag] = useState<Drag | null>(null)
  // A new effect (one whose uid this layer did not have) opens and scrolls into view.
  const seen = useRef<{ layer: number; uids: Set<number> } | null>(null)
  const uids = applied.map((effect) => effect.uid ?? -1).join(' ')
  const [reveal, setReveal] = useState<number | null>(null)
  useEffect(() => {
    const now = new Set(applied.map((effect) => effect.uid ?? -1))
    const before = seen.current
    seen.current = { layer: layer.id, uids: now }
    if (!group || !before || before.layer !== layer.id) return
    const added = applied.filter((effect) => effect.uid != null && !before.uids.has(effect.uid)).at(-1)
    if (!added) return
    remember(`${scope}:effects`, true)
    remember(`${scope}:${cardKey(added)}`, true)
    setSelected(added.uid ?? null)
    setReveal(added.uid ?? null)
  }, [uids, layer.id, group]) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (reveal == null) return
    const card = list.current?.querySelector<HTMLElement>(`[data-uid="${reveal}"]`)
    if (!card) return
    card.scrollIntoView({ block: 'nearest' })
    setReveal(null)
  })
  const slotAt = (y: number) => {
    const cards = [...(list.current?.querySelectorAll<HTMLElement>('.motion-effect-card') ?? [])]
    const index = cards.findIndex((card) => {
      const box = card.getBoundingClientRect()
      return y < box.top + box.height / 2
    })
    return index < 0 ? cards.length : index
  }
  const grip = (from: number) => (event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return
    event.preventDefault()
    event.stopPropagation()
    list.current?.setPointerCapture(event.pointerId)
    setDrag({ pointerId: event.pointerId, from, to: from, y: event.clientY })
  }
  // Where the line shows: above the card at `to` (the slot), not where the card already is.
  const lands = drag && drag.to !== drag.from && drag.to !== drag.from + 1 ? drag.to : null
  return (
    <Section
      scope={scope}
      id="effects"
      title={applied.length ? `${t('props.effects')} (${applied.length})` : t('props.effects')}
      testId="motion-section-effects"
      actions={
        <Button variant="ghost" disabled={readOnly} onClick={onAdd} data-testid="motion-effect-add">
          <Plus size={14} aria-hidden="true" />
          {t('effect.add')}
        </Button>
      }
    >
      <div
        ref={list}
        className="motion-effect-list-cards"
        data-dragging={drag ? true : undefined}
        onPointerMove={(event) => {
          if (drag?.pointerId === event.pointerId) setDrag({ ...drag, to: slotAt(event.clientY), y: event.clientY })
        }}
        onPointerUp={(event) => {
          if (drag?.pointerId !== event.pointerId) return
          setDrag(null)
          const effect = applied[drag.from]
          // Dropped in slot `to`: the index it gets once it has left its own place.
          const index = drag.to > drag.from ? drag.to - 1 : drag.to
          if (effect && index !== drag.from) void run('prop.moveGroup', { layer: layer.id, prop: effect.uid, index: index + 1 })
        }}
        onPointerCancel={() => setDrag(null)}
        data-testid="motion-effect-cards"
      >
        {applied.length ? null : <p className="kui-muted motion-effect-none">{t('effect.empty')}</p>}
        {applied.map((effect, index) => (
          <div key={effect.uid ?? effect.path} className="motion-effect-slot" data-drop={lands === index || undefined}>
            <EffectCard effect={effect} index={index} count={applied.length} scope={scope} selected={selected === effect.uid} onSelect={() => setSelected(effect.uid ?? null)} onGrip={grip(index)} />
          </div>
        ))}
        {lands === applied.length ? <div className="motion-effect-slot" data-drop /> : null}
      </div>
    </Section>
  )
}
