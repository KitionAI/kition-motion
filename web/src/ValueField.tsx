import { useEffect, useRef, useState } from 'react'

/** A value as the field shows it: at most `decimals` decimals. */
const rounded = (value: number, decimals: number) => {
  const scale = 10 ** decimals
  return Math.round(value * scale) / scale
}

/**
 * A value's text in at most `chars` characters: as many of its decimals as fit (a whole number
 * longer than that stays whole). Text that is not a number is left as it is.
 */
export const fitted = (text: string, chars: number) => {
  const value = Number(text)
  if (text.trim() === '' || !Number.isFinite(value) || text.length <= chars) return text
  const decimals = text.includes('.') ? text.length - text.indexOf('.') - 1 : 0
  for (let shown = decimals - 1; shown >= 0; shown -= 1) {
    const next = String(rounded(value, shown))
    if (next.length <= chars) return next
  }
  return String(Math.round(value))
}

/** How far the pointer moves before a press on the field becomes a scrub, in CSS pixels. */
const SCRUB_START = 3

/** One drag on a number: where it started, and the gesture's name (one undo step per gesture). */
type Scrub = { pointerId: number; x: number; from: number; last: number; moved: boolean; gesture: string }

export type NumberFieldProps = {
  value: number
  label: string
  onCommit: (value: number) => void
  testId: string
  readOnly: boolean
  /**
   * Dragged sideways the number scrubs: called per move with `final: false` and once more with
   * `final: true` on release, all with the same `gesture` (so the edit is one undo step).
   * Without it, the field only takes typing.
   */
  onScrub?: (value: number, final: boolean, gesture: string) => void
  /** Value units per CSS pixel dragged (Shift ten times faster, Alt ten times finer). */
  speed?: number
  min?: number
  max?: number
  /** Decimals shown (2 by default). */
  decimals?: number
  unit?: string
  /** The selection's characters (or layers) differ here: the field is empty, shows a dash, and takes a typed value for all. */
  mixed?: boolean
  /**
   * A narrow field (the timeline's three numbers): while it is not being typed in it shows at most
   * this many characters, dropping decimals; typing, and its tooltip, show the whole value.
   */
  fit?: number
}

let gestures = 0

/**
 * One number of a property (the Properties panel's and the timeline's): commits on Enter or
 * blur, Escape drops the typed text. With `onScrub`, a press dragged left or right scrubs the
 * value (Shift ×10, Alt ×0.1); a press without a drag starts typing.
 */
export function NumberField({ value, label, onCommit, testId, readOnly, onScrub, speed = 1, min = -Infinity, max = Infinity, decimals = 2, unit, mixed = false, fit }: NumberFieldProps) {
  const show = (next: number) => (mixed && next === value ? '' : String(rounded(next, decimals)))
  const [text, setText] = useState(show(value))
  const scrub = useRef<Scrub | null>(null)
  const input = useRef<HTMLInputElement>(null)
  const [focused, setFocused] = useState(false)
  // The field holds the whole value in `text`; a narrow one shows it shortened until it is focused.
  const shown = fit && !focused ? fitted(text, fit) : text
  useEffect(() => {
    if (!scrub.current?.moved) setText(show(value))
  }, [value, mixed]) // eslint-disable-line react-hooks/exhaustive-deps
  const commit = () => {
    const next = Number(text.trim().replace(',', '.'))
    if (text.trim() !== '' && Number.isFinite(next) && (mixed || next !== rounded(value, decimals))) onCommit(Math.min(max, Math.max(min, next)))
    else setText(show(value))
  }
  const scrubbing = !!onScrub && !readOnly
  return (
    <span className="motion-number-wrap" data-scrub={scrubbing || undefined}>
      <input
        ref={input}
        className="kui-field motion-number"
        aria-label={label}
        value={shown}
        title={fit && shown !== text ? `${label}: ${text}` : undefined}
        data-value={fit ? text : undefined}
        placeholder={mixed ? '—' : undefined}
        data-mixed={mixed || undefined}
        inputMode="decimal"
        readOnly={readOnly}
        aria-readonly={readOnly}
        onChange={(event) => setText(event.target.value)}
        onFocus={(event) => {
          if (!fit) return
          // Typing shows the whole value; where it replaces the shortened one, select it once it is there.
          setFocused(true)
          if (shown === text) return
          const field = event.currentTarget
          requestAnimationFrame(() => {
            if (document.activeElement === field) field.select()
          })
        }}
        onBlur={() => {
          setFocused(false)
          commit()
        }}
        onPointerDown={(event) => {
          // A field being typed in keeps its pointer for selecting text.
          if (!scrubbing || event.button !== 0 || document.activeElement === event.currentTarget) return
          event.preventDefault()
          gestures += 1
          scrub.current = { pointerId: event.pointerId, x: event.clientX, from: value, last: value, moved: false, gesture: `scrub-${Date.now()}-${gestures}` }
          event.currentTarget.setPointerCapture(event.pointerId)
        }}
        onPointerMove={(event) => {
          const current = scrub.current
          if (!current || current.pointerId !== event.pointerId) return
          const dx = event.clientX - current.x
          if (!current.moved && Math.abs(dx) < SCRUB_START) return
          current.moved = true
          const factor = event.shiftKey ? 10 : event.altKey ? 0.1 : 1
          const next = Math.min(max, Math.max(min, rounded(current.from + dx * speed * factor, decimals)))
          if (next === current.last) return
          current.last = next
          setText(show(next))
          onScrub?.(next, false, current.gesture)
        }}
        onPointerUp={(event) => {
          const current = scrub.current
          if (!current || current.pointerId !== event.pointerId) return
          scrub.current = null
          event.currentTarget.releasePointerCapture(event.pointerId)
          if (current.moved) {
            if (current.last !== current.from) onScrub?.(current.last, true, current.gesture)
            return
          }
          // A click without a drag: type into the field.
          event.currentTarget.focus()
          event.currentTarget.select()
        }}
        onPointerCancel={() => {
          scrub.current = null
          setText(show(value))
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') event.currentTarget.blur()
          if (event.key === 'Escape') {
            // The typed value is dropped: the editor (and Focus) does not hear this Escape as well.
            event.preventDefault()
            setText(show(value))
            event.currentTarget.blur()
          }
        }}
        data-testid={testId}
      />
      {unit ? <span className="motion-number-unit kui-muted">{unit}</span> : null}
    </span>
  )
}
