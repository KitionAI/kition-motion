import { PlusIcon as Plus, SparklesIcon as Sparkles, TrashIcon } from '@kition/tessera/icons/react'
import { Button, IconButton, MenuButton, MenuItem, MenuSeparator, useKitEditor } from '@kition/sdk/editor'
import type { TreeNode } from '../timeline/tree'
import { PropertyGroup, usePanel } from './PropertyTree'

/**
 * What a text animator animates (`layer.addTextAnimator`'s `properties`), in its menu's order,
 * each with the engine's English name (translated by PHRASES); `---` separates.
 */
export const ANIMATOR_PROPERTIES: ReadonlyArray<readonly [property: string, label: string] | '---'> = [
  ['position', 'Position'],
  ['scale', 'Scale'],
  ['rotation', 'Rotation'],
  ['opacity', 'Opacity'],
  ['anchor', 'Anchor Point'],
  ['skew', 'Skew'],
  ['transformAll', 'All Transform Properties'],
  '---',
  ['fillColor', 'Fill Color: RGB'],
  ['fillHue', 'Fill Color: Hue'],
  ['fillOpacity', 'Fill Color: Opacity'],
  ['strokeColor', 'Stroke Color: RGB'],
  ['strokeWidth', 'Stroke Width'],
  '---',
  ['tracking', 'Tracking'],
  ['lineSpacing', 'Line Spacing'],
  ['characterOffset', 'Character Offset'],
  ['blur', 'Blur'],
]

/** The selectors an animator can have (`layer.addTextSelector`'s `kind`). */
const SELECTORS: ReadonlyArray<readonly [kind: string, label: string]> = [
  ['range', 'Range Selector'],
  ['wiggly', 'Wiggly Selector'],
  ['expression', 'Expression Selector'],
]

/** The built-in text animation presets the panel offers (`layer.applyTextPreset`'s `preset`), with the engine's names. */
export const TEXT_PRESETS: ReadonlyArray<readonly [preset: string, label: string]> = [
  ['fadeUpCharacters', 'Fade Up Characters'],
  ['typewriter', 'Typewriter'],
  ['blurSlide', 'Blur Slide'],
  ['bounceInWords', 'Bounce In Words'],
  ['dropInLines', 'Drop In Lines'],
  ['trackingIn', 'Tracking In'],
  ['blurIn', 'Blur In'],
  ['scramble', 'Scramble'],
  ['letterpress', 'Letterpress'],
  ['jitter', 'Jitter'],
]

/**
 * The text layer's Animate menu (a new animator animating one property, with a Range Selector
 * over every character) and Presets menu (a built-in animation applied from the current time),
 * for the Text section's header. Each is one undo step.
 */
export function TextAnimateMenus({ layer }: { layer: number }) {
  const { run, t, phrase } = useKitEditor()
  const { readOnly } = usePanel()
  return (
    <>
      <MenuButton placement="auto"
        align="end"
        trigger={({ open, toggle }) => (
          <Button variant="ghost" aria-expanded={open} disabled={readOnly} onClick={toggle} data-testid="motion-text-animate">
            <Plus size={14} aria-hidden="true" />
            {t('animator.animate')}
          </Button>
        )}
      >
        {ANIMATOR_PROPERTIES.map((entry, at) =>
          entry === '---' ? <MenuSeparator key={`-${at}`} /> : <MenuItem key={entry[0]} label={phrase(entry[1])} onSelect={() => void run('layer.addTextAnimator', { layer, properties: [entry[0]] })} testId={`motion-text-animate-${entry[0]}`} />,
        )}
      </MenuButton>
      <MenuButton placement="auto"
        align="end"
        trigger={({ open, toggle }) => (
          <Button variant="ghost" aria-expanded={open} disabled={readOnly} onClick={toggle} data-testid="motion-text-presets">
            <Sparkles size={14} aria-hidden="true" />
            {t('animator.presets')}
          </Button>
        )}
      >
        {TEXT_PRESETS.map(([preset, label]) => (
          <MenuItem key={preset} label={phrase(label)} onSelect={() => void run('layer.applyTextPreset', { layer, preset })} testId={`motion-text-preset-${preset}`} />
        ))}
      </MenuButton>
    </>
  )
}

/**
 * A text layer's animators, each a group (its selectors with their range and Advanced settings,
 * and the properties it animates, all keyframable) under a header with Add Property, Add Selector
 * and a Remove menu (the animator, or one of its selectors or properties). Each change is one undo
 * step (`layer.addTextAnimatorProperty`, `layer.addTextSelector`, `prop.removeGroup`).
 */
export function TextAnimators({ node, scope }: { node: TreeNode; scope: string }) {
  const { run, t, phrase } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const animators = (node.children ?? []).filter((child) => child.children)
  if (!animators.length) return null
  return (
    <div className="motion-animators" data-testid="motion-text-animators">
      {animators.map((animator, index) => {
        const name = phrase(animator.name)
        const parts = (animator.children ?? []).flatMap((group) => (group.children ?? []).map((child) => ({ child, kind: group.match })))
        const props = parts.filter((part) => part.kind === 'properties')
        const selectors = parts.filter((part) => part.kind === 'selectors')
        const removable = [...selectors, ...props].filter((part) => part.child.uid != null)
        return (
          <div key={animator.path} className="motion-animator" data-testid={`motion-animator-${index + 1}`} data-uid={animator.uid}>
            <div className="motion-animator-head">
              <span className="motion-animator-name">{name}</span>
              <MenuButton placement="auto"
                align="end"
                trigger={({ open, toggle }) => <IconButton icon={Plus} label={t('animator.addProperty', { name })} pressed={open} disabled={readOnly} onClick={toggle} data-testid={`motion-animator-add-${index + 1}`} />}
              >
                {ANIMATOR_PROPERTIES.map((entry, at) =>
                  entry === '---' ? (
                    <MenuSeparator key={`-${at}`} />
                  ) : (
                    <MenuItem key={entry[0]} label={phrase(entry[1])} onSelect={() => void run('layer.addTextAnimatorProperty', { layer: layer.id, animator: animator.uid, property: entry[0] })} testId={`motion-animator-add-${index + 1}-${entry[0]}`} />
                  ),
                )}
                <MenuSeparator />
                {SELECTORS.map(([kind, label]) => (
                  <MenuItem key={kind} label={t('animator.addSelector', { name: phrase(label) })} onSelect={() => void run('layer.addTextSelector', { layer: layer.id, animator: animator.uid, kind })} testId={`motion-animator-selector-${index + 1}-${kind}`} />
                ))}
              </MenuButton>
              <MenuButton placement="auto"
                align="end"
                trigger={({ open, toggle }) => <IconButton icon={TrashIcon} label={t('animator.removeMenu', { name })} pressed={open} disabled={readOnly} onClick={toggle} data-testid={`motion-animator-remove-${index + 1}`} />}
              >
                <MenuItem label={t('animator.remove', { name })} onSelect={() => void run('prop.removeGroup', { layer: layer.id, prop: animator.uid })} testId={`motion-animator-remove-${index + 1}-all`} />
                {removable.length ? <MenuSeparator /> : null}
                {removable.map(({ child }) => (
                  <MenuItem key={child.path} label={t('animator.removePart', { name: phrase(child.name) })} onSelect={() => void run('prop.removeGroup', { layer: layer.id, prop: child.uid })} testId={`motion-animator-remove-${index + 1}-${child.match ?? ''}`} />
                ))}
              </MenuButton>
            </div>
            <PropertyGroup node={animator} scope={scope} depth={1} byDefault label={name} />
          </div>
        )
      })}
    </div>
  )
}
