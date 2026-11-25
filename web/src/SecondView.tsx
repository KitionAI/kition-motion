import { useEffect, useRef, useState } from 'react'
import { XIcon } from '@kition/tessera/icons/react'
import { Button, IconButton, Select, useKitEditor } from '@kition/sdk/editor'
import { THREE_D_VIEWS } from './menus'

type Engine = { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> }

/** The session's width of the second view, in CSS pixels. */
const WIDTH_KEY = 'kition-motion.second-view-width'
/** The second view is at least this wide; the viewer keeps at least this share of the two. */
const MIN_WIDTH = 200
const VIEWER_SHARE = 0.45

/** The second view's width when nothing is kept: about 28 % of the window at 1280 px, 32 % from 1440 px. */
export function defaultSecondWidth(windowWidth: number) {
  const share = 0.28 + 0.04 * Math.max(0, Math.min(1, (windowWidth - 1280) / 160))
  return Math.round(windowWidth * share)
}

/** `width` kept between 200 px and what leaves the viewer 45 % of `room` (the viewer and the second view together). */
export function clampSecondWidth(width: number, room: number) {
  const most = Math.max(MIN_WIDTH, Math.floor(room * (1 - VIEWER_SHARE)))
  return Math.round(Math.max(MIN_WIDTH, Math.min(most, width)))
}

type Head = {
  width: number
  height: number
  lines: Array<[number, number, number, number, number, number, number, number, boolean]>
  layers?: Array<{ id: number; name: string; label: [number, number, number]; selected: boolean; quad: Array<[number, number]> }>
}

/** Whether point (`x`, `y`) is inside the polygon `points`. */
function inside(points: ReadonlyArray<readonly [number, number]>, x: number, y: number) {
  let hit = false
  for (let i = 0, j = points.length - 1; i < points.length; j = i, i += 1) {
    const [xi, yi] = points[i]
    const [xj, yj] = points[j]
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi || 1e-9) + xi) hit = !hit
  }
  return hit
}

/** How far (canvas pixels) point (`x`, `y`) is from the outline `points` (a layer seen edge-on is a line). */
function distance(points: ReadonlyArray<readonly [number, number]>, x: number, y: number) {
  let best = Infinity
  for (let i = 0; i < points.length; i += 1) {
    const [ax, ay] = points[i]
    const [bx, by] = points[(i + 1) % points.length]
    const dx = bx - ax
    const dy = by - ay
    const length = dx * dx + dy * dy
    const u = length > 0 ? Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / length)) : 0
    best = Math.min(best, Math.hypot(x - (ax + u * dx), y - (ay + u * dy)))
  }
  return best
}

/**
 * The two-view layout's second view (View ▸ Two Views), beside the composition viewer: the
 * composition seen through another 3D view (Top by default), with that view's ground, 3D layers'
 * outlines, cameras and lights, drawn again a moment after each change (the engine's `viewFrame`).
 * Each 3D layer is filled with its label colour, faintly, outlined in it and named, so a layer
 * seen edge-on is still a visible, named line; the selected ones are outlined in the accent colour.
 * The picture is fitted to the pane (as wide as it is, or as tall), on the viewer's background (the
 * engine draws a 3D view at the composition's aspect, so a pane of another shape keeps margins);
 * a click on a layer selects it (Shift adds it or takes it out), the handles stay in the viewer.
 * The bar on its left edge sets its width (kept for the session; at least 200 px, the viewer keeps
 * at least 45 %). While playback runs it keeps its last picture and says so.
 * Its drop-down picks the view; Make Main shows it in the viewer and puts the viewer's view here.
 */
