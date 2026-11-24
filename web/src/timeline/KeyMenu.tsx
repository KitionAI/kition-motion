import { useEffect, useState } from 'react'
import { ContextMenu, MenuItem, MenuSeparator, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'
import { INTERPOLATIONS } from './interp'
import { SHORTCUT_OVERRIDES } from '../menus'

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
    void query<Array<{ spatial?: string | null; roving?: boolean; path?: string }>>('keys.info', {}).then(async (list) => {
      if (!current || !Array.isArray(list)) return
      const moving = list.filter((key) => typeof key.spatial === 'string')
      const spatial = moving.length > 0
      const roving = spatial && moving.every((key) => key.roving === true)
      // A first or last keyframe cannot rove: the engine, trying it on a copy, would change nothing.
      const rovable = !spatial || roving ? true : await (session.doc?.invoke<boolean>('wouldChange', 'keys.interpolation', JSON.stringify({ roving: true })) ?? Promise.resolve(true)).catch(() => true)
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
      <MenuItem label={p('Easy Ease')} shortcut={shortcut('keys.easyEase')} onSelect={() => onCommand('keys.easyEase')} testId="motion-key-menu-ease" />
      <MenuItem label={p('Easy Ease In')} shortcut={shortcut('keys.easyEaseIn')} onSelect={() => onCommand('keys.easyEaseIn')} testId="motion-key-menu-ease-in" />
      <MenuItem label={p('Easy Ease Out')} shortcut={shortcut('keys.easyEaseOut')} onSelect={() => onCommand('keys.easyEaseOut')} testId="motion-key-menu-ease-out" />
      <MenuSeparator />
      <MenuItem label={p('Keyframe Interpolation…')} shortcut={shortcut('keys.interpolation')} onSelect={() => onCommand('keys.interpolation')} testId="motion-key-menu-interpolation" />
      <MenuItem label={p('Keyframe Velocity…')} shortcut={shortcut('keys.velocity')} onSelect={() => onCommand('keys.velocity')} testId="motion-key-menu-velocity" />
      <MenuItem label={p('Toggle Hold Keyframe')} shortcut={shortcut('keys.toggleHold')} onSelect={() => onCommand('keys.toggleHold')} testId="motion-key-menu-toggle-hold" />
      {keys.spatial ? (
        // Off on a first or last keyframe, saying why.
        <div className="motion-menu-hinted" title={keys.rovable ? undefined : t('keyMenu.roveEnds')} data-testid="motion-key-menu-rove-row">
          <MenuItem label={p(keys.roving ? 'Rove Across Time (On)' : 'Rove Across Time')} disabled={!keys.rovable} onSelect={() => onCommand('kit:rove-across-time')} testId="motion-key-menu-rove" />
        </div>
      ) : null}
      <MenuSeparator />
      <MenuItem label={p('Time-Reverse Keyframes')} onSelect={() => onCommand('keys.timeReverse')} testId="motion-key-menu-reverse" />
      {keys.scale ? <MenuItem label={p('Exponential Scale')} onSelect={() => onCommand('keys.exponentialScale')} testId="motion-key-menu-exponential" /> : null}
      {keys.count >= 2 ? <MenuItem label={t('keyMenu.wiggler')} onSelect={() => onCommand('kit:wiggler')} testId="motion-key-menu-wiggler" /> : null}
      {keys.count >= 3 ? <MenuItem label={t('keyMenu.smoother')} onSelect={() => onCommand('kit:smoother')} testId="motion-key-menu-smoother" /> : null}
      <MenuSeparator />
      <MenuItem label={p('Select Equal Keyframes')} onSelect={() => onCommand('keys.selectEqual')} testId="motion-key-menu-select-equal" />
      <MenuItem label={p('Select Previous Keyframes')} onSelect={() => onCommand('keys.selectPrevious')} testId="motion-key-menu-select-previous" />
      <MenuItem label={p('Select Following Keyframes')} onSelect={() => onCommand('keys.selectFollowing')} testId="motion-key-menu-select-following" />
      <MenuSeparator />
      {keys.count === 1 ? <MenuItem label={t('keyMenu.edit')} onSelect={() => onCommand('kit:edit-key')} testId="motion-key-menu-edit" /> : null}
      <MenuItem label={t('keyMenu.copy')} onSelect={() => void run('keys.copy', {})} testId="motion-key-menu-copy" />
      <MenuItem label={t('keyMenu.delete')} onSelect={() => void run('keys.delete', {})} testId="motion-key-menu-delete" />
    </ContextMenu>
  )
}
