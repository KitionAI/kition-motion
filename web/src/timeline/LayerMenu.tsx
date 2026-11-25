import { ContextMenu, MenuItem, MenuSeparator, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'
import { LabelMenu } from '../labels'

/** The layer a right-click opened the menu on, the layers it acts on, and where it opened. */
export type LayerMenuTarget = { at: ContextMenuPoint; layer: number; layers: number[]; precomp?: boolean; source?: boolean; /** The colour label every layer it acts on has (none when they differ). */ label?: string }

/** The layer types made from a project item (footage, a composition, a solid): Reveal Layer Source in Project finds it. */
export const HAS_SOURCE: ReadonlySet<string> = new Set(['Footage', 'Composition', 'Solid'])

/**
 * A layer row's right-click menu: Rename, Duplicate, Split at the current time, Pre-compose, and
 * Delete, on the layers it was opened for (the selection when the row was part of it); on a precomp
 * layer, Open Composition first; on a layer made from a project item, Reveal Layer Source in Project.
 */
export function LayerMenu({ target, onClose, onRename, onOpen, onPrecompose, onReveal }: { target: LayerMenuTarget | null; onClose: () => void; onRename: (layer: number) => void; onOpen: (layer: number) => void; onPrecompose: () => void; onReveal: (layer: number) => void }) {
  const { run, t } = useKitEditor()
  const layers = target?.layers ?? []
  return (
    <ContextMenu at={target?.at ?? null} onClose={onClose} label={t('layerMenu')} testId="motion-layer-menu">
      {target?.precomp ? <MenuItem label={t('layerMenu.open')} onSelect={() => onOpen(target.layer)} testId="motion-layer-menu-open" /> : null}
      <MenuItem label={t('layerMenu.rename')} onSelect={() => target && onRename(target.layer)} testId="motion-layer-menu-rename" />
      <MenuItem label={t('layerMenu.duplicate')} onSelect={() => void run('edit.duplicate', { layers })} testId="motion-layer-menu-duplicate" />
      <MenuItem label={t('layerMenu.split')} onSelect={() => void run('edit.splitLayer', { layers })} testId="motion-layer-menu-split" />
      <MenuItem label={t('layerMenu.precompose')} onSelect={() => void run('layer.select', { layers }).then(() => onPrecompose())} testId="motion-layer-menu-precompose" />
      {target?.source ? <MenuItem label={t('layerMenu.revealSource')} onSelect={() => onReveal(target.layer)} testId="motion-layer-menu-reveal-source" /> : null}
      {/* The colour label (shown on the layers' bars); Select Label Group selects every layer with the clicked one's. */}
      <LabelMenu
        current={target?.label}
        onLabel={(label) => void run('edit.label', { label, layers, target: 'layers' })}
        onSelectGroup={() => target && void run('layer.select', { layers: [target.layer] }).then(() => run('edit.selectLabelGroup', {}))}
        testId="motion-layer-menu"
      />
      <MenuSeparator />
      <MenuItem label={t('layerMenu.delete')} onSelect={() => void run('edit.clear', { layers })} testId="motion-layer-menu-delete" />
    </ContextMenu>
  )
}
