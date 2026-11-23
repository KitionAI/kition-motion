import { useRef, useState, type ReactNode } from 'react'
import { Button, Dialog, Field, Select, TextInput, formatTimecode, parseHex, parseTimecode, rgbToHex, useKitEditor } from '@kition/sdk/editor'
import { ColorSwatch } from './properties/ColorSwatch'

/** A colour as the engine gives it: [r, g, b] from 0 to 1. */
export type Rgb = readonly [number, number, number]

export const toHex = (rgb: Rgb) => rgbToHex([Math.round(rgb[0] * 255), Math.round(rgb[1] * 255), Math.round(rgb[2] * 255)])

/** The composition a Composition dialog starts with. */
export type CompValues = { name: string; width: number; height: number; frameRate: number; duration: number; background: string }

/** The solid a Solid dialog starts with; `comp` is the active composition's size, for Make Comp Size. */
export type SolidValues = { name: string; width: number; height: number; color: string; comp: { width: number; height: number } }

/** The editor's own dialogs: what each asks and what it starts with. */
export type MotionDialog =
  | { kind: 'comp'; mode: 'new' | 'settings'; values: CompValues }
  | { kind: 'solid'; mode: 'new' | 'settings'; layer?: number; values: SolidValues }
  | { kind: 'time'; time: number; fps: number; duration: number }
  | { kind: 'rename'; layer: number; name: string }

/** Frame sizes a composition is often made at: [label key, width, height]. */
const SIZE_PRESETS: ReadonlyArray<readonly [string, number, number]> = [
  ['hd1080', 1920, 1080],
  ['uhd', 3840, 2160],
  ['hd720', 1280, 720],
  ['square', 1080, 1080],
  ['portrait', 1080, 1920],
  ['social', 1080, 1350],
]

const FRAME_RATES = [23.976, 24, 25, 29.97, 30, 50, 59.94, 60]

/** A number typed in a field, or null when it is not one within [min, max]. */
function numberIn(text: string, min: number, max: number) {
  const value = Number(text.trim().replace(',', '.'))
  return text.trim() !== '' && Number.isFinite(value) && value >= min && value <= max ? value : null
}

/** A dialog whose fields submit with Enter; Escape or Cancel closes it. */
function FormDialog({ title, testId, onSubmit, onClose, children, valid }: { title: string; testId: string; onSubmit: () => void; onClose: () => void; children: ReactNode; valid: boolean }) {
  const { t } = useKitEditor()
  return (
    <Dialog
      title={title}
      testId={testId}
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose} data-testid={`${testId}-cancel`}>
            {t('dialog.cancel')}
          </Button>
          <Button variant="primary" disabled={!valid} onClick={onSubmit} data-testid={`${testId}-ok`}>
            {t('dialog.ok')}
          </Button>
        </>
      }
    >
      <form
        className="motion-form"
        onSubmit={(event) => {
          event.preventDefault()
          if (valid) onSubmit()
        }}
      >
        {children}
        <button type="submit" hidden />
      </form>
    </Dialog>
  )
}

/** A text field with its label and its error. */
function TextRow({ label, value, onChange, error, testId, autoFocus, unit }: { label: string; value: string; onChange: (text: string) => void; error?: string | null; testId: string; autoFocus?: boolean; unit?: string }) {
  return (
    <Field label={label} error={error ?? undefined}>
      {({ id, describedBy, invalid }) => (
        <span className="motion-form-input">
          <TextInput id={id} aria-describedby={describedBy} invalid={invalid} value={value} onChange={(event) => onChange(event.target.value)} autoFocus={autoFocus} onFocus={(event) => event.target.select()} data-testid={testId} />
          {unit ? <span className="kui-muted">{unit}</span> : null}
        </span>
      )}
    </Field>
  )
}

/** A colour: its swatch opens the picker in a popover beside it, and its hex can be typed. */
function ColorRow({ label, value, onChange, testId }: { label: string; value: string; onChange: (hex: string) => void; testId: string }) {
  const { t } = useKitEditor()
  const [text, setText] = useState(value)
  return (
    <Field label={label} error={parseHex(text) ? undefined : t('dialog.badColor')}>
      {({ id, describedBy, invalid }) => (
        <span className="motion-form-input">
          <ColorSwatch
            value={value}
            label={label}
            onChange={(hex) => {
              setText(hex)
              onChange(hex)
            }}
            testId={testId}
          />
          <TextInput
            id={id}
            aria-describedby={describedBy}
            invalid={invalid}
            value={text}
            onChange={(event) => {
              setText(event.target.value)
              const hex = parseHex(event.target.value)
              if (hex) onChange(rgbToHex(hex))
            }}
            data-testid={testId}
          />
        </span>
      )}
    </Field>
  )
}

