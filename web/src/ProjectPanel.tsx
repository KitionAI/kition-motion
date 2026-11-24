import { useEffect, useMemo, useRef, useState, type ComponentType, type DragEvent, type KeyboardEvent, type MouseEvent } from 'react'
import {
  AudioLinesIcon as AudioIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  ClapperboardIcon as CompIcon,
  FilePlusIcon as NewCompIcon,
  FolderIcon,
  FolderPlusIcon,
  ImageIcon,
  SquareIcon as SolidIcon,
  UploadIcon,
  VideoIcon,
} from '@kition/tessera/icons/react'
import { Button, ContextMenu, Dialog, IconButton, MenuItem, MenuSeparator, RenameField, TextInput, toast, useEngineQuery, useKitEditor, useReadOnly, type ContextMenuPoint } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'
import { ITEM_MIME, ITEMS_MIME, isLayerSource, parseAnswer, type ProjectItem, type ProjectSummary } from './footage'
import { LabelMenu, labelHex } from './labels'

const KEY = 'kition-motion.project-width'
const OPEN_KEY = 'kition-motion.project-open'
const DEFAULT_WIDTH = 240
const MIN_WIDTH = 180
const MAX_WIDTH = 480

const clampWidth = (width: number) => Math.round(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, width)))

function stored(key: string) {
  try {
    return globalThis.sessionStorage?.getItem(key) ?? null
  } catch {
    return null
  }
}

function store(key: string, value: string) {
  try {
    sessionStorage.setItem(key, value)
  } catch {
    // Storage refused (private mode): the setting holds until the page closes.
  }
}

/** Whether the Project panel is shown, and its width: both kept for the session. */
export function useProjectPanel() {
  const [open, setOpen] = useState(() => stored(OPEN_KEY) === '1')
  const [width, setWidth] = useState(() => {
    const value = Number(stored(KEY))
    return Number.isFinite(value) && value >= MIN_WIDTH ? clampWidth(value) : DEFAULT_WIDTH
  })
  return {
    open,
    width,
    toggle: () =>
      setOpen((now) => {
        store(OPEN_KEY, now ? '0' : '1')
        return !now
      }),
    show: () => {
      store(OPEN_KEY, '1')
      setOpen(true)
    },
    setWidth: (next: number) => {
      const value = clampWidth(next)
      setWidth(value)
      store(KEY, String(value))
    },
  }
}

const ICONS: Record<string, ComponentType<{ size?: number; 'aria-hidden'?: boolean | 'true' }>> = {
  Composition: CompIcon,
  Folder: FolderIcon,
  Solid: SolidIcon,
  Image: ImageIcon,
  Video: VideoIcon,
  Audio: AudioIcon,
}

/** Footage (a file), as Interpret Footage and Replace Footage take it. */
const isFootage = (item: ProjectItem) => item.footage != null

/** The folder `item` is in, when that folder is in the project. */
const parentOf = (items: readonly ProjectItem[], item: ProjectItem) => (item.parent != null && items.some((other) => other.id === item.parent) ? item.parent : null)

/**
 * `items` in the project's tree order (each folder's items under it, by name), only inside the
 * `open` folders; with a `search`, the items whose names have it and the folders they are in.
 */
function treeOrder(items: readonly ProjectItem[], open: ReadonlySet<number>, search: string) {
  const children = new Map<number | null, ProjectItem[]>()
  for (const item of items) {
    const parent = parentOf(items, item)
    children.set(parent, [...(children.get(parent) ?? []), item])
  }
  const needle = search.trim().toLocaleLowerCase()
  // With a search: the matches and every folder above one.
  const wanted = new Set<number>()
  if (needle) {
    const byId = new Map(items.map((item) => [item.id, item]))
    for (const item of items) {
      if (!item.name.toLocaleLowerCase().includes(needle)) continue
      let at: ProjectItem | undefined = item
      for (let depth = 0; at && depth < 32 && !wanted.has(at.id); depth += 1) {
        wanted.add(at.id)
        at = at.parent != null ? byId.get(at.parent) : undefined
      }
    }
  }
  const out: Array<{ item: ProjectItem; depth: number }> = []
  // Each item once, and the nesting bounded, even in a project whose folders loop.
  const seen = new Set<number>()
  const walk = (parent: number | null, depth: number) => {
    if (depth > 32) return
    for (const item of [...(children.get(parent) ?? [])].sort((a, b) => a.name.localeCompare(b.name))) {
      if (seen.has(item.id) || (needle && !wanted.has(item.id))) continue
      seen.add(item.id)
      out.push({ item, depth })
      if (item.type === 'Folder' && (needle || open.has(item.id))) walk(item.id, depth + 1)
    }
  }
  walk(null, 0)
  return out
}

