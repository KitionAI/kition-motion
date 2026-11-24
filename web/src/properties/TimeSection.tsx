import { Button, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import type { TreeNode } from '../timeline/tree'
import { PropertyRow, usePanel } from './PropertyTree'
import { Section } from './Section'

/** Layers whose source time can be remapped: footage and compositions. */
export const REMAPPABLE = new Set(['Footage', 'Composition'])

/**
 * A footage or composition layer's time: Enable Time Remapping (keys at its in and out points,
 * the source's time as a value, keyframable like any property), Freeze Frame (holds the frame at
 * the current time), Time-Reverse Layer, and removing the remapping. Each is one undo step
 * (`layer.enableTimeRemap`, `layer.freezeFrame`, `layer.timeReverse`).
 */
export function TimeSection({ layer, remap }: { layer: MotionLayer; remap: TreeNode | undefined }) {
  const { run, t } = useKitEditor()
  const { readOnly } = usePanel()
  if (!REMAPPABLE.has(layer.type)) return null
  const layers = [layer.id]
  return (
    <Section scope={layer.type} id="time" title={t('props.time')} testId="motion-section-time">
      {remap ? <PropertyRow node={remap} label={t('remap.name')} /> : null}
      <div className="motion-section-buttons">
        <Button disabled={readOnly} onClick={() => void run('layer.enableTimeRemap', { layers, value: !remap })} data-testid="motion-remap-toggle">
          {remap ? t('remap.disable') : t('remap.enable')}
        </Button>
        <Button disabled={readOnly} onClick={() => void run('layer.freezeFrame', { layers })} data-testid="motion-freeze-frame">
          {t('remap.freeze')}
        </Button>
        <Button disabled={readOnly} onClick={() => void run('layer.timeReverse', { layers })} data-testid="motion-time-reverse">
          {t('remap.reverse')}
        </Button>
      </div>
    </Section>
  )
}
