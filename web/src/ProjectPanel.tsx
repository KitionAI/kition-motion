import { useMemo, useRef, useState, type ComponentType, type DragEvent, type KeyboardEvent } from 'react'
import {
  AudioLinesIcon as AudioIcon,
  ClapperboardIcon as CompIcon,
  FilePlusIcon as NewCompIcon,
  FolderIcon,
  ImageIcon,
  SquareIcon as SolidIcon,
  UploadIcon,
  VideoIcon,
} from '@kition/tessera/icons/react'
import { Button, ContextMenu, Dialog, IconButton, MenuItem, MenuSeparator, RenameField, useEngineQuery, useKitEditor, useReadOnly, type ContextMenuPoint } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'
import { ITEM_MIME, isLayerSource, type ProjectItem, type ProjectSummary } from './footage'

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

/** `item` in the project's tree order: each folder's items under it, by name. */
function treeOrder(items: readonly ProjectItem[]) {
  const children = new Map<number | null, ProjectItem[]>()
  for (const item of items) {
    const parent = item.parent != null && items.some((other) => other.id === item.parent) ? item.parent : null
    children.set(parent, [...(children.get(parent) ?? []), item])
  }
  const out: Array<{ item: ProjectItem; depth: number }> = []
  // Each item once, and the nesting bounded, even in a project whose folders loop.
  const seen = new Set<number>()
  const walk = (parent: number | null, depth: number) => {
    if (depth > 32) return
    for (const item of [...(children.get(parent) ?? [])].sort((a, b) => a.name.localeCompare(b.name))) {
      if (seen.has(item.id)) continue
      seen.add(item.id)
      out.push({ item, depth })
      if (item.type === 'Folder') walk(item.id, depth + 1)
    }
  }
  walk(null, 0)
  return out
}

/** What was asked of the person before deleting: the item and what uses it. */
type Deleting = { item: ProjectItem; layers: number; comps: number }

/**
 * The Project panel, left of the composition: the project's compositions, footage, solids and
 * folders from `project.summary`, each with its type's icon, name, and size or duration. A
 * double-click opens a composition or places footage and solids in the active composition at the
 * current time; a row drags onto the timeline or the composition the same way. F2 renames the
 * selected item (also from its menu), Delete removes it (asking first when layers use it), and
 * the header imports footage and makes a new composition.
 */
