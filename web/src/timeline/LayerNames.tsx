import { useRef, useState, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from 'react'
import { ChevronDownIcon as Open, ChevronRightIcon as Closed } from '@kition/tessera/icons/react'
import { RenameField, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import type { ColumnId } from './columns'
import { useTimeline } from './context'
import { ModeCells, SwitchCells, type SwitchDrag } from './SwitchCells'

/** How far (CSS pixels) a press on a name may wander and still be a click. */
const CLICK_SLOP = 4

/**
 * A layer's cells left of its track: the A/V switches (video, audio, solo, lock), the twirl that
 * shows its properties under it, its number, its name, then the layer switches and the blending
 * mode, track matte and parent drop-downs (the columns shown, `columns.ts`).
 * A click selects the layer (Shift or Cmd adds it); a vertical drag reorders it (`onDrag`, with
 * the pointer's y and whether it ended); a double-click renames it in place (Enter commits, Escape
 * cancels); a right-click opens the layer menu.
 */
export function LayerNames({
  layer,
  open,
  onTwirl,
  renaming,
  onRenameDone,
  onRename,
  onSelect,
  onDrag,
  onMenu,
  shown,
  drag,
  modes,
}: {
  shown: ReadonlySet<ColumnId>
  drag: SwitchDrag
  modes: readonly string[]
  layer: MotionLayer
  open: boolean
  onTwirl: () => void
  renaming: boolean
  onRename: () => void
  onRenameDone: (name: string | null) => void
  onSelect: (add: boolean) => void
  onDrag: (y: number, phase: 'move' | 'end' | 'cancel') => void
  onMenu: (event: ReactMouseEvent<HTMLElement>) => void
}) {
  const { t } = useKitEditor()
  const { readOnly, info } = useTimeline()
  const [text, setText] = useState(layer.name)
  const press = useRef<{ pointerId: number; y: number; moved: boolean } | null>(null)

  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || renaming) return
    event.currentTarget.setPointerCapture(event.pointerId)
    press.current = { pointerId: event.pointerId, y: event.clientY, moved: false }
  }
  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = press.current
    if (!current || current.pointerId !== event.pointerId || readOnly || layer.switches.locked) return
    if (!current.moved && Math.abs(event.clientY - current.y) <= CLICK_SLOP) return
    current.moved = true
    onDrag(event.clientY, 'move')
  }
  const up = (event: ReactPointerEvent<HTMLDivElement>, cancelled: boolean) => {
    const current = press.current
    if (!current || current.pointerId !== event.pointerId) return
    press.current = null
    event.currentTarget.releasePointerCapture(event.pointerId)
    if (current.moved) onDrag(event.clientY, cancelled ? 'cancel' : 'end')
    else if (!cancelled) onSelect(event.shiftKey || event.metaKey || event.ctrlKey)
  }

  return (
    <div
      className="motion-timeline-names motion-row-names"
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={(event) => up(event, false)}
      onPointerCancel={(event) => up(event, true)}
      onDoubleClick={() => {
        if (readOnly) return
        setText(layer.name)
        onRename()
      }}
      onContextMenu={onMenu}
    >
      <SwitchCells layer={layer} side="av" shown={shown} drag={drag} readOnly={readOnly} />
      <button
        type="button"
        className="motion-twirl"
        aria-expanded={open}
        aria-label={t('twirl', { name: layer.name })}
        onPointerDown={(event) => event.stopPropagation()}
        onDoubleClick={(event) => event.stopPropagation()}
        onClick={onTwirl}
        data-testid={`motion-twirl-layer-${layer.id}`}
      >
        {open ? <Open size={12} aria-hidden="true" /> : <Closed size={12} aria-hidden="true" />}
      </button>
      <span className="motion-row-index kui-muted">{layer.index}</span>
      {renaming ? (
        <RenameField value={text} label={t('layerMenu.rename')} onChange={setText} onDone={(commit) => onRenameDone(commit ? text : null)} testId={`motion-rename-${layer.id}`} />
      ) : (
        <span className="motion-row-name" data-testid={`motion-name-${layer.id}`}>
          {layer.name}
        </span>
      )}
      <SwitchCells layer={layer} side="switches" shown={shown} drag={drag} readOnly={readOnly} />
      <ModeCells info={info} layer={layer} shown={shown} modes={modes} readOnly={readOnly} />
    </div>
  )
}
