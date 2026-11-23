import { useEffect, useMemo, useState } from 'react'
import { useKitEditor, useReadOnly } from '@kition/sdk/editor'
import type { MotionInfo } from '../motion'
import { TimelineContext, type TimelineFrame } from './context'
import { LayerBar } from './LayerBar'
import { Ruler } from './Ruler'
import { leftOf, type TimeView } from './view'

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
 * The composition's layers over time, as in After Effects: one row per layer with its bar from
 * in to out point and its keyframes; the ruler scrubs the current time. A bar drags the layer in
 * time and its ends trim it; a click on a row's name selects its layer.
 */
export function Timeline({ info, onSeek, onSelect }: { info: MotionInfo; onSeek: (seconds: number) => void; onSelect: (layer: number, add: boolean) => void }) {
  const { t } = useKitEditor()
  const readOnly = useReadOnly()
  const duration = Math.max(info.comp?.duration ?? 0, info.frame)
  const view = useMemo<TimeView>(() => ({ start: 0, span: duration }), [duration])
  const [width, rulerRef] = useWidth()
  const [shift, setShift] = useState<{ layers: number[]; delta: number } | null>(null)
  const frame = useMemo<TimelineFrame>(() => ({ info, view, width, readOnly }), [info, view, width, readOnly])
  return (
    <TimelineContext.Provider value={frame}>
      <div className="motion-timeline" data-testid="motion-timeline">
        <div className="motion-timeline-rows">
          <div className="motion-timeline-head">
            <div className="motion-timeline-names kui-muted">{t('layers')}</div>
            <Ruler onSeek={onSeek} rulerRef={rulerRef} />
          </div>
          {info.layers.length ? (
            info.layers.map((layer) => (
              <div key={layer.id} className="motion-row" data-selected={layer.selected} data-testid={`motion-layer-${layer.id}`}>
                <div className="motion-timeline-names" onClick={(event) => onSelect(layer.id, event.shiftKey || event.metaKey || event.ctrlKey)}>
                  <span className="motion-row-index kui-muted">{layer.index}</span>
                  <span className="motion-row-name" data-testid={`motion-name-${layer.id}`}>
                    {layer.name}
                  </span>
                </div>
                <div className="motion-row-track">
                  <LayerBar layer={layer} shift={shift} onShift={setShift} />
                  {layer.animated.flatMap((property) =>
                    property.times.map((time) => (
                      <span key={`${property.path}-${time}`} className="motion-key" style={{ left: leftOf(view, time) }} title={`${property.name} ${time.toFixed(2)}s`} data-testid="motion-key" />
                    )),
                  )}
                  <span className="motion-playhead" style={{ left: leftOf(view, info.time) }} aria-hidden="true" />
                </div>
              </div>
            ))
          ) : (
            <p className="kui-muted motion-empty">{t('noLayers')}</p>
          )}
        </div>
      </div>
    </TimelineContext.Provider>
  )
}
