import { useCallback, useEffect, useRef, useState } from 'react'
import { Select, useEngineQuery, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import type { MotionInfo, MotionLayer } from '../motion'
import type { TreeNode } from '../timeline/tree'
import type { AlignTo } from '../menus'
import { AlignSection } from './AlignSection'
import { EffectsSection } from './EffectsSection'
import { LayerStylesSection } from './LayerStylesSection'
import { TimeSection } from './TimeSection'
import { LayerHeader } from './LayerHeader'
import { PanelEdge, usePanelWidth } from './PanelEdge'
import { PanelContext, PropertyChildren, PropertyGroup, hasRows, visible } from './PropertyTree'
import { Section, remember } from './Section'
import { ShapeControls } from './ShapeSection'
import { SolidSection } from './SolidSection'
import { TextControls, type TextSelection } from './TextSection'
import { TextAnimateMenus, TextAnimators } from './TextAnimators'
import { cameraOrLight } from '../timeline/SwitchCells'

/** How deep the panel reads a layer's property tree (shape groups nest). */
const TREE_DEPTH = 10

/** Top-level groups drawn in their own places rather than as the layer type's section. */
const PLACED = new Set(['effects', 'transform', 'masks', 'layerStyles'])

/** A text layer's rarely used groups, behind a closed "More" group. */
const TEXT_MORE = new Set(['text/pathOptions', 'text/moreOptions'])

/** The layer type's own section (Text, Contents, Camera Options…) from its top-level group. */
/** The light types `layer.lightSettings` takes, in the order the Light Type menu lists them. */
const LIGHT_KINDS = ['Parallel', 'Spot', 'Point', 'Ambient'] as const

/** A light's type above its options: a change rebuilds its options and keeps the shared values (one undo step). */
function LightType({ layer, kind }: { layer: number; kind: string }) {
  const { run, t } = useKitEditor()
  const readOnly = useReadOnly()
  const kinds: readonly string[] = LIGHT_KINDS.includes(kind as (typeof LIGHT_KINDS)[number]) ? LIGHT_KINDS : [...LIGHT_KINDS, kind]
  return (
    <div className="motion-prop" data-testid="motion-light-type-row">
      <span className="motion-prop-name kui-muted">{t('light.type')}</span>
      <span className="motion-prop-values">
        <Select value={kind} aria-label={t('light.type')} disabled={readOnly} onChange={(event) => void run('layer.lightSettings', { layer, kind: event.target.value })} data-testid="motion-light-type">
          {kinds.map((each) => (
            <option key={each} value={each}>
              {t(`light.kind.${each}`)}
            </option>
          ))}
        </Select>
      </span>
    </div>
  )
}

function TypeSection({ node, layer, focusText, onFocused, text, lightKind }: { node: TreeNode; layer: MotionLayer; focusText: boolean; onFocused: () => void; text: TextSelection | null; lightKind?: string }) {
  const { phrase, t } = useKitEditor()
  const scope = layer.type
  if (node.match === 'text') {
    // The content box and the character settings first; Source Text is the box.
    const main = { ...node, children: (node.children ?? []).filter((child) => !TEXT_MORE.has(child.path) && child.match !== 'sourceText' && child.path !== 'text/animators') }
    const more = (node.children ?? []).filter((child) => TEXT_MORE.has(child.path))
    const animators = (node.children ?? []).find((child) => child.path === 'text/animators')
    return (
      <Section scope={scope} id={node.path} title={phrase(node.name)} testId={`motion-section-${node.match}`} actions={<TextAnimateMenus layer={layer.id} />}>
        <TextControls node={node} focusNow={focusText} onFocused={onFocused} selection={text} />
        <PropertyChildren node={main} scope={scope} depth={0} />
        {animators ? <TextAnimators node={animators} scope={scope} /> : null}
        {more.length ? <PropertyGroup node={{ name: 'More', path: 'text/more', children: more }} label={t('props.more')} scope={scope} depth={0} /> : null}
      </Section>
    )
  }
  return (
    <Section scope={scope} id={node.path} title={phrase(node.name)} testId={`motion-section-${node.match}`}>
      {node.match === 'lightOptions' && lightKind ? (
        <div className="motion-props">
          <LightType layer={layer.id} kind={lightKind} />
        </div>
      ) : null}
      {node.match === 'contents' ? <ShapeControls contents={node} scope={scope} /> : <PropertyChildren node={node} scope={scope} depth={0} />}
    </Section>
  )
}

/**
 * The selected layer's properties at the current time, as collapsible sections: its name, type,
 * blending mode and parent; its type's own section (Text, Contents); its effects; Transform; and
 * Masks when it has any. With several layers selected it shows the first, and says so.
 */
/** `typingOnCanvas`: the Text tool is out, so a new text layer is typed into on the canvas, not here. */
/** `align`: the Align section's target, the rows that would change nothing now, and how a row runs. */
export function PropertiesPanel({ info, onAddEffect, typingOnCanvas = false, align }: { info: MotionInfo; onAddEffect: () => void; typingOnCanvas?: boolean; align: { to: AlignTo; onTo: (to: AlignTo) => void; idle: ReadonlySet<string>; onCommand: (command: string) => void } }) {
  const { t } = useKitEditor()
  // A viewer sees the values but cannot change them or add keyframes.
  const readOnly = useReadOnly()
  const [width, setWidth] = usePanelWidth()
  const first = info.transform?.layer
  const layer = first != null ? info.layers.find((item) => item.id === first) : undefined
  const selected = info.layers.filter((item) => item.selected).length
  const tree = useEngineQuery<{ properties?: TreeNode; text?: TextSelection; lightKind?: string }>('layer.tree', layer ? { layer: layer.id, depth: TREE_DEPTH, time: info.time } : { layer: -1 })
  const root = layer && tree?.properties ? tree.properties : null
  const top = root?.children ?? []
  const group = (match: string) => top.find((node) => node.match === match)
  const masks = group('masks')
  const transform = group('transform')
  // A text layer made just now: its content box takes the focus with its text selected, so typing replaces it.
  const known = useRef<Set<number> | null>(null)
  const [fresh, setFresh] = useState<number | null>(null)
  const onCanvas = useRef(typingOnCanvas)
  onCanvas.current = typingOnCanvas
  useEffect(() => {
    const ids = new Set(info.layers.map((item) => item.id))
    const before = known.current
    known.current = ids
    if (before && layer && layer.type === 'Text' && !before.has(layer.id) && !onCanvas.current) {
      remember('Text:text', true)
      setFresh(layer.id)
    }
  }, [info.layers, layer])
  const focused = useCallback(() => setFresh(null), [])
  return (
    <aside className="kui-panel motion-side" data-side="right" style={{ width }} data-testid="motion-properties-panel">
      <PanelEdge width={width} onWidth={setWidth} />
      <div className="kui-panel-body motion-side-body" data-testid="motion-properties-body">
        {layer && root ? (
          <PanelContext.Provider value={{ layer, time: info.time, frame: info.frame, readOnly }}>
            <LayerHeader info={info} layer={layer} readOnly={readOnly} count={selected} />
            {/* Align near the top, closed until opened (its state remembered), so it is reached without scrolling. */}
 {/* Cameras and lights have no picture to line up. */}
            {layer.type === 'Camera' || layer.type === 'Light' ? null : <AlignSection scope="all" alignTo={align.to} onAlignTo={align.onTo} idle={align.idle} onCommand={align.onCommand} readOnly={readOnly} />}
            {top
              .filter((node) => node.children && !PLACED.has(node.match ?? '') && visible(node, layer) && hasRows(node, layer))
              .map((node) => (
                <TypeSection key={node.path} node={node} layer={layer} focusText={fresh === layer.id} onFocused={focused} text={tree?.text ?? null} lightKind={tree?.lightKind} />
              ))}
            {layer.type === 'Solid' ? <SolidSection /> : null}
            <TimeSection layer={layer} remap={group('timeRemap')} />
            {/* A camera or a light has no pixels: no effects or layer styles. */}
            {cameraOrLight(layer) ? null : <EffectsSection layer={layer} group={group('effects')} onAdd={onAddEffect} />}
            {cameraOrLight(layer) ? null : <LayerStylesSection layer={layer} group={group('layerStyles')} />}
            {transform ? (
              <Section scope={layer.type} id="transform" title={t('props.transform')} testId="motion-section-transform">
                <PropertyChildren node={transform} scope={layer.type} depth={0} />
              </Section>
            ) : null}
            {masks && hasRows(masks, layer) ? (
              <Section scope={layer.type} id="masks" title={t('props.masks')} testId="motion-section-masks">
                <PropertyChildren node={masks} scope={layer.type} depth={0} />
              </Section>
            ) : null}
          </PanelContext.Provider>
        ) : layer ? null : (
          <p className="kui-muted motion-empty" data-testid="motion-properties-empty">
            {t('selectLayer')}
          </p>
        )}
      </div>
    </aside>
  )
}
