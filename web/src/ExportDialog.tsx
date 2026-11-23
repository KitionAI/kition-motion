import { useState } from 'react'
import { Button, Dialog, Field, Select, TextInput, toast, useKitEditor, type KitEngine } from '@kition/sdk/editor'
import { Spinner } from '@kition/tessera/ui'
import type { MotionInfo } from './motion'

/** The formats the web build encodes into one file (`bindings/wasm/src/movie.rs`), with their extensions. */
const FORMATS = [
  { id: 'h264', extension: 'mp4' },
  { id: 'webm', extension: 'webm' },
  { id: 'gif', extension: 'gif' },
] as const

type Format = (typeof FORMATS)[number]['id']

/** What the dialog sends `renderMovie` (`movie::queue_params`). */
type Settings = { format: Format; span: 'comp' | 'workArea'; size: 'full' | 'half' | { width: number; height: number }; frameRate: number | null }

/** A whole number of pixels from 16 to 8192 typed in a field, or null. */
function side(text: string) {
  const value = Number(text.trim())
  return Number.isInteger(value) && value >= 16 && value <= 8192 ? value : null
}

/** A name for a file: the composition's, without characters a file name cannot have. */
const fileName = (name: string) => name.replace(/[\\/:*?"<>|]+/g, '_').trim() || 'Composition'

/**
 * Export: the active composition (all of it or its work area) at full, half or a custom size, at
 * its frame rate or half of it, as H.264 MP4, WebM or animated GIF, written beside the project by
 * the host. The engine renders in one call that reports no progress and cannot be stopped, so the
 * dialog shows that it is busy until the file is written.
 */
export function ExportDialog({ info, doc, onClose }: { info: MotionInfo; doc: KitEngine | null; onClose: () => void }) {
  const { host, t } = useKitEditor()
  const comp = info.comp
  const [format, setFormat] = useState<Format>('h264')
  const [span, setSpan] = useState<'comp' | 'workArea'>('comp')
  const [size, setSize] = useState<'full' | 'half' | 'custom'>('full')
  const [width, setWidth] = useState(String(comp?.width ?? 1920))
  const [height, setHeight] = useState(String(comp?.height ?? 1080))
  const [rate, setRate] = useState<'comp' | 'half'>('comp')
  const [busy, setBusy] = useState(false)
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
    const settings: Settings = {
      format,
      span,
      size: size === 'custom' && w !== null && h !== null ? { width: w, height: h } : size === 'half' ? 'half' : 'full',
      frameRate: rate === 'half' ? Math.round((comp.fps / 2) * 1000) / 1000 : null,
    }
    try {
      const bytes = await doc.invoke<Uint8Array>('renderMovie', path.split('/').pop() ?? path, JSON.stringify(settings))
      const written = await host.exportFile(path, bytes)
      toast.success(t('export.done', { name: written.split('/').pop() ?? written }))
      onClose()
    } catch (error) {
      toast.error(t('renderFailed'), { description: error instanceof Error ? error.message : String(error) })
      setBusy(false)
    }
  }

  return (
    <Dialog
      title={t('export.title')}
      testId="motion-export-dialog"
      onClose={() => !busy && onClose()}
      actions={
        <>
          <Button disabled={busy} onClick={onClose} data-testid="motion-export-dialog-cancel">
            {t('dialog.cancel')}
          </Button>
          <Button variant="primary" disabled={!valid || busy || !doc} onClick={() => void render()} data-testid="motion-export-dialog-ok">
            {t('export.render')}
          </Button>
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
        {busy ? (
          <p className="motion-export-busy" role="status" data-testid="motion-export-busy">
            <Spinner size={14} />
            {t('export.busy')}
          </p>
        ) : null}
        <button type="submit" hidden />
      </form>
    </Dialog>
  )
}
