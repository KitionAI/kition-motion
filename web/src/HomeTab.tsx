import { DiamondIcon as Diamond, LayersIcon as Layers, PauseIcon as Pause, PlusIcon as Plus, ScissorsIcon as Scissors, Settings2Icon as Settings2, SparklesIcon as Sparkles, SplineIcon as Spline, SquareIcon as Square, SquareDashedIcon as SquareDashed, SquarePlusIcon as SquarePlus, TypeIcon as Type } from '@kition/tessera/icons/react'
import { CommandButton, CommandItem, MenuItem, MenuSeparator, RibbonButton, RibbonGroup, RibbonMenu, RibbonStack, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'
import { keyAt } from './properties/KeyDiamond'
import { NEW_LAYERS } from './Transport'
import { EFFECT_PREFIX, FIND_EFFECT } from './effects'

/** The layers Home's New Layer menu makes: the transport's, then a camera and a light. */
const HOME_LAYERS = [...NEW_LAYERS, { id: 'camera', command: 'layer.newCamera', params: {} }, { id: 'light', command: 'layer.newLight', params: {} }] as const

/** Effects applied most, by their display names (`effect.apply` looks a name up). */
const EFFECTS = ['Gaussian Blur', 'Glow', 'Drop Shadow', 'Fill', 'Tint', 'Hue/Saturation']

/**
 * Motion's Home tab: the motion tasks done most, in every display mode (a drop-down panel in
 * Menus): new layers, keyframes, effects, and the composition. Render stays in the top bar.
 */
export function HomeTab({ info, onCommand, wholeWorkArea, lastEffect }: { info: MotionInfo | null; onCommand: (command: string) => void; wholeWorkArea: boolean; lastEffect: boolean }) {
  const { run, enabled, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  // Add Keyframe keys the selected properties (those whose keyframes were clicked) at the current
  // time; with none selected, the Position of each selected layer that has no keyframe there.
  const selected = info?.layers.filter((layer) => layer.selected) ?? []
  const properties = enabled('anim.addKeyframe')
  const unkeyed = selected.filter((layer) => !keyAt(layer.animated.find((property) => property.path === 'transform/position')?.times ?? [], info?.time ?? 0, info?.frame ?? 1 / 30)).map((layer) => layer.id)
  const addKeyframe = () => void (properties ? run('anim.addKeyframe', {}) : run('keys.toggleTransform', { layers: unkeyed, prop: 'position' }))
  return (
    <>
      <RibbonGroup label={p('Layer')} icon={Layers}>
        <RibbonMenu icon={Plus} label={t('newLayer')} size="large" disabled={readOnly} testId="motion-home-new-layer" primary>
          {HOME_LAYERS.map((layer) => (
            <MenuItem key={layer.id} label={t(`layerKind.${layer.id}`)} onSelect={() => (layer.command === 'layer.newSolid' ? onCommand(layer.command) : void run(layer.command, { ...layer.params }))} testId={`motion-home-new-${layer.id}`} />
          ))}
        </RibbonMenu>
        <CommandButton command="layer.newText" params={{ text: 'Text' }} icon={Type} primary />
        <CommandButton command="layer.newShape" params={{ kind: 'rect' }} icon={Square} primary />
        <CommandButton command="layer.precompose" icon={Layers} primary />
        <CommandButton command="layer.addMask" icon={SquareDashed} />
      </RibbonGroup>

      <RibbonGroup label={p('Animation')} icon={Diamond}>
        <RibbonButton icon={Diamond} label={p('Add Keyframe')} size="large" primary title={t('home.addKeyHint')} disabled={readOnly || !selected.length || (!properties && !unkeyed.length)} onClick={addKeyframe} data-testid="motion-home-add-key" />
        <CommandButton command="keys.easyEase" icon={Spline} primary />
        <CommandButton command="keys.toggleHold" icon={Pause} />
      </RibbonGroup>

      <RibbonGroup label={p('Effect')} icon={Sparkles}>
        <RibbonMenu icon={Sparkles} label={t('home.effects')} size="large" disabled={readOnly || !enabled('effect.apply')} testId="motion-home-effects" primary>
          {EFFECTS.map((name) => (
            <MenuItem key={name} label={p(name)} onSelect={() => onCommand(`${EFFECT_PREFIX}${name}`)} testId={`motion-home-effect-${name.toLowerCase().replace(/[^a-z]+/g, '-')}`} />
          ))}
          <MenuSeparator />
          {/* Last Effect once one was applied here; every other effect is in Find Effect. */}
          {lastEffect ? <CommandItem command="effect.applyLast" onRun={onCommand} /> : null}
          <MenuItem label={p('Find Effect…')} onSelect={() => onCommand(FIND_EFFECT)} testId="motion-home-find-effect" />
        </RibbonMenu>
      </RibbonGroup>

      <RibbonGroup label={p('Composition')} icon={SquarePlus}>
        <CommandButton command="comp.new" icon={SquarePlus} primary onRun={onCommand} />
        <CommandButton command="comp.settings" icon={Settings2} primary onRun={onCommand} />
        <RibbonStack>
          {/* Trimming to a work area that is the whole composition would change nothing. */}
          <CommandButton command="comp.trimToWorkArea" icon={Scissors} onRun={onCommand} disabled={wholeWorkArea} />
        </RibbonStack>
      </RibbonGroup>
    </>
  )
}