/** What was asked of the person before deleting: the items and what uses them. */
type Deleting = { items: ProjectItem[]; layers: number; comps: number }

/** A name no project item has: `Folder 1`, `Folder 2`. */
const unusedName = (items: readonly ProjectItem[], base: string) => {
  const names = new Set(items.map((item) => item.name))
  let n = 1
  while (names.has(`${base} ${n}`)) n += 1
  return `${base} ${n}`
}

/** Seconds as `1.25 s`, rounded to hundredths. */
const round = (value: number) => Math.round(value * 100) / 100

/**
 * The Project panel, left of the composition: the project's compositions, footage, solids and
 * folders from `project.summary`, each with its type's icon, name, and size or duration, and the
 * selected item's details underneath (type, size, duration, frame rate, how many layers use it,
 * and for footage how it is interpreted). A search box filters the list by name.
 *
 * Click selects (Cmd or Ctrl adds or removes one, Shift a run of rows), the arrows move the
 * selection (Left and Right close and open a folder), Enter or F2 renames, Delete removes (asking
 * first when layers use what goes), a double-click opens a composition or places footage and
 * solids in the active composition. Rows drag into a folder (or out to the list's empty space),
 * and onto the timeline or the composition to make a layer. The selection is the engine's
 * (`project.select`): File ▸ New Composition from Selection and Interpret Footage act on it.
 */