export function SecondView({
  doc,
  view,
  onView,
  main,
  onMakeMain,
  onClose,
  onSelect,
  version,
  playing,
  compWidth,
  compHeight,
}: {
  doc: Engine | null
  view: string
  onView: (view: string) => void
  main: string
  onMakeMain: () => void
  onClose: () => void
  onSelect: (layer: number, add: boolean) => void
  version: unknown
  playing: boolean
  compWidth: number
  compHeight: number
}) {
  const { t, phrase } = useKitEditor()
  const section = useRef<HTMLElement>(null)
  const box = useRef<HTMLDivElement>(null)
  const canvas = useRef<HTMLCanvasElement>(null)
  const [size, setSize] = useState({ width: 0, height: 0 })
  const [room, setRoom] = useState(0)
  const [chosen, setChosen] = useState(() => {
    const stored = Number(globalThis.sessionStorage?.getItem(WIDTH_KEY))
    return Number.isFinite(stored) && stored >= MIN_WIDTH ? stored : defaultSecondWidth(window.innerWidth)
  })
  const width = room > 0 ? clampSecondWidth(chosen, room) : chosen
  const keep = (next: number) => {
    setChosen(next)
    try {
      sessionStorage.setItem(WIDTH_KEY, String(Math.round(next)))
    } catch {
      // Kept for this page only.
    }
  }
  const drag = useRef<{ pointerId: number; x: number; width: number } | null>(null)
  // Where the picture is drawn on the canvas (canvas pixels) and its scale from composition pixels,
  // with the layers' outlines: what a click hits.
  const placed = useRef<{ x: number; y: number; scale: number; layers: NonNullable<Head['layers']> } | null>(null)
  const [drawn, setDrawn] = useState<{ lines: number; width: number; height: number; layers: string; centres: string } | null>(null)
  useEffect(() => {
    const element = box.current
    const parent = section.current?.parentElement
    if (!element || !parent) return undefined
    const measure = () => {
      const rect = element.getBoundingClientRect()
      setSize({ width: Math.floor(rect.width), height: Math.floor(rect.height) })
      setRoom(Math.floor(parent.getBoundingClientRect().width))
    }
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    observer.observe(parent)
    measure()
    return () => observer.disconnect()
  }, [])
  useEffect(() => {
    if (!doc || size.width < 16 || size.height < 16) return undefined
    let current = true
    // While it plays, the second view keeps its last picture: the engine's time goes to the viewer.
    if (playing) return undefined
    const timer = window.setTimeout(async () => {
      const ratio = Math.min(2, window.devicePixelRatio || 1)
      const aspect = (compWidth || 16) / (compHeight || 9)
      // As wide as the pane, or as the pane's height allows at the composition's aspect.
      const fit = Math.min(size.width, size.height * aspect)
      const answer = await doc.invoke<Uint8Array>('viewFrame', view, Math.max(16, Math.round(fit * ratio))).catch(() => null)
      const element = canvas.current
      if (!current || !answer || !element || answer.length < 4) return
      const headSize = new DataView(answer.buffer, answer.byteOffset, 4).getUint32(0, true)
      const head = JSON.parse(new TextDecoder().decode(answer.subarray(4, 4 + headSize))) as Head
      const pixels = answer.subarray(4 + headSize)
      if (pixels.length < head.width * head.height * 4 || head.width < 1 || head.height < 1) return
      // The canvas covers the pane; the picture is fitted in its middle.
      element.width = Math.round(size.width * ratio)
      element.height = Math.round(size.height * ratio)
      const context = element.getContext('2d')
      if (!context) return
      const fitScale = Math.min(element.width / head.width, element.height / head.height)
      const pw = head.width * fitScale
      const ph = head.height * fitScale
      const ox = (element.width - pw) / 2
      const oy = (element.height - ph) / 2
      const picture = document.createElement('canvas')
      picture.width = head.width
      picture.height = head.height
      picture.getContext('2d')?.putImageData(new ImageData(new Uint8ClampedArray(pixels.slice(0, head.width * head.height * 4)), head.width, head.height), 0, 0)
      context.clearRect(0, 0, element.width, element.height)
      context.imageSmoothingEnabled = true
      context.drawImage(picture, ox, oy, pw, ph)
      // The lines are in composition pixels: drawn at the picture's scale.
      const scale = (head.width / (compWidth || head.width)) * fitScale
      const at = ([x, y]: readonly [number, number]) => [ox + x * scale, oy + y * scale] as const
      context.lineWidth = 1
      for (const [ax, ay, bx, by, r, g, b, a, dashed] of head.lines) {
        context.strokeStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`
        context.setLineDash(dashed ? [4, 3] : [])
        context.beginPath()
        context.moveTo(...at([ax, ay]))
        context.lineTo(...at([bx, by]))
        context.stroke()
      }
      // The 3D layers: filled faintly with their label colour, outlined in it (the selected ones in
      // the accent colour, thicker, over a light halo) and named at their middle.
      const accent = getComputedStyle(element).getPropertyValue('--kit-primary').trim() || '#3b82f6'
      const font = getComputedStyle(element).fontFamily || 'sans-serif'
      const layers = head.layers ?? []
      for (const layer of layers) {
        const points = layer.quad.map(at)
        if (points.length < 4) continue
        const [r, g, b] = layer.label
        context.beginPath()
        points.forEach(([x, y], index) => (index === 0 ? context.moveTo(x, y) : context.lineTo(x, y)))
        context.closePath()
        context.setLineDash([])
        context.fillStyle = `rgba(${r}, ${g}, ${b}, 0.28)`
        context.fill()
        if (layer.selected) {
          context.strokeStyle = 'rgba(255, 255, 255, 0.9)'
          context.lineWidth = 6
          context.stroke()
        }
        context.strokeStyle = layer.selected ? accent : `rgb(${r}, ${g}, ${b})`
        context.lineWidth = layer.selected ? 3.5 : 2
        context.stroke()
      }
      context.font = `${Math.round(11 * ratio)}px ${font}`
      context.textBaseline = 'middle'
      for (const layer of layers) {
        const points = layer.quad.map(at)
        if (!points.length) continue
        const cx = points.reduce((sum, [x]) => sum + x, 0) / points.length
        const cy = points.reduce((sum, [, y]) => sum + y, 0) / points.length
        const text = layer.name
        const w = context.measureText(text).width + 8
        const h = Math.round(16 * ratio)
        const x = Math.max(2, Math.min(element.width - w - 2, cx - w / 2))
        const y = Math.max(2, Math.min(element.height - h - 2, cy - h - 4))
        context.fillStyle = layer.selected ? accent : 'rgba(20, 20, 24, 0.78)'
        context.fillRect(x, y, w, h)
        context.fillStyle = '#ffffff'
        context.fillText(text, x + 4, y + h / 2)
      }
      placed.current = { x: ox, y: oy, scale, layers }
      // Each layer's middle in CSS pixels from the canvas's top left (for the browser tests' clicks).
      const centres = layers
        .map((layer) => {
          const points = layer.quad.map(at)
          if (!points.length) return ''
          const cx = points.reduce((sum, [x]) => sum + x, 0) / points.length / ratio
          const cy = points.reduce((sum, [, y]) => sum + y, 0) / points.length / ratio
          return `${layer.id}:${Math.round(cx)},${Math.round(cy)}`
        })
        .filter(Boolean)
        .join(' ')
      setDrawn({ lines: head.lines.length, width: Math.round(pw), height: Math.round(ph), layers: layers.map((layer) => `${layer.id}${layer.selected ? '*' : ''}`).join(' '), centres })
    }, 150)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [doc, view, size, version, playing, compWidth, compHeight])
  /** The layer drawn under a page point: the last drawn that holds it (drawn over the others). */
  const layerAt = (clientX: number, clientY: number) => {
    const element = canvas.current
    const shown = placed.current
    if (!element || !shown) return null
    const rect = element.getBoundingClientRect()
    const x = ((clientX - rect.left) / Math.max(1, rect.width)) * element.width
    const y = ((clientY - rect.top) / Math.max(1, rect.height)) * element.height
    const near = 6 * Math.min(2, window.devicePixelRatio || 1)
    const outline = (layer: NonNullable<Head['layers']>[number]) => layer.quad.map(([qx, qy]) => [shown.x + qx * shown.scale, shown.y + qy * shown.scale] as const)
    const hits = shown.layers.filter((layer) => layer.quad.length >= 2 && (inside(outline(layer), x, y) || distance(outline(layer), x, y) <= near))
    return hits.length ? hits[hits.length - 1] : null
  }
  return (
    <section
      ref={section}
      className="motion-second-view"
      style={{ flexBasis: `${width}px` }}
      aria-label={t('views.second')}
      data-testid="motion-second-view"
      data-view={view}
      data-lines={drawn?.lines}
      data-size={drawn ? `${drawn.width} ${drawn.height}` : undefined}
      data-layers={drawn?.layers}
      data-centres={drawn?.centres}
      data-width={width}
    >
      <div
        className="motion-second-divider"
        role="separator"
        aria-orientation="vertical"
        aria-label={t('views.divider')}
        title={t('views.divider')}
        aria-valuenow={width}
        tabIndex={0}
        onPointerDown={(event) => {
          if (event.button !== 0) return
          event.currentTarget.setPointerCapture(event.pointerId)
          drag.current = { pointerId: event.pointerId, x: event.clientX, width }
        }}
        onPointerMove={(event) => {
          const current = drag.current
          if (current?.pointerId === event.pointerId) keep(clampSecondWidth(current.width + current.x - event.clientX, room || window.innerWidth))
        }}
        onPointerUp={(event) => {
          if (drag.current?.pointerId === event.pointerId) drag.current = null
        }}
        onPointerCancel={() => {
          drag.current = null
        }}
        onKeyDown={(event) => {
          if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
          event.preventDefault()
          event.stopPropagation()
          keep(clampSecondWidth(width + (event.key === 'ArrowLeft' ? 16 : -16), room || window.innerWidth))
        }}
        data-testid="motion-second-view-divider"
      />
      <div className="motion-second-head">
        <Select value={view} aria-label={t('views.which')} title={t('views.which')} onChange={(event) => onView(event.target.value)} data-testid="motion-second-view-select">
          {THREE_D_VIEWS.filter(([, , each]) => each !== main).map(([command, label, each]) => (
            <option key={command} value={each}>
              {phrase(label)}
            </option>
          ))}
        </Select>
        <Button variant="ghost" onClick={onMakeMain} title={t('views.makeMainHint')} data-testid="motion-second-view-main">
          {t('views.makeMain')}
        </Button>
        <span style={{ flex: 1 }} />
        <IconButton icon={XIcon} label={t('views.close')} onClick={onClose} data-testid="motion-second-view-close" />
      </div>
      <div className="motion-second-body" ref={box}>
        <canvas
          ref={canvas}
          className="motion-second-canvas"
          title={t('views.clickToSelect')}
          onClick={(event) => {
            const layer = layerAt(event.clientX, event.clientY)
            if (layer) onSelect(layer.id, event.shiftKey)
          }}
          data-testid="motion-second-view-canvas"
        />
        {playing ? (
          <p className="motion-second-note kui-muted" data-testid="motion-second-view-paused">
            {t('views.notDuringPlayback')}
          </p>
        ) : null}
      </div>
    </section>
  )
}
