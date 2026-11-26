import { useEffect, useRef, useState, type ReactNode } from 'react'
import {
  BoldIcon,
  CaseSensitiveIcon,
  CaseUpperIcon,
  ItalicIcon,
  SubscriptIcon,
  SuperscriptIcon,
  TextAlignCenterIcon,
  TextAlignEndIcon,
  TextAlignJustifyIcon,
  TextAlignStartIcon,
  TextJustifyCenterIcon,
  TextJustifyEndIcon,
  TextJustifyStartIcon,
} from '@kition/tessera/icons/react'
import { Combobox } from '@kition/tessera/ui'
import { Select, Textarea, useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import { keyTimes } from '../timeline/keys'
import type { TreeNode } from '../timeline/tree'
import { NumberField } from '../ValueField'
import { useTypingUndo } from '../typingUndo'
import { ColorSwatch, hexOf } from './ColorSwatch'
import { KeyDiamond } from './KeyDiamond'
import { usePanel } from './PropertyTree'
import { useSender } from './useSetProp'

type Fonts = { families?: Array<{ family: string; styles: string[] }> }

/**
 * The families a font search shows: those whose name, or a word of it, starts with the query
 * (case aside), the names starting with it first; every family for an empty query.
 */
export function fontMatches<F extends { family: string }>(families: readonly F[], query: string): F[] {
  const wanted = query.trim().toLowerCase()
  if (!wanted) return [...families]
  const rank = (family: string) => {
    const name = family.toLowerCase()
    if (name.startsWith(wanted)) return 0
    return name.split(/[\s\-_]+/).some((word) => word.startsWith(wanted)) ? 1 : 2
  }
  return families.filter((each) => rank(each.family) < 2).sort((a, b) => rank(a.family) - rank(b.family))
}

let sessions = 0

/**
 * The text box: the content, live while typing (one undo step per visit), committed on blur or
 * Cmd+Enter. Cmd/Ctrl+Z in it undoes only this visit's typing (`useTypingUndo`); the box is made
 * anew for each layer (its `key`), so nothing typed for one layer is ever undone into another.
 */
function ContentBox({ node, focusNow, onFocused }: { node: TreeNode; focusNow: boolean; onFocused: () => void }) {
  const { t, phrase } = useKitEditor()
  const { layer, time, frame, readOnly } = usePanel()
  const value = typeof node.value === 'string' ? node.value : ''
  const [text, setText] = useState(value)
  const typing = useRef<{ gesture: string; from: string } | null>(null)
  const box = useRef<HTMLTextAreaElement>(null)
  const send = useSender('layer.setText')
  const history = useTypingUndo((next) => {
    setText(next)
    if (typing.current) send({ layer: layer.id, text: next }, typing.current.gesture)
  }, box)
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
          history.begin(text)
        }}
        onChange={(event) => {
          setText(event.target.value)
          history.record(event.target.value)
          if (typing.current) send({ layer: layer.id, text: event.target.value }, typing.current.gesture)
        }}
        onKeyDown={(event) => {
          if (history.key(event)) return
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
          history.end()
        }}
        data-testid="motion-text-content"
      />
    </div>
  )
}

/**
 * What `layer.tree` answers in `text` for a text layer: the character and paragraph settings of
 * the characters being edited (else the whole text), as `layer.setText` takes them; a setting that
 * differs across them is null and listed in `mixed`. `range` is the edited selection to give
 * `layer.setText`, null for the whole text.
 */
export type TextSelection = {
  character: Record<string, unknown>
  paragraph: Record<string, unknown>
  mixed: string[]
  range: [number, number] | null
  editing: boolean
  length: number
}

/** A labelled control in the Character and Paragraph grids. */
function Item({ label, children, wide }: { label: string; children: ReactNode; wide?: boolean }) {
  return (
    <div className="motion-text-item" data-wide={wide || undefined}>
      <span className="kui-muted">{label}</span>
      {children}
    </div>
  )
}

type Icon = typeof BoldIcon

