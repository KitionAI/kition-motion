import { useCallback, useEffect, useRef, useState } from 'react'
import { useEngineQuery, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import type { MotionInfo, MotionLayer } from '../motion'
import type { TreeNode } from '../timeline/tree'
import { EffectsSection } from './EffectsSection'
import { LayerHeader } from './LayerHeader'
import { PanelEdge, usePanelWidth } from './PanelEdge'
import { PanelContext, PropertyChildren, PropertyGroup, hasRows, visible } from './PropertyTree'
import { Section, remember } from './Section'
import { ShapeControls } from './ShapeSection'
import { SolidSection } from './SolidSection'
import { TextControls } from './TextSection'

/** How deep the panel reads a layer's property tree (shape groups nest). */
const TREE_DEPTH = 10

/** Top-level groups drawn in their own places rather than as the layer type's section. */
const PLACED = new Set(['effects', 'transform', 'masks'])

/** A text layer's rarely used groups, behind a closed "More" group. */
const TEXT_MORE = new Set(['text/pathOptions', 'text/moreOptions'])

/** The layer type's own section (Text, Contents, Camera Options…) from its top-level group. */
function TypeSection({ node, layer, focusText, onFocused }: { node: TreeNode; layer: MotionLayer; focusText: boolean; onFocused: () => void }) {
  const { phrase, t } = useKitEditor()
  const scope = layer.type
  if (node.match === 'text') {
    // The content box and the character settings first; Source Text is the box.
    const main = { ...node, children: (node.children ?? []).filter((child) => !TEXT_MORE.has(child.path) && child.match !== 'sourceText') }
    const more = (node.children ?? []).filter((child) => TEXT_MORE.has(child.path))
    return (
      <Section scope={scope} id={node.path} title={phrase(node.name)} testId={`motion-section-${node.match}`}>
        <TextControls node={node} focusNow={focusText} onFocused={onFocused} />
        <PropertyChildren node={main} scope={scope} depth={0} />
        {more.length ? <PropertyGroup node={{ name: 'More', path: 'text/more', children: more }} label={t('props.more')} scope={scope} depth={0} /> : null}
      </Section>
    )
  }
  return (
    <Section scope={scope} id={node.path} title={phrase(node.name)} testId={`motion-section-${node.match}`}>
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
export function PropertiesPanel({ info, onAddEffect, typingOnCanvas = false }: { info: MotionInfo; onAddEffect: () => void; typingOnCanvas?: boolean }) {
  const { t } = useKitEditor()
  // A viewer sees the values but cannot change them or add keyframes.
  const readOnly = useReadOnly()
  const [width, setWidth] = usePanelWidth()
  const first = info.transform?.layer
  const layer = first != null ? info.layers.find((item) => item.id === first) : undefined
  const selected = info.layers.filter((item) => item.selected).length
  const tree = useEngineQuery<{ properties?: TreeNode }>('layer.tree', layer ? { layer: layer.id, depth: TREE_DEPTH, time: info.time } : { layer: -1 })
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
            {top
              .filter((node) => node.children && !PLACED.has(node.match ?? '') && visible(node, layer) && hasRows(node, layer))
              .map((node) => (
                <TypeSection key={node.path} node={node} layer={layer} focusText={fresh === layer.id} onFocused={focused} />
              ))}
            {layer.type === 'Solid' ? <SolidSection /> : null}
            <EffectsSection layer={layer} group={group('effects')} onAdd={onAddEffect} />
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
