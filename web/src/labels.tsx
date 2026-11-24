import type { ComponentType } from 'react'
import { MenuItem, SubMenu, useKitEditor } from '@kition/sdk/editor'

/**
 * The colour labels a layer, a project item or a keyframe carries (the engine's names, as
 * `edit.label` and `project.setLabel` take them) and their colours (the engine's own).
 */
export const LABELS: ReadonlyArray<readonly [name: string, hex: string]> = [
  ['None', '#505050'],
  ['Red', '#b53838'],
  ['Yellow', '#e4d84c'],
  ['Aqua', '#a9cbc7'],
  ['Pink', '#e5bcc9'],
  ['Lavender', '#a9a9ca'],
  ['Peach', '#e7c19e'],
  ['Sea Foam', '#b3c7b3'],
  ['Blue', '#677de0'],
  ['Green', '#4aa44c'],
  ['Purple', '#8e2c9a'],
  ['Orange', '#e8920d'],
  ['Brown', '#7f452a'],
  ['Fuchsia', '#f46dd6'],
  ['Cyan', '#3da2a5'],
  ['Sandstone', '#a89677'],
  ['Dark Green', '#1e401e'],
]

/** A label's colour, or null for None (or a name the editor does not know). */
export function labelHex(name: string | undefined): string | null {
  if (!name || name === 'None') return null
  return LABELS.find(([each]) => each.toLowerCase() === name.toLowerCase())?.[1] ?? null
}

/** A label's square, as a menu row's icon. */
const swatches = new Map<string, ComponentType<{ size?: number }>>()
function swatch(hex: string) {
  let found = swatches.get(hex)
  if (!found) {
    found = ({ size = 14 }: { size?: number }) => <span className="motion-label-swatch" style={{ width: size - 2, height: size - 2, background: hex }} aria-hidden="true" />
    swatches.set(hex, found)
  }
  return found
}

/**
 * A Label submenu: each colour (checked where `current` is it), then Select Label Group when
 * `onSelectGroup` is given. `onLabel` sets the label (one undo step).
 */
export function LabelMenu({ current, onLabel, onSelectGroup, testId }: { current?: string; onLabel: (name: string) => void; onSelectGroup?: () => void; testId: string }) {
  const { t } = useKitEditor()
  return (
    <SubMenu label={t('label.menu')} path={`${testId}/label`} testId={`${testId}-label`}>
      {LABELS.map(([name, hex]) => (
        <MenuItem key={name} icon={swatch(hex) as never} label={t(`label.${name.replace(/ /g, '')}`)} checked={current === name} onSelect={() => onLabel(name)} testId={`${testId}-label-${name.replace(/ /g, '')}`} />
      ))}
      {onSelectGroup ? <MenuItem label={t('label.selectGroup')} onSelect={onSelectGroup} testId={`${testId}-label-group`} /> : null}
    </SubMenu>
  )
}
