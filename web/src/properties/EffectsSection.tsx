import { useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import type { TreeNode } from '../timeline/tree'
import { useEffectParams } from './catalog'
import { PropertyChildren } from './PropertyTree'
import { Section } from './Section'

/** The layer's effects, each with its parameters (ranges, sliders and drop-down labels from `effect.list`). */
export function EffectsSection({ layer, group }: { layer: MotionLayer; group: TreeNode | undefined }) {
  const { t, phrase } = useKitEditor()
  const paramsOf = useEffectParams()
  const applied = group?.children ?? []
  if (!applied.length) return null
  return (
    <Section scope={layer.type} id="effects" title={t('props.effects')} testId="motion-section-effects">
      {applied.map((effect) => (
        <div key={effect.uid ?? effect.path} className="motion-effect-params">
          <p className="motion-effect-name">{phrase(effect.name)}</p>
          <PropertyChildren node={effect} scope={layer.type} depth={0} params={paramsOf(effect.effect)} />
        </div>
      ))}
    </Section>
  )
}
