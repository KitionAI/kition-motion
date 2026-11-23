import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MutableRefObject } from 'react'
import { useKitEditor, useReadOnly } from '@kition/sdk/editor'
import type { MotionInfo, MotionLayer } from '../motion'
import { TimelineContext, type TimelineFrame } from './context'
import { LayerBar } from './LayerBar'
import { LayerMenu, type LayerMenuTarget } from './LayerMenu'
import { KeyMark } from './KeyMark'
import { layerKeyMarks } from './keys'
import { LayerNames } from './LayerNames'
import { PropertyRows } from './PropertyRows'
import { Ruler } from './Ruler'
import { SOLO_PATHS, type Twirl } from './tree'
import { useRowDrag } from './useRowDrag'
import { leftOf, type TimeView } from './view'

/** A key handler the editor asks first; true when the timeline took the key. */
export type TimelineKeys = MutableRefObject<((event: KeyboardEvent<HTMLDivElement>) => boolean) | null>

/** The width of an element, following resizes. */
function useWidth() {
  const [element, setElement] = useState<HTMLDivElement | null>(null)
  const [width, setWidth] = useState(0)
  useEffect(() => {
    if (!element) return undefined
    const observer = new ResizeObserver(() => setWidth(element.getBoundingClientRect().width))
    observer.observe(element)
    setWidth(element.getBoundingClientRect().width)
    return () => observer.disconnect()
  }, [element])
  return [width, setElement] as const
}

/**
 * The composition's layers over time, as in After Effects: one row per layer with its switches,
 * name, bar from in to out point, and keyframes; the ruler scrubs the current time. A bar drags
 * the layer in time and its ends trim it; a row drags to a new place in the stack; a double-click
 * renames; a right-click opens the layer menu; a click on empty timeline deselects.
 */
