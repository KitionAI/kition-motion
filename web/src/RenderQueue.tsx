import { useCallback, useEffect, useRef, useState } from 'react'
import { ArrowDownIcon, ArrowUpIcon, CopyIcon, PlayIcon, Settings2Icon, SquareIcon, TrashIcon, XIcon } from '@kition/tessera/icons/react'
import { Button, Checkbox, Dialog, Field, IconButton, Select, TextInput, formatTimecode, parseTimecode, toast, useEngineQuery, useKitEditor, useReadOnly, type HostConnection, type KitEngine } from '@kition/sdk/editor'
import { presetFolder } from './dialogs'
import { SEQUENCES, sequenceName, writeExport } from './ExportDialog'
import { parseAnswer, type ProjectItem, type ProjectSummary } from './footage'
import { CATALOGS, counted } from './strings'
import { failed, failedReason } from './errors'

/**
 * The Render Queue: export jobs kept in the project (the engine's `renderQueue.*`: each job is a
 * composition, a stretch of it, a format, a size, a frame rate and an output file), listed in a
 * panel under the Project panel. Render runs the jobs whose Render box is on, one after another,
 * on the same stepped export as File ▸ Export… (`exportStart`/`exportStep`), each written beside
 * the project by the host.
 *
 * The rule for edits made while it renders: Render takes a copy of the project when it is pressed
 * (`renderSnapshot`), and every job of that run renders from the copy. The editor stays usable:
 * edits, undo and new jobs go on in the document and reach the next run, never a job already
 * started or waiting in this one. The jobs of the run cannot be moved or removed until it ends.
 */

/** The formats a job renders here: movies in one file, PNG and JPEG sequences (a numbered file per frame) and WAV sound. Engine name (`renderQueue.add`), the item's serialized name, extension. */
export const QUEUE_FORMATS = [
  { id: 'h264', item: 'H264', extension: 'mp4' },
  { id: 'webm', item: 'WebM', extension: 'webm' },
  { id: 'gif', item: 'Gif', extension: 'gif' },
  { id: 'png', item: 'PngSequence', extension: 'png' },
  { id: 'jpeg', item: 'JpegSequence', extension: 'jpg' },
  { id: 'wav', item: 'Wav', extension: 'wav' },
] as const

export type QueueFormat = (typeof QUEUE_FORMATS)[number]['id']

/** A job's settings, as the job dialog edits them and presets keep them. */
export type JobSettings = {
  format: QueueFormat
  span: 'comp' | 'workArea' | 'custom'
  /** Seconds, for a custom span. */
  start: number
  end: number
  size: 'full' | 'half' | 'third' | 'quarter' | 'custom'
  width: number
  height: number
  /** Frames per second, or null for the composition's. */
  frameRate: number | null
  /** The file's name without its extension; empty: the composition's name. */
  name: string
  /** A folder inside the project's folder; empty: the project's folder. */
  folder: string
  /** A PNG sequence keeps its transparency (RGB + alpha). */
  alpha: boolean
}

/** A named preset of a job's output settings (not its composition, stretch or file name). */
export type QueuePreset = { name: string } & Pick<JobSettings, 'format' | 'size' | 'width' | 'height' | 'frameRate' | 'folder'> & Partial<Pick<JobSettings, 'alpha'>>

/** A job as `renderQueue.list` answers it (the fields the panel reads). */
export type QueueItem = {
  item?: number
  id: number
  index: number
  comp: number
  compName?: string | null
  render: boolean
  width?: number
  height?: number
  frames?: number
  frameRate?: number
  span?: { kind: 'workArea' | 'comp' | 'custom'; start: number; end: number }
  settings?: { resolution?: number }
  output?: { format?: string; output?: string; channels?: string; resize?: { enabled?: boolean; width?: number; height?: number } }
}

/** What happened to a job in this session's runs. */
export type JobState = {
  state: 'waiting' | 'rendering' | 'done' | 'failed' | 'stopped'
  done: number
  total: number
  /** `performance.now()` when it started, and how long it took once it ended (ms). */
  started?: number
  elapsed?: number
  /** The file written, or why it failed. */
  file?: string
  error?: string
}

const PRESETS_KEY = 'kition-motion.render-presets'
const LAST_KEY = 'kition-motion.render-last'
/** How long the engine renders per `exportStep` before it answers (and can hear Stop). */
const STEP_MS = 120
/** How long the engine is left idle between two steps of a render. */
const IDLE_MS = 40

const SCALES: Record<string, number> = { full: 1, half: 0.5, third: 1 / 3, quarter: 0.25 }

function readStored<T>(key: string, fallback: T): T {
  try {
    const text = globalThis.localStorage?.getItem(key)
    return text ? (JSON.parse(text) as T) : fallback
  } catch {
    return fallback
  }
}

function writeStored(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Kept for this page only.
  }
}

/** The presets saved in this browser. */
export const storedPresets = () => readStored<QueuePreset[]>(PRESETS_KEY, []).filter((preset) => preset && typeof preset.name === 'string' && QUEUE_FORMATS.some((format) => format.id === preset.format))

