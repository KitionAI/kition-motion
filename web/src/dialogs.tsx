import { useRef, useState, type ReactNode } from 'react'
import { Button, Checkbox, Dialog, Field, Select, TextInput, formatTimecode, parseHex, parseTimecode, rgbToHex, toast, useKitEditor } from '@kition/sdk/editor'
import { ColorSwatch } from './properties/ColorSwatch'
import { parseAnswer, type ProjectItem } from './footage'

/** A colour as the engine gives it: [r, g, b] from 0 to 1. */
export type Rgb = readonly [number, number, number]

export const toHex = (rgb: Rgb) => rgbToHex([Math.round(rgb[0] * 255), Math.round(rgb[1] * 255), Math.round(rgb[2] * 255)])

/** The composition a Composition dialog starts with. */
export type CompValues = { name: string; width: number; height: number; frameRate: number; duration: number; background: string }

/** The solid a Solid dialog starts with; `comp` is the active composition's size, for Make Comp Size. */
export type SolidValues = { name: string; width: number; height: number; color: string; comp: { width: number; height: number }; layers?: number }

/** Whether this page has already said that a new solid covers the layers below (once per session). */
let coverNoticeShown = false

/** The editor's own dialogs: what each asks and what it starts with. */
export type MotionDialog =
  | { kind: 'comp'; mode: 'new' | 'settings'; values: CompValues }
  | { kind: 'solid'; mode: 'new' | 'settings'; layer?: number; values: SolidValues }
  | { kind: 'time'; time: number; fps: number; duration: number }
  | { kind: 'rename'; layer: number; name: string }
  | { kind: 'interpolation'; spatial: boolean }
  | { kind: 'velocity'; values: VelocityValues; units: string }
  | { kind: 'stretch'; layers: number[]; stretch: number; span: number; fps: number }
  | { kind: 'savePreset'; name: string; folder: string; onSave: (name: string, folder: string) => void }
  | { kind: 'sequence'; fps: number }
  | { kind: 'grid'; spacing: number; onSave: (spacing: number) => void }
  | { kind: 'guide'; comp: { width: number; height: number } }
  | { kind: 'interpret'; item: ProjectItem }
  | { kind: 'wiggler'; spatial: boolean }
  | { kind: 'smoother' }
  | { kind: 'collect'; folder: string; onCollect: (folder: string) => void }
  | { kind: 'keyValue'; layer: number; path: string; name: string; time: number; value: unknown; fps: number; duration: number; /** The parts typed (a 2D layer's Position shows x and y; its z is kept). */ shown?: number }

/** The first selected keyframe's speed (units per second) and influence (%) on each side, for Keyframe Velocity. */
export type VelocityValues = { inSpeed: number; inInfluence: number; outSpeed: number; outInfluence: number }

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
    if (dialog.mode !== 'new') {
      void run('layer.settings', { ...params, layer: dialog.layer })
      onDone()
      return
    }
    // A new layer goes on top: a solid covering the whole frame hides what was there. Once per
    // session the editor says so and offers to send it to the back (its own undo step).
    const covers = !coverNoticeShown && (start.layers ?? 0) > 0 && params.width >= start.comp.width && params.height >= start.comp.height
    void run('layer.newSolid', params).then((answer) => {
      const layer = parseAnswer<{ layer?: number }>(answer)?.layer
      if (!covers || layer == null || coverNoticeShown) return
      coverNoticeShown = true
      toast.info(t('solid.covers'), { duration: 10000, action: { label: t('solid.sendToBack'), onClick: () => void run('layer.arrange', { layers: [layer], to: 'back' }) } })
    })
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

/** Keyframe Interpolation's choices for each kind (`keyframe.interpolation`): '' keeps the keys' own. */
const TEMPORAL = ['', 'linear', 'bezier', 'continuousBezier', 'autoBezier', 'hold'] as const
const SPATIAL = ['', 'linear', 'bezier', 'continuousBezier', 'autoBezier'] as const

