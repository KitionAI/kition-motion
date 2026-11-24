import { Button, toast, useKitEditor } from '@kition/sdk/editor'
import { timecode } from '../motion'
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
  const { readOnly, time, frame } = usePanel()
  if (!REMAPPABLE.has(layer.type)) return null
  const layers = [layer.id]
  const fps = frame > 0 ? 1 / frame : 30
  // The source time the remapping shows now, in seconds (the field) and as a timecode under it.
  const shown = typeof remap?.value === 'number' ? remap.value : null
  /** Runs a Time button and says what it did (the picture alone may not change at this time). */
  const act = async (command: string, done: string) => {
    if ((await run(command, { layers })) !== null) toast.success(done)
  }
  return (
    <Section scope={layer.type} id="time" title={t('props.time')} testId="motion-section-time">
      <div className="motion-props">
      {remap ? <PropertyRow node={remap} label={t('remap.name')} /> : null}
      {shown !== null ? (
        <span className="kui-muted motion-remap-time" data-testid="motion-remap-time">
          {t('remap.shows', { time: timecode(shown, fps) })}
        </span>
      ) : null}
      <div className="motion-section-buttons">
        <Button disabled={readOnly} onClick={() => void run('layer.enableTimeRemap', { layers, value: !remap })} data-testid="motion-remap-toggle">
          {remap ? t('remap.disable') : t('remap.enable')}
        </Button>
        <Button disabled={readOnly} onClick={() => void act('layer.freezeFrame', t('remap.frozen', { time: timecode(time, fps) }))} data-testid="motion-freeze-frame">
          {t('remap.freeze')}
        </Button>
        <Button disabled={readOnly} onClick={() => void act('layer.timeReverse', t('remap.reversed'))} data-testid="motion-time-reverse">
          {t('remap.reverse')}
        </Button>
      </div>
      </div>
    </Section>
  )
}