/** New Composition and Composition Settings: name, size (with presets), frame rate, duration and background. */
function CompDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'comp' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const start = dialog.values
  const [name, setName] = useState(start.name)
  const [width, setWidth] = useState(String(start.width))
  const [height, setHeight] = useState(String(start.height))
  const [rate, setRate] = useState(String(Math.round(start.frameRate * 1000) / 1000))
  const [duration, setDuration] = useState(String(Math.round(start.duration * 1000) / 1000))
  const [background, setBackground] = useState(start.background)
  const w = numberIn(width, 4, 30000)
  const h = numberIn(height, 4, 30000)
  const fps = numberIn(rate, 1, 999)
  const seconds = numberIn(duration, 0.001, 10800)
  const valid = name.trim() !== '' && w !== null && h !== null && fps !== null && seconds !== null && parseHex(background) !== null
  const preset = SIZE_PRESETS.find(([, pw, ph]) => pw === w && ph === h)?.[0] ?? ''
  const submit = () => {
    if (!valid) return
    const params = { name: name.trim(), width: Math.round(w), height: Math.round(h), frameRate: fps, duration: seconds, background }
    void run(dialog.mode === 'new' ? 'comp.new' : 'comp.settings', params)
    onDone()
  }
  const invalid = t('dialog.badNumber')
  return (
    <FormDialog title={t(dialog.mode === 'new' ? 'dialog.newComp' : 'dialog.compSettings')} testId="motion-comp-dialog" onSubmit={submit} onClose={onDone} valid={valid}>
      <TextRow label={t('dialog.name')} value={name} onChange={setName} error={name.trim() ? null : t('dialog.needName')} testId="motion-comp-name" autoFocus />
      <Field label={t('dialog.preset')}>
        <Select
          value={preset}
          onChange={(event) => {
            const chosen = SIZE_PRESETS.find(([key]) => key === event.target.value)
            if (!chosen) return
            setWidth(String(chosen[1]))
            setHeight(String(chosen[2]))
          }}
          data-testid="motion-comp-preset"
        >
          <option value="">{t('dialog.custom')}</option>
          {SIZE_PRESETS.map(([key, pw, ph]) => (
            <option key={key} value={key}>
              {t(`preset.${key}`)} · {pw} × {ph}
            </option>
          ))}
        </Select>
      </Field>
      <div className="motion-form-pair">
        <TextRow label={t('dialog.width')} value={width} onChange={setWidth} error={w === null ? invalid : null} testId="motion-comp-width" unit="px" />
        <TextRow label={t('dialog.height')} value={height} onChange={setHeight} error={h === null ? invalid : null} testId="motion-comp-height" unit="px" />
      </div>
      <div className="motion-form-pair">
        <Field label={t('dialog.frameRate')} error={fps === null ? invalid : undefined}>
          {({ id, invalid: bad }) => (
            <span className="motion-form-input">
              <TextInput id={id} invalid={bad} value={rate} onChange={(event) => setRate(event.target.value)} data-testid="motion-comp-fps" />
              <span className="kui-muted">fps</span>
            </span>
          )}
        </Field>
        <TextRow label={t('dialog.duration')} value={duration} onChange={setDuration} error={seconds === null ? invalid : null} testId="motion-comp-duration" unit={t('dialog.seconds')} />
      </div>
      <Field label={t('dialog.frameRatePreset')}>
        <Select value={FRAME_RATES.includes(fps ?? -1) ? String(fps) : ''} onChange={(event) => event.target.value && setRate(event.target.value)} data-testid="motion-comp-fps-preset">
          <option value="">{t('dialog.custom')}</option>
          {FRAME_RATES.map((value) => (
            <option key={value} value={String(value)}>
              {value} fps
            </option>
          ))}
        </Select>
      </Field>
      <ColorRow label={t('dialog.background')} value={background} onChange={setBackground} testId="motion-comp-background" />
    </FormDialog>
  )
}