/** Keyframe Interpolation: the selected keyframes' temporal and (on a moving property) spatial interpolation. */
function InterpolationDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'interpolation' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const [temporal, setTemporal] = useState<string>('')
  const [spatial, setSpatial] = useState<string>('')
  const choices = (list: readonly string[], value: string, set: (next: string) => void, testId: string) => (
    <Select value={value} onChange={(event) => set(event.target.value)} data-testid={testId}>
      {list.map((choice) => (
        <option key={choice} value={choice}>
          {t(choice ? `interp.${choice}` : 'interp.current')}
        </option>
      ))}
    </Select>
  )
  return (
    <FormDialog
      title={t('dialog.interpolation')}
      testId="motion-interp-dialog"
      onSubmit={() => {
        const params = { ...(temporal ? { interpolation: temporal } : {}), ...(spatial ? { spatial } : {}) }
        if (Object.keys(params).length) void run('keyframe.interpolation', params)
        onDone()
      }}
      onClose={onDone}
      valid
    >
      <Field label={t('interp.temporal')}>{choices(TEMPORAL, temporal, setTemporal, 'motion-interp-temporal')}</Field>
      {dialog.spatial ? <Field label={t('interp.spatial')}>{choices(SPATIAL, spatial, setSpatial, 'motion-interp-spatial')}</Field> : null}
    </FormDialog>
  )
}

/** Keyframe Velocity: the selected keyframes' incoming and outgoing speed and influence (`keyframe.velocity`). */
function VelocityDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'velocity' }>; onDone: () => void }) {
  const { run, t, phrase } = useKitEditor()
  const units = phrase(dialog.units)
  const round = (value: number) => String(Math.round(value * 1000) / 1000)
  const [inSpeed, setInSpeed] = useState(round(dialog.values.inSpeed))
  const [inInfluence, setInInfluence] = useState(round(dialog.values.inInfluence))
  const [outSpeed, setOutSpeed] = useState(round(dialog.values.outSpeed))
  const [outInfluence, setOutInfluence] = useState(round(dialog.values.outInfluence))
  const [continuous, setContinuous] = useState(false)
  const numbers = { inSpeed: numberIn(inSpeed, -1e9, 1e9), inInfluence: numberIn(inInfluence, 0.1, 100), outSpeed: numberIn(continuous ? inSpeed : outSpeed, -1e9, 1e9), outInfluence: numberIn(outInfluence, 0.1, 100) }
  const valid = Object.values(numbers).every((value) => value !== null)
  const invalid = t('dialog.badNumber')
  return (
    <FormDialog
      title={t('dialog.velocity')}
      testId="motion-velocity-dialog"
      onSubmit={() => {
        if (!valid) return
        void run('keyframe.velocity', { ...numbers, ...(continuous ? { continuous: true } : {}) })
        onDone()
      }}
      onClose={onDone}
      valid={valid}
    >
      <p className="kui-muted">{t('velocity.incoming')}</p>
      <div className="motion-form-pair">
        <TextRow label={t('velocity.speed')} value={inSpeed} onChange={setInSpeed} error={numbers.inSpeed === null ? invalid : null} testId="motion-velocity-in-speed" unit={units} autoFocus />
        <TextRow label={t('velocity.influence')} value={inInfluence} onChange={setInInfluence} error={numbers.inInfluence === null ? invalid : null} testId="motion-velocity-in-influence" unit="%" />
      </div>
      <p className="kui-muted">{t('velocity.outgoing')}</p>
      <div className="motion-form-pair">
        <TextRow label={t('velocity.speed')} value={continuous ? inSpeed : outSpeed} onChange={setOutSpeed} error={numbers.outSpeed === null ? invalid : null} testId="motion-velocity-out-speed" unit={units} />
        <TextRow label={t('velocity.influence')} value={outInfluence} onChange={setOutInfluence} error={numbers.outInfluence === null ? invalid : null} testId="motion-velocity-out-influence" unit="%" />
      </div>
      <Checkbox label={t('velocity.continuous')} checked={continuous} onChange={setContinuous} testId="motion-velocity-continuous" />
    </FormDialog>
  )
}

/**
 * Time Stretch: the selected layers' speed as a stretch factor (%), or the first layer's new
 * duration (the factor follows), and which point keeps its frame (the in point, the current
 * time or the out point). One undo step (`layer.timeStretch`).
 */
function StretchDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'stretch' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const round = (value: number) => String(Math.round(value * 1000) / 1000)
  const [factor, setFactor] = useState(round(dialog.stretch))
  const [duration, setDuration] = useState(formatTimecode(dialog.span, dialog.fps))
  const [hold, setHold] = useState<'in' | 'current' | 'out'>('in')
  const percent = numberIn(factor, -10000, 10000)
  const validFactor = percent !== null && Math.abs(percent) >= 1
  const seconds = (text: string) => parseTimecode(text, dialog.fps, false, dialog.span)
  // The two fields follow each other: a new factor gives the duration it makes, and the other way round.
  const changeFactor = (text: string) => {
    setFactor(text)
    const value = numberIn(text, -10000, 10000)
    if (value !== null && Math.abs(value) >= 1 && dialog.stretch !== 0) setDuration(formatTimecode(Math.abs((dialog.span * value) / dialog.stretch), dialog.fps))
  }
  const changeDuration = (text: string) => {
    setDuration(text)
    const value = seconds(text)
    if (value !== null && value > 0 && dialog.span > 0) setFactor(round((dialog.stretch * value) / dialog.span))
  }
  return (
    <FormDialog
      title={t('dialog.stretch')}
      testId="motion-stretch-dialog"
      onSubmit={() => {
        if (!validFactor) return
        void run('layer.timeStretch', { layers: dialog.layers, percent, hold })
        onDone()
      }}
      onClose={onDone}
      valid={validFactor}
    >
      <TextRow label={t('stretch.factor')} value={factor} onChange={changeFactor} error={validFactor ? null : t('stretch.badFactor')} testId="motion-stretch-factor" unit="%" autoFocus />
      <TextRow label={t('stretch.duration')} value={duration} onChange={changeDuration} error={seconds(duration) !== null ? null : t('stretch.badDuration')} testId="motion-stretch-duration" />
      <Field label={t('stretch.hold')}>
        {({ id }) => (
          <Select id={id} value={hold} onChange={(event) => setHold(event.target.value as 'in' | 'current' | 'out')} data-testid="motion-stretch-hold">
            <option value="in">{t('stretch.hold.in')}</option>
            <option value="current">{t('stretch.hold.current')}</option>
            <option value="out">{t('stretch.hold.out')}</option>
          </Select>
        )}
      </Field>
    </FormDialog>
  )
}