export function ProjectPanel({
  info,
  width,
  onWidth,
  onImport,
  onNewComp,
  onPlace,
  onReplace,
  onInterpret,
  onQueue,
  reveal,
}: {
  info: MotionInfo
  width: number
  onWidth: (width: number) => void
  onImport: () => void
  onNewComp: () => void
  onPlace: (item: number) => void
  onReplace: (item: ProjectItem) => void
  onInterpret: (item: ProjectItem) => void
  /** Add to Render Queue, for compositions. */
  onQueue: (comps: number[]) => void
  /** An item to show and select (Reveal Layer Source in Project), and with `rename` to rename at once (File ▸ New ▸ New Folder); a new object each time. */
  reveal: { item: number; rename?: boolean } | null
}) {
  const { run, query, t } = useKitEditor()
  const readOnly = useReadOnly()
  const summary = useEngineQuery<ProjectSummary>('project.summary', {})
  const items = useMemo(() => summary?.items ?? [], [summary])
  const [open, setOpen] = useState<ReadonlySet<number>>(() => new Set())
  const [search, setSearch] = useState('')
  const rows = useMemo(() => treeOrder(items, open, search), [items, open, search])
  const [selected, setSelected] = useState<number[]>([])
  // The row Shift extends a run from.
  const anchor = useRef<number | null>(null)
  // The selection last sent to the engine, and how many selections are on their way.
  const sent = useRef('')
  const pending = useRef(0)
  const [renaming, setRenaming] = useState<number | null>(null)
  const [name, setName] = useState('')
  const [menu, setMenu] = useState<{ at: ContextMenuPoint; item: ProjectItem | null } | null>(null)
  const [deleting, setDeleting] = useState<Deleting | null>(null)
  const [dropOn, setDropOn] = useState<number | 'root' | null>(null)
  const list = useRef<HTMLDivElement>(null)
  const edge = useRef<{ pointerId: number; x: number; width: number } | null>(null)
  const byId = (id: number | null) => items.find((item) => item.id === id) ?? null
  const chosen = selected.map(byId).filter((item): item is ProjectItem => item != null)
  const single = chosen.length === 1 ? chosen[0] : null

  /** Selects `ids` here and in the engine (where File's commands read it). */
  const choose = (ids: number[], from: number | null = ids[ids.length - 1] ?? null) => {
    setSelected(ids)
    anchor.current = from
    const key = ids.join()
    if (key === sent.current || readOnly) return
    sent.current = key
    pending.current += 1
    void run('project.select', { items: ids }).finally(() => (pending.current -= 1))
  }
  // The engine's selection changed by itself (an import, New Folder, a duplicate): shown here too.
  useEffect(() => {
    const engine = (summary?.selection ?? []).filter((id) => items.some((item) => item.id === id))
    const key = engine.join()
    if (pending.current > 0 || key === sent.current) return
    sent.current = key
    setSelected(engine)
  }, [summary, items])
  // Items gone (deleted, undone) leave the selection.
  useEffect(() => {
    setSelected((now) => {
      const kept = now.filter((id) => items.some((item) => item.id === id))
      return kept.length === now.length ? now : kept
    })
  }, [items])

  /** Opens the folders above `id`, so its row shows. */
  const showRow = (id: number) => {
    const above: number[] = []
    let at = byId(id)
    for (let depth = 0; at?.parent != null && depth < 32; depth += 1) {
      above.push(at.parent)
      at = byId(at.parent)
    }
    if (above.length) setOpen((now) => (above.every((folder) => now.has(folder)) ? now : new Set([...now, ...above])))
  }
  // Reveal Layer Source in Project: the item's folders open, it is selected and scrolled to.
  const revealed = useRef<{ item: number; rename?: boolean } | null>(null)
  useEffect(() => {
    if (!reveal || reveal === revealed.current || !items.some((item) => item.id === reveal.item)) return
    revealed.current = reveal
    setSearch('')
    showRow(reveal.item)
    choose([reveal.item])
    const shown = byId(reveal.item)
    if (reveal.rename && shown && !readOnly) {
      // As the header's New Folder button: the new folder's name is ready to type over.
      setName(shown.name)
      setRenaming(shown.id)
      return
    }
    requestAnimationFrame(() => {
      list.current?.querySelector(`[data-testid="motion-item-${reveal.item}"]`)?.scrollIntoView({ block: 'nearest' })
      list.current?.focus({ preventScroll: true })
    })
  }, [reveal, items]) // eslint-disable-line react-hooks/exhaustive-deps

  const typeName = (item: ProjectItem) => t(`project.type.${item.type}`)
  const detail = (item: ProjectItem) => {
    const size = item.size ? `${item.size[0]} × ${item.size[1]}` : ''
    const seconds = item.duration != null && item.type !== 'Image' && item.type !== 'Solid' ? t('project.seconds', { value: round(item.duration) }) : ''
    return [size, seconds].filter(Boolean).join(' · ')
  }
  /** A double-click: a composition opens, footage and solids become a layer, a folder opens or closes. */
  const activate = (item: ProjectItem) => {
    if (item.type === 'Composition') void run('comp.open', { comp: item.id })
    else if (item.type === 'Folder') toggleFolder(item.id)
    else if (isLayerSource(item) && !readOnly) onPlace(item.id)
  }
  const toggleFolder = (id: number, to?: boolean) =>
    setOpen((now) => {
      const on = to ?? !now.has(id)
      if (on === now.has(id)) return now
      const next = new Set(now)
      if (on) next.add(id)
      else next.delete(id)
      return next
    })
  const rename = (item: ProjectItem) => {
    if (readOnly) return
    setName(item.name)
    setRenaming(item.id)
  }
  /** Delete, asking first when layers use what goes (they go with it). */
  const remove = async (doomed: ProjectItem[]) => {
    if (readOnly || !doomed.length) return
    const ids = doomed.map((item) => item.id)
    const usage = await query<{ layers?: number; comps?: number }>('project.usage', { items: ids })
    if ((usage?.layers ?? 0) > 0) setDeleting({ items: doomed, layers: usage?.layers ?? 0, comps: usage?.comps ?? 0 })
    else void run('project.delete', { items: ids })
  }
  /** New Folder: inside `parent` (a folder) or at the top, named for renaming at once. */
  const newFolder = async (parent: number | null) => {
    if (readOnly) return
    const folderName = unusedName(items, t('project.folderName'))
    const made = parseAnswer<{ item?: number }>(await run('project.newFolder', { name: folderName, ...(parent != null ? { parent } : {}) }))
    if (made?.item == null) return
    if (parent != null) toggleFolder(parent, true)
    setSelected([made.item])
    sent.current = String(made.item)
    anchor.current = made.item
    setName(folderName)
    setRenaming(made.item)
  }
  /** New Composition from Selection: one composition holding every selected item (or one per item, when it is one). */
  const newCompFromSelection = async (from: ProjectItem[]) => {
    const ids = from.filter((item) => item.type !== 'Folder').map((item) => item.id)
    if (!ids.length || readOnly) return
    if ((await run('project.select', { items: ids })) === null) return
    sent.current = ids.join()
    await run('file.newCompFromSelection', { single: ids.length > 1 })
  }
  const removeUnused = async () => {
    const answer = parseAnswer<{ removed?: number }>(await run('file.removeUnusedFootage', {}))
    if (answer) toast.success(t('project.removedUnused', { count: answer.removed ?? 0 }))
  }
  /** Moves `ids` into `folder` (null: the top of the project), leaving out a folder dropped into itself. */
  const moveInto = (ids: number[], folder: number | null) => {
    const within = (inner: number | null, outer: number) => {
      for (let at = inner, depth = 0; at != null && depth < 32; depth += 1) {
        if (at === outer) return true
        at = byId(at)?.parent ?? null
      }
      return false
    }
    const moving = ids.filter((id) => byId(id) && byId(id)?.parent !== folder && !(folder != null && within(folder, id)))
    if (!moving.length || readOnly) return
    if (folder != null) toggleFolder(folder, true)
    void run('project.move', { items: moving, folder })
  }

  /** A click on a row: alone, or with Cmd or Ctrl in or out of the selection, or with Shift a run from the last one clicked. */
  const click = (event: MouseEvent, item: ProjectItem) => {
    const at = rows.findIndex((row) => row.item.id === item.id)
    if (event.shiftKey && anchor.current != null) {
      const from = rows.findIndex((row) => row.item.id === anchor.current)
      if (from >= 0 && at >= 0) {
        const [a, b] = from < at ? [from, at] : [at, from]
        choose(
          rows.slice(a, b + 1).map((row) => row.item.id),
          anchor.current,
        )
      }
    } else if (event.metaKey || event.ctrlKey) {
      choose(selected.includes(item.id) ? selected.filter((id) => id !== item.id) : [...selected, item.id], item.id)
    } else choose([item.id])
    list.current?.focus({ preventScroll: true })
  }

  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (renaming !== null || event.target !== event.currentTarget) return
    const plain = !event.metaKey && !event.ctrlKey && !event.altKey
    const current = byId(anchor.current ?? selected[selected.length - 1] ?? null)
    const move = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0
    if (move && plain) {
      const at = rows.findIndex((row) => row.item.id === current?.id)
      const next = rows[at < 0 ? 0 : Math.max(0, Math.min(rows.length - 1, at + move))]
      if (next) {
        if (event.shiftKey && selected.length) {
          // Shift with the arrows: the run from the first row selected to the next one.
          const from = rows.findIndex((row) => row.item.id === selected[0])
          const to = rows.findIndex((row) => row.item.id === next.item.id)
          const [a, b] = from < to ? [from, to] : [to, from]
          const ids = rows.slice(Math.max(0, a), b + 1).map((row) => row.item.id)
          choose(from <= to ? ids : ids.reverse(), next.item.id)
        } else choose([next.item.id])
        list.current?.querySelector(`[data-testid="motion-item-${next.item.id}"]`)?.scrollIntoView({ block: 'nearest' })
      }
    } else if ((event.key === 'a' || event.key === 'A') && (event.metaKey || event.ctrlKey) && !event.altKey) choose(rows.map((row) => row.item.id))
    else if (!current) return
    else if (event.key === 'ArrowRight' && plain && current.type === 'Folder') toggleFolder(current.id, true)
    else if (event.key === 'ArrowLeft' && plain) {
      if (current.type === 'Folder' && open.has(current.id)) toggleFolder(current.id, false)
      else if (current.parent != null && byId(current.parent)) choose([current.parent])
    } else if ((event.key === 'F2' || event.key === 'Enter') && plain && !event.shiftKey && single) rename(single)
    else if ((event.key === 'Delete' || event.key === 'Backspace') && plain) void remove(chosen)
    else return
    // The panel's keys are its own: Delete here is not the timeline's.
    event.preventDefault()
    event.stopPropagation()
  }

  /** Where a drag over `item` would put the dragged items: into it (a folder) or beside it. */
  const dropTarget = (item: ProjectItem | null): number | 'root' => (item == null ? 'root' : item.type === 'Folder' ? item.id : (parentOf(items, item) ?? 'root'))
  const dragOver = (event: DragEvent<HTMLElement>, item: ProjectItem | null) => {
    if (readOnly || !event.dataTransfer.types.includes(ITEMS_MIME)) return
    event.preventDefault()
    event.stopPropagation()
    event.dataTransfer.dropEffect = 'move'
    const target = dropTarget(item)
    setDropOn((now) => (now === target ? now : target))
  }
  const drop = (event: DragEvent<HTMLElement>, item: ProjectItem | null) => {
    setDropOn(null)
    if (readOnly || !event.dataTransfer.types.includes(ITEMS_MIME)) return
    event.preventDefault()
    event.stopPropagation()
    const ids = parseAnswer<number[]>(event.dataTransfer.getData(ITEMS_MIME))
    if (!Array.isArray(ids)) return
    const target = dropTarget(item)
    moveInto(
      ids.filter((id) => typeof id === 'number'),
      target === 'root' ? null : target,
    )
  }

  const menuItems = menu?.item ? (selected.includes(menu.item.id) ? chosen : [menu.item]) : []
  const menuFirst = menuItems.length === 1 ? menuItems[0] : null
  const sources = menuItems.filter((item) => isLayerSource(item) && item.id !== info.comp?.id)
  const comps = menuItems.filter((item) => item.type === 'Composition')
  const folderFor = (item: ProjectItem | null) => (item == null ? null : item.type === 'Folder' ? item.id : parentOf(items, item))

  return (
    <aside className="kui-panel motion-project" style={{ width }} data-testid="motion-project-panel">
      <div className="motion-project-head">
        <span className="motion-project-title">{t('project.title')}</span>
        <span style={{ flex: 1 }} />
        <IconButton icon={UploadIcon} label={t('project.import')} disabled={readOnly} onClick={onImport} data-testid="motion-project-import" />
        <IconButton icon={FolderPlusIcon} label={t('project.newFolder')} disabled={readOnly} onClick={() => void newFolder(folderFor(single))} data-testid="motion-project-new-folder" />
        <IconButton icon={NewCompIcon} label={t('project.newComp')} disabled={readOnly} onClick={onNewComp} data-testid="motion-project-new-comp" />
      </div>
      <div className="motion-project-search">
        <TextInput
          value={search}
          placeholder={t('project.search')}
          aria-label={t('project.search')}
          onChange={(event) => setSearch(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Escape' || event.key === 'ArrowDown' || event.key === 'Enter') {
              event.preventDefault()
              event.stopPropagation()
              if (event.key === 'Escape') setSearch('')
              list.current?.focus({ preventScroll: true })
            }
          }}
          data-testid="motion-project-search"
        />
      </div>
      <div
        ref={list}
        className="kui-panel-body motion-project-list"
        role="listbox"
        aria-multiselectable="true"
        aria-label={t('project.title')}
        tabIndex={0}
        onKeyDown={keyDown}
        data-drop={dropOn === 'root' || undefined}
        onDragOver={(event) => dragOver(event, null)}
        onDragLeave={(event) => {
          if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) setDropOn(null)
        }}
        onDrop={(event) => drop(event, null)}
        onClick={(event) => {
          if (event.target === event.currentTarget) choose([])
        }}
        onContextMenu={(event) => {
          if (event.target !== event.currentTarget) return
          event.preventDefault()
          setMenu({ at: { x: event.clientX, y: event.clientY }, item: null })
        }}
        data-testid="motion-project-list"
        data-selection={selected.join(' ')}
      >
        {rows.length ? (
          rows.map(({ item, depth }) => {
            const Icon = ICONS[item.type] ?? ImageIcon
            const active = item.type === 'Composition' && item.id === info.comp?.id
            const folder = item.type === 'Folder'
            const isOpen = folder && (open.has(item.id) || search.trim() !== '')
            return (
              <div
                key={item.id}
                role="option"
                aria-selected={selected.includes(item.id)}
                aria-expanded={folder ? isOpen : undefined}
                className="motion-project-row"
                data-active={active || undefined}
                data-type={item.type}
                data-drop={folder && dropOn === item.id ? true : undefined}
                data-missing={item.footage?.missing || undefined}
                style={{ paddingLeft: 4 + depth * 14 }}
                draggable={!readOnly && renaming !== item.id}
                onDragStart={(event: DragEvent<HTMLDivElement>) => {
                  const ids = selected.includes(item.id) ? selected : [item.id]
                  if (!selected.includes(item.id)) choose([item.id])
                  event.dataTransfer.setData(ITEMS_MIME, JSON.stringify(ids))
                  if (isLayerSource(item)) event.dataTransfer.setData(ITEM_MIME, String(item.id))
                  event.dataTransfer.setData('text/plain', item.name)
                  event.dataTransfer.effectAllowed = 'copyMove'
                }}
                onDragEnd={() => setDropOn(null)}
                onDragOver={(event) => dragOver(event, item)}
                onDrop={(event) => drop(event, item)}
                onClick={(event) => click(event, item)}
                onDoubleClick={() => activate(item)}
                onContextMenu={(event) => {
                  event.preventDefault()
                  event.stopPropagation()
                  if (!selected.includes(item.id)) choose([item.id])
                  setMenu({ at: { x: event.clientX, y: event.clientY }, item })
                }}
                title={item.name}
                data-testid={`motion-item-${item.id}`}
              >
                {folder ? (
                  <span
                    className="motion-project-twirl"
                    role="button"
                    aria-label={t(isOpen ? 'project.closeFolder' : 'project.openFolder')}
                    onClick={(event) => {
                      event.stopPropagation()
                      toggleFolder(item.id)
                    }}
                    onDoubleClick={(event) => event.stopPropagation()}
                    data-testid={`motion-item-twirl-${item.id}`}
                  >
                    {isOpen ? <ChevronDownIcon size={12} aria-hidden="true" /> : <ChevronRightIcon size={12} aria-hidden="true" />}
                  </span>
                ) : (
                  <span className="motion-project-twirl" aria-hidden="true" />
                )}
                {/* Its colour label (Label in its menu). */}
                <span className="motion-label-swatch motion-project-label" style={{ width: 8, height: 8, background: labelHex(item.label) ?? 'transparent', borderColor: labelHex(item.label) ? undefined : 'transparent' }} data-label={item.label} data-testid={`motion-item-label-${item.id}`} aria-hidden="true" />
                <Icon size={14} aria-hidden="true" />
                {renaming === item.id ? (
                  <RenameField
                    value={name}
                    label={t('project.rename')}
                    onChange={setName}
                    onDone={(commit) => {
                      setRenaming(null)
                      if (commit && name.trim() && name.trim() !== item.name) void run('project.rename', { item: item.id, name: name.trim() })
                      list.current?.focus({ preventScroll: true })
                    }}
                    testId={`motion-item-rename-${item.id}`}
                  />
                ) : (
                  <span className="motion-project-name">{item.name}</span>
                )}
                <span className="motion-project-detail kui-muted">{detail(item)}</span>
              </div>
            )
          })
        ) : (
          <p className="kui-muted motion-empty">{search.trim() ? t('project.noMatch', { search: search.trim() }) : t('project.empty')}</p>
        )}
      </div>
      <ProjectDetails items={chosen} typeName={typeName} />
      <div
        className="motion-panel-edge"
        data-side="right"
        role="separator"
        aria-orientation="vertical"
        aria-label={t('project.resize')}
        aria-valuenow={width}
        tabIndex={0}
        onPointerDown={(event) => {
          if (event.button !== 0) return
          event.preventDefault()
          event.currentTarget.setPointerCapture(event.pointerId)
          edge.current = { pointerId: event.pointerId, x: event.clientX, width }
        }}
        onPointerMove={(event) => {
          const current = edge.current
          if (current?.pointerId === event.pointerId) onWidth(current.width + event.clientX - current.x)
        }}
        onPointerUp={() => (edge.current = null)}
        onPointerCancel={() => (edge.current = null)}
        onKeyDown={(event) => {
          if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
          event.preventDefault()
          event.stopPropagation()
          onWidth(width + (event.key === 'ArrowRight' ? 16 : -16))
        }}
        data-testid="motion-project-edge"
      />
      <ContextMenu at={menu?.at ?? null} onClose={() => setMenu(null)} label={t('project.title')} testId="motion-item-menu">
        {menuFirst?.type === 'Composition' ? <MenuItem label={t('project.open')} onSelect={() => activate(menuFirst)} testId="motion-item-menu-open" /> : null}
        {sources.length ? <MenuItem label={t('project.addToComp')} disabled={readOnly || !info.comp} onSelect={() => sources.forEach((item) => onPlace(item.id))} testId="motion-item-menu-add" /> : null}
        {menuItems.some((item) => item.type !== 'Folder') ? (
          <MenuItem label={t('project.newCompFrom')} disabled={readOnly} onSelect={() => void newCompFromSelection(menuItems)} testId="motion-item-menu-new-comp" />
        ) : null}
        {comps.length ? <MenuItem label={t('project.addToQueue')} disabled={readOnly} onSelect={() => onQueue(comps.map((item) => item.id))} testId="motion-item-menu-queue" /> : null}
        {menuFirst && isFootage(menuFirst) ? (
          <>
            <MenuSeparator />
            <MenuItem label={t('project.interpret')} disabled={readOnly} onSelect={() => onInterpret(menuFirst)} testId="motion-item-menu-interpret" />
            <MenuItem label={t('project.replace')} disabled={readOnly} onSelect={() => onReplace(menuFirst)} testId="motion-item-menu-replace" />
          </>
        ) : null}
        {menuItems.length && !readOnly ? (
          <LabelMenu
            current={menuFirst?.label}
            onLabel={(label) => void run('project.setLabel', { items: menuItems.map((item) => item.id), label })}
            onSelectGroup={() => void run('edit.selectLabelGroup', {})}
            testId="motion-item-menu"
          />
        ) : null}
        <MenuSeparator />
        <MenuItem label={t('project.newFolder')} disabled={readOnly} onSelect={() => void newFolder(folderFor(menu?.item ?? null))} testId="motion-item-menu-new-folder" />
        {menu?.item == null ? (
          <>
            <MenuItem label={t('project.import')} disabled={readOnly} onSelect={onImport} testId="motion-item-menu-import" />
            <MenuItem label={t('project.newComp')} disabled={readOnly} onSelect={onNewComp} testId="motion-item-menu-comp" />
          </>
        ) : null}
        {menuFirst ? <MenuItem label={t('project.rename')} shortcut="Enter" disabled={readOnly} onSelect={() => rename(menuFirst)} testId="motion-item-menu-rename" /> : null}
        {menuItems.length ? <MenuItem label={t('project.delete')} shortcut="Delete" disabled={readOnly} onSelect={() => void remove(menuItems)} testId="motion-item-menu-delete" /> : null}
        <MenuSeparator />
        <MenuItem label={t('project.removeUnused')} disabled={readOnly || !items.some((item) => (item.type === 'Image' || item.type === 'Video' || item.type === 'Audio' || item.type === 'Solid') && (item.uses ?? 0) === 0)} onSelect={() => void removeUnused()} testId="motion-item-menu-remove-unused" />
      </ContextMenu>
      {deleting ? (
        <Dialog
          title={deleting.items.length === 1 ? t('project.deleteTitle', { name: deleting.items[0].name }) : t('project.deleteTitleMany', { count: deleting.items.length })}
          testId="motion-delete-dialog"
          onClose={() => setDeleting(null)}
          actions={
            <>
              <Button onClick={() => setDeleting(null)} data-testid="motion-delete-dialog-cancel">
                {t('dialog.cancel')}
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  void run('project.delete', { items: deleting.items.map((item) => item.id) })
                  setDeleting(null)
                }}
                data-testid="motion-delete-dialog-ok"
              >
                {t('project.delete')}
              </Button>
            </>
          }
        >
          <p>{t('project.deleteUsed', { layers: deleting.layers, comps: deleting.comps })}</p>
        </Dialog>
      ) : null}
    </aside>
  )
}

