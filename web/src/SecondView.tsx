import { useEffect, useRef, useState } from 'react'
import { XIcon } from '@kition/tessera/icons/react'
import { Button, IconButton, Select, useKitEditor } from '@kition/sdk/editor'
import { THREE_D_VIEWS } from './menus'

type Engine = { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> }

/**
 * The two-view layout's second view (View ▸ Two Views), beside the composition viewer: the
 * composition seen through another 3D view (Top by default), with that view's ground, 3D layers'
 * outlines, cameras and lights, drawn again a moment after each change (the engine's `viewFrame`).
 * Its drop-down picks the view; Make Main shows it in the viewer and puts the viewer's view here.
 */
export function SecondView({ doc, view, onView, main, onMakeMain, onClose, version, playing, compWidth }: { doc: Engine | null; view: string; onView: (view: string) => void; main: string; onMakeMain: () => void; onClose: () => void; version: unknown; playing: boolean; compWidth: number }) {
  const { t, phrase } = useKitEditor()
  const box = useRef<HTMLDivElement>(null)
  const canvas = useRef<HTMLCanvasElement>(null)
  const [width, setWidth] = useState(0)
  const [drawn, setDrawn] = useState<{ lines: number; width: number; height: number } | null>(null)
  useEffect(() => {
    const element = box.current
    if (!element) return undefined
    const observer = new ResizeObserver(() => setWidth(Math.floor(element.getBoundingClientRect().width)))
    observer.observe(element)
    setWidth(Math.floor(element.getBoundingClientRect().width))
    return () => observer.disconnect()
  }, [])
  useEffect(() => {
    if (!doc || width < 16) return undefined
    let current = true
    // While it plays, the second view keeps its last picture: the engine's time goes to the viewer.
    if (playing) return undefined
    const timer = window.setTimeout(async () => {
      const answer = await doc.invoke<Uint8Array>('viewFrame', view, Math.round(width * Math.min(2, window.devicePixelRatio || 1))).catch(() => null)
      const element = canvas.current
      if (!current || !answer || !element || answer.length < 4) return
      const size = new DataView(answer.buffer, answer.byteOffset, 4).getUint32(0, true)
      const head = JSON.parse(new TextDecoder().decode(answer.subarray(4, 4 + size))) as { width: number; height: number; lines: Array<[number, number, number, number, number, number, number, number, boolean]> }
      const pixels = answer.subarray(4 + size)
      if (pixels.length < head.width * head.height * 4) return
      element.width = head.width
      element.height = head.height
      const context = element.getContext('2d')
      if (!context) return
      context.putImageData(new ImageData(new Uint8ClampedArray(pixels.slice(0, head.width * head.height * 4)), head.width, head.height), 0, 0)
      // The lines are in composition pixels: drawn at the picture's scale.
      const comp = compWidth || head.width
      const scale = head.width / comp
      context.lineWidth = 1
      for (const [ax, ay, bx, by, r, g, b, a, dashed] of head.lines) {
        context.strokeStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`
        context.setLineDash(dashed ? [4, 3] : [])
        context.beginPath()
        context.moveTo(ax * scale, ay * scale)
        context.lineTo(bx * scale, by * scale)
        context.stroke()
      }
      setDrawn({ lines: head.lines.length, width: head.width, height: head.height })
    }, 150)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [doc, view, width, version, playing, compWidth])
  return (
    <section className="motion-second-view" aria-label={t('views.second')} data-testid="motion-second-view" data-view={view} data-lines={drawn?.lines} data-size={drawn ? `${drawn.width} ${drawn.height}` : undefined}>
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
        <canvas ref={canvas} className="motion-second-canvas" data-testid="motion-second-view-canvas" />
      </div>
    </section>
  )
}
