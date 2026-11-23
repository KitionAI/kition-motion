import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { TextAlignCenterIcon as Center, TextAlignEndIcon as Right, TextAlignStartIcon as Left } from '@kition/tessera/icons/react'
import { Combobox } from '@kition/tessera/ui'
import { Segmented, Select, Textarea, useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import { keyTimes } from '../timeline/keys'
import type { TreeNode } from '../timeline/tree'
import { NumberField } from '../ValueField'
import { ColorSwatch, hexOf } from './ColorSwatch'
import { KeyDiamond } from './KeyDiamond'
import { usePanel } from './PropertyTree'
import { useSender } from './useSetProp'

/** A text layer's character and paragraph settings, as the scripting model's TextDocument reads them. */
type TextStyle = { family: string; style: string; size: number; fill: number[]; stroke: number[]; strokeWidth: number; applyFill: boolean; applyStroke: boolean; tracking: number; leading: number; justify: 'left' | 'center' | 'right' | 'other' }

type Fonts = { families?: Array<{ family: string; styles: string[] }> }

/**
 * Reads the layer's text settings through the scripting object model (`script.run`, which edits
 * nothing here): no engine command answers them yet.
 */
const styleScript = (index: number) => `(function () {
  var d = app.project.activeItem.layer(${index}).property("ADBE Text Properties").property("ADBE Text Document").value;
  var j = d.justification;
  return JSON.stringify({ family: d.fontFamily || d.font, style: d.fontStyle, size: d.fontSize, fill: d.fillColor, stroke: d.strokeColor, strokeWidth: d.strokeWidth,
    applyFill: d.applyFill, applyStroke: d.applyStroke, tracking: d.tracking, leading: d.leading,
    justify: j === ParagraphJustification.LEFT_JUSTIFY ? "left" : j === ParagraphJustification.CENTER_JUSTIFY ? "center" : j === ParagraphJustification.RIGHT_JUSTIFY ? "right" : "other" });
})()`

let sessions = 0

/** The text box: the content, live while typing (one undo step per visit), committed on blur or Cmd+Enter. */
function ContentBox({ node, focusNow, onFocused }: { node: TreeNode; focusNow: boolean; onFocused: () => void }) {
  const { t, phrase } = useKitEditor()
  const { layer, time, frame, readOnly } = usePanel()
  const value = typeof node.value === 'string' ? node.value : ''
  const [text, setText] = useState(value)
  const typing = useRef<{ gesture: string; from: string } | null>(null)
  const box = useRef<HTMLTextAreaElement>(null)
  const send = useSender('layer.setText')
  useEffect(() => {
    if (!typing.current) setText(value)
  }, [value])
  useEffect(() => {
    if (!focusNow || !box.current) return
    box.current.focus()
    box.current.select()
    onFocused()
  }, [focusNow, onFocused])
  const name = phrase(node.name)
  return (
    <div className="motion-text-content">
      <div className="motion-prop">
        <KeyDiamond layer={layer.id} path={node.path} name={name} times={keyTimes(layer, node.path)} time={time} frame={frame} testId="motion-add-key-text.sourceText" readOnly={readOnly} />
        <span className="motion-prop-name kui-muted">{name}</span>
      </div>
      <Textarea
        ref={box}
        value={text}
        autoSize={{ minRows: 2, maxRows: 8 }}
        aria-label={t('text.content')}
        placeholder={t('text.content')}
        readOnly={readOnly}
        onFocus={() => {
          sessions += 1
          typing.current = { gesture: `text-${layer.id}-${sessions}`, from: value }
        }}
        onChange={(event) => {
          setText(event.target.value)
          if (typing.current) send({ layer: layer.id, text: event.target.value }, typing.current.gesture)
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
            event.preventDefault()
            event.currentTarget.blur()
          }
          if (event.key === 'Escape' && typing.current) {
            // Back to the text the box had when it took the focus, in the same undo step.
            event.preventDefault()
            setText(typing.current.from)
            send({ layer: layer.id, text: typing.current.from }, typing.current.gesture)
            event.currentTarget.blur()
          }
        }}
        onBlur={() => {
          typing.current = null
        }}
        data-testid="motion-text-content"
      />
    </div>
  )
}

/** A labelled control in the Text section's grid. */
function Item({ label, children, wide }: { label: string; children: ReactNode; wide?: boolean }) {
  return (
    <div className="motion-text-item" data-wide={wide || undefined}>
      <span className="kui-muted">{label}</span>
      {children}
    </div>
  )
}

/**
 * A text layer's Text section: the content box, then font family (searchable) and style, size,
 * fill, stroke and its width, tracking, leading and alignment, each change one undo step on the
 * whole text (`layer.setText`).
 */
