import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type RefObject } from 'react'
import { DownloadIcon as Download, PanelLeftIcon } from '@kition/tessera/icons/react'
import {
  Button,
  EditorFrame,
  EngineToolRail,
  FrameChrome,
  KitDocumentCanvas,
  KitEditorProvider,
  HAND_TOOL,
  ZOOM_TOOL,
  catalogTabs,
  createKitStrings,
  shortcutMatches,
  toast,
  useEngineTools,
  useKitEditorState,
  useKitSession,
  type FrameTab,
  type HostConnection,
  type KitCanvasHandle,
  type Phrases,
  type RasterView,
} from '@kition/sdk/editor'
import { engineManifest } from './engine'
import { CATALOGS } from './strings'
import { describe, picturePanBounds, timecode } from './motion'
import { ARRANGE, KIT, KIT_RUNS, MOTION_PRIMARY, SHORTCUT_OVERRIDES, TRIED_ROWS, VIEW_MENUS, hiddenRow, menuRows, shortcutCommands } from './menus'
import { EFFECT_PREFIX, EffectRibbon, FIND_EFFECT, FindEffectDialog, type MotionEffect } from './effects'
import { CanvasScrollbars, createViewStore } from './CanvasScrollbars'
import { HomeTab } from './HomeTab'
import { motionReport } from './errors'
import { MotionDialogs, toHex, type MotionDialog, type Rgb } from './dialogs'
import { Timeline, type TimelineKeys } from './timeline/Timeline'
import { Transport } from './Transport'
import { Splitter, useTimelineHeight } from './Splitter'
import { PropertiesPanel } from './properties/PropertiesPanel'
import { EffectCatalog } from './properties/catalog'
import { ProjectPanel, useProjectPanel } from './ProjectPanel'
import { ExportDialog } from './ExportDialog'
import { ITEM_MIME, parseAnswer, useFootage } from './footage'
import { EDITOR_KEYS, pressed } from './keys'
import { INTERPOLATIONS } from './timeline/interp'
import { SamplerContext, type CurveSampler } from './timeline/samples'

/** Plays the composition from the current time against the wall clock; frames the engine cannot render in time are skipped, as a preview does. */
function usePlayback(run: (command: string, params: Record<string, unknown>) => Promise<unknown>, time: number, duration: number) {
  const [playing, setPlaying] = useState(false)
  const latest = useRef({ time, duration })
  latest.current = { time, duration }
  const origin = useRef<{ wall: number; time: number } | null>(null)
  const stop = useCallback(() => {
    origin.current = null
    setPlaying(false)
  }, [])
  const toggle = useCallback(() => {
    if (playing) {
      stop()
      return
    }
    const from = latest.current.time >= latest.current.duration - 1e-3 ? 0 : latest.current.time
    origin.current = { wall: performance.now(), time: from }
    setPlaying(true)
  }, [playing, stop])
  useEffect(() => {
    if (!playing) return undefined
    let stopped = false
    let busy = false
    const step = async () => {
      if (stopped || !origin.current) return
      const at = origin.current.time + (performance.now() - origin.current.wall) / 1000
      if (at >= latest.current.duration) {
        stop()
        return
      }
      if (!busy) {
        busy = true
        await run('time.set', { time: at }).catch(stop)
        busy = false
      }
      requestAnimationFrame(() => void step())
    }
    void step()
    return () => {
      stopped = true
    }
  }, [playing, run, stop])
  return { playing, toggle, stop, origin }
}

/**
 * The `KIT_RUNS` and `TRIED_ROWS` rows that would change nothing now (the engine tries each on a
 * copy, `wouldChange`), asked again a moment after the document, the selection or the time
 * changes, but not during playback.
 */
