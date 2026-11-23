import { useEffect, useState } from 'react'

const round = (value: number) => Math.round(value * 100) / 100

/** One number of a property (the Properties panel's and the timeline's); commits on Enter or blur, Escape drops the typed text. */
export function NumberField({ value, label, onCommit, testId, readOnly }: { value: number; label: string; onCommit: (value: number) => void; testId: string; readOnly: boolean }) {
  const [text, setText] = useState(String(round(value)))
  useEffect(() => setText(String(round(value))), [value])
  const commit = () => {
    const next = Number(text)
    if (Number.isFinite(next) && next !== round(value)) onCommit(next)
    else setText(String(round(value)))
  }
  return (
    <input
      className="kui-field motion-number"
      aria-label={label}
      value={text}
      inputMode="decimal"
      readOnly={readOnly}
      aria-readonly={readOnly}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === 'Enter') event.currentTarget.blur()
        if (event.key === 'Escape') {
          // The typed value is dropped: the editor (and Focus) does not hear this Escape as well.
          event.preventDefault()
          setText(String(round(value)))
          event.currentTarget.blur()
        }
      }}
      data-testid={testId}
    />
  )
}