/** New Solid and Solid Settings: name, colour, and size (Make Comp Size fills in the composition's). */
function SolidDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'solid' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const start = dialog.values
  const [name, setName] = useState(start.name)
  const [width, setWidth] = useState(String(start.width))
  const [height, setHeight] = useState(String(start.height))
  const [color, setColor] = useState(start.color)
  const w = numberIn(width, 1, 30000)
  const h = numberIn(height, 1, 30000)
  const valid = name.trim() !== '' && w !== null && h !== null && parseHex(color) !== null
  const submit = () => {
    if (!valid) return
    const params = { name: name.trim(), width: Math.round(w), height: Math.round(h), color }
    void run(dialog.mode === 'new' ? 'layer.newSolid' : 'layer.settings', dialog.mode === 'new' ? params : { ...params, layer: dialog.layer })
    onDone()
  }
  const invalid = t('dialog.badNumber')
  return (
    <FormDialog title={t(dialog.mode === 'new' ? 'dialog.newSolid' : 'dialog.solidSettings')} testId="motion-solid-dialog" onSubmit={submit} onClose={onDone} valid={valid}>
      <TextRow label={t('dialog.name')} value={name} onChange={setName} error={name.trim() ? null : t('dialog.needName')} testId="motion-solid-name" autoFocus />
      <ColorRow label={t('dialog.color')} value={color} onChange={setColor} testId="motion-solid-color" />
      <div className="motion-form-pair">
        <TextRow label={t('dialog.width')} value={width} onChange={setWidth} error={w === null ? invalid : null} testId="motion-solid-width" unit="px" />
        <TextRow label={t('dialog.height')} value={height} onChange={setHeight} error={h === null ? invalid : null} testId="motion-solid-height" unit="px" />
      </div>
      <div>
        <Button
          onClick={() => {
            setWidth(String(start.comp.width))
            setHeight(String(start.comp.height))
          }}
          data-testid="motion-solid-comp-size"
        >
          {t('dialog.compSize')}
        </Button>
      </div>
    </FormDialog>
  )
}

/** Go to Time: a timecode, or a time relative to now (`+10` frames, `-1.00` a second). */
function TimeDialog({ dialog, onSeek, onDone }: { dialog: Extract<MotionDialog, { kind: 'time' }>; onSeek: (seconds: number) => void; onDone: () => void }) {
  const { t } = useKitEditor()
  const [text, setText] = useState(formatTimecode(dialog.time, dialog.fps))
  const parsed = parseTimecode(text, dialog.fps, false, dialog.time)
  const last = Math.max(0, dialog.duration - 1 / dialog.fps)
  const valid = parsed !== null && parsed >= 0 && parsed <= last + 1e-6
  return (
    <FormDialog
      title={t('dialog.goToTime')}
      testId="motion-time-dialog"
      onSubmit={() => {
        if (parsed === null) return
        onSeek(Math.min(last, Math.max(0, parsed)))
        onDone()
      }}
      onClose={onDone}
      valid={valid}
    >
      <TextRow label={t('dialog.time')} value={text} onChange={setText} error={valid ? null : t('dialog.badTime', { end: formatTimecode(last, dialog.fps) })} testId="motion-time-field" autoFocus />
    </FormDialog>
  )
}

/** Rename a layer. */
function RenameDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'rename' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const [name, setName] = useState(dialog.name)
  const valid = name.trim() !== ''
  return (
    <FormDialog
      title={t('dialog.rename')}
      testId="motion-rename-dialog"
      onSubmit={() => {
        if (name.trim() !== dialog.name) void run('layer.rename', { layer: dialog.layer, name: name.trim() })
        onDone()
      }}
      onClose={onDone}
      valid={valid}
    >
      <TextRow label={t('dialog.name')} value={name} onChange={setName} error={valid ? null : t('dialog.needName')} testId="motion-rename-field" autoFocus />
    </FormDialog>
  )
}

/** The open dialog, if any; each runs one engine command (one undo step) when confirmed. */
export function MotionDialogs({ dialog, onClose, onSeek }: { dialog: MotionDialog | null; onClose: () => void; onSeek: (seconds: number) => void }) {
  // A new dialog (even of the same kind) starts from its own values.
  const opened = useRef(0)
  const last = useRef<MotionDialog | null>(null)
  // The focus goes back where it was when the dialog opened, so the editor's keys work again.
  const focus = useRef<HTMLElement | null>(null)
  if (dialog !== last.current) {
    if (dialog && !last.current) focus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null
    if (!dialog && last.current) {
      const back = focus.current
      queueMicrotask(() => (back?.isConnected ? back : document.querySelector<HTMLElement>('.kui-app'))?.focus())
    }
    last.current = dialog
    opened.current += 1
  }
  if (!dialog) return null
  const key = opened.current
  switch (dialog.kind) {
    case 'comp':
      return <CompDialog key={key} dialog={dialog} onDone={onClose} />
    case 'solid':
      return <SolidDialog key={key} dialog={dialog} onDone={onClose} />
    case 'time':
      return <TimeDialog key={key} dialog={dialog} onSeek={onSeek} onDone={onClose} />
    case 'rename':
      return <RenameDialog key={key} dialog={dialog} onDone={onClose} />
  }
}
