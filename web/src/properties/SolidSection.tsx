import { useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import { NumberField } from '../ValueField'
import { ColorSwatch, hexOf } from './ColorSwatch'
import { usePanel } from './PropertyTree'
import { Section } from './Section'

type Solid = { item?: { color?: number[]; width?: number; height?: number } }

/** A solid layer's own section: its colour and size (the solid's, as Solid Settings sets them; one undo step each). */
export function SolidSection() {
  const { run, t } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const settings = useEngineQuery<{ solid?: number }>('layer.settings', { layer: layer.id })
  const solid = useEngineQuery<Solid>('item.metadata', { item: settings?.solid ?? -1 })
  const item = solid?.item
  if (settings?.solid == null || !item) return null
  const set = (params: Record<string, unknown>) => void run('layer.settings', { layer: layer.id, ...params })
  return (
    <Section scope={layer.type} id="solid" title={t('solid.title')} testId="motion-section-solid">
      <div className="motion-props">
        <div className="motion-prop">
          <span className="motion-key-spacer" />
          <span className="motion-prop-name kui-muted">{t('dialog.color')}</span>
          <span className="motion-prop-values">
            <ColorSwatch value={hexOf(item.color)} label={t('dialog.color')} readOnly={readOnly} onChange={(hex, final) => final && set({ color: hex })} testId="motion-panel-solid-color" />
          </span>
        </div>
        <div className="motion-prop">
          <span className="motion-key-spacer" />
          <span className="motion-prop-name kui-muted">{t('shape.size')}</span>
          <span className="motion-prop-values">
            <NumberField value={item.width ?? 0} label={t('dialog.width')} min={1} max={30000} decimals={0} readOnly={readOnly} onCommit={(width) => set({ width: Math.round(width) })} testId="motion-panel-solid-width" />
            <NumberField value={item.height ?? 0} label={t('dialog.height')} min={1} max={30000} decimals={0} readOnly={readOnly} onCommit={(height) => set({ height: Math.round(height) })} testId="motion-panel-solid-height" />
          </span>
        </div>
      </div>
    </Section>
  )
}
