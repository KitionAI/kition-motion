import { useMemo, useState } from 'react'
import { SearchIcon as Search, SparklesIcon as Sparkles, RotateCwIcon as Again } from '@kition/tessera/icons/react'
import { Button, Dialog, MenuItem, RibbonButton, RibbonGroup, RibbonMenu, TextInput, useKitEditor, useReadOnly, type MenuRow } from '@kition/sdk/editor'
import type { EffectParam } from './properties/meta'
import { counted } from './strings'

/** One effect the engine has (`effect.list`), with its parameters for the Properties panel. */
export type MotionEffect = { id: string; name: string; category: string; params?: EffectParam[] }

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

const RECENT_KEY = 'kition-motion.effects-recent'
const FAVOURITE_KEY = 'kition-motion.effects-favourite'
/** How many effects Recently Used keeps. */
const RECENT = 8

function stored(key: string): string[] {
  try {
    const list = JSON.parse(globalThis.localStorage?.getItem(key) ?? '[]') as unknown
    return Array.isArray(list) ? list.filter((id): id is string => typeof id === 'string') : []
  } catch {
    return []
  }
}
function store(key: string, list: string[]) {
  try {
    localStorage.setItem(key, JSON.stringify(list))
  } catch {
    // Kept for this page only.
  }
}

/** An effect just applied goes first in Recently Used (kept in this browser). */
export function rememberEffect(id: string) {
  store(RECENT_KEY, [id, ...stored(RECENT_KEY).filter((each) => each !== id)].slice(0, RECENT))
}

/**
 * Find Effect, the effect browser: with nothing typed, the favourites (starred), the effects used
 * last, then every category (opened with a click), each effect with a one-line description (what
 * it does, `effectAbout.ts`, and its first parameters); typing lists the effects whose name (English or
 * translated) or category contains it. Up and Down choose, Enter or a click applies to the
 * selected layers; the star beside an effect adds it to the favourites or takes it out (kept in
 * this browser).
 */