/** The Character panel's switches: the `layer.setText` key each sets, its icon and its label key. */
const TOGGLES: ReadonlyArray<readonly [key: string, icon: Icon, label: string]> = [
  ['fauxBold', BoldIcon, 'text.fauxBold'],
  ['fauxItalic', ItalicIcon, 'text.fauxItalic'],
  ['allCaps', CaseUpperIcon, 'text.allCaps'],
  ['smallCaps', CaseSensitiveIcon, 'text.smallCaps'],
  ['superscript', SuperscriptIcon, 'text.superscript'],
  ['subscript', SubscriptIcon, 'text.subscript'],
]

/** The Paragraph panel's seven alignments (`justify`). */
const JUSTIFY: ReadonlyArray<readonly [value: string, icon: Icon, label: string]> = [
  ['left', TextAlignStartIcon, 'text.left'],
  ['center', TextAlignCenterIcon, 'text.center'],
  ['right', TextAlignEndIcon, 'text.right'],
  ['justifyLeft', TextJustifyStartIcon, 'text.justifyLeft'],
  ['justifyCenter', TextJustifyCenterIcon, 'text.justifyCenter'],
  ['justifyRight', TextJustifyEndIcon, 'text.justifyRight'],
  ['justifyAll', TextAlignJustifyIcon, 'text.justifyAll'],
]

/** A row of icon buttons, each pressed, not pressed or mixed (`aria-pressed`). */
function ToggleRow({ items, label, testId, readOnly }: { items: ReadonlyArray<{ id: string; icon: Icon; label: string; pressed: boolean | 'mixed'; onPress: () => void }>; label: string; testId: string; readOnly: boolean }) {
  return (
    <div className="motion-text-toggles" role="group" aria-label={label} data-testid={testId}>
      {items.map((item) => {
        const Shown = item.icon
        return (
          <button
            key={item.id}
            type="button"
            className="motion-text-toggle"
            aria-pressed={item.pressed}
            aria-label={item.label}
            title={item.label}
            disabled={readOnly}
            onClick={item.onPress}
            data-testid={`${testId}-${item.id}`}
          >
            <Shown size={14} aria-hidden="true" />
          </button>
        )
      })}
    </div>
  )
}

/**
 * A text layer's Text section: the content box, then the Character settings (font family,
 * searchable, and style; size and leading, auto or set; tracking and kerning; fill and stroke with
 * its width; baseline shift and horizontal and vertical scale; faux bold and italic, all caps and
 * small caps, superscript and subscript) and the Paragraph settings (seven alignments, the left,
 * right and first-line indents, the space before and after). They show the characters being edited
 * on the canvas (the selected ones, or at a caret what the next typed text gets), else the whole
 * text; a setting that differs across them shows as mixed (an empty field with a dash, a striped
 * swatch, a half-pressed button). A change applies to the same characters, each one undo step
 * (`layer.setText` with their range).
 */
