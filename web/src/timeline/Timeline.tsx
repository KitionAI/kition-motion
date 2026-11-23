import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MutableRefObject } from 'react'
import { useKitEditor, useReadOnly } from '@kition/sdk/editor'
import type { MotionInfo, MotionLayer } from '../motion'
import { TimelineContext, type TimelineFrame } from './context'
import { LayerBar } from './LayerBar'
import { LayerMenu, type LayerMenuTarget } from './LayerMenu'
import { LayerNames } from './LayerNames'
import { Ruler } from './Ruler'
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

  keys.current = (event) => {
    const command = event.metaKey || event.ctrlKey
    const plain = !command && !event.altKey && !event.shiftKey
    if (readOnly) return false
    // Delete, or Backspace (the Mac's delete key): the selected keyframes, else the selected layers.
    if (plain && (event.key === 'Delete' || event.key === 'Backspace')) {
      if (!selected.length) return false
      void run('edit.clear', {})
      return true
    }
    if (command && !event.altKey && !event.shiftKey && event.code === 'KeyD') {
      if (selected.length) void run('edit.duplicate', { layers: selected })
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
                    {layer.animated.flatMap((property) =>
                      property.times.map((time) => (
                        <span key={`${property.path}-${time}`} className="motion-key" style={{ left: leftOf(view, time) }} title={`${property.name} ${time.toFixed(2)}s`} data-testid="motion-key" />
                      )),
                    )}
                    <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
                  </div>
                </div>
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