function useIdleRows(doc: { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> } | null, state: unknown, playing: boolean) {
  const [idle, setIdle] = useState<ReadonlySet<string>>(() => new Set([...Object.keys(KIT_RUNS), ...Object.keys(TRIED_ROWS)]))
  useEffect(() => {
    if (!doc || playing) return undefined
    let current = true
    const timer = window.setTimeout(async () => {
      const tries: Array<[string, string, Record<string, unknown>]> = [...Object.entries(KIT_RUNS).map(([row, [command, params]]) => [row, command, params] as [string, string, Record<string, unknown>]), ...Object.entries(TRIED_ROWS).map(([command, params]) => [command, command, params] as [string, string, Record<string, unknown>])]
      const next = new Set<string>()
      for (const [row, command, params] of tries) if (!(await doc.invoke<boolean>('wouldChange', command, JSON.stringify(params)).catch(() => false))) next.add(row)
      if (current) setIdle((before) => ([...before].sort().join() === [...next].sort().join() ? before : next))
    }, 120)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [doc, state, playing])
  return idle
}

/** How much sound (seconds) preview playback asks the engine for at a time, and how far ahead it keeps. */
const AUDIO_CHUNK = 1
const AUDIO_AHEAD = 2

/**
 * The composition's sound while it plays: the engine mixes it (`mixAudio`, every audible layer
 * with its levels) a second at a time, a little ahead, and the page plays it from the time
 * playback started, against the same clock. Nothing is asked for again once the engine answers
 * that nothing is audible. `scheduled` counts the stretches handed to the speakers (the footage
 * test reads it from the workspace's `data-audio`).
 */
function usePreviewAudio(doc: { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> } | null, playing: boolean, from: number | null, duration: number, scheduled: (count: number) => void) {
  useEffect(() => {
    if (!playing || !doc || from === null || typeof AudioContext === 'undefined') return undefined
    const context = new AudioContext()
    const started = context.currentTime
    let stopped = false
    let mixed = 0
    let count = 0
    const pump = async () => {
      while (!stopped && from + mixed < duration - 1e-6) {
        if (mixed - (context.currentTime - started) > AUDIO_AHEAD) {
          await new Promise((resolve) => window.setTimeout(resolve, 200))
          continue
        }
        const length = Math.min(AUDIO_CHUNK, duration - (from + mixed))
        const samples = await doc.invoke<Float32Array>('mixAudio', from + mixed, length, context.sampleRate).catch(() => null)
        if (stopped || !samples?.length) return
        const frames = Math.floor(samples.length / 2)
        const buffer = context.createBuffer(2, frames, context.sampleRate)
        const left = buffer.getChannelData(0)
        const right = buffer.getChannelData(1)
        for (let index = 0; index < frames; index += 1) {
          left[index] = samples[index * 2]
          right[index] = samples[index * 2 + 1]
        }
        const source = context.createBufferSource()
        source.buffer = buffer
        source.connect(context.destination)
        // A stretch that arrives late starts part of the way in, so the sound keeps to the picture.
        const at = started + mixed
        const late = Math.max(0, context.currentTime - at)
        if (late < length) source.start(Math.max(at, context.currentTime), late)
        mixed += length
        count += 1
        scheduled(count)
      }
    }
    void pump()
    return () => {
      stopped = true
      void context.close()
    }
  }, [doc, playing, from, duration, scheduled])
}

/** One step of View ▸ Zoom In or Zoom Out, as the frame's Cmd+= and Cmd+-. */
const ZOOM_STEP = 1.25

/** How fast Cmd or Ctrl and the wheel zoom the composition: the zoom changes by e^(pixels × this), about 15 % per 200-pixel notch. */
const WHEEL_ZOOM = 0.0007

/**
 * Cmd or Ctrl and the wheel over the composition zoom around the pointer, gently: taken before the
 * canvas's own, faster zoom. Over the transport and the timeline the wheel is theirs.
 */
function useGentleWheelZoom(workspace: RefObject<HTMLDivElement | null>, canvas: RefObject<KitCanvasHandle | null>) {
  useEffect(() => {
    const element = workspace.current
    if (!element) return undefined
    const wheel = (event: WheelEvent) => {
      if (!(event.ctrlKey || event.metaKey) || !(event.target instanceof Element)) return
      if (event.target.closest('.motion-timeline, .motion-transport, .motion-splitter')) return
      const handle = canvas.current
      const view = handle?.view()
      if (!handle || !view) return
      event.preventDefault()
      event.stopPropagation()
      // The canvas's own element: the first one up from the target that is the view's size.
      let box: Element | null = event.target
      while (box && box !== element && Math.abs(box.clientWidth - view.width) > 1) box = box.parentElement
      const rect = (box ?? element).getBoundingClientRect()
      const px = event.clientX - rect.left
      const py = event.clientY - rect.top
      const zoom = view.zoom * Math.exp(-event.deltaY * WHEEL_ZOOM)
      // The document point under the pointer stays under it.
      const dx = view.x + px / view.zoom
      const dy = view.y + py / view.zoom
      handle.zoomTo(zoom)
      const now = handle.view()?.zoom ?? zoom
      handle.panTo(dx - px / now, dy - py / now)
    }
    element.addEventListener('wheel', wheel, { passive: false, capture: true })
    return () => element.removeEventListener('wheel', wheel, { capture: true })
  }, [workspace, canvas])
}

/** The view tools are the editor's own; Selection, the shapes and Text run in the engine (bindings/wasm/src/tools.rs). */
const HOST_TOOLS = { hand: HAND_TOOL, zoom: ZOOM_TOOL }
/** The tools that change the document: a read-only one has only the view tools. */
const EDITING_TOOLS: ReadonlySet<string> = new Set(['selection', 'rectangle', 'ellipse', 'text'])

/** How far (screen pixels) the arrow keys move the selected layers: 1, or 10 with Shift (comp pixels). */
const NUDGE = { plain: 1, shift: 10 }

/** Keys typed here belong to the target: a field, a button or menu row it presses, a dialog. */
function ownsKeys(target: EventTarget | null) {
  if (!(target instanceof Element)) return false
  if (target instanceof HTMLElement && target.isContentEditable) return true
  return target.matches('input, textarea, select, button, [role="menuitem"]') || target.closest('.kui-dialog') != null
}

/**
 * Space held pans the composition (the canvas's own hand); a Space pressed and let go without a
 * drag plays or pauses, as it always did. Watched on the window before anything else takes the key
 * (the tools hold keys back while the engine is busy, as it is during playback).
 */
function useSpaceTap(onTap: () => void) {
  const latest = useRef(onTap)
  latest.current = onTap
  useEffect(() => {
    let held: { dragged: boolean } | null = null
    const keydown = (event: globalThis.KeyboardEvent) => {
      if (event.code !== 'Space' || event.repeat || event.metaKey || event.ctrlKey || event.altKey || ownsKeys(event.target)) return
      held ??= { dragged: false }
    }
    const pointerdown = () => {
      if (held) held.dragged = true
    }
    const keyup = (event: globalThis.KeyboardEvent) => {
      if (event.code !== 'Space' || !held) return
      const { dragged } = held
      held = null
      if (!dragged) latest.current()
    }
    window.addEventListener('keydown', keydown, true)
    window.addEventListener('pointerdown', pointerdown, true)
    window.addEventListener('keyup', keyup, true)
    return () => {
      window.removeEventListener('keydown', keydown, true)
      window.removeEventListener('pointerdown', pointerdown, true)
      window.removeEventListener('keyup', keyup, true)
    }
  }, [])
}

/** The physical key a held-back key came from (on a US layout), for handlers that read `code`. */
const CODES: Record<string, string> = { '=': 'Equal', '+': 'Equal', '-': 'Minus', _: 'Minus', '.': 'Period', ',': 'Comma', '[': 'BracketLeft', ']': 'BracketRight', '/': 'Slash', ';': 'Semicolon', "'": 'Quote', '`': 'Backquote', '\\': 'Backslash' }
function codeOf(key: string) {
  if (/^[a-z]$/i.test(key)) return `Key${key.toUpperCase()}`
  if (/^[0-9]$/.test(key)) return `Digit${key}`
  return CODES[key] ?? ''
}

/** A time as a file name can have it: `0-00-01-12`. */
const timecodeName = (seconds: number, fps: number) => timecode(seconds, fps).replace(/:/g, '-')

/**
 * Commands whose shortcuts the editor runs itself rather than the frame running the engine's: the
 * ones it asks values for in its own dialogs, Pre-compose (which then offers to open the new
 * composition), and the ones given another shortcut (`SHORTCUT_OVERRIDES`).
 */
const OWN_SHORTCUTS = ['comp.new', 'comp.settings', 'layer.newSolid', 'layer.settings', 'layer.rename', 'time.set', 'file.import', 'file.exportMovie', 'keys.interpolation', 'keys.velocity', 'layer.precompose', 'layer.timeStretch', ...Object.keys(SHORTCUT_OVERRIDES)]

export function App({ host, phrase }: { host: HostConnection; phrase: Phrases }) {
  const t = useMemo(() => createKitStrings(CATALOGS, host.init.locale), [host])
  const manifest = useMemo(engineManifest, [])
  const session = useKitSession(host, manifest, describe)
  // Failures read as short sentences; the engine's English goes in the details.
  const report = useMemo(() => motionReport(t, t('commandFailed')), [t])
  const editor = useKitEditorState(session, host, t, 'motion', phrase, undefined, { report })
  const { state } = session
  const toolsRef = useRef<ReturnType<typeof useEngineTools> | null>(null)
  const tools = useEngineTools(session.tool, session.doc != null, session.change, {
    initial: host.readOnly ? 'hand' : 'selection',
    hostTools: HOST_TOOLS,
    hidden: host.readOnly ? EDITING_TOOLS : undefined,
    onDeferredKey: deferredKey,
  })
  toolsRef.current = tools
  /**
   * A key pressed while the engine was answering (during playback it nearly always is), which was
   * not typed text. Space was the window's already (useSpaceTap): given back as a keydown with no
   * release it would leave the canvas panning. A tool's letter picks the tool; any other key goes
   * to the page again, as the editor would have had it.
   */
  function deferredKey(key: string, shift: boolean) {
    if (key === ' ') return
    const current = toolsRef.current
    const tool = current?.toolForKey(key, shift)
    if (tool) {
      current?.select(tool)
      return
    }
    // Given back after the engine's queue has run (from inside it, the key would be held back
    // again at once, for ever).
    window.setTimeout(() => {
      const target = document.activeElement ?? document.body
      target.dispatchEvent(new KeyboardEvent('keydown', { key, code: codeOf(key), shiftKey: shift, bubbles: true, cancelable: true }))
    }, 0)
  }
  // Typing ended (Escape, a click elsewhere): the keys are the editor's again (V, T, Space…),
  // not the hidden field's that took the text.
  useEffect(() => {
    if (tools.wantsText) return
    const focused = document.activeElement
    if (focused instanceof HTMLElement && focused.classList.contains('kui-text-sink')) focused.closest<HTMLElement>('.kui-app')?.focus({ preventScroll: true })
  }, [tools.wantsText])
  const canvas = useRef<KitCanvasHandle>(null)
  const viewMemory = useRef<RasterView | null>(null)
  const [zoom, setZoom] = useState<number | null>(null)
  const [dropping, setDropping] = useState(false)
  /** The composition point under a page point, from the canvas's view; null off the canvas. */
  const compPoint = (clientX: number, clientY: number): [number, number] | undefined => {
    const view = canvas.current?.view()
    const element = workspace.current?.querySelector('[data-testid="kit-canvas-surface"]')
    if (!view || !element) return undefined
    const rect = element.getBoundingClientRect()
    return [Math.round(view.x + (clientX - rect.left) / view.zoom), Math.round(view.y + (clientY - rect.top) / view.zoom)]
  }
  // Scrolling stops a margin past the picture: the view never leaves it for empty canvas.
  const picture = state?.picture
  const panBounds = useMemo(() => picturePanBounds(picture?.width ?? 0, picture?.height ?? 0, zoom), [picture?.width, picture?.height, zoom])
  const workspace = useRef<HTMLDivElement>(null)
  // The view a fit gave (the canvas opens fitted): Fit in Window is hidden while the view is still that.
  const fitting = useRef(true)
  const [fitView, setFitView] = useState<string | null>(null)
  const [viewKey, setViewKey] = useState('')
  const fit = useCallback(() => {
    fitting.current = true
    canvas.current?.fit()
    // A fit that changed nothing reports no view: the next view is not a fit's.
    window.setTimeout(() => (fitting.current = false), 300)
  }, [])
  // A composition opened (new, or switched to) fits the window once its picture has its size.
  const compId = state?.comp?.id
  const fitted = useRef<number | undefined>(undefined)
  useEffect(() => {
    if (compId === undefined || !picture) return
    if (fitted.current === undefined) {
      fitted.current = compId
      return
    }
    if (fitted.current === compId || picture.width !== state?.comp?.width || picture.height !== state?.comp?.height) return
    fitted.current = compId
    requestAnimationFrame(fit)
  }, [compId, picture, state?.comp?.width, state?.comp?.height])
  const views = useMemo(createViewStore, [])
  useGentleWheelZoom(workspace, canvas)
  const main = useRef<HTMLDivElement>(null)
  const [timelineHeight, setTimelineHeight] = useTimelineHeight()
  // Moving through time is not an edit: it goes to the engine without the editor's notices.
  const runQuietly = session.run
  const playback = usePlayback(runQuietly, state?.time ?? 0, state?.comp?.duration ?? 0)
  useSpaceTap(playback.toggle)
  const audioCount = useCallback((count: number) => workspace.current?.setAttribute('data-audio', String(count)), [])
  usePreviewAudio(session.doc, playback.playing, playback.playing ? (playback.origin.current?.time ?? null) : null, state?.comp?.duration ?? 0, audioCount)
  const [exporting, setExporting] = useState(false)
  const project = useProjectPanel()
  const seek = useCallback((seconds: number) => void runQuietly('time.set', { time: Math.max(0, seconds) }).catch(() => undefined), [runQuietly])
  const step = (frames: number) => void runQuietly('time.step', { frames }).catch(() => undefined)
  const { run, query } = editor
  const footage = useFootage({ host, run, t })
  /** Footage, a solid or a composition from the project as a layer at the current time (one undo step). */
  const placeItem = (item: number, position?: [number, number]) => footage.place(item, { time: state?.time ?? 0, ...(position ? { position } : {}) })
  /**
   * The compositions the person went into from a precomp layer, outermost first: the breadcrumb
   * back out. `entering` is the composition a precomp was opened from (by its bar, its menu, or a
   * double-click on the canvas) until the switch arrives; any other switch starts over.
   */
  const [trail, setTrail] = useState<number[]>([])
  const entering = useRef<number | null>(null)
  const shownComp = useRef<number | undefined>(undefined)
  useEffect(() => {
    const now = state?.comp?.id
    const before = shownComp.current
    shownComp.current = now
    if (now === undefined || before === undefined || now === before) return
    const from = entering.current
    entering.current = null
    setTrail((current) => {
      const at = current.indexOf(now)
      if (at >= 0) return current.slice(0, at)
      return from === before ? [...current, before] : []
    })
  }, [state?.comp?.id])
  const openPrecomp = (layer: number) => {
    entering.current = state?.comp?.id ?? null
    void run('layer.openSource', { layer })
  }
  /** Pre-compose, then say where the new composition is and offer to open it. */
  const precompose = async () => {
    const made = parseAnswer<{ comp?: number; layer?: number }>(await run('layer.precompose', {}))
    if (made?.layer == null) return
    const layer = made.layer
    toast.success(t('precomposed'), { action: { label: t('precomposed.open'), onClick: () => openPrecomp(layer) } })
  }
  /** Adds a marker at the current time: on the selected layers, or on the composition with none selected. */
  const addMarker = () => void run(state?.layers.some((layer) => layer.selected) ? 'layer.addMarker' : 'comp.addMarker', {})
  /** The composition's current frame as a PNG beside the project. */
  const saveFrame = async () => {
    const doc = session.doc
    const comp = state?.comp
    if (!doc || !comp) return
    const folder = host.init.document.path.split('/').slice(0, -1).join('/')
    const name = `${comp.name.replace(/[\\/:*?"<>|]+/g, '_').trim() || 'Composition'} ${timecodeName(state.time, comp.fps)}.png`
    try {
      if ((await run('comp.saveFrameAs', { path: name })) === null) return
      const bytes = await doc.invoke<Uint8Array | undefined>('takeFile', name)
      if (!bytes) throw new Error('the engine wrote no picture')
      const written = await host.exportFile([folder, name].filter(Boolean).join('/'), bytes)
      toast.success(t('frameSaved', { name: written.split('/').pop() ?? written }))
    } catch (error) {
      toast.error(t('frameFailed'), { description: error instanceof Error ? error.message : String(error) })
    }
  }
  const [dialog, setDialog] = useState<MotionDialog | null>(null)
  // The graph editor's curves: a property's values over time, sampled by the engine.
  const doc = session.doc
  const sampler = useCallback<CurveSampler>(
    async (layer, path, from, to, count) => (doc ? (JSON.parse(await doc.invoke<string>('curveSamples', layer, path, from, to, count)) as unknown[]) : null),
    [doc],
  )
  const [findEffect, setFindEffect] = useState(false)
  // Every effect the engine has, for the Effect menu; and whether one was applied here (Last Effect).
  const [effects, setEffects] = useState<MotionEffect[]>([])
  const [lastEffect, setLastEffect] = useState(false)
  useEffect(() => {
    if (!session.doc) return
    void query<MotionEffect[]>('effect.list', {}).then((list) => setEffects((list ?? []).map(({ id, name, category, params }) => ({ id, name, category, params }))))
  }, [session.doc, query])
  const applyEffect = (effect: string) => void run('effect.apply', { effect }).then((done) => done !== null && setLastEffect(true))
  /** A name not taken by a project item: `Comp 2`, `Solid 1`. */
  const unusedName = async (base: string) => {
    const summary = await query<{ items?: Array<{ name?: string }> }>('project.summary', {})
    const names = new Set((summary?.items ?? []).map((item) => item.name))
    let n = 1
    while (names.has(`${base} ${n}`)) n += 1
    return `${base} ${n}`
  }
  const openComp = async (mode: 'new' | 'settings') => {
    const comp = state?.comp
    if (mode === 'settings') {
      if (!comp) return
      const info = await query<{ background?: Rgb }>('comp.info', {})
      setDialog({ kind: 'comp', mode, values: { name: comp.name, width: comp.width, height: comp.height, frameRate: comp.fps, duration: comp.duration, background: toHex(info?.background ?? [0, 0, 0]) } })
    } else setDialog({ kind: 'comp', mode, values: { name: await unusedName(t('defaultComp')), width: 1920, height: 1080, frameRate: 30, duration: 10, background: '#000000' } })
  }
  const openSolid = async (mode: 'new' | 'settings') => {
    const comp = { width: state?.comp?.width ?? 1920, height: state?.comp?.height ?? 1080 }
    if (mode === 'new') {
      setDialog({ kind: 'solid', mode, values: { name: await unusedName(t('defaultSolid')), width: comp.width, height: comp.height, color: '#808080', comp } })
      return
    }
    const layer = state?.layers.find((item) => item.selected)
    if (!layer) return
    const solid = await query<{ solid?: number }>('layer.settings', { layer: layer.id })
    const meta = solid?.solid != null ? await query<{ item?: { name?: string; width?: number; height?: number; color?: Rgb } }>('item.metadata', { item: solid.solid }) : null
    const item = meta?.item
    setDialog({ kind: 'solid', mode, layer: layer.id, values: { name: item?.name ?? layer.name, width: item?.width ?? comp.width, height: item?.height ?? comp.height, color: toHex(item?.color ?? [0.5, 0.5, 0.5]), comp } })
  }
  const end = () => seek((state?.comp?.duration ?? 0) - (state?.frame ?? 0))
  /** What `keys.info` answers for the selected keyframes (the fields the dialogs read). */
  type SelectedKeyInfo = { dims?: number; spatial?: string; units?: string; inEase?: Array<{ speed: number; influence: number } | null>; outEase?: Array<{ speed: number; influence: number } | null> }
  /** Keyframe Interpolation, with Spatial when a selected keyframe is on a moving property (Position, Anchor Point). */
  const openInterpolation = async () => {
    const keys = await query<SelectedKeyInfo[]>('keys.info', {})
    if (!Array.isArray(keys) || !keys.length) return
    setDialog({ kind: 'interpolation', spatial: keys.some((key) => typeof key.spatial === 'string') })
  }
  /** Keyframe Velocity, starting from the first selected keyframe's first dimension. */
  const openVelocity = async () => {
    const keys = await query<SelectedKeyInfo[]>('keys.info', {})
    const first = Array.isArray(keys) ? keys[0] : undefined
    if (!first) return
    const side = (ease: SelectedKeyInfo['inEase']) => ease?.find((one) => one != null) ?? { speed: 0, influence: 33.33 }
    const [inside, outside] = [side(first.inEase), side(first.outEase)]
    setDialog({ kind: 'velocity', units: first.units ?? '', values: { inSpeed: inside.speed, inInfluence: inside.influence, outSpeed: outside.speed, outInfluence: outside.influence } })
  }
  /** Time Stretch, from the first selected layer's stretch and length. */
  const openStretch = async () => {
    const selected = state?.layers.filter((layer) => layer.selected) ?? []
    const first = selected[0]
    if (!first || !state?.comp) return
    const row = (await query<{ layers?: Array<{ id: number; stretch?: number }> }>('comp.info', {}))?.layers?.find((layer) => layer.id === first.id)
    setDialog({ kind: 'stretch', layers: selected.map((layer) => layer.id), stretch: row?.stretch ?? 100, span: Math.max(0, first.out - first.in), fps: state.comp.fps })
  }
  /** Commands the editor answers itself (the rest go to the engine as they are). */
  const own: Record<string, () => void> = {
    'playback.toggle': playback.toggle,
    'file.exportMovie': () => state?.comp && setExporting(true),
    'file.import': () => void footage.importPicked(),
    'layer.precompose': () => void precompose(),
    [KIT.saveFrame]: () => void saveFrame(),
    [KIT.addMarker]: addMarker,
    [KIT.projectPanel]: project.toggle,
    'time.start': () => seek(0),
    'time.end': end,
    'time.previousFrame': () => step(-1),
    'time.nextFrame': () => step(1),
    [FIND_EFFECT]: () => setFindEffect(true),
    'view.zoomIn': () => canvas.current?.zoomBy(ZOOM_STEP),
    'view.zoomOut': () => canvas.current?.zoomBy(1 / ZOOM_STEP),
    [KIT.zoomFit]: fit,
    [KIT.zoom100]: () => canvas.current?.zoomTo(1),
    'comp.new': () => void openComp('new'),
    'keys.interpolation': () => void openInterpolation(),
    'keys.velocity': () => void openVelocity(),
    'layer.timeStretch': () => void openStretch(),
    ...Object.fromEntries(INTERPOLATIONS.map((row) => [row.command, () => void run('keys.interpolation', { interpolation: row.interpolation })])),
    'comp.settings': () => void openComp('settings'),
    'layer.newSolid': () => void openSolid('new'),
    'layer.settings': () => void openSolid('settings'),
    'time.set': () => state?.comp && setDialog({ kind: 'time', time: state.time, fps: state.comp.fps, duration: state.comp.duration }),
    'layer.rename': () => {
      const layer = state?.layers.find((item) => item.selected)
      if (layer) setDialog({ kind: 'rename', layer: layer.id, name: layer.name })
    },
  }
  const latestOwn = useRef(own)
  const latestApply = useRef(applyEffect)
  latestApply.current = applyEffect
  latestOwn.current = own
  /** A menu row, a ribbon button, or a shortcut: the editor's own handling first, else the engine. */
  const dispatch = useCallback(
    (command: string) => {
      const handler = latestOwn.current[command]
      if (handler) handler()
      else if (command.startsWith(EFFECT_PREFIX)) latestApply.current(command.slice(EFFECT_PREFIX.length))
      else if (ARRANGE[command]) void run('layer.arrange', { to: ARRANGE[command] })
      else if (KIT_RUNS[command]) void run(KIT_RUNS[command][0], { ...KIT_RUNS[command][1] })
      else void run(command, {})
    },
    [run],
  )
  // Rows with nothing to act on are hidden; the predicate changes only when that does.
  const idle = useIdleRows(session.doc, state, playback.playing)
  const hiding = hiddenRow(state, lastEffect, { fitted: fitView === viewKey, actual: zoom === 1 }, editor.enabled('keys.interpolation'), idle)
  const hidden = useMemo(() => hiding.hidden, [hiding.key]) // eslint-disable-line react-hooks/exhaustive-deps
  const rows = useMemo(() => menuRows(effects), [effects])
  // Only the menus' engine commands (and a few more) run from their shortcuts.
  const allowed = useMemo(() => shortcutCommands(rows), [rows])
  const hiddenCommands = useCallback((command: string) => !allowed.has(command), [allowed])
  const wholeWorkArea = hiding.key.includes('comp.trimToWorkArea')
  // Home first (its own ribbon; a drop-down panel in Menus), then the editor's menus.
  const tabs = useMemo<FrameTab[]>(
    () => [
      { id: 'home', label: t('homeTab'), ribbon: <HomeTab info={state} onCommand={dispatch} wholeWorkArea={wholeWorkArea} lastEffect={lastEffect} /> },
      ...catalogTabs(rows, { phrase, view: VIEW_MENUS, primary: MOTION_PRIMARY, hidden, onCommand: dispatch, onKitCommand: dispatch }).map((tab) =>
        // The Effect tab's ribbon: a drop-down per category rather than a button per effect.
        tab.id === 'Effect' ? { ...tab, ribbon: <EffectRibbon effects={effects} onCommand={dispatch} lastEffect={lastEffect} /> } : tab,
      ),
    ],
    [dispatch, effects, hidden, lastEffect, phrase, rows, t, wholeWorkArea, state],
  )

  // The timeline's own keys (Delete, Cmd+D, …) come first.
  const timelineKeys: TimelineKeys = useRef(null)
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!state) return false
    const command = event.metaKey || event.ctrlKey
    const plain = !command && !event.altKey && !event.shiftKey
    // T picks the Text tool from the composition; in the timeline it shows Opacity, as it did.
    const inTimeline = event.target instanceof Element && event.target.closest('.motion-timeline') != null
    if (plain && event.code === 'KeyT' && !inTimeline && !host.readOnly) return false
    if (timelineKeys.current?.(event)) return true
    // The arrow keys move the selected layers (Shift: ten times as far), one undo step each.
    const arrow = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[event.key]
    if (arrow && pressed(EDITOR_KEYS.nudge, event) && tools.active === 'selection' && !host.readOnly && state.layers.some((layer) => layer.selected)) {
      const by = event.shiftKey ? NUDGE.shift : NUDGE.plain
      void editor.run('layer.nudge', { x: arrow[0] * by, y: arrow[1] * by })
      return true
    }
    // The editor's own commands take their menu shortcuts here (Cmd+Alt+N, Cmd+K, Cmd+Y…), before the engine would run them without values.
    for (const id of OWN_SHORTCUTS) {
      const shortcut = SHORTCUT_OVERRIDES[id] ?? editor.byId.get(id)?.shortcut
      if (shortcut && shortcutMatches(shortcut, event) && (editor.enabled(id) || id === 'comp.new')) {
        if (!host.readOnly || id === 'time.set') dispatch(id)
        return true
      }
    }
    // The Transform, Time and Align rows' own shortcuts ([ and ] move the in and out points, Alt with them trims, Cmd+Alt+F fits).
    for (const row of rows) {
      if (!KIT_RUNS[row.command] || !row.shortcut || !shortcutMatches(row.shortcut, event)) continue
      if (!host.readOnly && !idle.has(row.command)) dispatch(row.command)
      return true
    }
    // * (on the keypad, or Shift+8): a marker at the current time.
    // (EDITOR_KEYS, keys.ts: the help docs' shortcut table is printed from it.)
    if (pressed(EDITOR_KEYS.marker, event) && !host.readOnly) {
      addMarker()
      return true
    }
    // Space plays on its release (useSpaceTap); its press is taken here so nothing else runs.
    if (pressed(EDITOR_KEYS.play, event)) return true
    // View ▸ Zoom In and Zoom Out: . and , (Cmd or Ctrl with = and - zoom too, and Cmd+0 fits).
    else if (pressed(EDITOR_KEYS.zoomIn, event) || pressed(EDITOR_KEYS.zoomOut, event)) canvas.current?.zoomBy(event.key === '.' ? ZOOM_STEP : 1 / ZOOM_STEP)
    else if (pressed(EDITOR_KEYS.nextFrame, event)) step(event.shiftKey ? 10 : 1)
    else if (pressed(EDITOR_KEYS.previousFrame, event)) step(event.shiftKey ? -10 : -1)
    // Layer ▸ Arrange ▸ Bring Layer Forward / Send Layer Backward (with Shift: to the front / back).
    else if (command && !host.readOnly && (event.key === ']' || event.key === '}')) void editor.run('layer.arrange', { to: event.shiftKey ? 'front' : 'forward' })
    else if (command && !host.readOnly && (event.key === '[' || event.key === '{')) void editor.run('layer.arrange', { to: event.shiftKey ? 'back' : 'backward' })
    else if (pressed(EDITOR_KEYS.start, event)) seek(0)
    else if (pressed(EDITOR_KEYS.end, event)) end()
    else return false
    return true
  }

  /**
   * A letter typed into a text layer on the canvas is text, not a shortcut: it stays with the
   * engine's text field (which takes it as input) and never reaches the editor's single-key
   * shortcuts (V, P, Space…). Its default is kept, or the letter would not be typed.
   */
  const keepTypedKeys = (event: KeyboardEvent<HTMLDivElement>) => {
    const sink = event.target instanceof HTMLTextAreaElement && event.target.classList.contains('kui-text-sink')
    if (sink && tools.wantsText && !event.metaKey && !event.ctrlKey && (event.key.length === 1 || event.key === 'Process' || event.nativeEvent.isComposing)) event.stopPropagation()
  }

  return (
    <KitEditorProvider editor={editor} tools={tools}>
      <EffectCatalog.Provider value={effects}>
      <SamplerContext.Provider value={sampler}>
      <EditorFrame
        tabs={tabs}
        defaultTab="home"
        actions={
          <>
            <Button variant="ghost" aria-pressed={project.open} onClick={project.toggle} title={t('project.toggle')} data-testid="motion-project-toggle">
              <PanelLeftIcon size={14} aria-hidden="true" />
              {t('project.title')}
            </Button>
            <Button variant="ghost" disabled={!state?.comp} onClick={() => setExporting(true)} data-testid="motion-render">
              <Download size={14} aria-hidden="true" />
              {t('renderMovie')}
            </Button>
          </>
        }
        sidebar={
          state && project.open ? (
            <ProjectPanel info={state} width={project.width} onWidth={project.setWidth} onImport={() => void footage.importPicked()} onNewComp={() => void openComp('new')} onPlace={(item) => placeItem(item)} />
          ) : null
        }
        canvas={canvas}
        rail={<EngineToolRail />}
        zoom={zoom}
        onKeyDown={keyDown}
        hiddenCommands={hiddenCommands}
        status={
          <span data-testid="motion-status">
            {state?.comp ? t('summary', { name: state.comp.name, width: state.comp.width, height: state.comp.height, fps: Math.round(state.comp.fps * 100) / 100, layers: state.layers.length }) : t('noComp')}
          </span>
        }
        panels={state ? <PropertiesPanel info={state} onAddEffect={() => setFindEffect(true)} typingOnCanvas={tools.active === 'text' || tools.wantsText} /> : null}
      >
        <MotionDialogs dialog={dialog} onClose={() => setDialog(null)} onSeek={seek} />
        {exporting && state ? <ExportDialog info={state} doc={session.doc} onClose={() => setExporting(false)} /> : null}
        {findEffect ? <FindEffectDialog effects={effects} onApply={(effect) => applyEffect(effect.id)} onClose={() => setFindEffect(false)} /> : null}
        <div className="kui-row" ref={workspace} data-testid="motion-workspace" data-picture={picture ? `${picture.width} ${picture.height}` : undefined}>
          <div
            className="motion-main"
            ref={main}
            style={{ '--motion-timeline-height': `${timelineHeight}px` } as CSSProperties}
            onKeyDown={keepTypedKeys}
            data-drop={dropping || undefined}
            onDragOver={(event) => {
              const types = event.dataTransfer.types
              if (host.readOnly || !state?.comp || !(types.includes(ITEM_MIME) || types.includes('Files'))) return
              event.preventDefault()
              event.dataTransfer.dropEffect = 'copy'
              setDropping(true)
            }}
            onDragLeave={(event) => {
              if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) setDropping(false)
            }}
            onDrop={(event) => {
              setDropping(false)
              if (host.readOnly || !state?.comp) return
              const item = Number(event.dataTransfer.getData(ITEM_MIME))
              const files = [...event.dataTransfer.files]
              if (!item && !files.length) return
              event.preventDefault()
              // Dropped on the composition, the layer's centre goes where it was dropped; on the timeline, the middle.
              const onCanvas = !(event.target instanceof Element && event.target.closest('.motion-timeline, .motion-transport, .motion-splitter'))
              const position = onCanvas ? compPoint(event.clientX, event.clientY) : undefined
              if (item) placeItem(item, position)
              else void footage.importDropped(files, { time: state.time, ...(position ? { position } : {}) })
            }}
            onDoubleClickCapture={(event) => {
              // A double-click on the canvas may open a precomp layer (the Selection tool's): the switch it brings is a step in.
              if (event.target instanceof Element && !event.target.closest('.motion-timeline, .motion-transport, .motion-splitter') && tools.active === 'selection') {
                entering.current = state?.comp?.id ?? null
                window.setTimeout(() => {
                  if (shownComp.current === entering.current) entering.current = null
                }, 1500)
              }
            }}
          >
            <KitDocumentCanvas
              canvas={canvas}
              viewMemory={viewMemory}
              panBounds={panBounds}
              animated={tools.wantsText}
              onView={(view) => {
                setZoom(view.zoom)
                const key = `${view.zoom} ${view.x} ${view.y}`
                if (fitting.current) setFitView(key)
                fitting.current = false
                setViewKey(key)
                views.set(view)
                // The view, for the scroll test's checks (zoom and the document point at the top left).
                workspace.current?.setAttribute('data-view', `${view.zoom} ${view.x} ${view.y}`)
              }}
            >
              {/* The scrollbars: there only while the composition and its margin are longer than the view. */}
              <CanvasScrollbars views={views} content={(view) => picturePanBounds(picture?.width ?? 0, picture?.height ?? 0, view.zoom)} canvas={canvas} />
            </KitDocumentCanvas>
            {/* The transport and the timeline are the kit's own bars: Focus shows the composition alone. */}
            {state ? (
              <FrameChrome>
                <Splitter height={timelineHeight} onHeight={setTimelineHeight} main={main} />
                <Transport info={state} playing={playback.playing} onPlay={playback.toggle} onSeek={seek} onStep={step} onCommand={dispatch} trail={trail} onBack={(comp) => void run('comp.open', { comp })} />
                <Timeline
                  info={state}
                  onSeek={(seconds) => {
                    playback.stop()
                    seek(seconds)
                  }}
                  keys={timelineKeys}
                  onOpenPrecomp={openPrecomp}
                  onPrecompose={() => void precompose()}
                  onCommand={dispatch}
                />
              </FrameChrome>
            ) : null}
          </div>
        </div>
      </EditorFrame>
      </SamplerContext.Provider>
      </EffectCatalog.Provider>
    </KitEditorProvider>
  )
}
