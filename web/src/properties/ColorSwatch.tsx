import { useEffect, useState } from 'react'
import { ColorPicker, Popover, PopoverContent, PopoverTrigger, TextInput, parseHex, rgbToHex, useKitEditor } from '@kition/sdk/editor'

/** A colour as the engine gives it, [r, g, b] or [r, g, b, a] from 0 to 1, as "#rrggbb". */
export function hexOf(value: unknown): string {
  if (typeof value === 'string' && parseHex(value)) return rgbToHex(parseHex(value) ?? [0, 0, 0])
  const parts = Array.isArray(value) ? value : [0, 0, 0]
  const byte = (at: number) => Math.round(Math.min(1, Math.max(0, Number(parts[at]) || 0)) * 255)
  return rgbToHex([byte(0), byte(1), byte(2)])
}

/**
 * A colour's swatch: a click opens the colour picker in a popover beside it, with the hex to type.
 * `onChange` hears every move of the picker (`final: false`) and once more when it ends; typing a
 * hex calls it with `final: true`.
 */
export function ColorSwatch({ value, label, onChange, testId, readOnly = false }: { value: string; label: string; onChange: (hex: string, final: boolean) => void; testId: string; readOnly?: boolean }) {
  const { t } = useKitEditor()
  const [open, setOpen] = useState(false)
  const [text, setText] = useState(value)
  useEffect(() => setText(value), [value])
  // The picker shows what it was dragged to until the engine's value catches up.
  const rgb = parseHex(text) ?? parseHex(value) ?? [0, 0, 0]
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild disabled={readOnly}>
        <button type="button" className="motion-swatch" style={{ background: value }} aria-label={`${label}: ${value}`} title={t('dialog.pickColor')} data-testid={`${testId}-swatch`} />
      </PopoverTrigger>
      <PopoverContent
        className="motion-color-popover"
        align="start"
        onEscapeKeyDown={(event) => {
          // Escape closes the picker only, not a dialog it is in.
          event.stopPropagation()
        }}
        data-testid={`${testId}-popover`}
      >
        <ColorPicker
          value={rgb}
          onChange={(next, final) => {
            const hex = rgbToHex(next)
            setText(hex)
            onChange(hex, final)
          }}
          testId={`${testId}-picker`}
        />
        <TextInput
          value={text}
          aria-label={label}
          invalid={!parseHex(text)}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              event.currentTarget.blur()
            }
          }}
          onBlur={() => {
            const hex = parseHex(text)
            if (hex && rgbToHex(hex) !== value) onChange(rgbToHex(hex), true)
            else setText(value)
          }}
          data-testid={`${testId}-hex`}
        />
      </PopoverContent>
    </Popover>
  )
}