export function TextControls({ node, focusNow, onFocused, selection }: { node: TreeNode; focusNow: boolean; onFocused: () => void; selection: TextSelection | null }) {
  const { t } = useKitEditor()
  const { layer, readOnly } = usePanel()
  const source = (node.children ?? []).find((child) => child.match === 'sourceText')
  const fonts = useEngineQuery<Fonts>('text.fonts', {})
  const families = fonts?.families ?? []
  // The font search: words of a family's name that start with what is typed, whole-name prefixes first.
  const [fontQuery, setFontQuery] = useState('')
  const send = useSender('layer.setText')
  const sendScrub = useSender('layer.setText')
  if (!selection) return <div className="motion-text" data-testid="motion-text-controls">{source ? <ContentBox key={layer.id} node={source} focusNow={focusNow} onFocused={onFocused} /> : null}</div>
  const c = selection.character
  const p = selection.paragraph
  const mixed = new Set(selection.mixed)
  const num = (value: unknown, fallback = 0) => (typeof value === 'number' && Number.isFinite(value) ? value : fallback)
  const range = selection.range ? { range: selection.range } : {}
  const set = (params: Record<string, unknown>, gesture?: string) => (gesture ? sendScrub : send)({ layer: layer.id, ...range, ...params }, gesture)
  const family = typeof c.font === 'string' ? c.font : ''
  const style = typeof c.style === 'string' ? c.style : ''
  const styles = families.find((each) => each.family === family)?.styles ?? (style ? [style] : [])
  const size = num(c.size, 72)
  const autoLeading = c.leading === 'auto'
  const leading = autoLeading ? size * 1.2 : num(c.leading, size * 1.2)
  const kerning = c.kerning === 'optical' ? 'optical' : c.kerning === 'metrics' ? 'metrics' : mixed.has('kerning') ? '' : 'manual'
  const flag = (key: string): boolean | 'mixed' => (mixed.has(key) ? 'mixed' : c[key] === true)
  const baseline = typeof c.baseline === 'string' ? c.baseline : 'normal'
  const field = (key: string, label: string, value: number, options: { min?: number; max?: number; decimals?: number; unit?: string; speed?: number } = {}) => (
    <Item label={label}>
      <NumberField
        value={value}
        mixed={mixed.has(key)}
        label={label}
        min={options.min}
        max={options.max}
        decimals={options.decimals ?? 1}
        unit={options.unit}
        speed={options.speed}
        readOnly={readOnly}
        onCommit={(next) => set({ [key]: next })}
        onScrub={(next, _final, gesture) => set({ [key]: next }, gesture)}
        testId={`motion-text-${key}`}
      />
    </Item>
  )
  return (
    <div className="motion-text" data-testid="motion-text-controls" data-editing={selection.editing || undefined}>
      {source ? <ContentBox key={layer.id} node={source} focusNow={focusNow} onFocused={onFocused} /> : null}
      {selection.editing ? (
        <p className="kui-muted motion-text-scope" data-testid="motion-text-scope">
          {selection.range && selection.range[0] !== selection.range[1] ? t('text.scope.selection', { count: selection.range[1] - selection.range[0] }) : t('text.scope.caret')}
        </p>
      ) : null}
      <h4 className="motion-text-heading kui-muted">{t('text.character')}</h4>
      <div className="motion-text-grid">
        <Item label={t('text.font')} wide>
          <Combobox
            value={mixed.has('font') ? '' : family}
            label={t('text.font')}
            options={fontMatches(families, fontQuery).map((each) => ({ value: each.family, label: each.family, text: each.family }))}
            onSearch={setFontQuery}
            disabled={readOnly}
            onChange={(next) => {
              if (!next) return
              const nextStyles = families.find((entry) => entry.family === next)?.styles ?? []
              set({ font: next, style: nextStyles.includes(style) ? style : (nextStyles[0] ?? 'Regular') })
            }}
            className="motion-text-font"
            testId="motion-text-font"
          />
        </Item>
        <Item label={t('text.style')}>
          <Select value={mixed.has('style') ? '' : style} disabled={readOnly} onChange={(event) => set({ font: family, style: event.target.value })} data-testid="motion-text-style">
            {mixed.has('style') ? <option value="">{t('text.mixed')}</option> : null}
            {styles.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </Select>
        </Item>
        {field('size', t('text.size'), size, { min: 1, max: 5000, unit: 'px' })}
        <Item label={t('text.leading')}>
          <span className="motion-text-pair">
            <NumberField
              value={leading}
              mixed={mixed.has('leading')}
              label={t('text.leading')}
              min={0}
              max={5000}
              decimals={1}
              unit="px"
              readOnly={readOnly}
              onCommit={(next) => set({ leading: next })}
              onScrub={(next, _final, gesture) => set({ leading: next }, gesture)}
              testId="motion-text-leading"
            />
            <button
              type="button"
              className="motion-text-toggle motion-text-auto"
              aria-pressed={mixed.has('leading') ? 'mixed' : autoLeading}
              title={t('text.autoLeading')}
              disabled={readOnly}
              onClick={() => set({ leading: autoLeading ? Math.round(leading * 10) / 10 : 'auto' })}
              data-testid="motion-text-leading-auto"
            >
              {t('text.auto')}
            </button>
          </span>
        </Item>
        {field('tracking', t('text.tracking'), num(c.tracking), { min: -1000, max: 10000, decimals: 0 })}
        <Item label={t('text.kerning')}>
          <Select value={kerning} disabled={readOnly} onChange={(event) => set({ kerning: event.target.value === 'manual' ? 0 : event.target.value })} data-testid="motion-text-kerning">
            {kerning === '' ? <option value="">{t('text.mixed')}</option> : null}
            <option value="metrics">{t('text.kerning.metrics')}</option>
            <option value="optical">{t('text.kerning.optical')}</option>
            <option value="manual">{t('text.kerning.none')}</option>
          </Select>
        </Item>
        <Item label={t('text.fill')}>
          <span className="motion-text-stroke">
            <ColorSwatch value={hexOf(c.fill)} mixed={mixed.has('fill')} label={t('text.fill')} readOnly={readOnly} onChange={(hex, final) => set({ fill: hex, applyFill: true }, final ? undefined : `fill-${layer.id}`)} testId="motion-text-fill" />
            <button
              type="button"
              className="motion-text-toggle motion-text-auto"
              aria-pressed={mixed.has('applyFill') ? 'mixed' : c.applyFill !== false}
              title={t('text.applyFill')}
              disabled={readOnly}
              onClick={() => set({ applyFill: c.applyFill === false })}
              data-testid="motion-text-apply-fill"
            >
              {mixed.has('applyFill') ? '—' : c.applyFill === false ? t('text.off') : t('text.on')}
            </button>
          </span>
        </Item>
        <Item label={t('text.stroke')}>
          <span className="motion-text-stroke">
            <ColorSwatch value={hexOf(c.stroke)} mixed={mixed.has('stroke')} label={t('text.stroke')} readOnly={readOnly} onChange={(hex, final) => set({ stroke: hex, applyStroke: true }, final ? undefined : `stroke-${layer.id}`)} testId="motion-text-stroke" />
            <NumberField
              value={c.applyStroke === true ? num(c.strokeWidth) : 0}
              mixed={mixed.has('strokeWidth') || mixed.has('applyStroke')}
              label={t('text.strokeWidth')}
              min={0}
              max={1000}
              decimals={1}
              unit="px"
              readOnly={readOnly}
              onCommit={(next) => set({ strokeWidth: next, applyStroke: next > 0 })}
              onScrub={(next, _final, gesture) => set({ strokeWidth: next, applyStroke: next > 0 }, gesture)}
              testId="motion-text-stroke-width"
            />
          </span>
        </Item>
        {field('baselineShift', t('text.baselineShift'), num(c.baselineShift), { min: -5000, max: 5000, unit: 'px' })}
        {field('hScale', t('text.hScale'), num(c.hScale, 100), { min: 1, max: 1000, unit: '%' })}
        {field('vScale', t('text.vScale'), num(c.vScale, 100), { min: 1, max: 1000, unit: '%' })}
        <Item label={t('text.styles')} wide>
          <ToggleRow
            label={t('text.styles')}
            testId="motion-text-toggles"
            readOnly={readOnly}
            items={TOGGLES.map(([key, icon, label]) => {
              const pressed = key === 'superscript' ? (mixed.has('baseline') ? 'mixed' : baseline === 'superscript') : key === 'subscript' ? (mixed.has('baseline') ? 'mixed' : baseline === 'subscript') : flag(key)
              return { id: key, icon, label: t(label), pressed, onPress: () => set({ [key]: pressed !== true }) }
            })}
          />
        </Item>
      </div>
      <h4 className="motion-text-heading kui-muted">{t('text.paragraph')}</h4>
      <div className="motion-text-grid">
        <Item label={t('text.align')} wide>
          <ToggleRow
            label={t('text.align')}
            testId="motion-text-align"
            readOnly={readOnly}
            items={JUSTIFY.map(([value, icon, label]) => ({
              id: value,
              icon,
              label: t(label),
              pressed: mixed.has('justify') ? false : p.justify === value,
              onPress: () => set({ justify: value }),
            }))}
          />
        </Item>
        {paraField('indentLeft', t('text.indentLeft'))}
        {paraField('indentRight', t('text.indentRight'))}
        {paraField('indentFirst', t('text.indentFirst'))}
        {paraField('spaceBefore', t('text.spaceBefore'))}
        {paraField('spaceAfter', t('text.spaceAfter'))}
      </div>
    </div>
  )

  function paraField(key: string, label: string) {
    return (
      <Item label={label} key={key}>
        <NumberField
          value={num(p[key])}
          mixed={mixed.has(key)}
          label={label}
          min={-5000}
          max={5000}
          decimals={1}
          unit="px"
          readOnly={readOnly}
          onCommit={(next) => set({ [key]: next })}
          onScrub={(next, _final, gesture) => set({ [key]: next }, gesture)}
          testId={`motion-text-${key}`}
        />
      </Item>
    )
  }
}