/** Save Animation Preset: the preset's name (its file is `<name>.ecpreset` beside the project). */
function SavePresetDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'savePreset' }>; onDone: () => void }) {
  const { t } = useKitEditor()
  const [name, setName] = useState(dialog.name)
  const [folder, setFolder] = useState(dialog.folder)
  const clean = name.replace(/[\\/:*?"<>|]+/g, '_').trim()
  const place = presetFolder(folder)
  return (
    <FormDialog
      title={t('dialog.savePreset')}
      testId="motion-preset-dialog"
      onSubmit={() => {
        if (!clean || place === null) return
        dialog.onSave(clean, place)
        onDone()
      }}
      onClose={onDone}
      valid={clean !== '' && place !== null}
    >
      <TextRow label={t('preset.name')} value={name} onChange={setName} error={clean ? null : t('preset.badName')} testId="motion-preset-name" autoFocus />
      <TextRow label={t('preset.folder')} value={folder} onChange={setFolder} error={place === null ? t('preset.badFolder') : null} testId="motion-preset-folder" />
      <p className="kui-muted">{t('preset.where', { file: `${place ? `${place}/` : ''}${clean || '…'}.ecpreset` })}</p>
    </FormDialog>
  )
}

/**
 * A preset's folder as typed: relative to the project's folder, `/`-separated, each part a file
 * name; empty is the project's folder itself. Null when it would leave the project's folder.
 */
export function presetFolder(text: string): string | null {
  const parts = text.replace(/\\/g, '/').split('/').map((part) => part.trim()).filter(Boolean)
  if (parts.some((part) => part === '..' || part === '.' || /[:*?"<>|]/.test(part))) return null
  return parts.join('/')
}

/** View ▸ Grid Settings…: the grid's spacing in composition pixels (drawn and snapped to). */
function GridDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'grid' }>; onDone: () => void }) {
  const { t } = useKitEditor()
  const [spacing, setSpacing] = useState(String(dialog.spacing))
  const value = numberIn(spacing, 2, 10000)
  return (
    <FormDialog
      title={t('dialog.grid')}
      testId="motion-grid-dialog"
      onSubmit={() => {
        if (value === null) return
        dialog.onSave(value)
        onDone()
      }}
      onClose={onDone}
      valid={value !== null}
    >
      <TextRow label={t('grid.spacing')} value={spacing} onChange={setSpacing} error={value === null ? t('grid.badSpacing') : null} testId="motion-grid-spacing" unit="px" autoFocus />
    </FormDialog>
  )
}

/** View ▸ New Guide…: a horizontal or vertical guide at a position (composition pixels), without the rulers. One undo step (`view.addGuide`). */
function GuideDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'guide' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const [orientation, setOrientation] = useState<'horizontal' | 'vertical'>('horizontal')
  const [position, setPosition] = useState(String(Math.round(dialog.comp.height / 2)))
  const value = numberIn(position, -100000, 100000)
  return (
    <FormDialog
      title={t('dialog.guide')}
      testId="motion-guide-dialog"
      onSubmit={() => {
        if (value === null) return
        void run('view.addGuide', { orientation, position: value })
        onDone()
      }}
      onClose={onDone}
      valid={value !== null}
    >
      <Field label={t('guide.orientation')}>
        {({ id }) => (
          <Select
            id={id}
            value={orientation}
            onChange={(event) => {
              const next = event.target.value === 'vertical' ? 'vertical' : 'horizontal'
              setOrientation(next)
              setPosition(String(Math.round((next === 'vertical' ? dialog.comp.width : dialog.comp.height) / 2)))
            }}
            data-testid="motion-guide-orientation"
          >
            <option value="horizontal">{t('guide.horizontal')}</option>
            <option value="vertical">{t('guide.vertical')}</option>
          </Select>
        )}
      </Field>
      <TextRow label={t('guide.position')} value={position} onChange={setPosition} error={value === null ? t('dialog.badNumber') : null} testId="motion-guide-position" unit="px" />
    </FormDialog>
  )
}

/**
 * Sequence Layers: the selected layers one after another in the order they were selected, each
 * starting where the one before ends, or overlapping it by a duration with a transition (none,
 * the front layer dissolving out, or both dissolving across). One undo step (`layer.sequence`).
 */
function SequenceDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'sequence' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const [overlap, setOverlap] = useState(false)
  const [duration, setDuration] = useState(formatTimecode(1, dialog.fps))
  const [transition, setTransition] = useState<'off' | 'dissolveFront' | 'crossDissolve'>('off')
  const seconds = parseTimecode(duration, dialog.fps, false, 1)
  const valid = !overlap || (seconds !== null && seconds > 0)
  return (
    <FormDialog
      title={t('dialog.sequence')}
      testId="motion-sequence-dialog"
      onSubmit={() => {
        if (!valid) return
        void run('layer.sequence', overlap ? { overlap: true, duration: seconds, transition } : { overlap: false })
        onDone()
      }}
      onClose={onDone}
      valid={valid}
    >
      <Checkbox label={t('sequence.overlap')} checked={overlap} onChange={setOverlap} testId="motion-sequence-overlap" />
      {overlap ? (
        <>
          <TextRow label={t('sequence.duration')} value={duration} onChange={setDuration} error={seconds !== null && seconds > 0 ? null : t('sequence.badDuration')} testId="motion-sequence-duration" />
          <Field label={t('sequence.transition')}>
            {({ id }) => (
              <Select id={id} value={transition} onChange={(event) => setTransition(event.target.value as 'off' | 'dissolveFront' | 'crossDissolve')} data-testid="motion-sequence-transition">
                <option value="off">{t('sequence.transition.off')}</option>
                <option value="dissolveFront">{t('sequence.transition.dissolveFront')}</option>
                <option value="crossDissolve">{t('sequence.transition.crossDissolve')}</option>
              </Select>
            )}
          </Field>
        </>
      ) : null}
      <p className="kui-muted">{t('sequence.order')}</p>
    </FormDialog>
  )
}

/** Pixel aspect ratios footage is often made with: [label key, ratio]. */
const PIXEL_ASPECTS: ReadonlyArray<readonly [string, number]> = [
  ['square', 1],
  ['ntsc', 0.9091],
  ['ntscWide', 1.2121],
  ['pal', 1.094],
  ['palWide', 1.4587],
  ['hdv', 1.3333],
  ['dvcpro', 1.5],
  ['anamorphic', 2],
]

