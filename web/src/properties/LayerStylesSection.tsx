import { EyeIcon, EyeOffIcon, PlusIcon as Plus, TrashIcon } from '@kition/tessera/icons/react'
import { Button, IconButton, MenuButton, MenuItem, MenuSeparator, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { pathId } from '../timeline/PropertyRows'
import type { TreeNode } from '../timeline/tree'
import { PropertyGroup, usePanel } from './PropertyTree'
import { Section } from './Section'

/** The styles a layer can have, by `layer.style.add`'s id and the engine's English name (translated by PHRASES). */
export const LAYER_STYLES: ReadonlyArray<readonly [id: string, label: string]> = [
  ['dropShadow', 'Drop Shadow'],
  ['innerShadow', 'Inner Shadow'],
  ['outerGlow', 'Outer Glow'],
  ['innerGlow', 'Inner Glow'],
  ['bevelEmboss', 'Bevel and Emboss'],
  ['satin', 'Satin'],
  ['colorOverlay', 'Color Overlay'],
  ['gradientOverlay', 'Gradient Overlay'],
  ['stroke', 'Stroke'],
]

/** A new drop shadow's settings: farther, larger and darker than the engine's, so it shows on a dark background. */
export const DROP_SHADOW = { distance: 20, size: 20, opacity: 100 }

/** Layers that take styles: the ones that draw pixels of their own. */
const NO_STYLES = new Set(['Camera', 'Light', 'Null', 'Adjustment', 'Audio'])

/**
 * The layer's styles (Layer ▸ Layer Styles): an Add menu with each style it does not have yet; each
 * style a group with its parameters (keyframable), an on/off button and a remove button; Blending
 * Options (the styles' shared light and blending) first, kept while any style is. Each change is
 * one undo step (`layer.style.add`, `layer.style.toggle`, `layer.style.remove`, `layer.style.removeAll`).
 */
export function LayerStylesSection({ layer, group }: { layer: MotionLayer; group: TreeNode | undefined }) {
  const { run, t, phrase } = useKitEditor()
  const { readOnly } = usePanel()
  if (NO_STYLES.has(layer.type)) return null
  const styles = group?.children?.filter((child) => child.children) ?? []
  const has = new Set(styles.map((style) => style.match))
  const applied = styles.filter((style) => style.match !== 'blendingOptions')
  const missing = LAYER_STYLES.filter(([id]) => !has.has(id))
  return (
    <Section
      scope={layer.type}
      id="layerStyles"
      title={applied.length ? `${t('props.layerStyles')} (${applied.length})` : t('props.layerStyles')}
      testId="motion-section-layerStyles"
      byDefault={applied.length > 0}
      actions={
        <MenuButton placement="auto"
          align="end"
          trigger={({ open, toggle }) => (
            <Button variant="ghost" aria-expanded={open} disabled={readOnly || missing.length === 0} onClick={toggle} data-testid="motion-style-add">
              <Plus size={14} aria-hidden="true" />
              {t('style.add')}
            </Button>
          )}
        >
          {missing.map(([id, label]) => (
            <MenuItem key={id} label={phrase(label)} onSelect={() => void run('layer.style.add', { layers: [layer.id], style: id, ...(id === 'dropShadow' ? { params: DROP_SHADOW } : {}) })} testId={`motion-style-add-${id}`} />
          ))}
          {applied.length ? <MenuSeparator /> : null}
          {applied.length ? <MenuItem label={t('style.removeAll')} onSelect={() => void run('layer.style.removeAll', { layers: [layer.id] })} testId="motion-style-remove-all" /> : null}
        </MenuButton>
      }
    >
      {applied.length ? null : <p className="kui-muted motion-effect-none">{t('style.empty')}</p>}
      <div className="motion-props">
      {styles.map((style) => {
        const name = phrase(style.name)
        const id = pathId(style.path)
        const on = style.enabled !== false
        return (
          <div key={style.path} className="motion-style" data-off={!on || undefined} data-testid={`motion-style-${style.match}`}>
            {style.match === 'blendingOptions' ? null : (
              <span className="motion-style-actions">
                <IconButton
                  icon={on ? EyeIcon : EyeOffIcon}
                  label={t(on ? 'style.off' : 'style.on', { name })}
                  pressed={on}
                  disabled={readOnly}
                  onClick={() => void run('layer.style.toggle', { layer: layer.id, style: style.match, value: !on })}
                  data-testid={`motion-style-enabled-${style.match}`}
                />
                <IconButton icon={TrashIcon} label={t('style.remove', { name })} disabled={readOnly} onClick={() => void run('layer.style.remove', { layer: layer.id, style: style.match })} data-testid={`motion-style-remove-${style.match}`} />
              </span>
            )}
            <PropertyGroup node={style} scope={layer.type} depth={0} byDefault={style.match !== 'blendingOptions'} label={name} key={id} />
          </div>
        )
      })}
      </div>
    </Section>
  )
}
