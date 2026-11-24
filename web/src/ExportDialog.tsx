import { useRef, useState } from 'react'
import { Button, Checkbox, Dialog, Field, Select, TextInput, toast, useKitEditor, type KitEngine } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'

/** The formats the web build encodes into one file (`bindings/wasm/src/movie.rs`), with their extensions. */
const FORMATS = [
  { id: 'h264', extension: 'mp4' },
  { id: 'webm', extension: 'webm' },
  { id: 'gif', extension: 'gif' },
  // A Lottie animation (`file.exportLottie`): vector, for the web and apps; no size, range or rate to choose.
  { id: 'lottie', extension: 'json' },
] as const

type Format = (typeof FORMATS)[number]['id']

/** What the dialog sends `renderMovie` (`movie::queue_params`). */
type Settings = { format: Exclude<Format, 'lottie'>; span: 'comp' | 'workArea'; size: 'full' | 'half' | { width: number; height: number }; frameRate: number | null }

/** A whole number of pixels from 16 to 8192 typed in a field, or null. */
function side(text: string) {
  const value = Number(text.trim())
  return Number.isInteger(value) && value >= 16 && value <= 8192 ? value : null
}

/** A name for a file: the composition's, without characters a file name cannot have. */
const fileName = (name: string) => name.replace(/[\\/:*?"<>|]+/g, '_').trim() || 'Composition'

/** How far the engine's export is (`exportStart` and `exportStep` answer it). */
type Progress = { done: number; total: number; finished: boolean }

/** How long the engine renders per `exportStep` call before it answers (and can hear Cancel). */
const STEP_MS = 120

/**
 * Export: the active composition (all of it or its work area) at full, half or a custom size, at
 * its frame rate or half of it, as H.264 MP4, WebM or animated GIF, written beside the project by
 * the host. The engine renders a little at a time (`exportStart`, then `exportStep` until it has
 * finished, then `exportFinish` hands over the file), so the dialog shows the frames done and
 * Cancel stops the render between two steps and writes no file.
 */
export function ExportDialog({ info, doc, lottie = false, onClose }: { info: MotionInfo; doc: KitEngine | null; lottie?: boolean; onClose: () => void }) {
  const { host, t, run } = useKitEditor()
  const comp = info.comp
  const [format, setFormat] = useState<Format>(lottie ? 'lottie' : 'h264')
  const [span, setSpan] = useState<'comp' | 'workArea'>('comp')
  const [size, setSize] = useState<'full' | 'half' | 'custom'>('full')
  const [width, setWidth] = useState(String(comp?.width ?? 1920))
  const [height, setHeight] = useState(String(comp?.height ?? 1080))
  const [rate, setRate] = useState<'comp' | 'half'>('comp')
  const [busy, setBusy] = useState(false)
  const [progress, setProgress] = useState<Progress | null>(null)
  // Lottie: its options, and what the engine said the file could not carry (shown before closing).
  const [expressions, setExpressions] = useState(false)
  const [textAsShapes, setTextAsShapes] = useState(false)
  const [warnings, setWarnings] = useState<string[] | null>(null)
  const cancelled = useRef(false)
  if (!comp) return null
  const w = side(width)
  const h = side(height)
  const valid = size !== 'custom' || (w !== null && h !== null)
  const [start, end] = comp.workArea
  const seconds = (value: number) => Math.round(value * 100) / 100

  const render = async () => {
    if (!doc || !valid || busy) return
    setBusy(true)
    const extension = FORMATS.find((each) => each.id === format)?.extension ?? 'mp4'
    const folder = host.init.document.path.split('/').slice(0, -1).join('/')
    const path = [folder, `${fileName(comp.name)}.${extension}`].filter(Boolean).join('/')
    if (format === 'lottie') {
      await exportLottie(path)
      return
    }
    const settings: Settings = {
      format,
      span,
      size: size === 'custom' && w !== null && h !== null ? { width: w, height: h } : size === 'half' ? 'half' : 'full',
      frameRate: rate === 'half' ? Math.round((comp.fps / 2) * 1000) / 1000 : null,
    }
    cancelled.current = false
    const read = (json: string) => JSON.parse(json) as Progress
    try {
      let now = read(await doc.invoke<string>('exportStart', path.split('/').pop() ?? path, JSON.stringify(settings)))
      setProgress(now)
      while (!now.finished && !cancelled.current) {
        now = read(await doc.invoke<string>('exportStep', STEP_MS))
        if (!cancelled.current) setProgress(now)
      }
      if (cancelled.current) {
        await doc.invoke<boolean>('exportCancel')
        toast.info(t('export.cancelled'))
        setBusy(false)
        setProgress(null)
        return
      }
      const bytes = await doc.invoke<Uint8Array>('exportFinish')
      const written = await host.exportFile(path, bytes)
      toast.success(t('export.done', { name: written.split('/').pop() ?? written }))
      onClose()
    } catch (error) {
      await doc.invoke<boolean>('exportCancel').catch(() => false)
      toast.error(t('renderFailed'), { description: error instanceof Error ? error.message : String(error) })
      setBusy(false)
      setProgress(null)
    }
  }

  /**
   * Lottie: the engine writes the file in memory (`file.exportLottie`), the host saves it beside
   * the project; what Lottie could not carry (the engine's warnings) stays in the dialog to read.
   */
  const exportLottie = async (path: string) => {
    if (!doc) return
    const name = path.split('/').pop() ?? path
    try {
      const answer = await run('file.exportLottie', { path: name, includeExpressions: expressions, textAsShapes })
      if (answer === null) throw new Error(t('export.lottieFailed'))
      const bytes = await doc.invoke<Uint8Array | undefined>('takeFile', name)
      if (!bytes) throw new Error(t('export.lottieFailed'))
      const written = await host.exportFile(path, bytes)
      toast.success(t('export.done', { name: written.split('/').pop() ?? written }))
      const said = (JSON.parse(answer) as { warnings?: unknown }).warnings
      const list = Array.isArray(said) ? said.map((warning) => (typeof warning === 'string' ? warning : JSON.stringify(warning))) : []
      setBusy(false)
      if (list.length) setWarnings(list)
      else onClose()
    } catch (error) {
      toast.error(t('renderFailed'), { description: error instanceof Error ? error.message : String(error) })
      setBusy(false)
    }
  }

  /** Cancel while rendering stops the render (the loop above ends at the engine's next answer). */
  const cancel = () => {
    if (busy) cancelled.current = true
    else onClose()
  }

  return (
    <Dialog
      title={t('export.title')}
      testId="motion-export-dialog"
      onClose={cancel}
      actions={
        <>
          <Button onClick={cancel} data-testid="motion-export-dialog-cancel">
            {t('dialog.cancel')}
          </Button>
          {warnings ? (
            <Button variant="primary" onClick={onClose} data-testid="motion-export-dialog-close">
              {t('export.close')}
            </Button>
          ) : (
            <Button variant="primary" disabled={!valid || busy || !doc} onClick={() => void render()} data-testid="motion-export-dialog-ok">
              {t('export.render')}
            </Button>
          )}
        </>
      }
    >
      <form
        className="motion-form"
        onSubmit={(event) => {
          event.preventDefault()
          void render()
        }}
      >
        <Field label={t('export.format')}>
          <Select value={format} disabled={busy} onChange={(event) => setFormat(event.target.value as Format)} data-testid="motion-export-format">
            {FORMATS.map((each) => (
              <option key={each.id} value={each.id}>
                {t(`export.format.${each.id}`)}
              </option>
            ))}
          </Select>
        </Field>
        {format === 'lottie' ? (
          <>
            <Checkbox label={t('export.lottie.expressions')} checked={expressions} onChange={setExpressions} disabled={busy} testId="motion-export-lottie-expressions" />
            <Checkbox label={t('export.lottie.textAsShapes')} checked={textAsShapes} onChange={setTextAsShapes} disabled={busy} testId="motion-export-lottie-text" />
            <p className="kui-muted motion-export-note">{t('export.lottie.note')}</p>
          </>
        ) : (
          <>
        <Field label={t('export.range')}>
          <Select value={span} disabled={busy} onChange={(event) => setSpan(event.target.value as 'comp' | 'workArea')} data-testid="motion-export-range">
            <option value="comp">{t('export.range.comp', { seconds: seconds(comp.duration) })}</option>
            <option value="workArea">{t('export.range.workArea', { start: seconds(start), end: seconds(end) })}</option>
          </Select>
        </Field>
        <Field label={t('export.size')}>
          <Select value={size} disabled={busy} onChange={(event) => setSize(event.target.value as 'full' | 'half' | 'custom')} data-testid="motion-export-size">
            <option value="full">{t('export.size.full', { width: comp.width, height: comp.height })}</option>
            <option value="half">{t('export.size.half', { width: Math.round(comp.width / 2), height: Math.round(comp.height / 2) })}</option>
            <option value="custom">{t('dialog.custom')}</option>
          </Select>
        </Field>
        {size === 'custom' ? (
          <div className="motion-form-pair">
            <Field label={t('dialog.width')} error={w === null ? t('export.badSize') : undefined}>
              {({ id, invalid }) => (
                <span className="motion-form-input">
                  <TextInput id={id} invalid={invalid} value={width} disabled={busy} onChange={(event) => setWidth(event.target.value)} data-testid="motion-export-width" />
                  <span className="kui-muted">px</span>
                </span>
              )}
            </Field>
            <Field label={t('dialog.height')} error={h === null ? t('export.badSize') : undefined}>
              {({ id, invalid }) => (
                <span className="motion-form-input">
                  <TextInput id={id} invalid={invalid} value={height} disabled={busy} onChange={(event) => setHeight(event.target.value)} data-testid="motion-export-height" />
                  <span className="kui-muted">px</span>
                </span>
              )}
            </Field>
          </div>
        ) : null}
        <Field label={t('dialog.frameRate')}>
          <Select value={rate} disabled={busy} onChange={(event) => setRate(event.target.value as 'comp' | 'half')} data-testid="motion-export-rate">
            <option value="comp">{t('export.rate.comp', { fps: Math.round(comp.fps * 100) / 100 })}</option>
            <option value="half">{t('export.rate.half', { fps: Math.round((comp.fps / 2) * 100) / 100 })}</option>
          </Select>
        </Field>
          </>
        )}
        {warnings ? (
          <div className="motion-export-warnings" role="status" data-testid="motion-export-warnings">
            <p>{t('export.lottie.warnings', { count: warnings.length })}</p>
            <ul>
              {warnings.map((warning, index) => (
                <li key={index}>{warning}</li>
              ))}
            </ul>
          </div>
        ) : null}
        {busy ? (
          <div className="motion-export-busy" role="status" data-testid="motion-export-busy">
            <div
              className="motion-export-bar"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={progress?.total ?? 0}
              aria-valuenow={progress?.done ?? 0}
              data-testid="motion-export-progress"
              data-done={progress?.done ?? 0}
              data-total={progress?.total ?? 0}
            >
              <span style={{ width: `${progress && progress.total > 0 ? Math.min(100, (progress.done / progress.total) * 100) : 0}%` }} />
            </div>
            <span className="kui-muted">
              {progress && progress.total > 0 && progress.done >= progress.total
                ? t('export.finishing')
                : t('export.frames', { done: progress?.done ?? 0, total: progress?.total ?? 0 })}
            </span>
          </div>
        ) : null}
        <button type="submit" hidden />
      </form>
    </Dialog>
  )
}