/** The output settings used last (what a new job starts from). */
const lastSettings = (): Partial<JobSettings> => readStored<Partial<JobSettings>>(LAST_KEY, {})

/** A name for a file: without the characters a file name cannot have. */
export const fileName = (name: string) => name.replace(/[\\/:*?"<>|]+/g, '_').trim()

/** The `renderQueue.add` / `setRenderSettings` / `setOutputModule` parameters for `settings`. */
function renderParams(settings: JobSettings) {
  return {
    timeSpan: settings.span,
    ...(settings.span === 'custom' ? { start: settings.start, end: settings.end } : {}),
    resolution: settings.size === 'custom' ? 'full' : settings.size,
    frameRate: settings.frameRate,
  }
}
function outputParams(settings: JobSettings, compName: string) {
  const extension = QUEUE_FORMATS.find((format) => format.id === settings.format)?.extension ?? 'mp4'
  const name = fileName(settings.name) || fileName(compName) || 'Composition'
  return {
    format: settings.format,
    resize: settings.size === 'custom' ? { enabled: true, width: settings.width, height: settings.height, lockAspect: false } : false,
    // A sequence: the frame number after the name (`Comp 1_00000.png`, …).
    output: [settings.folder, SEQUENCES.has(settings.format) ? sequenceName(name, extension) : `${name}.${extension}`].filter(Boolean).join('/'),
    ...(settings.format === 'png' ? { channels: settings.alpha ? 'rgba' : 'rgb' } : {}),
  }
}

/** A job's format id, from the item's serialized format. */
const formatOf = (item: QueueItem) => QUEUE_FORMATS.find((format) => format.item === item.output?.format)

/** A job's output, folder and file, with `[compName]` and `[fileExtension]` filled in. */
export function outputOf(item: QueueItem) {
  const format = formatOf(item)
  const extension = format?.extension ?? (item.output?.format ?? '').toLowerCase()
  const raw = (item.output?.output || '[compName].[fileExtension]').replace(/\[compName\]/g, item.compName ?? 'Composition').replace(/\[fileExtension\]/g, extension)
  const parts = raw.replace(/\\/g, '/').split('/').filter(Boolean)
  const file = fileName(parts.pop() ?? 'Composition') || 'Composition'
  const named = extension && !file.toLowerCase().endsWith(`.${extension}`) ? `${file.replace(/\.[^.]*$/, '')}.${extension}` : file
  return { folder: presetFolder(parts.join('/')) ?? '', file: named }
}

/** The settings of an existing job, for its dialog. */
function settingsOf(item: QueueItem): JobSettings {
  const scale = item.settings?.resolution ?? 1
  const resize = item.output?.resize
  const size = resize?.enabled ? 'custom' : ((Object.entries(SCALES).find(([, value]) => Math.abs(value - scale) < 1e-3)?.[0] ?? 'full') as JobSettings['size'])
  const { folder, file } = outputOf(item)
  return {
    format: formatOf(item)?.id ?? 'h264',
    span: item.span?.kind ?? 'workArea',
    start: item.span?.start ?? 0,
    end: item.span?.end ?? 1,
    size,
    width: resize?.width ?? item.width ?? 1920,
    height: resize?.height ?? item.height ?? 1080,
    frameRate: null,
    name: file.replace(/\.[^.]*$/, '').replace(/_\[#+\]$/, ''),
    folder,
    alpha: (item.output?.channels ?? '').toLowerCase().includes('a'),
  }
}

/**
 * A new job's settings for `comp`: the ones used last (format, size, rate; the folder of the last
 * job that was not a sequence), the whole composition, its name. An image sequence (one file per
 * frame) goes into a folder of its own, named after the composition (`Comp 1_sequence`, numbered
 * when a job in the queue already writes there: `taken`), never loose in the project's folder.
 */
function newSettings(comp: { name: string; width: number; height: number }, t: Strings, taken: ReadonlySet<string>): JobSettings {
  const last = lastSettings()
  const format: QueueFormat = QUEUE_FORMATS.some((each) => each.id === last.format) ? (last.format as QueueFormat) : 'h264'
  return {
    format,
    span: last.span === 'workArea' ? 'workArea' : 'comp',
    start: 0,
    end: 1,
    size: last.size && (last.size === 'custom' || SCALES[last.size]) ? last.size : 'full',
    width: last.width ?? comp.width,
    height: last.height ?? comp.height,
    frameRate: typeof last.frameRate === 'number' && last.frameRate > 0 ? last.frameRate : null,
    name: '',
    folder: defaultFolder(format, comp.name, t, taken),
    alpha: last.alpha !== false,
  }
}

type Strings = (key: string, values?: Record<string, string | number>) => string

/**
 * The folder a job of `format` gets by itself: an image sequence its own folder (`sequenceFolder`),
 * anything else the folder of the last job that was not a sequence (else the project's folder).
 */
function defaultFolder(format: QueueFormat, compName: string, t: Strings, taken: ReadonlySet<string>) {
  if (SEQUENCES.has(format)) return sequenceFolder(compName, t, taken)
  const last = lastSettings().folder
  return typeof last === 'string' ? (presetFolder(last) ?? '') : ''
}

/** A composition's image-sequence folder name, before any number, from a `queue.sequenceFolder` pattern. */
const baseFrom = (pattern: string, compName: string) => presetFolder(fileName(pattern.replace('{name}', fileName(compName) || 'Composition'))) || 'sequence'

/** The name of a composition's image-sequence folder in the interface's language, before any number (what a new folder is called). */
const sequenceBase = (compName: string, t: Strings) => baseFrom(t('queue.sequenceFolder', { name: '{name}' }), compName)

/**
 * The sequence-folder names the editor makes for a composition in every language it speaks
 * (`Comp 1_sequence`, `Comp 1_序列`), before any number: which ones the editor made does not
 * depend on the language the interface shows now.
 */
const sequenceBases = (compName: string) => [...new Set(Object.values(CATALOGS).map((catalog) => catalog['queue.sequenceFolder']).filter((pattern): pattern is string => typeof pattern === 'string').map((pattern) => baseFrom(pattern, compName)))]

/**
 * Whether a job's folder is still one the editor chose for it (none, or its composition's
 * image-sequence folder in any language, numbered or not) rather than one the person typed or
 * took from a preset.
 */
function automaticFolder(folder: string, compName: string) {
  if (!folder) return true
  return sequenceBases(compName).some((base) => folder === base || (folder.startsWith(`${base} `) && /^\d+$/.test(folder.slice(base.length + 1))))
}

/** An image sequence's own folder, named after its composition, numbered past the folders in `taken`. */
function sequenceFolder(compName: string, t: Strings, taken: ReadonlySet<string>) {
  const base = sequenceBase(compName, t)
  for (let n = 1; n < 1000; n += 1) {
    const folder = n === 1 ? base : `${base} ${n}`
    if (!taken.has(folder)) return folder
  }
  return base
}

/** The folders the queue's jobs write to. */
const foldersOf = (items: readonly QueueItem[] | undefined, t: Strings) => new Set((items ?? []).map((item) => jobFolder(item, t)).filter(Boolean))

/**
 * The folder a job really writes to: its own, or for an image sequence without one its
 * composition's sequence folder (`Comp 1_sequence`): frames never land loose in the project's
 * folder. The job's row shows this folder, so what it shows is what the render does. A sequence
 * job keeps its folder from when it is made or becomes a sequence (the project holds it, so every
 * language renders to the same place); only a job saved without one falls back to this name.
 */
function jobFolder(item: QueueItem, t: Strings) {
  const { folder } = outputOf(item)
  const format = formatOf(item)
  return folder || (format && SEQUENCES.has(format.id) ? sequenceBase(item.compName ?? '', t) : '')
}

/**
 * The folder each job of this page got from the job before it when it was added (the folder used
 * last), by job id: a default, not the person's choice, so a change of format may still replace
 * it. A job whose folder was typed or taken from a preset in its own dialog leaves this list.
 */
const inheritedFolders = new Map<number, string>()

/** Milliseconds as `1:05` (minutes and seconds). */
const clock = (ms: number) => {
  const seconds = Math.max(0, Math.round(ms / 1000))
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}

/**
 * The queue's runs: Render, Stop (the job rendering; the rest go on) and Stop All, and what each
 * job of this session came to. Lives with the editor, so a run goes on with the panel closed.
 */
export function useRenderQueue({ doc, host, run, query, t }: { doc: KitEngine | null; host: HostConnection; run: (command: string, params?: Record<string, unknown>) => Promise<string | null>; query: <T>(command: string, params?: Record<string, unknown>) => Promise<T | null>; t: Strings }) {
  const [jobs, setJobs] = useState<ReadonlyMap<number, JobState>>(() => new Map())
  const [rendering, setRendering] = useState(false)
  const [current, setCurrent] = useState<number | null>(null)
  const stopOne = useRef(false)
  const stopAll = useRef(false)
  const busy = useRef(false)
  const update = (id: number, change: Partial<JobState>) =>
    setJobs((now) => {
      const next = new Map(now)
      next.set(id, { ...(now.get(id) ?? { state: 'waiting', done: 0, total: 0 }), ...change })
      return next
    })
  const folder = host.init.document.path.split('/').slice(0, -1).join('/')

  /** Adds jobs for `comps`, each with the settings used last; answers the new jobs' ids. */
  const add = useCallback(
    async (comps: Array<{ id: number; name: string; width: number; height: number }>, settings?: JobSettings) => {
      const made: number[] = []
      const taken = foldersOf((await query<{ items?: QueueItem[] }>('renderQueue.list', {}).catch(() => null))?.items, t)
      for (const comp of comps) {
        const values = settings ?? newSettings(comp, t, taken)
        taken.add(values.folder)
        const answer = parseAnswer<{ item?: number }>(await run('renderQueue.add', { comp: comp.id, ...renderParams(values), ...outputParams(values, comp.name) }))
        if (answer?.item == null) continue
        made.push(answer.item)
        if (!settings && values.folder) inheritedFolders.set(answer.item, values.folder)
      }
      if (made.length) toast.success(made.length === 1 ? t('queue.added', { name: comps[0]?.name ?? '' }) : t('queue.addedMany', { count: made.length }))
      return made
    },
    [run, query, t],
  )

  /** Render: every job whose Render box is on, one after another, from a copy of the project taken now. */
  const render = useCallback(
    async (items: readonly QueueItem[]) => {
      if (!doc || busy.current) return
      busy.current = true
      stopAll.current = false
      setRendering(true)
      let ids: number[] = []
      try {
        ids = (parseAnswer<number[]>(await doc.invoke<string>('renderSnapshot')) ?? []).filter((id) => typeof id === 'number')
      } catch (error) {
        failed(t, t('queue.failed'), error)
      }
      const byId = new Map(items.map((item) => [item.id, item]))
      setJobs((now) => {
        const next = new Map(now)
        for (const id of ids) next.set(id, { state: 'waiting', done: 0, total: byId.get(id)?.frames ?? 0 })
        return next
      })
      let written = 0
      for (const id of ids) {
        if (stopAll.current) break
        const item = byId.get(id)
        const { file } = item ? outputOf(item) : { file: `render-${id}.mp4` }
        const inside = item ? jobFolder(item, t) : ''
        stopOne.current = false
        setCurrent(id)
        const started = performance.now()
        update(id, { state: 'rendering', started, done: 0 })
        const read = (json: string) => JSON.parse(json) as { done: number; total: number; finished: boolean }
        try {
          let now = read(await doc.invoke<string>('exportStart', file, JSON.stringify({ queueItem: id })))
          update(id, { done: now.done, total: now.total })
          while (!now.finished && !stopOne.current && !stopAll.current) {
            now = read(await doc.invoke<string>('exportStep', STEP_MS))
            update(id, { done: now.done, total: now.total })
            // A pause between steps: the engine's queue empties, so keys held while it answered
            // (Delete, the tools' letters) and the person's edits get their turn.
            await new Promise((resolve) => window.setTimeout(resolve, IDLE_MS))
          }
          if (stopOne.current || stopAll.current) {
            await doc.invoke<boolean>('exportCancel')
            update(id, { state: 'stopped', elapsed: performance.now() - started })
            continue
          }
          // The movie or sound, or every frame of a sequence, into the job's folder.
          const paths = await writeExport(doc, host, [folder, inside].filter(Boolean).join('/'))
          written += 1
          const shown = (path: string) => path.split('/').slice(folder ? folder.split('/').length : 0).join('/') || path
          update(id, { state: 'done', elapsed: performance.now() - started, file: paths.length === 1 ? shown(paths[0]) : t('queue.files', { count: paths.length, first: shown(paths[0] ?? '') }) })
        } catch (error) {
          await doc.invoke<boolean>('exportCancel').catch(() => false)
          update(id, { state: 'failed', elapsed: performance.now() - started, error: failedReason(t, `render queue job ${id} (${file})`, error) })
        }
      }
      // Jobs this run never reached (Stop All) wait for the next.
      setJobs((now) => {
        const next = new Map(now)
        for (const id of ids) if (next.get(id)?.state === 'waiting') next.delete(id)
        return next
      })
      await doc.invoke<boolean>('dropRenderSnapshot').catch(() => false)
      setCurrent(null)
      setRendering(false)
      busy.current = false
      if (written) toast.success(counted(t, 'queue.finished', written))
    },
    [doc, folder, host, t],
  )

  return {
    jobs,
    rendering,
    current,
    add,
    render,
    stop: () => (stopOne.current = true),
    stopAll: () => (stopAll.current = true),
  }
}

export type RenderQueueRuns = ReturnType<typeof useRenderQueue>

/**
 * The Render Queue panel: the project's jobs in order, each with its composition, output file,
 * format, size and frame rate, and in this session's runs its progress (frames, time taken and
 * left) or result (the file written, or why it failed). Its buttons move a job up or down,
 * duplicate it, change its settings and remove it; the header renders, stops and closes.
 */
export function RenderQueuePanel({ queue, onClose }: { queue: RenderQueueRuns; onClose: () => void }) {
  const { run, t } = useKitEditor()
  const comps = compsOf(useEngineQuery<ProjectSummary>('project.summary', {}))
  const readOnly = useReadOnly()
  const list = useEngineQuery<{ items?: QueueItem[] }>('renderQueue.list', {})
  const items = list?.items ?? []
  const [editing, setEditing] = useState<{ item: QueueItem | null; comp: ProjectItem | null } | null>(null)
  const [, tick] = useState(0)
  // While it renders, the times shown move on each second.
  useEffect(() => {
    if (!queue.rendering) return undefined
    const timer = window.setInterval(() => tick((n) => n + 1), 1000)
    return () => window.clearInterval(timer)
  }, [queue.rendering])
  const locked = readOnly || queue.rendering
  const anyOn = items.some((item) => item.render)

  return (
    <section className="kui-panel motion-queue" aria-label={t('queue.title')} data-testid="motion-queue-panel" data-rendering={queue.rendering || undefined}>
      {/* The title and Close on one line, the run's buttons on their own: nothing wraps in a narrow column. */}
      <div className="motion-project-head">
        <span className="motion-project-title motion-queue-title">{t('queue.title')}</span>
        <span style={{ flex: 1 }} />
        <IconButton icon={XIcon} label={t('queue.close')} onClick={onClose} data-testid="motion-queue-close" />
      </div>
      <div className="motion-queue-actions">
        {queue.rendering ? (
          <>
            <Button variant="ghost" className="motion-queue-stop" onClick={queue.stop} title={t('queue.stop.hint')} data-testid="motion-queue-stop">
              <SquareIcon size={11} aria-hidden="true" />
              {t('queue.stop')}
            </Button>
            <Button variant="ghost" className="motion-queue-stop" onClick={queue.stopAll} title={t('queue.stopAll.hint')} data-testid="motion-queue-stop-all">
              {t('queue.stopAll')}
            </Button>
          </>
        ) : (
          <Button variant="ghost" disabled={readOnly || !anyOn} onClick={() => void queue.render(items)} title={anyOn ? undefined : t('queue.nothingOn')} data-testid="motion-queue-render">
            <PlayIcon size={12} aria-hidden="true" />
            {t('queue.render')}
          </Button>
        )}
      </div>
      <div className="kui-panel-body motion-queue-list" data-testid="motion-queue-list">
        {items.length ? (
          items.map((item, at) => {
            const job = queue.jobs.get(item.id)
            const { file } = outputOf(item)
            const folder = jobFolder(item, t)
            const format = formatOf(item)
            const canRender = format != null
            const elapsed = job?.state === 'rendering' && job.started != null ? performance.now() - job.started : (job?.elapsed ?? 0)
            const left = job?.state === 'rendering' && job.done > 0 && job.total > job.done ? (elapsed * (job.total - job.done)) / job.done : null
            return (
              <div key={item.id} className="motion-queue-row" data-state={job?.state ?? (item.render ? 'queued' : 'off')} data-testid={`motion-queue-item-${item.id}`}>
                <div className="motion-queue-line">
                  <input
                    type="checkbox"
                    checked={item.render}
                    disabled={locked}
                    aria-label={t('queue.renderBox', { name: item.compName ?? '' })}
                    onChange={() => void run('renderQueue.setRender', { item: item.id, render: !item.render })}
                    data-testid={`motion-queue-item-${item.id}-on`}
                  />
                  <span className="motion-queue-index kui-muted">{at + 1}</span>
                  <span className="motion-queue-comp" title={item.compName ?? ''}>
                    {item.compName ?? t('queue.noComp')}
                  </span>
                </div>
                {/* The job's buttons on a line of their own, under its name: the name keeps the row's width. */}
                <div className="motion-queue-line motion-queue-tools-line">
                  <span className="motion-queue-tools">
                    <IconButton icon={Settings2Icon} label={t('queue.settings')} disabled={locked} onClick={() => setEditing({ item, comp: comps.find((comp) => comp.id === item.comp) ?? null })} data-testid={`motion-queue-item-${item.id}-settings`} />
                    <IconButton icon={ArrowUpIcon} label={t('queue.up')} disabled={locked || at === 0} onClick={() => void run('renderQueue.move', { item: item.id, to: at })} data-testid={`motion-queue-item-${item.id}-up`} />
                    <IconButton icon={ArrowDownIcon} label={t('queue.down')} disabled={locked || at === items.length - 1} onClick={() => void run('renderQueue.move', { item: item.id, to: at + 2 })} data-testid={`motion-queue-item-${item.id}-down`} />
                    <IconButton icon={CopyIcon} label={t('queue.duplicate')} disabled={locked} onClick={() => void run('renderQueue.duplicate', { item: item.id })} data-testid={`motion-queue-item-${item.id}-duplicate`} />
                    <IconButton icon={TrashIcon} label={t('queue.remove')} disabled={locked} onClick={() => void run('renderQueue.remove', { item: item.id })} data-testid={`motion-queue-item-${item.id}-remove`} />
                  </span>
                </div>
                {/* Where it goes, on a line of its own: a new job takes the folder used last. */}
                <div className="motion-queue-folder" data-testid={`motion-queue-item-${item.id}-folder`} data-folder={folder} title={t('queue.folderHint')}>
                  {t('queue.into', { folder: folder ? `${folder}/` : t('queue.projectFolder') })}
                </div>
                <div className="motion-queue-detail kui-muted" data-testid={`motion-queue-item-${item.id}-detail`}>
                  {[folder ? `${folder}/${file}` : file, format ? t(`export.format.${format.id}`) : t('queue.unsupported', { format: item.output?.format ?? '' }), item.width && item.height ? `${item.width} × ${item.height}` : '', item.frameRate ? t('queue.fps', { fps: Math.round(item.frameRate * 100) / 100 }) : '', item.span ? t(`queue.span.${item.span.kind}`, { start: Math.round(item.span.start * 100) / 100, end: Math.round(item.span.end * 100) / 100 }) : '']
                    .filter(Boolean)
                    .join(' · ')}
                </div>
                {job ? (
                  <div className="motion-queue-status" role="status" data-testid={`motion-queue-item-${item.id}-status`} data-state={job.state} data-done={job.done} data-total={job.total}>
                    {job.state === 'rendering' ? (
                      <>
                        <div className="motion-export-bar" role="progressbar" aria-valuemin={0} aria-valuemax={job.total} aria-valuenow={job.done}>
                          <span style={{ width: `${job.total > 0 ? Math.min(100, (job.done / job.total) * 100) : 0}%` }} />
                        </div>
                        <span>{t('queue.progress', { done: job.done, total: job.total, elapsed: clock(elapsed), left: left == null ? '–' : clock(left) })}</span>
                      </>
                    ) : (
                      <span>{t(`queue.state.${job.state}`, { file: job.file ?? '', error: job.error ?? '', elapsed: clock(job.elapsed ?? 0) })}</span>
                    )}
                  </div>
                ) : !canRender ? (
                  <div className="motion-queue-status" data-state="unsupported">
                    {t('queue.cannotRender')}
                  </div>
                ) : null}
              </div>
            )
          })
        ) : (
          <p className="kui-muted motion-empty">{t('queue.empty')}</p>
        )}
      </div>
      {editing ? <JobDialog item={editing.item} comp={editing.comp} queued={items} onClose={() => setEditing(null)} /> : null}
    </section>
  )
}

/**
 * A job's settings: its stretch (the whole composition, its work area, or from a time to a time),
 * format, size (full, half, third, quarter or a width and height), frame rate (the composition's or
 * another) and output file (name and folder inside the project's folder), with presets of the
 * output settings kept in this browser. OK changes the job (`renderQueue.setRenderSettings` and
 * `setOutputModule`, one undo step each).
 */
function JobDialog({ item, comp, queued, onClose }: { item: QueueItem | null; comp: ProjectItem | null; queued: readonly QueueItem[]; onClose: () => void }) {
  const { run, t } = useKitEditor()
  const compName = item?.compName ?? comp?.name ?? ''
  const fps = comp?.frameRate ?? item?.frameRate ?? 30
  // The queue's jobs as the panel lists them (already read when a job's settings open: a change of
  // format right away still sees which sequence folders the other jobs take).
  const start = item ? settingsOf(item) : newSettings({ name: compName, width: comp?.size?.[0] ?? 1920, height: comp?.size?.[1] ?? 1080 }, t, foldersOf(queued, t))
  const [settings0] = useState<JobSettings>(() => ({ ...start, frameRate: item?.frameRate != null && Math.abs(item.frameRate - fps) > 1e-3 ? item.frameRate : null }))
  const [settings, setSettings] = useState<JobSettings>(settings0)
  // While the folder is still the one the editor chose (empty, the composition's sequence folder,
  // numbered or not, or the folder this job took from the job before it), a change of format
  // chooses it again: an image sequence its own folder, anything else the folder it started with
  // (when that was the editor's too) or the folder a new job of that format would get. A box that
  // is empty or holds the composition's sequence folder is automatic whatever was typed before (a
  // typed name equal to the generated one cannot be told apart from it); any other folder typed or
  // taken from a preset in this job's dialog stays.
  const [startedAuto] = useState(() => automaticFolder(settings0.folder, compName) || (item != null && inheritedFolders.get(item.id) === settings0.folder))
  const [autoFolder, setAutoFolder] = useState(startedAuto)
  const isAuto = (value: string) => autoFolder || automaticFolder(value.trim(), compName)
  /** The folder a job that is not a sequence gets by itself: never the composition's sequence folder. */
  const movieFolder = (format: QueueFormat) => {
    const folder = startedAuto && !SEQUENCES.has(settings0.format) ? settings0.folder : defaultFolder(format, compName, t, new Set())
    return folder && automaticFolder(folder, compName) ? '' : folder
  }
  const changeFormat = (format: QueueFormat) => {
    if (!isAuto(settings.folder) || SEQUENCES.has(format) === SEQUENCES.has(settings.format)) return set({ format })
    const own = item ? jobFolder(item, t) : ''
    const others = new Set([...foldersOf(queued, t)].filter((each) => each !== own))
    // The folder chosen here is the editor's again, not the person's.
    setAutoFolder(true)
    set({ format, folder: SEQUENCES.has(format) ? defaultFolder(format, compName, t, others) : movieFolder(format) })
  }
  const [from, setFrom] = useState(formatTimecode(start.start, fps))
  const [to, setTo] = useState(formatTimecode(start.end, fps))
  const [width, setWidth] = useState(String(settings.width))
  const [height, setHeight] = useState(String(settings.height))
  const [rate, setRate] = useState(String(settings.frameRate ?? Math.round(fps * 1000) / 1000))
  const [presets, setPresets] = useState(storedPresets)
  const [presetName, setPresetName] = useState('')
  const set = (change: Partial<JobSettings>) => setSettings((now) => ({ ...now, ...change }))
  const startAt = parseTimecode(from, fps)
  const endAt = parseTimecode(to, fps)
  const side = (text: string) => {
    const value = Number(text.trim())
    return Number.isInteger(value) && value >= 16 && value <= 8192 ? value : null
  }
  const w = side(width)
  const h = side(height)
  const customRate = Number(rate.replace(',', '.'))
  const rateOk = settings.frameRate === null || (Number.isFinite(customRate) && customRate > 0 && customRate <= 240)
  const spanOk = settings.span !== 'custom' || (startAt !== null && endAt !== null && endAt > startAt)
  const sizeOk = settings.size !== 'custom' || (w !== null && h !== null)
  const folder = presetFolder(settings.folder)
  const valid = spanOk && sizeOk && rateOk && folder !== null && !/[\\/:*?"<>|]/.test(settings.name)
  // A job that is not a sequence never keeps the composition's sequence folder (in any language):
  // that box is automatic. A sequence left without a folder gets the composition's sequence folder
  // (the one an empty box always meant), kept in the job now: where it renders never depends on
  // the interface language at render time.
  const finalFolder = () => {
    if (folder && !SEQUENCES.has(settings.format) && automaticFolder(folder, compName)) return movieFolder(settings.format)
    if (!folder && SEQUENCES.has(settings.format)) return sequenceBase(compName, t)
    return folder ?? ''
  }
  const final = (): JobSettings => ({ ...settings, start: startAt ?? 0, end: endAt ?? 1, width: w ?? settings.width, height: h ?? settings.height, frameRate: settings.frameRate === null ? null : customRate, folder: finalFolder() })
  const save = async () => {
    if (!valid || !item) return
    const values = final()
    // A sequence's folder holds that sequence's frames: the next job does not take it (it keeps
    // the folder of the last job that was not a sequence, else the project's folder).
    const folderNext = SEQUENCES.has(values.format) ? (lastSettings().folder ?? '') : values.folder
    writeStored(LAST_KEY, { format: values.format, span: values.span === 'custom' ? 'comp' : values.span, size: values.size, width: values.width, height: values.height, frameRate: values.frameRate, folder: folderNext, alpha: values.alpha })
    // Only what changed: the engine refuses a change that changes nothing.
    const was = { ...start, frameRate: settings0.frameRate }
    const renderChanged = JSON.stringify(renderParams(values)) !== JSON.stringify(renderParams(was))
    const outputChanged = JSON.stringify(outputParams(values, compName)) !== JSON.stringify(outputParams(was, compName))
    if (renderChanged && (await run('renderQueue.setRenderSettings', { item: item.id, ...renderParams(values) })) === null) return
    if (outputChanged && (await run('renderQueue.setOutputModule', { item: item.id, ...outputParams(values, compName) })) === null) return
    // A folder the person chose here is theirs from now on.
    if (!isAuto(settings.folder) && values.folder) inheritedFolders.delete(item.id)
    onClose()
  }
  const applyPreset = (name: string) => {
    const preset = presets.find((each) => each.name === name)
    if (!preset) return
    setAutoFolder(false)
    set({ format: preset.format, size: preset.size, frameRate: preset.frameRate, folder: preset.folder, ...(preset.alpha != null ? { alpha: preset.alpha } : {}) })
    setWidth(String(preset.width))
    setHeight(String(preset.height))
    if (preset.frameRate !== null) setRate(String(preset.frameRate))
  }
  const savePreset = () => {
    const name = presetName.trim()
    if (!name || !valid) return
    const values = final()
    const next = [...presets.filter((each) => each.name !== name), { name, format: values.format, size: values.size, width: values.width, height: values.height, frameRate: values.frameRate, folder: values.folder, alpha: values.alpha }]
    writeStored(PRESETS_KEY, next)
    setPresets(next)
    setPresetName('')
    toast.success(t('queue.presetSaved', { name }))
  }
  const deletePreset = (name: string) => {
    const next = presets.filter((each) => each.name !== name)
    writeStored(PRESETS_KEY, next)
    setPresets(next)
  }
  const [chosenPreset, setChosenPreset] = useState('')
  return (
    <Dialog
      title={t('queue.dialog', { name: compName })}
      testId="motion-job-dialog"
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose} data-testid="motion-job-dialog-cancel">
            {t('dialog.cancel')}
          </Button>
          <Button variant="primary" disabled={!valid} onClick={() => void save()} data-testid="motion-job-dialog-ok">
            {t('dialog.ok')}
          </Button>
        </>
      }
    >
      <form
        className="motion-form"
        onSubmit={(event) => {
          event.preventDefault()
          void save()
        }}
      >
        <Field label={t('queue.preset')}>
          {({ id }) => (
            <span className="motion-form-input">
              <Select
                id={id}
                value={chosenPreset}
                onChange={(event) => {
                  setChosenPreset(event.target.value)
                  applyPreset(event.target.value)
                }}
                data-testid="motion-job-preset"
              >
                <option value="">{presets.length ? t('queue.preset.choose') : t('queue.preset.none')}</option>
                {presets.map((preset) => (
                  <option key={preset.name} value={preset.name}>
                    {preset.name}
                  </option>
                ))}
              </Select>
              {chosenPreset ? (
                <Button
                  variant="ghost"
                  onClick={() => {
                    deletePreset(chosenPreset)
                    setChosenPreset('')
                  }}
                  data-testid="motion-job-preset-delete"
                >
                  {t('queue.preset.delete')}
                </Button>
              ) : null}
            </span>
          )}
        </Field>
        <Field label={t('export.range')}>
          {({ id }) => (
            <Select id={id} value={settings.span} onChange={(event) => set({ span: event.target.value as JobSettings['span'] })} data-testid="motion-job-span">
              <option value="comp">{t('queue.span.choice.comp')}</option>
              <option value="workArea">{t('queue.span.choice.workArea')}</option>
              <option value="custom">{t('queue.span.choice.custom')}</option>
            </Select>
          )}
        </Field>
        {settings.span === 'custom' ? (
          <div className="motion-form-pair">
            <Field label={t('queue.from')} error={startAt === null ? t('queue.badTime') : undefined}>
              {({ id, invalid }) => <TextInput id={id} invalid={invalid} value={from} onChange={(event) => setFrom(event.target.value)} data-testid="motion-job-start" />}
            </Field>
            <Field label={t('queue.to')} error={endAt === null || (startAt !== null && endAt <= startAt) ? t('queue.badEnd') : undefined}>
              {({ id, invalid }) => <TextInput id={id} invalid={invalid} value={to} onChange={(event) => setTo(event.target.value)} data-testid="motion-job-end" />}
            </Field>
          </div>
        ) : null}
        <Field label={t('export.format')}>
          {({ id }) => (
            <Select id={id} value={settings.format} onChange={(event) => changeFormat(event.target.value as QueueFormat)} data-testid="motion-job-format">
              {QUEUE_FORMATS.map((format) => (
                <option key={format.id} value={format.id}>
                  {t(`export.format.${format.id}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        {settings.format === 'png' ? <Checkbox label={t('export.alpha')} checked={settings.alpha} onChange={(alpha) => set({ alpha })} testId="motion-job-alpha" /> : null}
        <Field label={t('export.size')}>
          {({ id }) => (
            <Select id={id} value={settings.size} onChange={(event) => set({ size: event.target.value as JobSettings['size'] })} data-testid="motion-job-size">
              {(['full', 'half', 'third', 'quarter'] as const).map((size) => (
                <option key={size} value={size}>
                  {t(`queue.size.${size}`)}
                </option>
              ))}
              <option value="custom">{t('dialog.custom')}</option>
            </Select>
          )}
        </Field>
        {settings.size === 'custom' ? (
          <div className="motion-form-pair">
            <Field label={t('dialog.width')} error={w === null ? t('export.badSize') : undefined}>
              {({ id, invalid }) => <TextInput id={id} invalid={invalid} value={width} onChange={(event) => setWidth(event.target.value)} data-testid="motion-job-width" />}
            </Field>
            <Field label={t('dialog.height')} error={h === null ? t('export.badSize') : undefined}>
              {({ id, invalid }) => <TextInput id={id} invalid={invalid} value={height} onChange={(event) => setHeight(event.target.value)} data-testid="motion-job-height" />}
            </Field>
          </div>
        ) : null}
        <Field label={t('dialog.frameRate')}>
          {({ id }) => (
            <Select id={id} value={settings.frameRate === null ? 'comp' : 'custom'} onChange={(event) => set({ frameRate: event.target.value === 'comp' ? null : Number(rate) || fps })} data-testid="motion-job-rate-mode">
              <option value="comp">{t('export.rate.comp', { fps: Math.round(fps * 100) / 100 })}</option>
              <option value="custom">{t('dialog.custom')}</option>
            </Select>
          )}
        </Field>
        {settings.frameRate !== null ? (
          <Field label={t('queue.customRate')} error={rateOk ? undefined : t('dialog.badNumber')}>
            {({ id, invalid }) => <TextInput id={id} invalid={invalid} value={rate} onChange={(event) => setRate(event.target.value)} data-testid="motion-job-rate" />}
          </Field>
        ) : null}
        <Field label={t('queue.fileName')} error={/[\\/:*?"<>|]/.test(settings.name) ? t('queue.badName') : undefined}>
          {({ id, invalid }) => <TextInput id={id} invalid={invalid} value={settings.name} placeholder={fileName(compName)} onChange={(event) => set({ name: event.target.value })} data-testid="motion-job-name" />}
        </Field>
        <Field label={t('queue.folder')} error={folder === null ? t('preset.badFolder') : undefined}>
          {({ id, invalid }) => (
            <TextInput
              id={id}
              invalid={invalid}
              value={settings.folder}
              onChange={(event) => {
                setAutoFolder(false)
                set({ folder: event.target.value })
              }}
              data-testid="motion-job-folder"
            />
          )}
        </Field>
        <div className="motion-form-input">
          <TextInput value={presetName} placeholder={t('queue.presetName')} aria-label={t('queue.presetName')} onChange={(event) => setPresetName(event.target.value)} data-testid="motion-job-preset-name" />
          <Button disabled={!presetName.trim() || !valid} onClick={savePreset} data-testid="motion-job-preset-save">
            {t('queue.savePreset')}
          </Button>
        </div>
        <button type="submit" hidden />
      </form>
    </Dialog>
  )
}

/** The compositions of a project summary. */
export const compsOf = (summary: ProjectSummary | null) => (summary?.items ?? []).filter((item) => item.type === 'Composition')
