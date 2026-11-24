import { useRef, type PointerEvent as ReactPointerEvent } from 'react'
import {
  BoxIcon,
  CircleDotIcon,
  ContrastIcon,
  EyeIcon,
  EyeOffIcon,
  FilmIcon,
  FoldHorizontalIcon,
  GaugeIcon,
  GlassesIcon,
  LockIcon,
  LockOpenIcon,
  Volume2Icon,
  WandSparklesIcon,
  WindIcon,
} from '@kition/tessera/icons/react'
import { Select, useKitEditor } from '@kition/sdk/editor'
import type { MotionInfo, MotionLayer, MotionMatte } from '../motion'
import { SWITCH_COLUMNS, type ColumnId, type SwitchColumn, type SwitchId } from './columns'

type Icon = typeof EyeIcon

/** Each switch's icon (on, and off where it differs). */
const ICONS: Record<SwitchId, readonly [on: Icon, off?: Icon]> = {
  video: [EyeIcon, EyeOffIcon],
  audio: [Volume2Icon],
  solo: [CircleDotIcon],
  lock: [LockIcon, LockOpenIcon],
  shy: [GlassesIcon],
  collapse: [FoldHorizontalIcon],
  quality: [GaugeIcon],
  effects: [WandSparklesIcon],
  frameBlend: [FilmIcon],
  motionBlur: [WindIcon],
  adjustment: [ContrastIcon],
  threeD: [BoxIcon],
}

/** The composition switch a column header turns on and off (`comp.setSwitch`). */
const COMP_SWITCHES: Partial<Record<SwitchId, { name: 'hideShy' | 'motionBlur' | 'frameBlending'; field: 'hideShy' | 'motionBlur' | 'frameBlending' }>> = {
  shy: { name: 'hideShy', field: 'hideShy' },
  motionBlur: { name: 'motionBlur', field: 'motionBlur' },
  frameBlend: { name: 'frameBlending', field: 'frameBlending' },
}

/** The track matte kinds `layer.setTrackMatte` takes, in its menu's order. */
const MATTE_KINDS: readonly MotionMatte['kind'][] = ['alpha', 'alphaInverted', 'luma', 'lumaInverted']

/** Where a press on a switch started a drag down the column: every layer it passes takes the same value, one undo step. */
export type SwitchDrag = { start: (layer: number, column: SwitchColumn, value: boolean) => void }

/**
 * A drag down a switch column: the press sets the pressed layer's switch, and each layer the
 * pointer then passes over in that column gets the same value, all one undo step (`merge`).
 */
export function useSwitchDrag(): SwitchDrag {
  const { run } = useKitEditor()
  const serial = useRef(0)
  return {
    start: (layer, column, value) => {
      serial.current += 1
      const merge = `timeline-switch-${column.id}-${Date.now()}-${serial.current}`
      const done = new Set<number>([layer])
      const set = (id: number) => void run('layer.setSwitch', { layers: [id], switch: column.name, value, merge })
      set(layer)
      const move = (event: PointerEvent) => {
        const cell = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-switch-column]')
        const id = Number(cell?.dataset.layer)
        if (!cell || cell.dataset.switchColumn !== column.id || !Number.isFinite(id) || done.has(id)) return
        done.add(id)
        set(id)
      }
      const end = () => {
        window.removeEventListener('pointermove', move, true)
        window.removeEventListener('pointerup', end, true)
        window.removeEventListener('pointercancel', end, true)
      }
      window.addEventListener('pointermove', move, true)
      window.addEventListener('pointerup', end, true)
      window.addEventListener('pointercancel', end, true)
    },
  }
}