/** The selected item's details under the list: what it is, its size, length and rate, how many layers use it, and how footage is interpreted. */
function ProjectDetails({ items, typeName }: { items: ProjectItem[]; typeName: (item: ProjectItem) => string }) {
  const { t } = useKitEditor()
  if (!items.length) return null
  if (items.length > 1)
    return (
      <div className="motion-project-info" role="status" data-testid="motion-project-info">
        <span className="kui-muted">{t('project.info.many', { count: items.length })}</span>
      </div>
    )
  const item = items[0]
  const footage = item.footage
  const rows: Array<[string, string, string]> = [['type', t('project.info.type'), typeName(item)]]
  if (item.size) rows.push(['size', t('project.info.size'), `${item.size[0]} × ${item.size[1]}`])
  if (item.duration != null && item.type !== 'Image' && item.type !== 'Solid' && item.type !== 'Folder') rows.push(['duration', t('project.info.duration'), t('project.seconds', { value: round(item.duration) })])
  if (item.frameRate != null) rows.push(['frameRate', t('project.info.frameRate'), footage?.conformed ? t('project.info.conformed', { fps: round(item.frameRate), native: round(footage.nativeFrameRate) }) : t('project.info.fps', { fps: round(item.frameRate) })])
  if (footage) {
    if (footage.hasVideo) rows.push(['alpha', t('project.info.alpha'), t(`interpret.alpha.${footage.alpha}`)])
    if (footage.loop > 1) rows.push(['loop', t('project.info.loop'), t('project.info.times', { count: footage.loop })])
    if (footage.hasVideo && Math.abs(footage.pixelAspect - 1) > 1e-6) rows.push(['pixelAspect', t('project.info.pixelAspect'), String(round(footage.pixelAspect))])
    if (footage.missing) rows.push(['missing', t('project.info.file'), t('project.info.missing')])
  }
  if (item.uses != null) rows.push(['uses', t('project.info.uses'), t('project.info.usedTimes', { count: item.uses })])
  return (
    <div className="motion-project-info" data-testid="motion-project-info" data-item={item.id}>
      <strong className="motion-project-info-name" title={item.name}>
        {item.name}
      </strong>
      <dl>
        {rows.map(([key, label, value]) => (
          <div key={key} data-testid={`motion-project-info-${key}`}>
            <dt className="kui-muted">{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}