export function FindEffectDialog({ effects, onApply, onClose }: { effects: readonly MotionEffect[]; onApply: (effect: MotionEffect) => void; onClose: () => void }) {
  const { enabled, phrase: p, t } = useKitEditor()
  const [text, setText] = useState('')
  const [at, setAt] = useState(0)
  const [favourites, setFavourites] = useState(() => stored(FAVOURITE_KEY))
  const [recent] = useState(() => stored(RECENT_KEY))
  const [openCategories, setOpenCategories] = useState<ReadonlySet<string>>(() => new Set())
  const can = enabled('effect.apply')
  const categories = useMemo(() => effectCategories(effects), [effects])
  const all = useMemo(() => [...categories.values()].flat(), [categories])
  const byId = useMemo(() => new Map(all.map((effect) => [effect.id, effect])), [all])
  const query = text.trim().toLowerCase()
  const matches = useMemo(() => {
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
  }, [all, p, query])
  // With nothing typed: the groups shown, in order (a category's effects only once it is opened).
  const groups = useMemo(() => {
    if (query) return [{ id: 'matches', label: '', effects: matches.slice(0, SHOWN), open: true }]
    const pick = (ids: string[]) => ids.map((id) => byId.get(id)).filter((effect): effect is MotionEffect => effect != null)
    return [
      ...(favourites.length ? [{ id: 'favourites', label: t('effect.favourites'), effects: pick(favourites), open: true }] : []),
      ...(recent.length ? [{ id: 'recent', label: t('effect.recent'), effects: pick(recent), open: true }] : []),
      ...[...categories].map(([category, list]) => ({ id: `category-${category}`, label: p(category), effects: list, open: openCategories.has(category), category })),
    ]
  }, [query, matches, favourites, recent, categories, openCategories, byId, p, t])
  // The options a key moves through: every effect shown, in order.
  const shown = groups.flatMap((group) => (group.open ? group.effects.map((effect) => ({ effect, key: `${group.id}:${effect.id}` })) : []))
  const current = shown[Math.min(at, shown.length - 1)]
  const apply = (effect: MotionEffect | undefined) => {
    if (!effect || !can) return
    onApply(effect)
    onClose()
  }
  const toggleFavourite = (id: string) =>
    setFavourites((now) => {
      const next = now.includes(id) ? now.filter((each) => each !== id) : [...now, id]
      store(FAVOURITE_KEY, next)
      return next
    })
  /** An effect's one-line description: what it does (`effect.desc.<id>`, where it has one), and its first parameters. */
  const about = (effect: MotionEffect) => {
    // Its first settings, by their names in the editor's language (`PARAM_NAMES_ZH` names them in
    // Chinese; none is left out).
    const all = effect.params ?? []
    const named = all.map((param) => p(param.name))
    const params = named.slice(0, 3)
    const more = all.length > params.length ? '…' : ''
    const settings = params.length ? t('effect.params', { names: params.join(t('effect.listSeparator')) + more }) : all.length ? counted(t, 'effect.paramCount', all.length) : t('effect.noParams')
    const key = `effect.desc.${effect.id}`
    const own = t(key)
    // Without its own line, only its settings (not its category's words standing in for it).
    return own && own !== key ? `${own} · ${settings}` : settings
  }
  const option = (effect: MotionEffect, key: string, group: string) => (
    <div key={key} className="motion-effect-row" data-group={group}>
      <button
        type="button"
        role="option"
        aria-selected={current?.key === key}
        className="kui-menu-item motion-effect-option"
        disabled={!can}
        title={about(effect)}
        onMouseEnter={() => setAt(shown.findIndex((each) => each.key === key))}
        onClick={() => apply(effect)}
        data-testid={group === 'matches' || group.startsWith('category-') ? `motion-effect-option-${effect.id}` : `motion-effect-${group}-${effect.id}`}
      >
        <span className="motion-effect-name">
          <span>{p(effect.name)}</span>
          {/* Its category, where the list is not already under it. */}
          {group.startsWith('category-') ? null : <span className="kui-muted">{p(effect.category)}</span>}
        </span>
        <span className="motion-effect-about kui-muted" data-testid={`motion-effect-about-${effect.id}`}>
          {about(effect)}
        </span>
      </button>
      <button
        type="button"
        className="motion-effect-star"
        aria-pressed={favourites.includes(effect.id)}
        aria-label={t(favourites.includes(effect.id) ? 'effect.unfavourite' : 'effect.favourite', { name: p(effect.name) })}
        title={t(favourites.includes(effect.id) ? 'effect.unfavourite' : 'effect.favourite', { name: p(effect.name) })}
        onClick={() => toggleFavourite(effect.id)}
        data-testid={`motion-effect-star-${group}-${effect.id}`}
      >
        {favourites.includes(effect.id) ? '★' : '☆'}
      </button>
    </div>
  )
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
          <Button variant="primary" disabled={!can || !current} onClick={() => apply(current?.effect)} data-testid="motion-effect-dialog-ok">
            {t('effect.apply')}
          </Button>
        </>
      }
    >
      <form
        className="motion-form motion-effect-find"
        onSubmit={(event) => {
          event.preventDefault()
          apply(current?.effect)
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
          {query && !matches.length ? <p className="kui-muted">{t('effect.none')}</p> : null}
          {groups.map((group) =>
            group.id === 'matches' ? (
              group.effects.map((effect) => option(effect, `${group.id}:${effect.id}`, group.id))
            ) : (
              <div key={group.id} className="motion-effect-group" role="group" aria-label={group.label} data-testid={`motion-effect-group-${group.id}`}>
                {'category' in group && group.category ? (
                  <button
                    type="button"
                    className="motion-effect-group-head"
                    aria-expanded={group.open}
                    onClick={() =>
                      setOpenCategories((now) => {
                        const next = new Set(now)
                        if (next.has(group.category)) next.delete(group.category)
                        else next.add(group.category)
                        return next
                      })
                    }
                    data-testid={`motion-effect-head-${group.id}`}
                  >
                    {group.open ? '▾' : '▸'} {group.label} <span className="kui-muted">{group.effects.length}</span>
                  </button>
                ) : (
                  <p className="motion-effect-group-head kui-muted">{group.label}</p>
                )}
                {group.open ? group.effects.map((effect) => option(effect, `${group.id}:${effect.id}`, group.id)) : null}
              </div>
            ),
          )}
        </div>
        {query && matches.length > SHOWN ? <p className="kui-muted">{t('effect.more', { count: matches.length - SHOWN })}</p> : null}
        <button type="submit" hidden />
      </form>
    </Dialog>
  )
}
