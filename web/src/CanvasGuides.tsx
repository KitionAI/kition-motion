import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { ContextMenu, MenuItem, useEngineQuery, useKitEditor, type ContextMenuPoint, type KitCanvasView } from '@kition/sdk/editor'
import type { ViewStore } from './CanvasScrollbars'

/** What View shows over the composition and what a moved layer snaps to: the person's view, not the document. */
export type Overlays = { rulers: boolean; guides: boolean; lockGuides: boolean; grid: boolean; snapGuides: boolean; snapGrid: boolean; safe: boolean }

export const OVERLAY_KEYS = ['rulers', 'guides', 'lockGuides', 'grid', 'snapGuides', 'snapGrid', 'safe'] as const

const STORED = 'motion-view-overlays'

/** Guides show and snap until turned off; rulers, the grid and the safe areas start off. */
const DEFAULTS: Overlays = { rulers: false, guides: true, lockGuides: false, grid: false, snapGuides: true, snapGrid: false, safe: false }

/** The View menu's switches, kept for the session (sessionStorage). */
export function useOverlays() {
  const [overlays, setOverlays] = useState<Overlays>(() => {
    try {
      return { ...DEFAULTS, ...(JSON.parse(globalThis.sessionStorage?.getItem(STORED) ?? '{}') as Partial<Overlays>) }
    } catch {
      return DEFAULTS
    }
  })
  const toggle = (key: keyof Overlays) =>
    setOverlays((current) => {
      const next = { ...current, [key]: !current[key] }
      try {
        sessionStorage.setItem(STORED, JSON.stringify(next))
      } catch {
        // A storage the browser refuses keeps the switch for this page only.
      }
      return next
    })
  return { overlays, toggle }
}

/** A guide as `comp.info` lists it. */
type Guide = { orientation: 'vertical' | 'horizontal'; position: number }

/** The rulers' thickness, in CSS pixels. */
export const RULER = 18

/** How near (CSS pixels) a press must be to a guide to take it. */
const GRAB = 4

/** A guide being dragged: a new one out of a ruler, or an existing one (`index`); where it is now, in composition pixels. */
type Dragging = { index: number | null; vertical: boolean; position: number; pointerId: number; overRuler: boolean }

/** The view, a frame after the canvas reports it (as the scrollbars read it). */
export function useView(views: ViewStore) {
  const [view, setView] = useState<KitCanvasView | null>(views.get)
  useEffect(() => {
    let frame = 0
    const later = () => {
      if (frame) return
      frame = requestAnimationFrame(() => {
        frame = 0
        setView(views.get())
      })
    }
    later()
    const stop = views.subscribe(later)
    return () => {
      stop()
      cancelAnimationFrame(frame)
    }
  }, [views])
  return view
}

/** Round numbers of composition pixels for a ruler's ticks: about every 80 screen pixels. */
function tickStep(zoom: number) {
  const raw = 80 / Math.max(zoom, 1e-6)
  const power = 10 ** Math.floor(Math.log10(raw))
  return [1, 2, 5, 10].map((factor) => factor * power).find((candidate) => candidate >= raw) ?? raw
}

/**
 * What View draws over the composition: the rulers (top and left, in composition pixels), the
 * composition's guides, the grid and the title/action safe areas. A guide drags out of a ruler
 * (`view.addGuide`), an unlocked guide drags to a new place (`view.moveGuide`) or back onto a
 * ruler to remove it (`view.removeGuide`), each one undo step. The Selection tool snaps to the
 * guides and the grid (the engine's `setSnapTargets`, set by App).
 */
