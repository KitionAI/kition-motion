import { useState } from 'react'
import { DiamondIcon as Diamond } from '@kition/tessera/icons/react'
import { ContextMenu, MenuItem, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'

/** Whether `times` (composition seconds) has a keyframe at `time`, within half a frame. */
export function keyAt(times: readonly number[], time: number, frame: number) {
  return times.some((at) => Math.abs(at - time) < frame / 2)
}

/**
 * A property's keyframe diamond (the Properties panel's and the timeline's), the stopwatch:
 * hollow when the property is not animated, outlined in the accent colour when it is, filled when
 * it has a keyframe at the current time. A click starts animation on a still property (a keyframe
 * here), adds a keyframe between keyframes, or removes the one here. Alt-click, or 清除动画 in its
 * right-click menu, removes every keyframe (the property keeps its value here). Each is one undo step.
 */
export function KeyDiamond({ layer, path, name, times, time, frame, testId, readOnly }: { layer: number; path: string; name: string; times: readonly number[]; time: number; frame: number; testId: string; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  const [menu, setMenu] = useState<ContextMenuPoint | null>(null)
  const here = keyAt(times, time, frame)
  const animated = times.length > 0
  const label = t(here ? 'removeKey' : 'addKey', { name })
  const clear = () => void run('prop.toggleAnimation', { layer, path, value: false })
  return (
    <>
      <button
        type="button"
        className="motion-key-button"
        data-animated={animated}
        data-key-here={here || undefined}
        aria-label={label}
        title={animated ? `${label}${t('clearAnimation.hint')}` : label}
        disabled={readOnly}
        onClick={(event) => {
          if (event.altKey) {
            if (animated) clear()
            return
          }
          void (here ? run('prop.toggleKey', { layer, path }) : run('prop.addKey', { layer, path, time, timeBase: 'comp' }))
        }}
        onContextMenu={(event) => {
          if (readOnly) return
          event.preventDefault()
          event.stopPropagation()
          setMenu({ x: event.clientX, y: event.clientY })
        }}
        data-testid={testId}
      >
        <Diamond size={12} aria-hidden="true" />
      </button>
      <ContextMenu at={menu} onClose={() => setMenu(null)} label={name} testId={`${testId}-menu`}>
        <MenuItem label={label} onSelect={() => void (here ? run('prop.toggleKey', { layer, path }) : run('prop.addKey', { layer, path, time, timeBase: 'comp' }))} testId={`${testId}-menu-key`} />
        <MenuItem label={t('clearAnimation')} disabled={!animated} onSelect={clear} testId={`${testId}-menu-clear`} />
      </ContextMenu>
    </>
  )
}