export function Timeline({ info, onSeek, keys }: { info: MotionInfo; onSeek: (seconds: number) => void; keys: TimelineKeys }) {
  const { run, t } = useKitEditor()
  const readOnly = useReadOnly()
  const duration = Math.max(info.comp?.duration ?? 0, info.frame)
  const view = useMemo<TimeView>(() => ({ start: 0, span: duration }), [duration])
  const [width, rulerRef] = useWidth()
  const [shift, setShift] = useState<{ layers: number[]; delta: number } | null>(null)
  const [renaming, setRenaming] = useState<number | null>(null)
  const [menu, setMenu] = useState<LayerMenuTarget | null>(null)
  const rows = useRef<HTMLDivElement>(null)
  const order = useMemo(() => info.layers.map((layer) => layer.id), [info.layers])
  const rowDrag = useRowDrag(rows, order)
  const frame = useMemo<TimelineFrame>(() => ({ info, view, width, readOnly }), [info, view, width, readOnly])
  const selected = info.layers.filter((layer) => layer.selected).map((layer) => layer.id)
  // Which layers are twirled open and how, and per layer the groups the person opened or closed.
  const [twirls, setTwirls] = useState<ReadonlyMap<number, Twirl>>(new Map())
  const [toggled, setToggled] = useState<ReadonlyMap<number, ReadonlySet<string>>>(new Map())
  const noKeys = useMemo(() => new Set<string>(), [])

  const twirl = (layer: number) =>
    setTwirls((current) => {
      const next = new Map(current)
      if (next.has(layer)) next.delete(layer)
      else next.set(layer, { kind: 'tree' })
      return next
    })
  const toggleGroup = (layer: number, path: string) =>
    setToggled((current) => {
      const next = new Map(current)
      const paths = new Set(next.get(layer))
      if (paths.has(path)) paths.delete(path)
      else paths.add(path)
      return next.set(layer, paths)
    })
  /** U, P, S, R, T: the selected layers (all, with none selected) show only those properties; the same key again closes them. */
  const solo = (key: string, paths: Twirl & { kind: 'only' }) => {
    const targets = selected.length ? selected : order
    setTwirls((current) => {
      const next = new Map(current)
      const again = targets.every((layer) => {
        const shown = current.get(layer)
        return shown?.kind === 'only' && shown.key === key
      })
      for (const layer of targets) {
        if (again) next.delete(layer)
        else next.set(layer, paths)
      }
      return next
    })
  }

  keys.current = (event) => {
    const command = event.metaKey || event.ctrlKey
    const plain = !command && !event.altKey && !event.shiftKey
    // Delete, or Backspace (the Mac's delete key): the selected keyframes, else the selected layers.
    if (plain && (event.key === 'Delete' || event.key === 'Backspace')) {
      if (readOnly || !selected.length) return false
      void run('edit.clear', {})
      return true
    }
    if (plain && event.code === 'KeyU') {
      solo('KeyU', { kind: 'only', key: 'KeyU', paths: 'animated' })
      return true
    }
    if (plain && SOLO_PATHS[event.code]) {
      solo(event.code, { kind: 'only', key: event.code, paths: SOLO_PATHS[event.code] })
      return true
    }
    if (command && !event.altKey && !event.shiftKey && event.code === 'KeyD') {
      if (!readOnly && selected.length) void run('edit.duplicate', { layers: selected })
      return true
    }
    return false
  }

  const select = (layer: number, add: boolean) => void run('layer.select', { layers: [layer], add, toggle: add })
  const openMenu = (layer: MotionLayer, at: { x: number; y: number }) => {
    if (!layer.selected) void run('layer.select', { layers: [layer.id] })
    setMenu({ at, layer: layer.id, layers: layer.selected ? selected : [layer.id] })
  }

  return (
    <TimelineContext.Provider value={frame}>
      <div className="motion-timeline" data-testid="motion-timeline">
        <div
          ref={rows}
          className="motion-timeline-rows"
          onClick={(event) => {
            // A click on empty timeline (below the rows, or a track beside its bar) deselects.
            const target = event.target as HTMLElement
            if (target === event.currentTarget || target.classList.contains('motion-row-track')) void run('edit.deselectAll', {})
          }}
          data-testid="motion-timeline-rows"
        >
          <div className="motion-timeline-head">
            <div className="motion-timeline-names kui-muted">{t('layers')}</div>
            <Ruler onSeek={onSeek} rulerRef={rulerRef} />
          </div>
          {info.layers.length ? (
            info.layers.map((layer) => (
              <div key={layer.id} className="motion-layer-block" data-layer-block={layer.id} data-testid={`motion-layer-block-${layer.id}`}>
                <div className="motion-row" data-selected={layer.selected} data-testid={`motion-layer-${layer.id}`}>
                  <LayerNames
                    layer={layer}
                    open={twirls.has(layer.id)}
                    onTwirl={() => twirl(layer.id)}
                    renaming={renaming === layer.id}
                    onRename={() => setRenaming(layer.id)}
                    onRenameDone={(name) => {
                      setRenaming(null)
                      if (name !== null && name.trim() && name !== layer.name) void run('layer.rename', { layer: layer.id, name: name.trim() })
                    }}
                    onSelect={(add) => select(layer.id, add)}
                    onDrag={(y, phase) => {
                      const index = rowDrag.drag(layer.id, y, phase)
                      if (index !== null) void run('layer.arrange', { layers: [layer.id], index })
                    }}
                    onMenu={(event) => {
                      event.preventDefault()
                      openMenu(layer, { x: event.clientX, y: event.clientY })
                    }}
                  />
                  <div
                    className="motion-row-track"
                    onContextMenu={(event) => {
                      if (!(event.target as HTMLElement).closest('.motion-bar')) return
                      event.preventDefault()
                      openMenu(layer, { x: event.clientX, y: event.clientY })
                    }}
                  >
                    <LayerBar layer={layer} shift={shift} onShift={setShift} />
                    {/* A collapsed layer keeps its keyframes on its own row; twirled open, its property rows show them. */}
                    {twirls.has(layer.id) ? null : layerKeyMarks(layer).map((mark) => <KeyMark key={mark.time} time={mark.time} keys={mark.keys} selected={noKeys} label={layer.name} row={String(layer.id)} />)}
                    <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
                  </div>
                </div>
                {twirls.has(layer.id) ? (
                  <PropertyRows layer={layer} twirl={twirls.get(layer.id) ?? { kind: 'tree' }} toggled={toggled.get(layer.id) ?? noKeys} onToggle={(path) => toggleGroup(layer.id, path)} selected={noKeys} />
                ) : null}
              </div>
            ))
          ) : (
            <p className="kui-muted motion-empty">{t('noLayers')}</p>
          )}
          {rowDrag.drop ? <div className="motion-drop-line" style={{ top: rowDrag.drop.top }} data-testid="motion-drop-line" /> : null}
        </div>
        <LayerMenu target={menu} onClose={() => setMenu(null)} onRename={(layer) => setRenaming(layer)} />
      </div>
    </TimelineContext.Provider>
  )
}