export function CanvasGuides({ views, comp, overlays, gridSpacing, readOnly }: { views: ViewStore; comp: { width: number; height: number }; overlays: Overlays; gridSpacing: number; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  const view = useView(views)
  const info = useEngineQuery<{ guides?: Guide[] }>('comp.info', {})
  const guides = Array.isArray(info?.guides) ? info.guides : []
  const [dragging, setDragging] = useState<Dragging | null>(null)
  // A guide's right-click menu: Delete Guide, Clear Guides.
  const [guideMenu, setGuideMenu] = useState<{ index: number; at: ContextMenuPoint } | null>(null)
  const box = useRef<HTMLDivElement>(null)
  if (!view || !(view.zoom > 0)) return null
  const sx = (x: number) => (x - view.x) * view.zoom
  const sy = (y: number) => (y - view.y) * view.zoom
  const local = (event: { clientX: number; clientY: number }) => {
    const rect = box.current?.getBoundingClientRect()
    return rect ? { x: event.clientX - rect.left, y: event.clientY - rect.top } : { x: 0, y: 0 }
  }
  const compAt = (event: { clientX: number; clientY: number }, vertical: boolean) => {
    const { x, y } = local(event)
    return Math.round(vertical ? view.x + x / view.zoom : view.y + y / view.zoom)
  }
  const onRuler = (event: { clientX: number; clientY: number }) => {
    const { x, y } = local(event)
    return overlays.rulers && (x < RULER || y < RULER)
  }

  const start = (event: ReactPointerEvent<HTMLElement>, index: number | null, vertical: boolean) => {
    if (event.button !== 0 || readOnly) return
    event.preventDefault()
    event.stopPropagation()
    event.currentTarget.setPointerCapture(event.pointerId)
    setDragging({ index, vertical, position: compAt(event, vertical), pointerId: event.pointerId, overRuler: index === null })
  }
  const move = (event: ReactPointerEvent<HTMLElement>) => {
    if (!dragging || dragging.pointerId !== event.pointerId) return
    setDragging({ ...dragging, position: compAt(event, dragging.vertical), overRuler: onRuler(event) })
  }
  const end = (event: ReactPointerEvent<HTMLElement>, cancelled: boolean) => {
    if (!dragging || dragging.pointerId !== event.pointerId) return
    setDragging(null)
    if (cancelled) return
    // Onto a ruler, or out of the composition's frame: the guide goes.
    const out = compAt(event, dragging.vertical)
    const removed = onRuler(event) || out < 0 || out > (dragging.vertical ? comp.width : comp.height)
    const orientation = dragging.vertical ? 'vertical' : 'horizontal'
    // Where the pointer is let go (WebKit may not report the last moves before the release).
    const position = compAt(event, dragging.vertical)
    if (dragging.index === null) {
      if (!removed) void run('view.addGuide', { orientation, position })
    } else if (removed) void run('view.removeGuide', { index: dragging.index })
    else if (guides[dragging.index]?.position !== position) void run('view.moveGuide', { index: dragging.index, position })
  }

  const step = tickStep(view.zoom)
  const ticks = (from: number, to: number) => {
    const out: number[] = []
    for (let value = Math.ceil(from / step) * step; value <= to && out.length < 200; value += step) out.push(Math.round(value))
    return out
  }
  const across = { x0: view.x, x1: view.x + view.width / view.zoom, y0: view.y, y1: view.y + view.height / view.zoom }
  const gridLines = (count: number, spacing: number) => (spacing * view.zoom >= 6 ? Array.from({ length: Math.floor(count / spacing) + 1 }, (_, at) => at * spacing) : [])
  const shown = guides.map((guide, index) => ({ ...guide, index, position: dragging?.index === index ? dragging.position : guide.position }))
  return (
    <>
    <div
      ref={box}
      className="motion-guides"
      data-testid="motion-guides"
      data-zoom={view.zoom}
      data-origin={`${view.x} ${view.y}`}
      onPointerMove={move}
      onPointerUp={(event) => end(event, false)}
      onPointerCancel={(event) => end(event, true)}
    >
      <svg width={view.width} height={view.height} aria-hidden="true">
        {overlays.grid
          ? [
              ...gridLines(comp.width, gridSpacing).map((x) => <line key={`gx${x}`} className="motion-grid-line" x1={sx(x)} x2={sx(x)} y1={sy(0)} y2={sy(comp.height)} />),
              ...gridLines(comp.height, gridSpacing).map((y) => <line key={`gy${y}`} className="motion-grid-line" x1={sx(0)} x2={sx(comp.width)} y1={sy(y)} y2={sy(y)} />),
            ]
          : null}
        {overlays.safe
          ? [0.05, 0.1].map((inset) => (
              <rect
                key={inset}
                className="motion-safe"
                x={sx(comp.width * inset)}
                y={sy(comp.height * inset)}
                width={comp.width * (1 - 2 * inset) * view.zoom}
                height={comp.height * (1 - 2 * inset) * view.zoom}
                data-testid={inset === 0.05 ? 'motion-safe-action' : 'motion-safe-title'}
              />
            ))
          : null}
        {overlays.guides
          ? shown.map((guide) =>
              guide.orientation === 'vertical' ? (
                <line key={guide.index} className="motion-guide-line" x1={sx(guide.position)} x2={sx(guide.position)} y1={0} y2={view.height} />
              ) : (
                <line key={guide.index} className="motion-guide-line" x1={0} x2={view.width} y1={sy(guide.position)} y2={sy(guide.position)} />
              ),
            )
          : null}
        {dragging?.index === null ? (
          dragging.vertical ? (
            <line className="motion-guide-line" data-new x1={sx(dragging.position)} x2={sx(dragging.position)} y1={0} y2={view.height} />
          ) : (
            <line className="motion-guide-line" data-new x1={0} x2={view.width} y1={sy(dragging.position)} y2={sy(dragging.position)} />
          )
        ) : null}
      </svg>
      {/* Each unlocked guide's grab strip, over the line. */}
      {overlays.guides && !overlays.lockGuides && !readOnly
        ? shown.map((guide) => {
            const vertical = guide.orientation === 'vertical'
            const at = vertical ? sx(guide.position) : sy(guide.position)
            return (
              <div
                key={guide.index}
                className="motion-guide-grab"
                data-vertical={vertical || undefined}
                style={vertical ? { left: at - GRAB, width: 2 * GRAB } : { top: at - GRAB, height: 2 * GRAB }}
                title={t('guides.dragHint')}
                onPointerDown={(event) => start(event, guide.index, vertical)}
                onContextMenu={(event) => {
                  event.preventDefault()
                  setGuideMenu({ index: guide.index, at: { x: event.clientX, y: event.clientY } })
                }}
                data-testid={`motion-guide-${guide.index}`}
                data-position={guide.position}
              />
            )
          })
        : null}
      {overlays.rulers ? (
        <>
          <div className="motion-ruler-bar" data-side="top" title={t('guides.fromRuler')} onPointerDown={(event) => start(event, null, false)} data-testid="motion-ruler-top">
            {ticks(across.x0, across.x1).map((value) => (
              <span key={value} style={{ left: sx(value) }}>
                {value}
              </span>
            ))}
          </div>
          <div className="motion-ruler-bar" data-side="left" title={t('guides.fromRuler')} onPointerDown={(event) => start(event, null, true)} data-testid="motion-ruler-left">
            {ticks(across.y0, across.y1).map((value) => (
              <span key={value} style={{ top: sy(value) }}>
                {value}
              </span>
            ))}
          </div>
        </>
      ) : null}
    </div>
    <ContextMenu at={guideMenu?.at ?? null} onClose={() => setGuideMenu(null)} label={t('guides.menu')} testId="motion-guide-menu">
      <MenuItem label={t('guides.delete')} onSelect={() => guideMenu && void run('view.removeGuide', { index: guideMenu.index })} testId="motion-guide-menu-delete" />
      <MenuItem label={t('guides.clear')} onSelect={() => void run('view.clearGuides', {})} testId="motion-guide-menu-clear" />
    </ContextMenu>
    </>
  )
}
