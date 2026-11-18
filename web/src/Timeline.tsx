import { useRef, type PointerEvent as ReactPointerEvent } from 'react'
import { useKitEditor } from '@kition/kit-editor'
import type { MotionInfo } from './motion'

/** Where a pointer is along the time area, in seconds within the composition. */
function timeAt(event: { clientX: number }, area: HTMLElement, duration: number) {
  const box = area.getBoundingClientRect()
  return Math.min(duration, Math.max(0, ((event.clientX - box.left) / Math.max(1, box.width)) * duration))
}

/**
 * The composition's layers over time, as in After Effects: one row per layer with its bar from
 * in to out point and its keyframes; the ruler scrubs the current time, and a click on a row
 * selects its layer.
 */
export function Timeline({ info, onSeek, onSelect }: { info: MotionInfo; onSeek: (seconds: number) => void; onSelect: (layer: number, add: boolean) => void }) {
  const { t } = useKitEditor()
  const area = useRef<HTMLDivElement>(null)
  const duration = Math.max(info.comp?.duration ?? 0, info.frame)
  const percent = (seconds: number) => `${(Math.min(duration, Math.max(0, seconds)) / duration) * 100}%`
  const seconds = Math.floor(duration)
  const step = seconds > 30 ? 5 : seconds > 10 ? 2 : 1
  const scrub = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!area.current) return
    event.currentTarget.setPointerCapture(event.pointerId)
    onSeek(timeAt(event, area.current, duration))
  }
  return (
    <div className="motion-timeline" data-testid="motion-timeline">
      <div className="motion-timeline-head">
        <div className="motion-timeline-names kui-muted">{t('layers')}</div>
        <div
          ref={area}
          className="motion-ruler"
          onPointerDown={scrub}
          onPointerMove={(event) => {
            if (event.buttons && area.current) onSeek(timeAt(event, area.current, duration))
          }}
          data-testid="motion-ruler"
        >
          {Array.from({ length: Math.floor(seconds / step) + 1 }, (_, index) => index * step).map((mark) => (
            <span key={mark} className="motion-ruler-mark" style={{ left: percent(mark) }}>
              {mark}s
            </span>
          ))}
          <span className="motion-playhead" style={{ left: percent(info.time) }} aria-hidden="true" />
        </div>
      </div>
      <div className="motion-timeline-rows">
        {info.layers.length ? (
          info.layers.map((layer) => (
            <div key={layer.id} className="motion-row" data-selected={layer.selected} onClick={(event) => onSelect(layer.id, event.shiftKey || event.metaKey)} data-testid={`motion-layer-${layer.id}`}>
              <div className="motion-timeline-names">
                <span className="motion-row-index kui-muted">{layer.index}</span>
                <span className="motion-row-name">{layer.name}</span>
              </div>
              <div className="motion-row-track">
                <div className="motion-bar" style={{ left: percent(layer.in), right: `calc(100% - ${percent(layer.out)})` }} />
                {layer.animated.flatMap((property) =>
                  property.times.map((time) => (
                    <span key={`${property.path}-${time}`} className="motion-key" style={{ left: percent(time) }} title={`${property.name} ${time.toFixed(2)}s`} data-testid="motion-key" />
                  )),
                )}
                <span className="motion-playhead" style={{ left: percent(info.time) }} aria-hidden="true" />
              </div>
            </div>
          ))
        ) : (
          <p className="kui-muted motion-empty">{t('noLayers')}</p>
        )}
      </div>
    </div>
  )
}
