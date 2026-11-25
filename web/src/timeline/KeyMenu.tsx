import { useEffect, useState } from 'react'
import { ContextMenu, MenuItem, MenuSeparator, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'
import { INTERPOLATIONS } from './interp'
import { ASSISTANT_NEEDS, SHORTCUT_OVERRIDES } from '../menus'

/**
 * A keyframe's right-click menu, on the selected keyframes (the clicked one is selected first):
 * its interpolation (Linear, Bezier, Auto Bezier, Hold), Easy Ease and its in and out halves, the
 * Keyframe Interpolation and Keyframe Velocity dialogs, Toggle Hold, Rove Across Time (for
 * keyframes moving through space, saying when it is on), Time-Reverse Keyframes, Exponential Scale
 * (two Scale keyframes), Select Previous, Following and Equal Keyframes, Edit Value… (one
 * keyframe), Copy and Delete. `onCommand` is the
 * editor's dispatch, which runs the rows the Animation menu has as well.
 */
export function KeyMenu({ at, onClose, onCommand }: { at: ContextMenuPoint | null; onClose: () => void; onCommand: (command: string) => void }) {
  const { run, query, t, phrase: p, byId, session } = useKitEditor()
  // What the selected keyframes are: how many, whether any moves through space (and all rove), whether two are Scale's.
  const [keys, setKeys] = useState({ count: 0, spatial: false, roving: false, scale: false, rovable: true })
  useEffect(() => {
    if (!at) return undefined
    let current = true
    void query<Array<{ spatial?: string | null; roving?: boolean; path?: string }>>('keyframe.info', {}).then(async (list) => {
      if (!current || !Array.isArray(list)) return
      const moving = list.filter((key) => typeof key.spatial === 'string')
      const spatial = moving.length > 0
      const roving = spatial && moving.every((key) => key.roving === true)
      // A first or last keyframe cannot rove: the engine, trying it on a copy, would change nothing.
      const rovable = !spatial || roving ? true : await (session.doc?.invoke<boolean>('wouldChange', 'keyframe.interpolation', JSON.stringify({ roving: true })) ?? Promise.resolve(true)).catch(() => true)
      if (!current) return
      setKeys({ count: list.length, spatial, roving, rovable, scale: list.filter((key) => key.path === 'transform/scale').length >= 2 })
    })
    return () => {
      current = false
    }
  }, [at, query])
  const shortcut = (command: string) => SHORTCUT_OVERRIDES[command] ?? byId.get(command)?.shortcut ?? undefined
  return (
    <ContextMenu at={at} onClose={onClose} label={t('keyMenu')} testId="motion-key-menu">
      {INTERPOLATIONS.map((row) => (
        <MenuItem key={row.command} label={p(row.label)} onSelect={() => onCommand(row.command)} testId={`motion-key-menu-${row.interpolation}`} />
      ))}
      <MenuSeparator />
      <MenuItem label={p('Easy Ease')} shortcut={shortcut('keyframe.easyEase')} onSelect={() => onCommand('keyframe.easyEase')} testId="motion-key-menu-ease" />
      <MenuItem label={p('Easy Ease In')} shortcut={shortcut('keyframe.easyEaseIn')} onSelect={() => onCommand('keyframe.easyEaseIn')} testId="motion-key-menu-ease-in" />
      <MenuItem label={p('Easy Ease Out')} shortcut={shortcut('keyframe.easyEaseOut')} onSelect={() => onCommand('keyframe.easyEaseOut')} testId="motion-key-menu-ease-out" />
      <MenuSeparator />
      <MenuItem label={p('Keyframe Interpolation…')} shortcut={shortcut('keyframe.interpolation')} onSelect={() => onCommand('keyframe.interpolation')} testId="motion-key-menu-interpolation" />
      <MenuItem label={p('Keyframe Velocity…')} shortcut={shortcut('keyframe.velocity')} onSelect={() => onCommand('keyframe.velocity')} testId="motion-key-menu-velocity" />
      <MenuItem label={p('Toggle Hold Keyframe')} shortcut={shortcut('keyframe.toggleHold')} onSelect={() => onCommand('keyframe.toggleHold')} testId="motion-key-menu-toggle-hold" />
      {keys.spatial ? (
        // Off on a first or last keyframe, saying why.
        <div className="motion-menu-hinted" title={keys.rovable ? undefined : t('keyMenu.roveEnds')} data-testid="motion-key-menu-rove-row">
          <MenuItem label={p(keys.roving ? 'Rove Across Time (On)' : keys.rovable ? 'Rove Across Time' : 'Rove Across Time (Not on the First or Last Keyframe)')} disabled={!keys.rovable} onSelect={() => onCommand('kit:rove-across-time')} testId="motion-key-menu-rove" />
        </div>
      ) : null}
      <MenuSeparator />
      <MenuItem label={p('Time-Reverse Keyframes')} onSelect={() => onCommand('keyframe.timeReverse')} testId="motion-key-menu-reverse" />
      {keys.scale ? <MenuItem label={p('Exponential Scale')} onSelect={() => onCommand('keyframe.exponentialScale')} testId="motion-key-menu-exponential" /> : null}
      {/* Off (saying what they wait for) until enough keyframes are selected. */}
      <MenuItem label={keys.count >= 2 ? t('keyMenu.wiggler') : p(ASSISTANT_NEEDS.wiggler)} disabled={keys.count < 2} onSelect={() => onCommand('kit:wiggler')} testId="motion-key-menu-wiggler" />
      <MenuItem label={keys.count >= 3 ? t('keyMenu.smoother') : p(ASSISTANT_NEEDS.smoother)} disabled={keys.count < 3} onSelect={() => onCommand('kit:smoother')} testId="motion-key-menu-smoother" />
      <MenuSeparator />
      <MenuItem label={p('Select Equal Keyframes')} onSelect={() => onCommand('keyframe.selectEqual')} testId="motion-key-menu-select-equal" />
      <MenuItem label={p('Select Previous Keyframes')} onSelect={() => onCommand('keyframe.selectPrevious')} testId="motion-key-menu-select-previous" />
      <MenuItem label={p('Select Following Keyframes')} onSelect={() => onCommand('keyframe.selectFollowing')} testId="motion-key-menu-select-following" />
      <MenuSeparator />
      {keys.count === 1 ? <MenuItem label={t('keyMenu.edit')} onSelect={() => onCommand('kit:edit-key')} testId="motion-key-menu-edit" /> : null}
      <MenuItem label={t('keyMenu.copy')} onSelect={() => void run('keyframe.copy', {})} testId="motion-key-menu-copy" />
      <MenuItem label={t('keyMenu.delete')} onSelect={() => void run('keyframe.delete', {})} testId="motion-key-menu-delete" />
    </ContextMenu>
  )
}
