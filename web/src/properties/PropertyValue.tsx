import { useEffect, useRef, useState } from 'react'
import { LinkIcon as Linked, UnlinkIcon as Unlinked } from '@kition/tessera/icons/react'
import { Checkbox, GradientSlider, IconButton, Select, TextInput, useKitEditor } from '@kition/sdk/editor'
import type { MotionLayer } from '../motion'
import { editableNumbers, type TreeNode } from '../timeline/tree'
import { NumberField } from '../ValueField'
import { GradientEditor } from './GradientEditor'
import { ColorSwatch, hexOf } from './ColorSwatch'
import { LAYER_SOURCE_OPTIONS, type PropMeta } from './meta'
import { useSetProp } from './useSetProp'

/** Whether Scale keeps its proportions, for the session (on by default). */
let scaleLinked = true

/** The characters a number shows in the timeline's three-number rows while it is not typed in. */
const FIT_CHARS = 6

/** Properties whose two numbers can be linked to keep proportions. */
const LINKABLE = new Set(['scale'])

/** How far a scrub moves a number per CSS pixel: a slider's range over 200 pixels, else one unit. */
function speedOf(meta: PropMeta | null) {
  if (meta?.slider) return Math.max(10 ** -(meta.decimals ?? 2), (meta.slider[1] - meta.slider[0]) / 200)
  return 1
}

/**
 * The value control of one property, by its type (the Properties panel's and the timeline's
 * twirl-down rows share it): numbers that type and scrub (a slider under a scalar with a range),
 * x and y (z on a 3D layer) with a link for Scale, a colour swatch with its picker in a popover,
 * a drop-down with the option labels, a check box, or a text field. Each edit is one undo step,
 * at the current time (a keyframe there when the property is animated).
 */
export function PropertyValue({ layer, node, meta, time, name, testId, readOnly, compact = false }: { layer: MotionLayer; node: TreeNode; meta: PropMeta | null; time: number; name: string; testId: string; readOnly: boolean; compact?: boolean }) {
  const { t, phrase } = useKitEditor()
  const set = useSetProp(layer.id, node.path, time)
  const [linked, setLinked] = useState(scaleLinked)
  const numbers = editableNumbers(node, layer)
  if (numbers) {
    const decimals = meta?.decimals ?? 2
    const parts = Array.isArray(node.value) ? (node.value as number[]) : null
    const linkable = !!parts && LINKABLE.has(node.match ?? '')
    const valueAt = (index: number, next: number) => {
      if (!parts) return next
      const old = parts[index] ?? 0
      if (linkable && linked && old !== 0) return parts.map((part, at) => (at === index ? next : (part * next) / old))
      return parts.map((part, at) => (at === index ? next : part))
    }
    const slider = !compact && node.type === 'scalar' ? meta?.slider : undefined
    return (
      <span className="motion-prop-values" data-kind={slider ? 'slider' : undefined} data-count={numbers.length}>
        {numbers.map((value, index) => (
          <NumberField
            key={index}
            value={value}
            label={numbers.length > 1 ? `${name} ${'XYZ'[index]}` : name}
            onCommit={(next) => set(valueAt(index, next))}
            onScrub={(next, _final, gesture) => set(valueAt(index, next), gesture)}
            speed={speedOf(meta)}
            min={meta?.min}
            max={meta?.max}
            decimals={decimals}
            unit={compact || numbers.length > 1 ? undefined : meta?.unit}
            testId={`${testId}-${index}`}
            readOnly={readOnly}
            fit={compact && numbers.length > 2 ? FIT_CHARS : undefined}
          />
        ))}
        {linkable && !compact ? (
          <IconButton
            icon={linked ? Linked : Unlinked}
            label={t('prop.link')}
            pressed={linked}
            onClick={() => {
              scaleLinked = !linked
              setLinked(!linked)
            }}
            data-testid={`${testId}-link`}
          />
        ) : null}
        {slider && meta ? (
          <span className="motion-prop-slider">
            <GradientSlider
              value={numbers[0] ?? 0}
              min={meta.min ?? slider[0]}
              max={meta.max ?? slider[1]}
              softMin={slider[0]}
              softMax={slider[1]}
              decimals={decimals}
              default={typeof meta.fallback === 'number' ? meta.fallback : undefined}
              label={name}
              disabled={readOnly}
              onChange={(next, phase) => set(next, phase === 'end' ? undefined : `slider-${node.path}`)}
              testId={`${testId}-slider`}
            />
          </span>
        ) : null}
      </span>
    )
  }
  if (node.type === 'gradient') {
    // The timeline's rows are too narrow for the stops: the Properties panel edits them.
    if (compact) return null
    return <GradientEditor value={node.value} name={name} testId={testId} readOnly={readOnly} set={set} />
  }
  if (node.type === 'color') {
    return (
      <span className="motion-prop-values">
        <ColorSwatch value={hexOf(node.value)} label={name} onChange={(hex, final) => set(hex, final ? undefined : `color-${node.path}`)} testId={testId} readOnly={readOnly} />
      </span>
    )
  }
  if (node.type === 'enum') {
    const stored = typeof node.value === 'number' ? node.value : 0
    const options: NonNullable<PropMeta['options']> = meta?.options ?? Array.from({ length: Math.max(stored + 1, 10) }, (_, at) => ({ label: String(at), value: at }))
    // A layer source's values from 2 up all mean Effects & Masks.
    const value = options === LAYER_SOURCE_OPTIONS ? Math.min(stored, 2) : stored
    return (
      <span className="motion-prop-values">
        <Select value={String(value)} aria-label={name} disabled={readOnly} onChange={(event) => set(Number(event.target.value))} data-testid={testId}>
          {options.map((option) => (
            <option key={option.value} value={String(option.value)}>
              {option.key ? t(option.key) : phrase(option.label)}
            </option>
          ))}
        </Select>
      </span>
    )
  }
  if (node.type === 'bool') {
    return (
      <span className="motion-prop-values">
        <Checkbox label={<span className="motion-hidden-label">{name}</span>} checked={node.value === true} disabled={readOnly} onChange={(checked) => set(checked)} testId={testId} />
      </span>
    )
  }
  if (node.type === 'text' || node.type === 'string') {
    return (
      <span className="motion-prop-values">
        <TextField value={typeof node.value === 'string' ? node.value : ''} label={name} onCommit={(text) => set(text)} testId={testId} readOnly={readOnly} />
      </span>
    )
  }
  return null
}

/** A one-line text value: commits on Enter or blur, Escape drops the typed text. */
function TextField({ value, label, onCommit, testId, readOnly }: { value: string; label: string; onCommit: (text: string) => void; testId: string; readOnly: boolean }) {
  const [text, setText] = useState(value)
  const typing = useRef(false)
  const dropped = useRef(false)
  useEffect(() => {
    if (!typing.current) setText(value)
  }, [value])
  return (
    <TextInput
      value={text}
      aria-label={label}
      readOnly={readOnly}
      onFocus={() => (typing.current = true)}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => {
        typing.current = false
        if (!dropped.current && text !== value) onCommit(text)
        dropped.current = false
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') event.currentTarget.blur()
        if (event.key === 'Escape') {
          event.preventDefault()
          dropped.current = true
          setText(value)
          event.currentTarget.blur()
        }
      }}
      data-testid={testId}
    />
  )
}
