import { useEffect, useState } from 'react'
import { ColorPicker, Popover, PopoverContent, PopoverTrigger, parseHex, rgbToHex, useKitEditor } from '@kition/sdk/editor'

/** A colour as the engine gives it, [r, g, b] or [r, g, b, a] from 0 to 1, as "#rrggbb". */
export function hexOf(value: unknown): string {
  if (typeof value === 'string' && parseHex(value)) return rgbToHex(parseHex(value) ?? [0, 0, 0])
  const parts = Array.isArray(value) ? value : [0, 0, 0]
  const byte = (at: number) => Math.round(Math.min(1, Math.max(0, Number(parts[at]) || 0)) * 255)
  return rgbToHex([byte(0), byte(1), byte(2)])
}

/**
 * A colour's swatch: a click opens the colour picker (with its hex and RGB fields) in a popover
 * beside it. `onChange` hears every move of the picker (`final: false`) and once more when it ends.
 */
export function ColorSwatch({ value, label, onChange, testId, readOnly = false, mixed = false }: { value: string; label: string; onChange: (hex: string, final: boolean) => void; testId: string; readOnly?: boolean; mixed?: boolean }) {
  const { t } = useKitEditor()
  const [open, setOpen] = useState(false)
  // The picker shows what it was dragged to until the engine's value catches up.
  const [shown, setShown] = useState(value)
  useEffect(() => setShown(value), [value])
  const rgb = parseHex(shown) ?? parseHex(value) ?? [0, 0, 0]
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild disabled={readOnly}>
        <button
          type="button"
          className="motion-swatch"
          style={{ background: mixed ? undefined : value }}
          data-mixed={mixed || undefined}
          aria-label={`${label}: ${mixed ? t('text.mixed') : value}`}
          title={t('dialog.pickColor')}
          data-testid={`${testId}-swatch`}
        />
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
            setShown(hex)
            onChange(hex, final)
          }}
          testId={`${testId}-picker`}
        />
      </PopoverContent>
    </Popover>
  )
}
