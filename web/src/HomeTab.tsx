import { DiamondIcon as Diamond, LayersIcon as Layers, PauseIcon as Pause, PlusIcon as Plus, ScissorsIcon as Scissors, Settings2Icon as Settings2, SparklesIcon as Sparkles, SplineIcon as Spline, SquareIcon as Square, SquareDashedIcon as SquareDashed, SquarePlusIcon as SquarePlus, TypeIcon as Type } from '@kition/tessera/icons/react'
import { CommandButton, CommandItem, MenuItem, MenuSeparator, RibbonGroup, RibbonMenu, RibbonStack, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { NEW_LAYERS } from './Transport'

/** The layers Home's New Layer menu makes: the transport's, then a camera and a light. */
const HOME_LAYERS = [...NEW_LAYERS, { id: 'camera', command: 'layer.newCamera', params: {} }, { id: 'light', command: 'layer.newLight', params: {} }] as const

/** Effects applied most, by their display names (`effect.apply` looks a name up). */
const EFFECTS = ['Gaussian Blur', 'Glow', 'Drop Shadow', 'Fill', 'Tint', 'Hue/Saturation']

/**
 * Motion's Home tab: the motion tasks done most, in every display mode (a drop-down panel in
 * Menus): new layers, keyframes, effects, and the composition. Render stays in the top bar.
 */
export function HomeTab({ onCommand, wholeWorkArea }: { onCommand: (command: string) => void; wholeWorkArea: boolean }) {
  const { run, enabled, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
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
        <CommandButton command="anim.addKeyframe" icon={Diamond} size="large" primary />
        <CommandButton command="keys.easyEase" icon={Spline} primary />
        <CommandButton command="keys.toggleHold" icon={Pause} />
      </RibbonGroup>

      <RibbonGroup label={p('Effect')} icon={Sparkles}>
        <RibbonMenu icon={Sparkles} label={t('home.effects')} size="large" disabled={readOnly || !enabled('effect.apply')} testId="motion-home-effects" primary>
          {EFFECTS.map((name) => (
            <MenuItem key={name} label={p(name)} onSelect={() => void run('effect.apply', { effect: name })} testId={`motion-home-effect-${name.toLowerCase().replace(/[^a-z]+/g, '-')}`} />
          ))}
          <MenuSeparator />
          <CommandItem command="effect.applyLast" />
          <CommandItem command="effect.removeAll" />
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