export function TextControls({ node, focusNow, onFocused }: { node: TreeNode; focusNow: boolean; onFocused: () => void }) {
  const { t } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const source = (node.children ?? []).find((child) => child.match === 'sourceText')
  const read = useEngineQuery<{ ok?: boolean; result?: string }>('script.run', { code: styleScript(layer.index) }, source?.value)
  const style = useMemo<TextStyle | null>(() => {
    try {
      return read?.ok && read.result ? (JSON.parse(read.result) as TextStyle) : null
    } catch {
      return null
    }
  }, [read])
  const fonts = useEngineQuery<Fonts>('text.fonts', {})
  const families = fonts?.families ?? []
  const styles = families.find((family) => family.family === style?.family)?.styles ?? (style ? [style.style] : [])
  const font = useSender('layer.setText')
  const size = useSender('layer.setText')
  const fill = useSender('layer.setText')
  const stroke = useSender('layer.setText')
  const width = useSender('layer.setText')
  const tracking = useSender('layer.setText')
  const leading = useSender('layer.setText')
  const set = (sender: typeof font, params: Record<string, unknown>, gesture?: string) => sender({ layer: layer.id, ...params }, gesture)
  return (
    <div className="motion-text" data-testid="motion-text-controls">
      {source ? <ContentBox node={source} focusNow={focusNow} onFocused={onFocused} /> : null}
      {style ? (
        <div className="motion-text-grid">
          <Item label={t('text.font')} wide>
            <Combobox
              value={style.family}
              label={t('text.font')}
              options={families.map((family) => ({ value: family.family, label: family.family, text: family.family }))}
              disabled={readOnly}
              onChange={(family) => {
                if (!family) return
                const styles = families.find((entry) => entry.family === family)?.styles ?? []
                set(font, { font: family, style: styles.includes(style.style) ? style.style : (styles[0] ?? 'Regular') })
              }}
              testId="motion-text-font"
            />
          </Item>
          <Item label={t('text.style')}>
            <Select value={style.style} disabled={readOnly} onChange={(event) => set(font, { font: style.family, style: event.target.value })} data-testid="motion-text-style">
              {styles.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </Select>
          </Item>
          <Item label={t('text.size')}>
            <NumberField value={style.size} label={t('text.size')} min={1} max={5000} decimals={1} unit="px" readOnly={readOnly} onCommit={(next) => set(size, { size: next })} onScrub={(next, _final, gesture) => set(size, { size: next }, gesture)} testId="motion-text-size" />
          </Item>
          <Item label={t('text.fill')}>
            <ColorSwatch value={hexOf(style.fill)} label={t('text.fill')} readOnly={readOnly} onChange={(hex, final) => set(fill, { fill: hex, applyFill: true }, final ? undefined : `fill-${layer.id}`)} testId="motion-text-fill" />
          </Item>
          <Item label={t('text.stroke')}>
            <span className="motion-text-stroke">
              <ColorSwatch value={hexOf(style.stroke)} label={t('text.stroke')} readOnly={readOnly} onChange={(hex, final) => set(stroke, { stroke: hex, applyStroke: true }, final ? undefined : `stroke-${layer.id}`)} testId="motion-text-stroke" />
              <NumberField
                value={style.applyStroke ? style.strokeWidth : 0}
                label={t('text.strokeWidth')}
                min={0}
                max={1000}
                decimals={1}
                unit="px"
                readOnly={readOnly}
                onCommit={(next) => set(width, { strokeWidth: next, applyStroke: next > 0 })}
                onScrub={(next, _final, gesture) => set(width, { strokeWidth: next, applyStroke: next > 0 }, gesture)}
                testId="motion-text-stroke-width"
              />
            </span>
          </Item>
          <Item label={t('text.tracking')}>
            <NumberField value={style.tracking} label={t('text.tracking')} min={-1000} max={10000} decimals={0} readOnly={readOnly} onCommit={(next) => set(tracking, { tracking: next })} onScrub={(next, _final, gesture) => set(tracking, { tracking: next }, gesture)} testId="motion-text-tracking" />
          </Item>
          <Item label={t('text.leading')}>
            <NumberField value={style.leading} label={t('text.leading')} min={0} max={5000} decimals={1} unit="px" readOnly={readOnly} onCommit={(next) => set(leading, { leading: next })} onScrub={(next, _final, gesture) => set(leading, { leading: next }, gesture)} testId="motion-text-leading" />
          </Item>
          <Item label={t('text.align')}>
            <Segmented
              label={t('text.align')}
              value={style.justify}
              disabled={readOnly}
              segments={[
                { value: 'left', icon: Left, label: t('text.left') },
                { value: 'center', icon: Center, label: t('text.center') },
                { value: 'right', icon: Right, label: t('text.right') },
              ]}
              onChange={(justify) => set(font, { justify })}
              testId="motion-text-align"
            />
          </Item>
        </div>
      ) : null}
    </div>
  )
}
