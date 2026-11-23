import { useMemo, useState } from 'react'
import { SearchIcon as Search, SparklesIcon as Sparkles, RotateCwIcon as Again } from '@kition/tessera/icons/react'
import { Button, Dialog, MenuItem, RibbonButton, RibbonGroup, RibbonMenu, TextInput, useKitEditor, useReadOnly, type MenuRow } from '@kition/sdk/editor'

/** One effect the engine has (`effect.list`). */
export type MotionEffect = { id: string; name: string; category: string }

/** The kit row that applies an effect: `kit:effect:<id>`. */
export const EFFECT_PREFIX = 'kit:effect:'
/** The kit row that opens Find Effect. */
export const FIND_EFFECT = 'kit:find-effect'

/** Categories left out of the menus: effects kept only so old projects open. */
const LEFT_OUT = new Set(['Obsolete'])

/** The effects by category, categories and effects in alphabetical order (as the engine lists them). */
export function effectCategories(effects: readonly MotionEffect[]) {
  const categories = new Map<string, MotionEffect[]>()
  for (const effect of effects) {
    if (LEFT_OUT.has(effect.category)) continue
    const list = categories.get(effect.category) ?? []
    list.push(effect)
    categories.set(effect.category, list)
  }
  return categories
}

/** The Effect menu: Find Effect, Last Effect, then a submenu per category. */
export function effectRows(effects: readonly MotionEffect[]): MenuRow[] {
  const rows: MenuRow[] = [
    { path: ['Effect'], label: 'Find Effect…', command: FIND_EFFECT },
    { path: ['Effect'], label: 'Last Effect', command: 'effect.applyLast' },
  ]
  const categories = effectCategories(effects)
  if (categories.size) rows.push({ path: ['Effect'], label: '---', command: '---' })
  for (const [category, list] of categories) for (const effect of list) rows.push({ path: ['Effect', category], label: effect.name, command: `${EFFECT_PREFIX}${effect.id}` })
  return rows
}

/**
 * The Effect tab's ribbon (Single line and Classic): Find Effect, Last Effect, and a drop-down
 * per category, instead of a button for each of several hundred effects.
 */
export function EffectRibbon({ effects, onCommand, lastEffect }: { effects: readonly MotionEffect[]; onCommand: (command: string) => void; lastEffect: boolean }) {
  const { enabled, phrase: p } = useKitEditor()
  const readOnly = useReadOnly()
  const can = !readOnly && enabled('effect.apply')
  return (
    <>
      <RibbonGroup label={p('Effect')} icon={Sparkles}>
        <RibbonButton icon={Search} label={p('Find Effect…')} size="large" primary disabled={readOnly} onClick={() => onCommand(FIND_EFFECT)} data-testid="motion-effect-find" />
        <RibbonButton icon={Again} label={p('Last Effect')} primary disabled={!can || !lastEffect} onClick={() => onCommand('effect.applyLast')} data-testid="motion-effect-last" />
      </RibbonGroup>
      <RibbonGroup label={p('Categories')} icon={Sparkles}>
        {[...effectCategories(effects)].map(([category, list]) => (
          <RibbonMenu key={category} label={p(category)} size="small" disabled={!can} testId={`motion-effect-category-${category}`}>
            {list.map((effect) => (
              <MenuItem key={effect.id} label={p(effect.name)} onSelect={() => onCommand(`${EFFECT_PREFIX}${effect.id}`)} />
            ))}
          </RibbonMenu>
        ))}
      </RibbonGroup>
    </>
  )
}

/** How many matches Find Effect lists at once. */
const SHOWN = 60

/**
 * Find Effect: the effects whose name (English or translated) or category contains what is typed,
 * as it is typed; Up and Down choose, Enter or a click applies to the selected layers.
 */
export function FindEffectDialog({ effects, onApply, onClose }: { effects: readonly MotionEffect[]; onApply: (effect: MotionEffect) => void; onClose: () => void }) {
  const { enabled, phrase: p, t } = useKitEditor()
  const [text, setText] = useState('')
  const [at, setAt] = useState(0)
  const can = enabled('effect.apply')
  const all = useMemo(() => [...effectCategories(effects).values()].flat(), [effects])
  const matches = useMemo(() => {
    const query = text.trim().toLowerCase()
    if (!query) return all
    // Names that start with the text first, then names that contain it, then categories that do.
    const rank = (effect: MotionEffect) => {
      const names = [effect.name, p(effect.name)].map((name) => name.toLowerCase())
      if (names.some((name) => name.startsWith(query))) return 0
      if (names.some((name) => name.includes(query))) return 1
      return [effect.category, p(effect.category)].some((name) => name.toLowerCase().includes(query)) ? 2 : 3
    }
    return all
      .map((effect) => ({ effect, rank: rank(effect) }))
      .filter((match) => match.rank < 3)
      .sort((a, b) => a.rank - b.rank)
      .map((match) => match.effect)
  }, [all, p, text])
  const shown = matches.slice(0, SHOWN)
  const current = shown[Math.min(at, shown.length - 1)]
  const apply = (effect: MotionEffect | undefined) => {
    if (!effect || !can) return
    onApply(effect)
    onClose()
  }
  return (
    <Dialog
      title={t('effect.find')}
      testId="motion-effect-dialog"
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose} data-testid="motion-effect-dialog-cancel">
            {t('dialog.cancel')}
          </Button>
          <Button variant="primary" disabled={!can || !current} onClick={() => apply(current)} data-testid="motion-effect-dialog-ok">
            {t('effect.apply')}
          </Button>
        </>
      }
    >
      <form
        className="motion-form motion-effect-find"
        onSubmit={(event) => {
          event.preventDefault()
          apply(current)
        }}
      >
        <TextInput
          value={text}
          autoFocus
          prefix={<Search size={14} aria-hidden="true" />}
          placeholder={t('effect.search')}
          aria-label={t('effect.search')}
          onChange={(event) => {
            setText(event.target.value)
            setAt(0)
          }}
          onKeyDown={(event) => {
            if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
            event.preventDefault()
            setAt((index) => Math.max(0, Math.min(shown.length - 1, index + (event.key === 'ArrowDown' ? 1 : -1))))
          }}
          data-testid="motion-effect-search"
        />
        {can ? null : <p className="kui-muted">{t('effect.selectFirst')}</p>}
        <div className="motion-effect-list" role="listbox" aria-label={t('effect.find')} data-testid="motion-effect-list">
          {shown.length ? (
            shown.map((effect) => (
              <button
                key={effect.id}
                type="button"
                role="option"
                aria-selected={effect === current}
                className="kui-menu-item motion-effect-option"
                disabled={!can}
                onMouseEnter={() => setAt(shown.indexOf(effect))}
                onClick={() => apply(effect)}
                data-testid={`motion-effect-option-${effect.id}`}
              >
                <span>{p(effect.name)}</span>
                <span className="kui-muted">{p(effect.category)}</span>
              </button>
            ))
          ) : (
            <p className="kui-muted">{t('effect.none')}</p>
          )}
        </div>
        {matches.length > SHOWN ? <p className="kui-muted">{t('effect.more', { count: matches.length - SHOWN })}</p> : null}
        <button type="submit" hidden />
      </form>
    </Dialog>
  )
}
