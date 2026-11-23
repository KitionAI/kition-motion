import { ContextMenu, MenuItem, MenuSeparator, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'

/** The layer a right-click opened the menu on, the layers it acts on, and where it opened. */
export type LayerMenuTarget = { at: ContextMenuPoint; layer: number; layers: number[]; precomp?: boolean }

/**
 * A layer row's right-click menu: Rename, Duplicate, Split at the current time, Pre-compose, and
 * Delete, on the layers it was opened for (the selection when the row was part of it); on a precomp
 * layer, Open Composition first.
 */
export function LayerMenu({ target, onClose, onRename, onOpen, onPrecompose }: { target: LayerMenuTarget | null; onClose: () => void; onRename: (layer: number) => void; onOpen: (layer: number) => void; onPrecompose: () => void }) {
  const { run, t } = useKitEditor()
  const layers = target?.layers ?? []
  return (
    <ContextMenu at={target?.at ?? null} onClose={onClose} label={t('layerMenu')} testId="motion-layer-menu">
      {target?.precomp ? <MenuItem label={t('layerMenu.open')} onSelect={() => onOpen(target.layer)} testId="motion-layer-menu-open" /> : null}
      <MenuItem label={t('layerMenu.rename')} onSelect={() => target && onRename(target.layer)} testId="motion-layer-menu-rename" />
      <MenuItem label={t('layerMenu.duplicate')} onSelect={() => void run('edit.duplicate', { layers })} testId="motion-layer-menu-duplicate" />
      <MenuItem label={t('layerMenu.split')} onSelect={() => void run('edit.splitLayer', { layers })} testId="motion-layer-menu-split" />
      <MenuItem label={t('layerMenu.precompose')} onSelect={() => void run('layer.select', { layers }).then(() => onPrecompose())} testId="motion-layer-menu-precompose" />
      <MenuSeparator />
      <MenuItem label={t('layerMenu.delete')} onSelect={() => void run('edit.clear', { layers })} testId="motion-layer-menu-delete" />
    </ContextMenu>
  )
}
