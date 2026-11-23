import { ContextMenu, MenuItem, MenuSeparator, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'
import { INTERPOLATIONS } from './interp'

/**
 * A keyframe's right-click menu, on the selected keyframes (the clicked one is selected first):
 * its interpolation (Linear, Bezier, Auto Bezier, Hold), Easy Ease and its in and out halves, the
 * Keyframe Interpolation and Keyframe Velocity dialogs, Copy and Delete. `onCommand` is the
 * editor's dispatch, which runs the rows the Animation menu has as well.
 */
export function KeyMenu({ at, onClose, onCommand }: { at: ContextMenuPoint | null; onClose: () => void; onCommand: (command: string) => void }) {
  const { run, t, phrase: p, byId } = useKitEditor()
  const shortcut = (command: string) => byId.get(command)?.shortcut ?? undefined
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
      <MenuSeparator />
      <MenuItem label={t('keyMenu.copy')} onSelect={() => void run('keys.copy', {})} testId="motion-key-menu-copy" />
      <MenuItem label={t('keyMenu.delete')} onSelect={() => void run('keys.delete', {})} testId="motion-key-menu-delete" />
    </ContextMenu>
  )
}
