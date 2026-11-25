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

/**
 * The label the editor gives a new layer it makes, by kind (`setLayerNames`' `labels`): a calm
 * colour each, so the timeline groups the kinds at a glance; footage keeps its project item's
 * label and a precomposition the engine's. Layers already in a document keep theirs.
 */
export const NEW_LAYER_LABELS: Readonly<Record<string, string>> = {
  text: 'Peach',
  shape: 'Blue',
  solid: 'Sea Foam',
  adjustment: 'Green',
  null: 'Brown',
  camera: 'Cyan',
  light: 'Orange',
}

/** The Layer menu's Label rows: `kit:label-<name>` (the name without spaces). */
export const labelRow = (name: string) => `kit:label-${name.replace(/ /g, '')}`

/** A label's colour, or null for None (or a name the editor does not know). */
export function labelHex(name: string | undefined): string | null {
  if (!name || name === 'None') return null
  return LABELS.find(([each]) => each.toLowerCase() === name.toLowerCase())?.[1] ?? null
}

/**
 * A label's row icon: a tick where it is the current label (a blank of the same width elsewhere,
 * so the names line up), then the colour's square; the tick never takes the square's place.
 */
const swatches = new Map<string, ComponentType<{ size?: number }>>()
function swatch(hex: string, current: boolean) {
  const key = `${hex} ${current}`
  let found = swatches.get(key)
  if (!found) {
    found = ({ size = 14 }: { size?: number }) => (
      <span className="motion-label-icon" data-current={current || undefined} aria-hidden="true">
        <span className="motion-label-tick">{current ? '✓' : ''}</span>
        <span className="motion-label-swatch" style={{ width: size - 2, height: size - 2, background: hex }} />
      </span>
    )
    swatches.set(key, found)
  }
  return found
}

/**
 * The menu bar's Layer ▸ Label rows (`kit:label-<name>`, drawn by the SDK without an icon): each
 * colour's square before its name, from a style sheet keyed by the rows' Kit UI ids; the current
 * one is ticked by the menu's own check mark (`checked`).
 */
export const LABEL_ROW_STYLES = LABELS.map(([name, hex]) => `.kui-menu-item[data-kit-ui-id="control:label-${name.replace(/ /g, '')}"] > span:last-of-type::before { background: ${hex}; }`).join('\n')

/** The label every selected layer has, or null (none selected, or several labels). */
export function currentLabel(layers: ReadonlyArray<{ selected: boolean; label: string }>): string | null {
  const chosen = layers.filter((layer) => layer.selected)
  return chosen.length && chosen.every((layer) => layer.label === chosen[0].label) ? chosen[0].label : null
}

/**
 * A Label submenu: each colour with its square (a tick beside the square where `current` is it),
 * then Select Label Group when `onSelectGroup` is given. `onLabel` sets the label (one undo step).
 */
export function LabelMenu({ current, onLabel, onSelectGroup, testId }: { current?: string; onLabel: (name: string) => void; onSelectGroup?: () => void; testId: string }) {
  const { t } = useKitEditor()
  return (
    <SubMenu label={t('label.menu')} path={`${testId}/label`} testId={`${testId}-label`}>
      {LABELS.map(([name, hex]) => (
        <MenuItem key={name} icon={swatch(hex, current === name) as never} label={t(`label.${name.replace(/ /g, '')}`)} onSelect={() => onLabel(name)} testId={`${testId}-label-${name.replace(/ /g, '')}`} />
      ))}
      {onSelectGroup ? <MenuItem label={t('label.selectGroup')} onSelect={onSelectGroup} testId={`${testId}-label-group`} /> : null}
    </SubMenu>
  )
}