/** One layer's switch in one column: a press toggles it (and starts a drag down the column); Enter or Space toggles it from the keyboard. */
function SwitchCell({ layer, column, drag, readOnly }: { layer: MotionLayer; column: SwitchColumn; drag: SwitchDrag; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  const on = layer.switches[column.field]
  const [iconOn, iconOff] = ICONS[column.id]
  const Shown = on ? iconOn : (iconOff ?? iconOn)
  const label = t(`switch.${column.id}`)
  return (
    <button
      type="button"
      className="motion-switch"
      aria-pressed={on}
      aria-label={label}
      title={label}
      disabled={readOnly}
      data-switch-column={column.id}
      data-layer={layer.id}
      data-off-dim={iconOff ? undefined : true}
      onPointerDown={(event: ReactPointerEvent<HTMLButtonElement>) => {
        event.stopPropagation()
        if (event.button !== 0 || readOnly) return
        event.preventDefault()
        drag.start(layer.id, column, !on)
      }}
      onClick={(event) => {
        // A press already toggled it; a click from the keyboard (no pointer) toggles it here.
        if (event.detail !== 0 || readOnly) return
        void run('layer.setSwitch', { layers: [layer.id], switch: column.name, value: !on })
      }}
      onDoubleClick={(event) => event.stopPropagation()}
      data-testid={`motion-switch-${column.id}-${layer.id}`}
    >
      <Shown size={13} aria-hidden="true" />
    </button>
  )
}

/** The switch cells of one side (A/V left of the name, the layer switches right of it). */
export function SwitchCells({ layer, side, shown, drag, readOnly }: { layer: MotionLayer; side: SwitchColumn['side']; shown: ReadonlySet<ColumnId>; drag: SwitchDrag; readOnly: boolean }) {
  const columns = SWITCH_COLUMNS.filter((column) => column.side === side && shown.has(column.id))
  if (!columns.length) return null
  return (
    <span className="motion-switches" data-side={side} onPointerDown={(event) => event.stopPropagation()} onDoubleClick={(event) => event.stopPropagation()}>
      {columns.map((column) => (
        <SwitchCell key={column.id} layer={layer} column={column} drag={drag} readOnly={readOnly} />
      ))}
    </span>
  )
}

/** The blending mode, track matte and parent drop-downs of one layer, each change one undo step. */
export function ModeCells({ info, layer, shown, modes, readOnly }: { info: MotionInfo; layer: MotionLayer; shown: ReadonlySet<ColumnId>; modes: readonly string[]; readOnly: boolean }) {
  const { run, t, phrase } = useKitEditor()
  if (!shown.has('mode') && !shown.has('matte') && !shown.has('parent')) return null
  const others = info.layers.filter((other) => other.id !== layer.id)
  const matte = layer.matte ? `${layer.matte.kind}:${layer.matte.layer}` : ''
  const matteLabel = (kind: MotionMatte['kind'], other: MotionLayer) => t('matte.option', { kind: t(`matte.${kind}`), index: other.index, name: other.name })
  return (
    <span className="motion-mode-cells" onPointerDown={(event) => event.stopPropagation()} onDoubleClick={(event) => event.stopPropagation()}>
      {shown.has('mode') ? (
        <Select
          className="motion-cell-select"
          value={layer.blendMode}
          disabled={readOnly}
          aria-label={t('column.mode')}
          title={`${t('column.mode')}: ${phrase(layer.blendMode)}`}
          onChange={(event) => void run('layer.setBlendMode', { layers: [layer.id], mode: event.target.value })}
          data-testid={`motion-cell-mode-${layer.id}`}
        >
          {modes.map((mode) => (
            <option key={mode} value={mode}>
              {phrase(mode)}
            </option>
          ))}
        </Select>
      ) : null}
      {shown.has('matte') ? (
        <Select
          className="motion-cell-select"
          value={matte}
          disabled={readOnly}
          aria-label={t('column.matte')}
          title={layer.matte ? matteLabel(layer.matte.kind, others.find((other) => other.id === layer.matte?.layer) ?? layer) : t('matte.none')}
          onChange={(event) => {
            const [kind, id] = event.target.value.split(':')
            void run('layer.setTrackMatte', id ? { layer: layer.id, matte: Number(id), kind } : { layer: layer.id, matte: null })
          }}
          data-testid={`motion-cell-matte-${layer.id}`}
        >
          <option value="">{t('matte.none')}</option>
          {MATTE_KINDS.map((kind) => (
            <optgroup key={kind} label={t(`matte.${kind}`)}>
              {others.map((other) => (
                <option key={other.id} value={`${kind}:${other.id}`}>
                  {matteLabel(kind, other)}
                </option>
              ))}
            </optgroup>
          ))}
        </Select>
      ) : null}
      {shown.has('parent') ? (
        <Select
          className="motion-cell-select"
          value={layer.parent == null ? '' : String(layer.parent)}
          disabled={readOnly}
          aria-label={t('props.parent')}
          title={t('props.parent')}
          onChange={(event) => void run('layer.setParent', { layers: [layer.id], parent: event.target.value === '' ? null : Number(event.target.value) })}
          data-testid={`motion-cell-parent-${layer.id}`}
        >
          <option value="">{t('props.noParent')}</option>
          {others.map((other) => (
            <option key={other.id} value={String(other.id)}>
              {other.index}. {other.name}
            </option>
          ))}
        </Select>
      ) : null}
    </span>
  )
}

/**
 * The column headers of one side: a switch column's icon (the Shy, Motion Blur and Frame Blend
 * headers turn the composition's switch on and off: Hide Shy Layers, and whether motion blur and
 * frame blending render), and the drop-down columns' names.
 */
export function ColumnHeads({ info, side, shown, readOnly }: { info: MotionInfo; side: SwitchColumn['side'] | 'modes'; shown: ReadonlySet<ColumnId>; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  if (side === 'modes') {
    const names = (['mode', 'matte', 'parent'] as const).filter((id) => shown.has(id))
    if (!names.length) return null
    return (
      <span className="motion-mode-cells motion-column-heads">
        {names.map((id) => (
          <span key={id} className="motion-cell-head kui-muted">
            {t(id === 'parent' ? 'props.parent' : `column.${id}`)}
          </span>
        ))}
      </span>
    )
  }
  const columns = SWITCH_COLUMNS.filter((column) => column.side === side && shown.has(column.id))
  if (!columns.length) return null
  return (
    <span className="motion-switches motion-column-heads" data-side={side}>
      {columns.map((column) => {
        const comp = COMP_SWITCHES[column.id]
        const Shown = ICONS[column.id][0]
        if (comp && info.comp) {
          const on = info.comp.switches[comp.field]
          const label = t(`compSwitch.${comp.name}`)
          return (
            <button
              key={column.id}
              type="button"
              className="motion-switch motion-comp-switch"
              aria-pressed={on}
              aria-label={label}
              title={label}
              disabled={readOnly}
              onClick={() => void run('comp.setSwitch', { switch: comp.name, value: !on })}
              data-testid={`motion-comp-switch-${comp.name}`}
            >
              <Shown size={13} aria-hidden="true" />
            </button>
          )
        }
        return (
          <span key={column.id} className="motion-switch motion-cell-head kui-muted" title={t(`switch.${column.id}`)} aria-hidden="true">
            <Shown size={13} aria-hidden="true" />
          </span>
        )
      })}
    </span>
  )
}
