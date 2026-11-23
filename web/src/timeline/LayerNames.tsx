import { useRef, useState, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from 'react'
import { CircleDotIcon as Solo, LockIcon as Lock, LockOpenIcon as Unlock } from '@kition/tessera/icons/react'
import { EyeButton, IconButton, RenameField, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { useTimeline } from './context'

/** How far (CSS pixels) a press on a name may wander and still be a click. */
const CLICK_SLOP = 4

/**
 * A layer's cells left of its track: the eye, solo and lock switches, its number, and its name.
 * A click selects the layer (Shift or Cmd adds it); a vertical drag reorders it (`onDrag`, with
 * the pointer's y and whether it ended); a double-click renames it in place (Enter commits, Escape
 * cancels); a right-click opens the layer menu.
 */
export function LayerNames({
  layer,
  renaming,
  onRenameDone,
  onRename,
  onSelect,
  onDrag,
  onMenu,
}: {
  layer: MotionLayer
  renaming: boolean
  onRename: () => void
  onRenameDone: (name: string | null) => void
  onSelect: (add: boolean) => void
  onDrag: (y: number, phase: 'move' | 'end' | 'cancel') => void
  onMenu: (event: ReactMouseEvent<HTMLElement>) => void
}) {
  const { run, t } = useKitEditor()
  const { readOnly } = useTimeline()
  const [text, setText] = useState(layer.name)
  const press = useRef<{ pointerId: number; y: number; moved: boolean } | null>(null)
  const toggle = (name: 'video' | 'solo' | 'lock') => void run('layer.setSwitch', { layers: [layer.id], switch: name })

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
      <span className="motion-switches" onPointerDown={(event) => event.stopPropagation()} onDoubleClick={(event) => event.stopPropagation()}>
        <EyeButton visible={layer.switches.video} label={t('switch.video')} onPress={() => !readOnly && toggle('video')} testId={`motion-switch-video-${layer.id}`} />
        <IconButton
          icon={Solo}
          label={t('switch.solo')}
          pressed={layer.switches.solo}
          disabled={readOnly}
          className="motion-switch"
          onClick={() => toggle('solo')}
          data-testid={`motion-switch-solo-${layer.id}`}
        />
        <IconButton
          icon={layer.switches.locked ? Lock : Unlock}
          label={t('switch.lock')}
          pressed={layer.switches.locked}
          disabled={readOnly}
          className="motion-switch"
          onClick={() => toggle('lock')}
          data-testid={`motion-switch-lock-${layer.id}`}
        />
      </span>
      <span className="motion-row-index kui-muted">{layer.index}</span>
      {renaming ? (
        <RenameField value={text} label={t('layerMenu.rename')} onChange={setText} onDone={(commit) => onRenameDone(commit ? text : null)} testId={`motion-rename-${layer.id}`} />
      ) : (
        <span className="motion-row-name" data-testid={`motion-name-${layer.id}`}>
          {layer.name}
        </span>
      )}
    </div>
  )
}