/**
 * Interpret Footage: the frame rate it plays at (the file's, or conformed to another), how its
 * alpha is read (straight, premultiplied, ignored, or guessed from the picture), how many times it
 * loops, and its pixel aspect ratio. One undo step (`file.interpretFootage`); layers that ran to
 * the footage's end still do.
 */
function InterpretDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'interpret' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const item = dialog.item
  const footage = item.footage
  const moving = item.frameRate != null
  const pictures = footage?.hasVideo ?? false
  const [conform, setConform] = useState(footage?.conformed ?? false)
  const [rate, setRate] = useState(String(Math.round((item.frameRate ?? 30) * 1000) / 1000))
  const [alpha, setAlpha] = useState<string>(footage?.alpha ?? 'straight')
  const [loop, setLoop] = useState(String(footage?.loop ?? 1))
  const start = footage?.pixelAspect ?? 1
  const known = PIXEL_ASPECTS.find(([, ratio]) => Math.abs(ratio - start) < 1e-3)
  const [aspect, setAspect] = useState(String(known ? known[1] : start))
  const fps = numberIn(rate, 0.01, 999)
  const loops = numberIn(loop, 1, 9999)
  const valid = (!conform || fps !== null) && loops !== null && Number.isInteger(loops)
  return (
    <FormDialog
      title={t('dialog.interpret', { name: item.name })}
      testId="motion-interpret-dialog"
      onSubmit={() => {
        if (!valid) return
        void run('file.interpretFootage', {
          items: [item.id],
          ...(moving ? { frameRate: conform && fps !== null ? fps : 'file' } : {}),
          ...(pictures ? { alpha, pixelAspect: Number(aspect) } : {}),
          loop: loops,
        })
        onDone()
      }}
      onClose={onDone}
      valid={valid}
    >
      {moving ? (
        <>
          <Field label={t('interpret.frameRate')}>
            {({ id }) => (
              <Select id={id} value={conform ? 'conform' : 'file'} onChange={(event) => setConform(event.target.value === 'conform')} data-testid="motion-interpret-rate-mode">
                <option value="file">{t('interpret.rate.file', { fps: Math.round((footage?.nativeFrameRate ?? item.frameRate ?? 30) * 1000) / 1000 })}</option>
                <option value="conform">{t('interpret.rate.conform')}</option>
              </Select>
            )}
          </Field>
          {conform ? <TextRow label={t('interpret.conformTo')} value={rate} onChange={setRate} error={fps === null ? t('dialog.badNumber') : null} testId="motion-interpret-rate" unit={t('interpret.fps')} /> : null}
        </>
      ) : null}
      {pictures ? (
        <>
          <Field label={t('interpret.alpha')}>
            {({ id }) => (
              <Select id={id} value={alpha} onChange={(event) => setAlpha(event.target.value)} data-testid="motion-interpret-alpha">
                {['straight', 'premultiplied', 'ignore', 'guess'].map((mode) => (
                  <option key={mode} value={mode}>
                    {t(`interpret.alpha.${mode}`)}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label={t('interpret.pixelAspect')}>
            {({ id }) => (
              <Select id={id} value={aspect} onChange={(event) => setAspect(event.target.value)} data-testid="motion-interpret-aspect">
                {known ? null : <option value={String(start)}>{t('interpret.aspect.other', { ratio: Math.round(start * 10000) / 10000 })}</option>}
                {PIXEL_ASPECTS.map(([key, ratio]) => (
                  <option key={key} value={String(ratio)}>
                    {t(`interpret.aspect.${key}`, { ratio })}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        </>
      ) : null}
      <TextRow label={t('interpret.loop')} value={loop} onChange={setLoop} error={loops === null || !Number.isInteger(loops) ? t('interpret.badLoop') : null} testId="motion-interpret-loop" unit={t('interpret.times')} />
    </FormDialog>
  )
}

/**
 * A keyframe's value and time typed in (a double-click on the keyframe, or Edit Value… on its
 * menu): each of a number's parts (x, y, z), and the time as timecode. One undo step (`keyframe.set`
 * with the new value and time).
 */
function KeyValueDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'keyValue' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const numbers = typeof dialog.value === 'number' ? [dialog.value] : Array.isArray(dialog.value) && dialog.value.every((part) => typeof part === 'number') ? (dialog.value as number[]) : null
  // The parts not shown (a 2D layer's z) go back as they were.
  const kept = numbers && dialog.shown != null && dialog.shown > 0 ? numbers.slice(dialog.shown) : []
  const [parts, setParts] = useState(() => (numbers ?? []).slice(0, numbers && dialog.shown != null && dialog.shown > 0 ? dialog.shown : undefined).map((part) => String(Math.round(part * 1000) / 1000)))
  const [time, setTime] = useState(formatTimecode(dialog.time, dialog.fps))
  const seconds = parseTimecode(time, dialog.fps, false, dialog.time)
  const values = parts.map((part) => numberIn(part, -1e9, 1e9))
  const timeOk = seconds !== null && seconds >= 0 && seconds <= dialog.duration
  const valid = timeOk && values.every((value) => value !== null)
  const names = ['X', 'Y', 'Z', 'W']
  return (
    <FormDialog
      title={t('dialog.keyValue', { name: dialog.name })}
      testId="motion-key-dialog"
      onSubmit={() => {
        if (!valid || seconds === null) return
        const value = numbers == null ? undefined : typeof dialog.value === 'number' ? values[0] : [...values, ...kept]
        void run('keyframe.set', { layer: dialog.layer, path: dialog.path, time: dialog.time, timeBase: 'comp', ...(Math.abs(seconds - dialog.time) > 1e-6 ? { newTime: seconds } : {}), ...(value !== undefined ? { value } : {}) })
        onDone()
      }}
      onClose={onDone}
      valid={valid}
    >
      {parts.map((part, at) => (
        <TextRow
          key={at}
          label={parts.length > 1 ? t('keyValue.part', { part: names[at] ?? String(at + 1) }) : t('keyValue.value')}
          value={part}
          onChange={(text) => setParts((now) => now.map((each, index) => (index === at ? text : each)))}
          error={values[at] === null ? t('dialog.badNumber') : null}
          testId={`motion-key-value-${at}`}
          autoFocus={at === 0}
        />
      ))}
      {numbers == null ? <p className="kui-muted">{t('keyValue.notNumber')}</p> : null}
      <TextRow label={t('keyValue.time')} value={time} onChange={setTime} error={timeOk ? null : t('dialog.badTime', { end: formatTimecode(dialog.duration, dialog.fps) })} testId="motion-key-time" autoFocus={numbers == null} />
    </FormDialog>
  )
}

/**
 * Animation ▸ Keyframe Assistant ▸ Wiggler: random deviations added between the selected
 * keyframes (`keyframe.wiggle`, one undo step): along the path or in time (a moving property only),
 * smooth or jagged noise, each dimension on its own, all alike or one, how many keys a second and
 * how far.
 */
function WigglerDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'wiggler' }>; onDone: () => void }) {
  const { run, t } = useKitEditor()
  const [apply, setApply] = useState<'spatial' | 'temporal'>(dialog.spatial ? 'spatial' : 'temporal')
  const [noise, setNoise] = useState<'smooth' | 'jagged'>('smooth')
  const [dimensions, setDimensions] = useState<'independent' | 'same' | 'one'>('independent')
  const [frequency, setFrequency] = useState('5')
  const [magnitude, setMagnitude] = useState('10')
  const perSecond = numberIn(frequency, 0.01, 1000)
  const howFar = numberIn(magnitude, 0, 1e7)
  return (
    <FormDialog
      title={t('dialog.wiggler')}
      testId="motion-wiggler-dialog"
      onSubmit={() => {
        if (perSecond === null || howFar === null) return
        void run('keyframe.wiggle', { apply, noise, dimensions, ...(dimensions === 'one' ? { dimension: 0 } : {}), frequency: perSecond, magnitude: howFar })
        onDone()
      }}
      onClose={onDone}
      valid={perSecond !== null && howFar !== null}
    >
      {dialog.spatial ? (
        <Field label={t('wiggler.apply')}>
          <Select value={apply} onChange={(event) => setApply(event.target.value as 'spatial' | 'temporal')} data-testid="motion-wiggler-apply">
            <option value="spatial">{t('wiggler.apply.spatial')}</option>
            <option value="temporal">{t('wiggler.apply.temporal')}</option>
          </Select>
        </Field>
      ) : null}
      <Field label={t('wiggler.noise')}>
        <Select value={noise} onChange={(event) => setNoise(event.target.value as 'smooth' | 'jagged')} data-testid="motion-wiggler-noise">
          <option value="smooth">{t('wiggler.noise.smooth')}</option>
          <option value="jagged">{t('wiggler.noise.jagged')}</option>
        </Select>
      </Field>
      <Field label={t('wiggler.dimensions')}>
        <Select value={dimensions} onChange={(event) => setDimensions(event.target.value as 'independent' | 'same' | 'one')} data-testid="motion-wiggler-dimensions">
          <option value="independent">{t('wiggler.dimensions.independent')}</option>
          <option value="same">{t('wiggler.dimensions.same')}</option>
          <option value="one">{t('wiggler.dimensions.one')}</option>
        </Select>
      </Field>
      <TextRow label={t('wiggler.frequency')} value={frequency} onChange={setFrequency} error={perSecond === null ? t('dialog.badNumber') : null} testId="motion-wiggler-frequency" unit={t('wiggler.perSecond')} autoFocus />
      <TextRow label={t('wiggler.magnitude')} value={magnitude} onChange={setMagnitude} error={howFar === null ? t('dialog.badNumber') : null} testId="motion-wiggler-magnitude" />
    </FormDialog>
  )
}

/**
 * Animation ▸ Keyframe Assistant ▸ Smoother: the selected keyframes' curve kept with fewer keys,
 * within a tolerance (`keyframe.smooth`, one undo step).
 */
function SmootherDialog({ onDone }: { onDone: () => void }) {
  const { run, t } = useKitEditor()
  const [tolerance, setTolerance] = useState('1')
  const value = numberIn(tolerance, 0, 1e6)
  return (
    <FormDialog
      title={t('dialog.smoother')}
      testId="motion-smoother-dialog"
      onSubmit={() => {
        if (value === null) return
        void run('keyframe.smooth', { tolerance: value })
        onDone()
      }}
      onClose={onDone}
      valid={value !== null}
    >
      <p className="kui-muted motion-export-note">{t('smoother.note')}</p>
      <TextRow label={t('smoother.tolerance')} value={tolerance} onChange={setTolerance} error={value === null ? t('dialog.badNumber') : null} testId="motion-smoother-tolerance" autoFocus />
    </FormDialog>
  )
}

/** File ▸ Dependencies ▸ Collect Files: the folder (inside the project's) the copy goes into. */
function CollectDialog({ dialog, onDone }: { dialog: Extract<MotionDialog, { kind: 'collect' }>; onDone: () => void }) {
  const { t } = useKitEditor()
  const [folder, setFolder] = useState(dialog.folder)
  const ok = folder.trim() !== '' && !/[\\:*?"<>|]|(^|\/)\.\.?(\/|$)/.test(folder.trim())
  return (
    <FormDialog
      title={t('dialog.collect')}
      testId="motion-collect-dialog"
      onSubmit={() => {
        if (!ok) return
        dialog.onCollect(folder.trim())
        onDone()
      }}
      onClose={onDone}
      valid={ok}
    >
      <p className="kui-muted motion-export-note">{t('collect.note')}</p>
      <TextRow label={t('export.folder')} value={folder} onChange={setFolder} error={ok ? null : t('preset.badFolder')} testId="motion-collect-folder" autoFocus />
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
    case 'interpolation':
      return <InterpolationDialog key={key} dialog={dialog} onDone={onClose} />
    case 'velocity':
      return <VelocityDialog key={key} dialog={dialog} onDone={onClose} />
    case 'stretch':
      return <StretchDialog key={key} dialog={dialog} onDone={onClose} />
    case 'savePreset':
      return <SavePresetDialog key={key} dialog={dialog} onDone={onClose} />
    case 'sequence':
      return <SequenceDialog key={key} dialog={dialog} onDone={onClose} />
    case 'grid':
      return <GridDialog key={key} dialog={dialog} onDone={onClose} />
    case 'guide':
      return <GuideDialog key={key} dialog={dialog} onDone={onClose} />
    case 'interpret':
      return <InterpretDialog key={key} dialog={dialog} onDone={onClose} />
    case 'keyValue':
      return <KeyValueDialog key={key} dialog={dialog} onDone={onClose} />
    case 'wiggler':
      return <WigglerDialog key={key} dialog={dialog} onDone={onClose} />
    case 'smoother':
      return <SmootherDialog key={key} onDone={onClose} />
    case 'collect':
      return <CollectDialog key={key} dialog={dialog} onDone={onClose} />
  }
}