export function ProjectPanel({
  info,
  width,
  onWidth,
  onImport,
  onNewComp,
  onPlace,
}: {
  info: MotionInfo
  width: number
  onWidth: (width: number) => void
  onImport: () => void
  onNewComp: () => void
  onPlace: (item: number) => void
}) {
  const { run, query, t } = useKitEditor()
  const readOnly = useReadOnly()
  const summary = useEngineQuery<ProjectSummary>('project.summary', {})
  const rows = useMemo(() => treeOrder(summary?.items ?? []), [summary])
  const [selected, setSelected] = useState<number | null>(null)
  const [renaming, setRenaming] = useState<number | null>(null)
  const [name, setName] = useState('')
  const [menu, setMenu] = useState<{ at: ContextMenuPoint; item: ProjectItem } | null>(null)
  const [deleting, setDeleting] = useState<Deleting | null>(null)
  const list = useRef<HTMLDivElement>(null)
  const edge = useRef<{ pointerId: number; x: number; width: number } | null>(null)
  const byId = (id: number | null) => rows.find((row) => row.item.id === id)?.item ?? null

  const detail = (item: ProjectItem) => {
    const size = item.size ? `${item.size[0]} × ${item.size[1]}` : ''
    const seconds = item.duration != null && item.type !== 'Image' && item.type !== 'Solid' ? t('project.seconds', { value: Math.round(item.duration * 100) / 100 }) : ''
    return [size, seconds].filter(Boolean).join(' · ')
  }
  /** A double-click (or Enter): a composition opens, footage and solids become a layer. */
  const activate = (item: ProjectItem) => {
    if (item.type === 'Composition') void run('comp.open', { comp: item.id })
    else if (isLayerSource(item) && !readOnly) onPlace(item.id)
  }
  const rename = (item: ProjectItem) => {
    if (readOnly) return
    setName(item.name)
    setRenaming(item.id)
  }
  /** Delete, asking first when layers use the item (they go with it). */
  const remove = async (item: ProjectItem) => {
    if (readOnly) return
    const usage = await query<{ layers?: number; comps?: number }>('project.usage', { items: [item.id] })
    if ((usage?.layers ?? 0) > 0) setDeleting({ item, layers: usage?.layers ?? 0, comps: usage?.comps ?? 0 })
    else void run('project.delete', { items: [item.id] })
  }
  const newCompFrom = async (item: ProjectItem) => {
    if ((await run('project.select', { items: [item.id] })) !== null) await run('file.newCompFromSelection', {})
  }

  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (renaming !== null) return
    const item = byId(selected)
    const plain = !event.metaKey && !event.ctrlKey && !event.altKey
    const move = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0
    if (move && plain) {
      const at = rows.findIndex((row) => row.item.id === selected)
      const next = rows[Math.max(0, Math.min(rows.length - 1, at + move))]
      if (next) setSelected(next.item.id)
    } else if (!item) return
    else if (event.key === 'F2' && plain) rename(item)
    else if ((event.key === 'Delete' || event.key === 'Backspace') && plain) void remove(item)
    else if (event.key === 'Enter' && plain) activate(item)
    else return
    // The panel's keys are its own: Delete here is not the timeline's.
    event.preventDefault()
    event.stopPropagation()
  }

  return (
    <aside className="kui-panel motion-project" style={{ width }} data-testid="motion-project-panel">
      <div className="motion-project-head">
        <span className="motion-project-title">{t('project.title')}</span>
        <span style={{ flex: 1 }} />
        <IconButton icon={UploadIcon} label={t('project.import')} disabled={readOnly} onClick={onImport} data-testid="motion-project-import" />
        <IconButton icon={NewCompIcon} label={t('project.newComp')} disabled={readOnly} onClick={onNewComp} data-testid="motion-project-new-comp" />
      </div>
      <div ref={list} className="kui-panel-body motion-project-list" role="listbox" aria-label={t('project.title')} tabIndex={0} onKeyDown={keyDown} data-testid="motion-project-list">
        {rows.length ? (
          rows.map(({ item, depth }) => {
            const Icon = ICONS[item.type] ?? ImageIcon
            const active = item.type === 'Composition' && item.id === info.comp?.id
            return (
              <div
                key={item.id}
                role="option"
                aria-selected={item.id === selected}
                className="motion-project-row"
                data-active={active || undefined}
                data-type={item.type}
                style={{ paddingLeft: 8 + depth * 14 }}
                draggable={!readOnly && isLayerSource(item) && renaming !== item.id}
                onDragStart={(event: DragEvent<HTMLDivElement>) => {
                  event.dataTransfer.setData(ITEM_MIME, String(item.id))
                  event.dataTransfer.setData('text/plain', item.name)
                  event.dataTransfer.effectAllowed = 'copy'
                  setSelected(item.id)
                }}
                onClick={() => {
                  setSelected(item.id)
                  list.current?.focus({ preventScroll: true })
                }}
                onDoubleClick={() => activate(item)}
                onContextMenu={(event) => {
                  event.preventDefault()
                  setSelected(item.id)
                  setMenu({ at: { x: event.clientX, y: event.clientY }, item })
                }}
                title={item.name}
                data-testid={`motion-item-${item.id}`}
              >
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
          <p className="kui-muted motion-empty">{t('project.empty')}</p>
        )}
      </div>
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
        {menu && menu.item.type === 'Composition' ? <MenuItem label={t('project.open')} onSelect={() => activate(menu.item)} testId="motion-item-menu-open" /> : null}
        {menu && menu.item.type !== 'Folder' && menu.item.id !== info.comp?.id ? (
          <MenuItem label={t('project.addToComp')} disabled={readOnly || !info.comp} onSelect={() => onPlace(menu.item.id)} testId="motion-item-menu-add" />
        ) : null}
        {menu && (menu.item.type === 'Image' || menu.item.type === 'Video' || menu.item.type === 'Audio') ? (
          <MenuItem label={t('project.newCompFrom')} disabled={readOnly} onSelect={() => void newCompFrom(menu.item)} testId="motion-item-menu-new-comp" />
        ) : null}
        <MenuSeparator />
        <MenuItem label={t('project.rename')} shortcut="F2" disabled={readOnly} onSelect={() => menu && rename(menu.item)} testId="motion-item-menu-rename" />
        <MenuItem label={t('project.delete')} disabled={readOnly} onSelect={() => menu && void remove(menu.item)} testId="motion-item-menu-delete" />
      </ContextMenu>
      {deleting ? (
        <Dialog
          title={t('project.deleteTitle', { name: deleting.item.name })}
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
                  void run('project.delete', { items: [deleting.item.id] })
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
