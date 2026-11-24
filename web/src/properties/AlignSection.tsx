import {
  AlignCenterHorizontalIcon,
  AlignCenterVerticalIcon,
  AlignEndHorizontalIcon,
  AlignEndVerticalIcon,
  AlignHorizontalJustifyCenterIcon,
  AlignStartHorizontalIcon,
  AlignStartVerticalIcon,
  AlignVerticalJustifyCenterIcon,
  AlignVerticalJustifyEndIcon,
  AlignVerticalJustifyStartIcon,
  BetweenHorizontalStartIcon,
  BetweenVerticalStartIcon,
} from '@kition/tessera/icons/react'
import { IconButton, Segmented, useKitEditor } from '@kition/sdk/editor'
import type { AlignTo } from '../menus'
import { Section } from './Section'

type Icon = typeof AlignStartVerticalIcon

/** The Align section's buttons: the `KIT_RUNS` row each runs (menus.ts), its icon and its label (the menu's, translated). */
const ALIGN: ReadonlyArray<readonly [command: string, icon: Icon, label: string]> = [
  ['kit:align-left', AlignStartVerticalIcon, 'Align Left'],
  ['kit:align-hcenter', AlignCenterVerticalIcon, 'Align Horizontal Centers'],
  ['kit:align-right', AlignEndVerticalIcon, 'Align Right'],
  ['kit:align-top', AlignStartHorizontalIcon, 'Align Top'],
  ['kit:align-vcenter', AlignCenterHorizontalIcon, 'Align Vertical Centers'],
  ['kit:align-bottom', AlignEndHorizontalIcon, 'Align Bottom'],
]

const DISTRIBUTE: ReadonlyArray<readonly [command: string, icon: Icon, label: string]> = [
  ['kit:distribute-top', AlignVerticalJustifyStartIcon, 'Distribute Top Edges'],
  ['kit:distribute-vcenter', AlignVerticalJustifyCenterIcon, 'Distribute Vertical Centers'],
  ['kit:distribute-bottom', AlignVerticalJustifyEndIcon, 'Distribute Bottom Edges'],
  ['kit:distribute-left', BetweenVerticalStartIcon, 'Distribute Left Edges'],
  ['kit:distribute-hcenter', AlignHorizontalJustifyCenterIcon, 'Distribute Horizontal Centers'],
  ['kit:distribute-right', BetweenHorizontalStartIcon, 'Distribute Right Edges'],
]

/**
 * The Align panel, at the foot of the Properties panel while layers are selected: what to align
 * to (the composition, or the selected layers' bounds; kept in this browser, as Layer ▸ Align's
 * row), six align buttons and six distribute buttons, each one undo step. A button that would
 * change nothing now (one layer aligned to itself, distributing fewer than three) is disabled.
 */
export function AlignSection({ scope, alignTo, onAlignTo, idle, onCommand, readOnly }: { scope: string; alignTo: AlignTo; onAlignTo: (to: AlignTo) => void; idle: ReadonlySet<string>; onCommand: (command: string) => void; readOnly: boolean }) {
  const { t, phrase } = useKitEditor()
  const button = ([command, icon, label]: readonly [string, Icon, string]) => (
    <IconButton key={command} icon={icon} label={phrase(label)} disabled={readOnly || idle.has(command)} onClick={() => onCommand(command)} data-testid={`motion-align-${command.slice(4)}`} />
  )
  return (
    <Section scope={scope} id="align" title={t('align.title')} testId="motion-section-align">
      <div className="motion-align" data-testid="motion-align-panel">
        <div className="motion-align-row">
          <span className="kui-muted">{t('align.to')}</span>
          <Segmented
            label={t('align.to')}
            value={alignTo}
            disabled={readOnly}
            segments={[
              { value: 'composition', label: t('align.composition') },
              { value: 'selection', label: t('align.selection') },
            ]}
            onChange={(to) => onAlignTo(to === 'selection' ? 'selection' : 'composition')}
            testId="motion-align-to"
          />
        </div>
        <div className="motion-align-row" role="group" aria-label={t('align.layers')}>
          <span className="kui-muted">{t('align.layers')}</span>
          {ALIGN.map(button)}
        </div>
        <div className="motion-align-row" role="group" aria-label={t('align.distribute')}>
          <span className="kui-muted">{t('align.distribute')}</span>
          {DISTRIBUTE.map(button)}
        </div>
      </div>
    </Section>
  )
}
