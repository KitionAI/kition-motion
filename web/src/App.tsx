import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type DragEvent, type KeyboardEvent, type RefObject } from 'react'
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
  Select,
} from '@kition/sdk/editor'
import { engineManifest } from './engine'
import { CATALOGS, PHRASES, counted } from './strings'
import { learnEffectNames } from './effectNames'
import { describe, picturePanBounds, timecode } from './motion'
import { ARRANGE, ASSISTANT_NEEDS, type Convertible, KIT, KIT_RUNS, type SelectedKeys, MOTION_PRIMARY, OVERLAY_ROWS, RESOLUTIONS, SHORTCUT_OVERRIDES, THREE_D_VIEWS, type AlignTo, type ThreeDView, TRIED_ROWS, VIEW_MENUS, hiddenRow, menuRows, shortcutCommands, waitingRowStyles, waitingRows } from './menus'
import { EFFECT_PREFIX, EffectRibbon, FIND_EFFECT, FindEffectDialog, rememberEffect, type MotionEffect } from './effects'
import { CanvasScrollbars, createViewStore } from './CanvasScrollbars'
import { HomeTab } from './HomeTab'
import { Said, failed, motionReport, toolStatus } from './errors'
import { MotionDialogs, toHex, type MotionDialog, type Rgb } from './dialogs'
import { Timeline, type TimelineKeys } from './timeline/Timeline'
import { Transport } from './Transport'
import { LABELS, LABEL_ROW_STYLES, NEW_LAYER_LABELS, currentLabel, labelRow } from './labels'
import { Splitter, useTimelineHeight } from './Splitter'
import { PropertiesPanel } from './properties/PropertiesPanel'
import { DROP_SHADOW } from './properties/LayerStylesSection'
import { EffectCatalog } from './properties/catalog'
import { ProjectPanel, useProjectPanel } from './ProjectPanel'
import { ExportDialog } from './ExportDialog'
import { CanvasGuides, OVERLAY_KEYS, useOverlays } from './CanvasGuides'
import { RegionOfInterest, type Region } from './RegionOfInterest'
import { SecondView } from './SecondView'
import { installPointerdownFallback } from './pointerFallback'
import { installFocusBack } from './focusBack'
import { ITEM_MIME, parseAnswer, relativePath, useFootage, type ProjectItem, type ProjectSummary } from './footage'
import { RenderQueuePanel, compsOf, useRenderQueue } from './RenderQueue'
import { CacheFullMenu } from './CacheFullMenu'
import { EDITOR_KEYS, pressed } from './keys'
import { AudioMeters, createMeterFeed, usePreviewAudio } from './audio'
import { INTERPOLATIONS } from './timeline/interp'
import { distinctKeys, type SelectedKey } from './timeline/keys'
import { SamplerContext, WaveformContext, type CurveSampler, type Waveform, type WaveformReader } from './timeline/samples'
import { isThreeD } from './timeline/tree'

/**
 * Plays the composition from the current time. With `skip` (Skip Frames to Keep Real Time, the
 * default) it keeps to the wall clock: frames the engine cannot render in time are skipped. Without
 * it every frame is shown, one after another, as fast as the engine draws them (slower than real
 * time when they take long). While it plays, the time goes to the engine with a repaint only (no
 * full refresh of the editor's state each frame); `at` is the time shown, for the playhead and the
 * timecode, and the editor's state catches up when playback stops.
 */
function usePlayback(run: (command: string, params?: Record<string, unknown>, damage?: null, options?: { refresh?: 'all' | 'repaint' | 'none' }) => Promise<unknown>, time: number, duration: number, frame: number, skip: boolean, paint: () => Promise<unknown>, loop: boolean, workArea: readonly [number, number]) {
  const [playing, setPlaying] = useState(false)
  const [at, setAt] = useState<number | null>(null)
  // Counts the times playback went round the loop: the sound starts again with each.
  const [rounds, setRounds] = useState(0)
  const latest = useRef({ time, duration, frame, skip, paint, loop, workArea })
  latest.current = { time, duration, frame, skip, paint, loop, workArea }
  const origin = useRef<{ wall: number; time: number } | null>(null)
  const last = useRef<number | null>(null)
  const stop = useCallback(() => {
    origin.current = null
    setPlaying(false)
  }, [])
  /** Where looping playback goes round: the work area (the whole composition when it has none). */
  const span = () => {
    const { duration, workArea } = latest.current
    const from = Math.max(0, Math.min(workArea[0], duration))
    const to = Math.min(duration, workArea[1])
    return to - from > 1e-3 ? [from, to] : [0, duration]
  }
  const toggle = useCallback(() => {
    if (playing) {
      stop()
      return
    }
    const now = latest.current.time
    const [from, to] = span()
    // Looping, playback starts at the work area's start when the time is outside it; otherwise at
    // the start once the end is reached.
    const start = latest.current.loop ? (now < from - 1e-3 || now >= to - 1e-3 ? from : now) : now >= latest.current.duration - 1e-3 ? 0 : now
    origin.current = { wall: performance.now(), time: start }
    last.current = null
    setPlaying(true)
  }, [playing, stop]) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!playing) {
      // The editor's state catches up with the last frame shown.
      if (last.current !== null) {
        const time = last.current
        last.current = null
        void run('time.set', { time })
          .catch(() => undefined)
          .finally(() => setAt(null))
      }
      return undefined
    }
    let stopped = false
    let busy = false
    // Without skipping: the frame shown last, the next one follows it.
    let shown = origin.current?.time ?? 0
    let first = true
    const step = async () => {
      if (stopped || !origin.current) return
      const { duration, frame, skip, loop } = latest.current
      const [from, to] = loop ? span() : [0, duration]
      let next = skip ? origin.current.time + (performance.now() - origin.current.wall) / 1000 : first ? shown : shown + frame
      // Past the last frame inside: a time within half a frame of the end shows the end's frame
      // (the time snaps to the nearest), which is outside the loop.
      if (next >= to - frame / 2) {
        if (!loop) {
          stop()
          return
        }
        // Round again from the work area's start.
        next = from
        origin.current = { wall: performance.now(), time: from }
        shown = from
        first = true
        setRounds((count) => count + 1)
      }
      if (!busy) {
        busy = true
        first = false
        try {
          await run('time.set', { time: next }, null, { refresh: 'repaint' })
          last.current = next
          shown = next
          if (!stopped) setAt(next)
          // Every frame shown: the next waits until this one is painted.
          if (!skip) await latest.current.paint()
        } catch {
          stop()
        }
        busy = false
      }
      requestAnimationFrame(() => void step())
    }
    void step()
    return () => {
      stopped = true
    }
  }, [playing, run, stop]) // eslint-disable-line react-hooks/exhaustive-deps
  return { playing, toggle, stop, origin, at, rounds }
}

/** What the engine's frame cache reports (`previewCache`). */
type PreviewCache = { cached: number[]; bytes: number; shown: number; rendered: number; hits: number; renderMs: number }

/**
 * The frames the preview keeps (drawn as a bar over the ruler), asked again a moment after each
 * change and twice a second while playing; and, while playing, the frames per second the viewer
 * really shows (frames drawn, from the cache or rendered, over the wall clock).
 */
function usePreviewCache(doc: { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> } | null, state: unknown, playing: boolean) {
  const [cache, setCache] = useState<PreviewCache | null>(null)
  const [fps, setFps] = useState<number | null>(null)
  const ask = useCallback(async () => {
    if (!doc) return null
    const answer = await doc.invoke<string>('previewCache').catch(() => null)
    if (!answer) return null
    const next = JSON.parse(answer) as PreviewCache
    setCache((before) => (before && before.cached.join() === next.cached.join() && before.bytes === next.bytes && before.shown === next.shown ? before : next))
    return next
  }, [doc])
  useEffect(() => {
    const timer = window.setTimeout(() => void ask(), 250)
    return () => window.clearTimeout(timer)
  }, [ask, state])
  useEffect(() => {
    if (!playing) return undefined
    let first: { shown: number; at: number } | null = null
    const timer = window.setInterval(async () => {
      const now = await ask()
      if (!now) return
      const at = performance.now()
      if (first && at - first.at > 250) setFps(((now.shown - first.shown) * 1000) / (at - first.at))
      if (!first) first = { shown: now.shown, at }
    }, 500)
    return () => {
      window.clearInterval(timer)
    }
  }, [playing, ask])
  // For the browser tests and the measurements: frames kept, shown, rendered, found in the cache, render milliseconds.
  const stats = cache ? `${cache.cached.length} ${cache.shown} ${cache.rendered} ${cache.hits} ${Math.round(cache.renderMs)}` : undefined
  return { cached: cache?.cached ?? [], fps, refresh: ask, stats }
}

/**
 * What the engine's `cacheAhead` answers: frames of the work area held, of how many, whether memory
 * ran out, the bytes the cache holds (its frames are kept at the preview resolution and packed
 * without loss), its budget in bytes and the preview resolution's divisor.
 */
type CacheAhead = { done: number; total: number; full: boolean; bytes?: number; budget?: number; preview?: number }

/**
 * The most memory the preview cache keeps, in megabytes: 192 MB per gigabyte the browser reports
 * (`navigator.deviceMemory`, which it rounds and caps at 8), at least 400 MB and at most 1536 MB.
 * A browser that does not report it gets 400 MB.
 */
/** What 使用更多内存 raises the budget to for the session, where the browser does not report its memory. */
export const MORE_MEMORY_MB = 1024
/** The most the preview cache ever takes. */
export const MOST_MEMORY_MB = 1536

/**
 * The runs of consecutive frames the cache holds inside the work area (`cached` are comp frame
 * numbers, `fps` the comp's rate): each as its first frame and its frame count, in time order.
 */
export function cachedRuns(cached: readonly number[], workArea: readonly [number, number], fps: number): Array<{ first: number; count: number }> {
  const low = Math.round(workArea[0] * fps)
  const high = Number.isFinite(workArea[1]) ? Math.round(workArea[1] * fps) : Number.POSITIVE_INFINITY
  const frames = [...new Set(cached)].filter((frame) => frame >= low && frame < high).sort((a, b) => a - b)
  const runs: Array<{ first: number; count: number }> = []
  for (const frame of frames) {
    const last = runs[runs.length - 1]
    if (last && frame === last.first + last.count) last.count += 1
    else runs.push({ first: frame, count: 1 })
  }
  return runs
}

/** Seconds with one decimal, as the cache's messages show them. */
const tenths = (seconds: number) => (Math.round(seconds * 10) / 10).toFixed(1)

export function cacheBudgetMb(deviceMemory: number | undefined): number {
  if (!(typeof deviceMemory === 'number' && Number.isFinite(deviceMemory) && deviceMemory > 0)) return 400
  return Math.min(1536, Math.max(400, Math.round(deviceMemory * 192)))
}

/**
 * View ▸ Cache Work Area in Background: while it is on and nothing plays, the engine renders the
 * work area's frames the cache lacks a little at a time (`cacheAhead`, about 60 ms a step with a
 * pause between, so edits and keys keep their turn), from the current time on, until all are held
 * or the cache's memory budget is reached; the cache bar fills as it goes. An edit empties the
 * cache (the engine cannot tell which frames it changed): caching starts again a moment later.
 */
function useCacheAhead(doc: { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> } | null, on: boolean, playing: boolean, state: unknown, preview: number, refresh: () => Promise<unknown>, budget: number) {
  const [progress, setProgress] = useState<CacheAhead | null>(null)
  const latestRefresh = useRef(refresh)
  latestRefresh.current = refresh
  // Turned off, or another preview resolution: what it reached is forgotten (it shows its new progress).
  useEffect(() => {
    setProgress(null)
  }, [on, preview])
  // It starts again after an edit, at another resolution, and with a larger budget (使用更多内存: a full cache goes on).
  useEffect(() => {
    if (!doc || !on || playing) return undefined
    let stopped = false
    const pause = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms))
    void (async () => {
      // A moment after the last change, so a drag or typing is not held up.
      await pause(300)
      while (!stopped) {
        const answer = await doc.invoke<string>('cacheAhead', 60).catch(() => null)
        if (stopped || !answer) break
        const next = JSON.parse(answer) as CacheAhead
        setProgress((before) => (before && before.done === next.done && before.total === next.total && before.full === next.full ? before : next))
        void latestRefresh.current()
        if (next.done >= next.total || next.full) break
        await pause(30)
      }
    })()
    return () => {
      stopped = true
    }
  }, [doc, on, playing, state, preview, budget])
  return on ? progress : null
}

/**
 * The `KIT_RUNS` and `TRIED_ROWS` rows that would change nothing now (the engine tries each on a
 * copy, `wouldChange`), asked again a moment after the document, the selection or the time
 * changes, but not during playback.
 */
function useIdleRows(doc: { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> } | null, state: unknown, playing: boolean, alignTo: string) {
  const [idle, setIdle] = useState<ReadonlySet<string>>(() => new Set([...Object.keys(KIT_RUNS), ...Object.keys(TRIED_ROWS)]))
  useEffect(() => {
    if (!doc || playing) return undefined
    let current = true
    const timer = window.setTimeout(async () => {
      const tries: Array<[string, string, Record<string, unknown>]> = [...Object.entries(KIT_RUNS).map(([row, [command, params]]) => [row, command, command === 'layer.align' ? { ...params, to: alignTo } : params] as [string, string, Record<string, unknown>]), ...Object.entries(TRIED_ROWS).map(([command, params]) => [command, command, params] as [string, string, Record<string, unknown>])]
      // Asked all at once: one at a time, a slow engine's answers took longer than the state
      // stays the same, so the rows never came back.
      const answers = await Promise.all(tries.map(([, command, params]) => doc.invoke<boolean>('wouldChange', command, JSON.stringify(params)).catch(() => false)))
      const next = new Set<string>(tries.filter((_, at) => !answers[at]).map(([row]) => row))
      if (current) setIdle((before) => ([...before].sort().join() === [...next].sort().join() ? before : next))
    }, 120)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [doc, state, playing, alignTo])
  return idle
}

/** A composition without a work area plays all of it. */
const NO_WORK_AREA: readonly [number, number] = [0, Number.POSITIVE_INFINITY]

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

/** What the engine's Selection tool says while paths are edited (bindings/wasm/src/tools.rs). */
const PATH_EDIT_HINT = 'Editing the path: drag its vertices or edges; Esc or a click on empty space ends it'

/** The camera tools, left off the rail while the composition has nothing 3D. */
const CAMERA_TOOLS: ReadonlySet<string> = new Set(['orbit', 'track', 'dolly'])

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

/** Where View ▸ Grid Settings… keeps the grid's spacing, and Save Animation Preset… the folder last used. */
const GRID_KEY = 'kition-motion.grid-spacing'
/** Where Skip Frames to Keep Real Time is kept. */
const SKIP_KEY = 'kition-motion.skip-frames'
/** Where View ▸ Two Views is kept. */
const TWO_VIEWS_KEY = 'kition-motion.two-views'
/** Where Cache Work Area in Background is kept. */
const CACHE_AHEAD_KEY = 'kition-motion.cache-ahead'
/** Where Loop (playback goes round the work area, on by default) is kept. */
const LOOP_KEY = 'kition-motion.loop'
const PRESET_FOLDER_KEY = 'kition-motion.preset-folder'
/** Where Layer ▸ Align's target (the composition or the selection) is kept. */
const ALIGN_KEY = 'kition-motion.align-to'
/** Where the status bar's colour readout keeps its units (0-255 or percent). */
const INFO_KEY = 'kition-motion.info-units'
function storedPresetFolder() {
  try {
    return globalThis.localStorage?.getItem(PRESET_FOLDER_KEY) ?? ''
  } catch {
    return ''
  }
}

/** A time as a file name can have it: `0-00-01-12`. */
const timecodeName = (seconds: number, fps: number) => timecode(seconds, fps).replace(/:/g, '-')

/**
 * Commands whose shortcuts the editor runs itself rather than the frame running the engine's: the
 * ones it asks values for in its own dialogs, Pre-compose (which then offers to open the new
 * composition), and the ones given another shortcut (`SHORTCUT_OVERRIDES`).
 */
const OWN_SHORTCUTS = ['project.newFolder', 'comp.new', 'comp.settings', 'layer.newText', 'layer.newSolid', 'layer.newCamera', 'layer.newLight', 'layer.settings', 'layer.rename', 'time.set', 'file.import', 'file.exportMovie', 'keyframe.interpolation', 'keyframe.velocity', 'layer.precompose', 'layer.timeStretch', 'file.exportLottie', 'animation.savePreset', 'animation.applyPreset', ...Object.keys(OVERLAY_ROWS), ...RESOLUTIONS.map(([command]) => command), ...Object.keys(SHORTCUT_OVERRIDES)]

export function App({ host, phrase }: { host: HostConnection; phrase: Phrases }) {
  const t = useMemo(() => createKitStrings(CATALOGS, host.init.locale), [host])
  const manifest = useMemo(engineManifest, [])
  const session = useKitSession(host, manifest, describe)
  // Failures read as short sentences; the engine's English goes in the details.
  const report = useMemo(() => motionReport(t, t('commandFailed')), [t])
  const editor = useKitEditorState(session, host, t, 'motion', phrase, undefined, { report })
  const { state } = session
  const toolsRef = useRef<ReturnType<typeof useEngineTools> | null>(null)
  // Whether a button is held down (a gesture may be under way), for Escape.
  const pointerHeld = useRef(false)
  useEffect(() => {
    const down = () => (pointerHeld.current = true)
    const up = () => (pointerHeld.current = false)
    window.addEventListener('pointerdown', down, true)
    window.addEventListener('pointerup', up, true)
    window.addEventListener('pointercancel', up, true)
    return () => {
      window.removeEventListener('pointerdown', down, true)
      window.removeEventListener('pointerup', up, true)
      window.removeEventListener('pointercancel', up, true)
    }
  }, [])
  // The camera tools are on the rail only while the composition has a 3D layer, a camera or a light.
  const anyThreeD = state?.layers.some(isThreeD) ?? false
  const hiddenTools = useMemo(() => (host.readOnly ? EDITING_TOOLS : anyThreeD ? undefined : CAMERA_TOOLS), [host.readOnly, anyThreeD])
  const hiddenRail = useMemo(() => [...(hiddenTools ?? [])], [hiddenTools])
  const tools = useEngineTools(session.tool, session.doc != null, session.change, {
    initial: host.readOnly ? 'hand' : 'selection',
    hostTools: HOST_TOOLS,
    hidden: hiddenTools,
    onDeferredKey: deferredKey,
    // While typing, during a drag (Escape undoes the gesture), and while paths are edited (Escape ends that).
    escapable: () => {
      const current = toolsRef.current
      return current != null && (current.wantsText || pointerHeld.current || current.status === PATH_EDIT_HINT)
    },
  })
  toolsRef.current = tools
  // The last 3D layer gone (or a composition without any opened) while a camera tool was out: back to Selection.
  useEffect(() => {
    if (!anyThreeD && CAMERA_TOOLS.has(tools.active) && !host.readOnly) tools.select('selection')
  }, [anyThreeD, tools, host.readOnly])
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
  // Text an input method is composing while the person types on the canvas (the engine gets it
  // when it is committed): shown where they last pressed, so they see what they type.
  const [composing, setComposing] = useState<string | null>(null)
  const lastPress = useRef<{ x: number; y: number }>({ x: 24, y: 24 })
  // Where the person pressed last: on the timeline (and the transport), or on the composition.
  const lastArea = useRef<'timeline' | 'canvas'>('canvas')
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
  // Composition ▸ Preview ▸ Skip Frames to Keep Real Time (on by default), kept in this browser.
  const [skipFrames, setSkipFrames] = useState(() => {
    try {
      return globalThis.localStorage?.getItem(SKIP_KEY) !== 'false'
    } catch {
      return true
    }
  })
  const toggleSkipFrames = useCallback(() => {
    setSkipFrames((current) => {
      try {
        localStorage.setItem(SKIP_KEY, String(!current))
      } catch {
        // Kept for this page only.
      }
      return !current
    })
  }, [])
  // The transport's Loop (on by default): playback goes round the work area, kept in this browser.
  const [loop, setLoop] = useState(() => {
    try {
      return globalThis.localStorage?.getItem(LOOP_KEY) !== 'false'
    } catch {
      return true
    }
  })
  const toggleLoop = useCallback(() => {
    setLoop((current) => {
      try {
        localStorage.setItem(LOOP_KEY, String(!current))
      } catch {
        // Kept for this page only.
      }
      return !current
    })
  }, [])
  const paintNow = useCallback(async () => canvas.current?.retryRender(), [])
  const workArea = state?.comp?.workArea ?? NO_WORK_AREA
  // 只循环已缓存的部分: looping playback goes round the cached frames only (set below, once the cache is known).
  const [cachedArea, setCachedArea] = useState<readonly [number, number] | null>(null)
  const playback = usePlayback(runQuietly, state?.time ?? 0, state?.comp?.duration ?? 0, state?.frame ?? 1 / 30, skipFrames, paintNow, loop, cachedArea ?? workArea)
  const frameCache = usePreviewCache(session.doc, state, playback.playing)
  // View ▸ Cache Work Area in Background (off by default), kept in this browser.
  const [cacheAhead, setCacheAhead] = useState(() => {
    try {
      return globalThis.localStorage?.getItem(CACHE_AHEAD_KEY) === 'true'
    } catch {
      return false
    }
  })
  const toggleCacheAhead = useCallback(() => {
    setCacheAhead((current) => {
      try {
        localStorage.setItem(CACHE_AHEAD_KEY, String(!current))
      } catch {
        // Kept for this page only.
      }
      return !current
    })
  }, [])
  // While playing, the transport and the timeline show the time playback has reached.
  const shownState = useMemo(() => (state && playback.at !== null ? { ...state, time: playback.at } : state), [state, playback.at])
  useSpaceTap(playback.toggle)
  const audioCount = useCallback((count: number) => workspace.current?.setAttribute('data-audio', String(count)), [])
  // The meters read the mixed sound's peaks at the time playback shows.
  const meterFeed = useMemo(createMeterFeed, [])
  const shownTime = useRef<number | null>(null)
  shownTime.current = playback.playing ? playback.at : null
  meterFeed.at = () => shownTime.current
  usePreviewAudio(session.doc, playback.playing, playback.playing ? (playback.origin.current?.time ?? null) : null, loop ? Math.min(state?.comp?.duration ?? 0, workArea[1]) : (state?.comp?.duration ?? 0), audioCount, playback.rounds, state, meterFeed)
  // The Export dialog, open on a movie format or on Lottie (File ▸ Export ▸ Lottie JSON…).
  const [exporting, setExporting] = useState<false | 'movie' | 'lottie'>(false)
  const project = useProjectPanel()
  const seek = useCallback((seconds: number) => void runQuietly('time.set', { time: Math.max(0, seconds) }).catch(() => undefined), [runQuietly])
  const step = (frames: number) => void runQuietly('time.step', { frames }).catch(() => undefined)
  const { run, query } = editor
  const footage = useFootage({ host, run, t })
  // Reveal Layer Source in Project: the item the Project panel shows and selects.
  const [reveal, setReveal] = useState<{ item: number; rename?: boolean } | null>(null)
  const revealSource = async (layer?: number) => {
    const answer = parseAnswer<{ item?: number }>(await run('layer.revealSource', layer != null ? { layer } : {}))
    if (answer?.item == null) return
    project.show()
    setReveal({ item: answer.item })
  }
  // The Render Queue: its panel under the Project panel, and its runs (which go on with the panel closed).
  const renderQueue = useRenderQueue({ doc: session.doc, host, run, query, t })
  const [queueOpen, setQueueOpen] = useState(false)
  /** Add to Render Queue: jobs for `comps` (ids), with the settings used last; the panel opens. */
  const addToQueue = async (comps: number[]) => {
    const summary = await query<ProjectSummary>('project.summary', {})
    const found = compsOf(summary).filter((item) => comps.includes(item.id))
    if (!found.length) return
    setQueueOpen(true)
    await renderQueue.add(found.map((item) => ({ id: item.id, name: item.name, width: item.size?.[0] ?? 1920, height: item.size?.[1] ?? 1080 })))
  }
  /** A new folder from File ▸ New ▸ Folder, at the top of the project (the panel shows it). */
  const newFolder = async () => {
    project.show()
    const made = parseAnswer<{ item?: number }>(await run('project.newFolder', { name: await unusedName(t('project.folderName')) }))
    // Its name is ready to type over, as after the panel's own New Folder button.
    if (made?.item != null) setReveal({ item: made.item, rename: true })
  }
  /** Footage, a solid or a composition from the project as a layer at the current time (one undo step). */
  const placeItem = (item: number, position?: [number, number]) => footage.place(item, { time: state?.time ?? 0, ...(position ? { position } : {}) })
  /** The composition time under a page x on the timeline (its ruler's stretch of time), on a frame; null off it. */
  const timeUnder = (clientX: number) => {
    const ruler = workspace.current?.querySelector<HTMLElement>('[data-testid="motion-ruler"]')
    const comp = state?.comp
    if (!ruler || !comp) return null
    const box = ruler.getBoundingClientRect()
    const start = Number(ruler.dataset.viewStart)
    const span = Number(ruler.dataset.viewSpan)
    if (!(box.width > 0) || !Number.isFinite(start) || !(span > 0)) return null
    const seconds = start + ((clientX - box.left) / box.width) * span
    const frames = Math.round(seconds * comp.fps)
    return Math.min(Math.max(0, comp.duration - 1 / comp.fps), Math.max(0, frames / comp.fps))
  }
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
    const made = parseAnswer<{ comp?: number; layer?: number }>(await run('layer.precompose', { name: await unusedName(t('defaultPrecomp')) }))
    if (made?.layer == null) return
    const layer = made.layer
    toast.success(t('precomposed'), { action: { label: t('precomposed.open'), onClick: () => openPrecomp(layer) } })
  }
  /** Adds a marker at the current time: on the selected layers, or on the composition with none selected. */
  const addMarker = () => void run(state?.layers.some((layer) => layer.selected) ? 'layer.addMarker' : 'comp.addMarker', {})
  /**
   * File ▸ Dependencies ▸ Collect Files: a copy of the project and every footage file it uses, in
   * a folder beside the project (the engine's `collectFiles` makes them in memory, the host writes
   * each one); the copy's footage points at its `(Footage)` folder.
   */
  const collectFiles = async (inside: string) => {
    const doc = session.doc
    if (!doc) return
    const folder = host.init.document.path.split('/').slice(0, -1).join('/')
    try {
      const answer = JSON.parse(await doc.invoke<string>('collectFiles', inside)) as { files: string[]; missing: string[] }
      for (const name of answer.files) {
        const bytes = await doc.invoke<Uint8Array | undefined>('takeFile', name)
        if (!bytes) throw new Error(name)
        await host.exportFile([folder, name].filter(Boolean).join('/'), bytes)
      }
      toast.success(counted(t, 'collect.done', answer.files.length, { folder: inside }), answer.missing.length ? { description: t('collect.missing', { names: answer.missing.join(', ') }) } : undefined)
    } catch (error) {
      failed(t, t('collect.failed'), error)
    }
  }
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
      failed(t, t('frameFailed'), error)
    }
  }
  /** A Project item or files dragged over the composition or the timeline: taken as a copy. */
  const acceptDrag = (event: DragEvent<HTMLDivElement>) => {
    const types = event.dataTransfer.types
    if (host.readOnly || !state?.comp || !(types.includes(ITEM_MIME) || types.includes('Files'))) return
    event.preventDefault()
    event.dataTransfer.dropEffect = 'copy'
    setDropping(true)
  }
  const [dialog, setDialog] = useState<MotionDialog | null>(null)
  // The first press after a drag-and-drop gets its pointerdown in WebKit too (pointerFallback.ts).
  useEffect(() => installPointerdownFallback(document), [])
  // A focused field taken out with its layer (the editor's Undo from that field): the keys are the editor's again.
  useEffect(() => installFocusBack(document), [])
  // View's rulers, guides, grid and safe areas, and what a moved layer snaps to (the engine's Selection tool asks).
  const { overlays, toggle: toggleOverlay } = useOverlays()
  // View ▸ Grid Settings…: the grid's spacing (composition pixels), kept in this browser.
  const [gridSpacing, setGridSpacing] = useState(() => {
    const stored = Number(globalThis.localStorage?.getItem(GRID_KEY))
    return Number.isFinite(stored) && stored >= 2 && stored <= 10000 ? stored : 100
  })
  // Layer ▸ Align: to the composition, or to the selected layers' bounds.
  const [alignTo, setAlignToState] = useState<AlignTo>(() => {
    try {
      return globalThis.localStorage?.getItem(ALIGN_KEY) === 'selection' ? 'selection' : 'composition'
    } catch {
      return 'composition'
    }
  })
  const setAlignTo = useCallback((next: AlignTo | ((current: AlignTo) => AlignTo)) => {
    setAlignToState((current) => {
      const value = typeof next === 'function' ? next(current) : next
      try {
        localStorage.setItem(ALIGN_KEY, value)
      } catch {
        // Kept for this page only.
      }
      return value
    })
  }, [])
  // View ▸ Resolution: the engine renders every n-th pixel; the canvas paints again.
  const [preview, setPreview] = useState(() => {
    const stored = Number(globalThis.sessionStorage?.getItem('motion-preview-resolution'))
    return [1, 2, 3, 4].includes(stored) ? stored : 1
  })
  useEffect(() => {
    if (!session.doc) return
    void session.doc
      .invoke('setPreviewResolution', preview)
      .then(() => canvas.current?.retryRender())
      .catch(() => undefined)
    try {
      sessionStorage.setItem('motion-preview-resolution', String(preview))
    } catch {
      // Kept for this page only.
    }
  }, [session.doc, preview])
  // The preview cache's memory budget, from the device's memory; a browser that does not say how
  // much it has gets the safe 400 MB, raised for the session by 使用更多内存 when the cache fills.
  const deviceMemory = (navigator as Navigator & { deviceMemory?: number }).deviceMemory
  const memoryReported = typeof deviceMemory === 'number' && Number.isFinite(deviceMemory) && deviceMemory > 0
  const [moreMemory, setMoreMemory] = useState(false)
  const budgetMb = Math.min(MOST_MEMORY_MB, moreMemory && !memoryReported ? MORE_MEMORY_MB : cacheBudgetMb(deviceMemory))
  useEffect(() => {
    if (!session.doc) return
    void session.doc.invoke<number>('setPreviewCacheBudget', budgetMb).catch(() => undefined)
  }, [session.doc, budgetMb])
  const ahead = useCacheAhead(session.doc, cacheAhead, playback.playing, state, preview, frameCache.refresh, budgetMb)
  // What a full cache holds: the runs of cached frames in the work area, and what the message says of them.
  const compFps = state?.comp?.fps || 30
  const runs = useMemo(() => (ahead?.full ? cachedRuns(frameCache.cached, workArea, compFps) : []), [ahead?.full, frameCache.cached, workArea, compFps])
  const heldFrames = runs.reduce((sum, run) => sum + run.count, 0)
  const startFrame = Math.round(workArea[0] * compFps)
  const held =
    runs.length === 1 && runs[0].first === startFrame
      ? t('cacheAhead.heldFirst', { done: heldFrames, total: ahead?.total ?? 0, seconds: tenths(heldFrames / compFps) })
      : runs.length === 1
        ? t('cacheAhead.heldFrom', { done: heldFrames, total: ahead?.total ?? 0, seconds: tenths(heldFrames / compFps), from: tenths(runs[0].first / compFps), to: tenths((runs[0].first + runs[0].count) / compFps) })
        : t('cacheAhead.heldSome', { done: heldFrames, total: ahead?.total ?? 0, seconds: tenths(heldFrames / compFps) })
  // The cached run to loop: the longest (the earliest of equal ones); frames drawn while playing
  // past it do not move the loop.
  const [loopCached, setLoopCached] = useState(false)
  const loopRun = runs.reduce<{ first: number; count: number } | undefined>((best, run) => (!best || run.count > best.count ? run : best), undefined)
  const loopFrom = loopRun && loopRun.count > 1 ? loopRun.first / compFps : null
  const loopTo = loopRun && loopRun.count > 1 ? (loopRun.first + loopRun.count) / compFps : null
  useEffect(() => {
    setCachedArea((before) => {
      const next: readonly [number, number] | null = loopCached && loop && loopFrom !== null && loopTo !== null ? [loopFrom, loopTo] : null
      return before?.[0] === next?.[0] && before?.[1] === next?.[1] ? before : next
    })
  }, [loopCached, loop, loopFrom, loopTo])
  // The cache is full: the next lower preview resolution holds about that many times more frames.
  const lowerResolution = ahead?.full ? RESOLUTIONS.find(([, , divisor]) => divisor === preview + 1) : undefined
  useEffect(() => {
    if (!session.doc) return
    void session.doc.invoke<number>('setSnapTargets', overlays.snapGuides, overlays.snapGrid).catch(() => undefined)
  }, [session.doc, overlays.snapGuides, overlays.snapGrid])
  useEffect(() => {
    if (!session.doc) return
    void session.doc.invoke<number>('setGridSpacing', gridSpacing).catch(() => undefined)
    try {
      localStorage.setItem(GRID_KEY, String(gridSpacing))
    } catch {
      // Kept for this page only.
    }
  }, [session.doc, gridSpacing])
  // View ▸ 3D View: whether the composition has 3D layers, the view shown, and whether it has a
  // camera layer (`view.get3D`), asked again whenever the state changes.
  const has3D = state?.layers.some(isThreeD) ?? false
  const [view3d, setView3d] = useState<ThreeDView>({ has3D: false, view: 'activeCamera', camera: false })
  useEffect(() => {
    if (!session.doc || !state?.comp) return undefined
    let current = true
    void query<{ view?: string; activeCameraLayer?: unknown }>('view.get3D', {}).then((answer) => {
      if (!current || !answer) return
      const next = { has3D, view: answer.view ?? 'activeCamera', camera: answer.activeCameraLayer != null }
      setView3d((before) => (before.has3D === next.has3D && before.view === next.view && before.camera === next.camera ? before : next))
    })
    return () => {
      current = false
    }
  }, [session.doc, state, has3D, query])
  /**
   * The camera tools move a Front, Left, Top or Custom view without an edit, which the canvas
   * does not repaint for by itself: it paints again after each of their drags' moves (once per
   * frame) and after the release.
   */
  const cameraTool = tools.active === 'orbit' || tools.active === 'track' || tools.active === 'dolly'
  const repaintSoon = useRef(0)
  const repaintAfterCamera = (event: { buttons: number; type: string }) => {
    if (!cameraTool || view3d.view === 'activeCamera' || (event.buttons === 0 && event.type !== 'pointerup')) return
    if (repaintSoon.current) return
    repaintSoon.current = window.setTimeout(() => {
      repaintSoon.current = 0
      void canvas.current?.retryRender()
    }, 30)
  }
  // Where the Selection tool's 3D handles are (the engine's `gizmoHandles`, composition pixels),
  // on the workspace for the browser tests, which press there.
  useEffect(() => {
    const doc = session.doc
    const element = workspace.current
    if (!doc || !element || !zoom || tools.active !== 'selection' || !anyThreeD) {
      element?.removeAttribute('data-gizmo')
      return undefined
    }
    let current = true
    void doc
      .invoke<string>('gizmoHandles', zoom)
      .then((answer) => {
        if (current) element.setAttribute('data-gizmo', answer)
      })
      .catch(() => undefined)
    return () => {
      current = false
    }
  }, [session.doc, state, zoom, view3d.view, tools.active, anyThreeD])
  // The graph editor's curves: a property's values over time, sampled by the engine.
  const doc = session.doc
  const sampler = useCallback<CurveSampler>(
    async (layer, path, from, to, count) => (doc ? (JSON.parse(await doc.invoke<string>('curveSamples', layer, path, from, to, count)) as unknown[]) : null),
    [doc],
  )
  // The timeline's waveform rows: a layer's sound as peaks (the engine's `layerWaveform`).
  const waveform = useCallback<WaveformReader>(async (layer, count) => (doc ? (JSON.parse(await doc.invoke<string>('layerWaveform', layer, count)) as Waveform) : null), [doc])
  // The status bar's readout: the composition pixel under the pointer and its colour (the
  // engine's `pixelAt`, one question at a time, the latest point next), in 0-255 or percent.
  const [pointer, setPointer] = useState<{ x: number; y: number; rgba: number[] | null } | null>(null)
  const [percent, setPercent] = useState(() => {
    try {
      return globalThis.localStorage?.getItem(INFO_KEY) === 'percent'
    } catch {
      return false
    }
  })
  const togglePercent = () =>
    setPercent((now) => {
      try {
        localStorage.setItem(INFO_KEY, now ? 'bytes' : 'percent')
      } catch {
        // Kept for this page only.
      }
      return !now
    })
  const pointerAsk = useRef<{ busy: boolean; next: [number, number] | null; gone: boolean }>({ busy: false, next: null, gone: false })
  /** The pointer left the composition: the readout empties (it would read a place no longer pointed at). */
  const clearPointer = () => {
    pointerAsk.current.next = null
    pointerAsk.current.gone = true
    setPointer(null)
  }
  const readPointer = (event: { clientX: number; clientY: number; target: EventTarget | null }) => {
    if (event.target instanceof Element && event.target.closest('.motion-timeline, .motion-transport, .motion-splitter')) {
      clearPointer()
      return
    }
    const view = canvas.current?.view()
    const element = workspace.current?.querySelector('[data-testid="kit-canvas-surface"]')
    if (!view || !element || !state?.comp) return
    const rect = element.getBoundingClientRect()
    if (event.clientX < rect.left || event.clientY < rect.top || event.clientX > rect.right || event.clientY > rect.bottom) {
      clearPointer()
      return
    }
    const ask = pointerAsk.current
    ask.gone = false
    ask.next = [view.x + (event.clientX - rect.left) / view.zoom, view.y + (event.clientY - rect.top) / view.zoom]
    if (ask.busy) return
    ask.busy = true
    void (async () => {
      while (ask.next) {
        const [x, y] = ask.next
        ask.next = null
        // While it plays the engine is busy with frames: the position only.
        const rgba = playback.playing || !session.doc ? null : await session.doc.invoke<Uint8Array>('pixelAt', x, y).catch(() => null)
        // Left meanwhile: an answer arriving late does not bring the readout back.
        if (ask.gone) break
        setPointer({ x: Math.floor(x), y: Math.floor(y), rgba: rgba && rgba.length === 4 ? [...rgba] : null })
      }
      ask.busy = false
    })()
  }
  const [findEffect, setFindEffect] = useState(false)
  // Every effect the engine has, for the Effect menu; and whether one was applied here (Last Effect).
  const [effects, setEffects] = useState<MotionEffect[]>([])
  const [lastEffect, setLastEffect] = useState(false)
  useEffect(() => {
    if (!session.doc) return
    void query<MotionEffect[]>('effect.list', {}).then((list) => {
      // Names translated by id (effectNames.ts) are known once the engine names them.
      learnEffectNames(list ?? [])
      setEffects((list ?? []).map(({ id, name, category, params }) => ({ id, name, category, params })))
    })
  }, [session.doc, query])
  const applyEffect = (effect: string) =>
    void run('effect.apply', { effect }).then((done) => {
      if (done === null) return
      setLastEffect(true)
      // Find Effect lists it under Recently used.
      rememberEffect(effect)
    })
  /** A name not taken by a project item: `Comp 2`, `Solid 1`. */
  const unusedName = async (base: string) => {
    const summary = await query<{ items?: Array<{ name?: string }> }>('project.summary', {})
    const names = new Set((summary?.items ?? []).map((item) => item.name))
    let n = 1
    while (names.has(`${base} ${n}`)) n += 1
    return `${base} ${n}`
  }
  /** A layer name not taken in this composition: `摄像机 1`, `灯光 2`. */
  const unusedLayerName = (base: string) => {
    const names = new Set(state?.layers.map((layer) => layer.name))
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
      setDialog({ kind: 'solid', mode, values: { name: await unusedName(t('defaultSolid')), width: comp.width, height: comp.height, color: '#808080', comp, layers: state?.layers.length ?? 0 } })
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
  /** What `keyframe.info` answers for the selected keyframes (the fields the dialogs read). */
  type SelectedKeyInfo = { dims?: number; spatial?: string; units?: string; inEase?: Array<{ speed: number; influence: number } | null>; outEase?: Array<{ speed: number; influence: number } | null> }
  /** Keyframe Interpolation, with Spatial when a selected keyframe is on a moving property (Position, Anchor Point). */
  const openInterpolation = async () => {
    const keys = await query<SelectedKeyInfo[]>('keyframe.info', {})
    if (!Array.isArray(keys) || !keys.length) return
    setDialog({ kind: 'interpolation', spatial: keys.some((key) => typeof key.spatial === 'string') })
  }
  /** Keyframe Velocity, starting from the first selected keyframe's first dimension. */
  const openVelocity = async () => {
    const keys = await query<SelectedKeyInfo[]>('keyframe.info', {})
    const first = Array.isArray(keys) ? keys[0] : undefined
    if (!first) return
    const side = (ease: SelectedKeyInfo['inEase']) => ease?.find((one) => one != null) ?? { speed: 0, influence: 33.33 }
    const [inside, outside] = [side(first.inEase), side(first.outEase)]
    setDialog({ kind: 'velocity', units: first.units ?? '', values: { inSpeed: inside.speed, inInfluence: inside.influence, outSpeed: outside.speed, outInfluence: outside.influence } })
  }
  /** The keyframe value dialog, on the one selected keyframe (a double-click on it, or its menu's Edit Value…). */
  const openKeyValue = async () => {
    const keys = await query<Array<{ layer: number; path?: string; name?: string; compTime: number; value: unknown }>>('keyframe.info', {})
    const key = Array.isArray(keys) && keys.length === 1 ? keys[0] : null
    if (!key || !state?.comp || !key.path) return
    // The property's name as the timeline shows it (in the editor's language); a 2D layer's
    // Position, Anchor Point or Scale shows x and y only, as the timeline does.
    const layer = state.layers.find((each) => each.id === key.layer)
    const flat = layer != null && !isThreeD(layer) && /^transform\/(position|anchor|scale)$/.test(key.path) && Array.isArray(key.value) && key.value.length === 3
    setDialog({ kind: 'keyValue', layer: key.layer, path: key.path, name: phrase(key.name ?? key.path), time: key.compTime, value: key.value, fps: state.comp.fps, duration: state.comp.duration, ...(flat ? { shown: 2 } : {}) })
  }
  /** Time Stretch, from the first selected layer's stretch and length. */
  const openStretch = async () => {
    const selected = state?.layers.filter((layer) => layer.selected) ?? []
    const first = selected[0]
    if (!first || !state?.comp) return
    const row = (await query<{ layers?: Array<{ id: number; stretch?: number }> }>('comp.info', {}))?.layers?.find((layer) => layer.id === first.id)
    setDialog({ kind: 'stretch', layers: selected.map((layer) => layer.id), stretch: row?.stretch ?? 100, span: Math.max(0, first.out - first.in), fps: state.comp.fps })
  }
  /**
   * Animation presets go through the host: Save writes the selected properties' and effects'
   * animation (`animation.savePreset`, in memory) to `<name>.ecpreset` beside the project; Apply picks
   * a preset in the workspace and applies it to the selected layers at the current time
   * (`animation.applyPreset`, one undo step).
   */
  const savePreset = async (name: string, inside: string) => {
    const file = `${name}.ecpreset`
    const folder = host.init.document.path.split('/').slice(0, -1).join('/')
    try {
      localStorage.setItem(PRESET_FOLDER_KEY, inside)
    } catch {
      // Asked again next time.
    }
    try {
      if ((await run('animation.savePreset', { path: file, name })) === null) return
      const bytes = await session.doc?.invoke<Uint8Array | undefined>('takeFile', file)
      if (!bytes) throw new Said(t('preset.notWritten'))
      const written = await host.exportFile([folder, inside, file].filter(Boolean).join('/'), bytes)
      // Named with the folder it went into, inside the project folder (Presets/Moves/Slide in.ecpreset).
      const shown = [inside.replace(/^\/+|\/+$/g, ''), written.split('/').pop() ?? written].filter(Boolean).join('/')
      toast.success(t('preset.saved', { name: shown }))
    } catch (error) {
      failed(t, t('preset.saveFailed'), error)
    }
  }
  const applyPreset = async () => {
    if (!host.init.capabilities.includes('pick-file')) {
      toast.error(t('import.noPicker'))
      return
    }
    const folder = host.init.document.path.split('/').slice(0, -1).join('/')
    const path = await host.pick('preset', ['ecpreset']).catch(() => null)
    if (!path) {
      toast.info(t('preset.noneChosen'))
      return
    }
    if ((await run('animation.applyPreset', { path: relativePath(folder, path) })) !== null) toast.success(t('preset.applied', { name: path.split('/').pop() ?? path }))
  }
  // What the canvas tools name the shape layers they make, in the editor's language.
  useEffect(() => {
    if (!session.doc) return
    // And the labels the Layer ▸ New commands give new layers, by kind.
    void session.doc.invoke('setLayerNames', JSON.stringify({ shape: t('layerName.shape'), text: t('layerName.text'), labels: NEW_LAYER_LABELS })).catch(() => undefined)
  }, [session.doc, t])
  // The Pen's option on a shape layer: draw a mask on it instead of a new path in it.
  const [penMasks, setPenMasks] = useState(false)
  useEffect(() => {
    if (!session.doc) return
    void session.doc.invoke('setPenMasks', penMasks).catch(() => undefined)
  }, [session.doc, penMasks])
  // View ▸ Two Views: a second 3D view beside the viewer (Top, or the active camera when the
  // viewer shows Top), kept in this browser.
  const [twoViews, setTwoViews] = useState(() => {
    try {
      return globalThis.localStorage?.getItem(TWO_VIEWS_KEY) === 'true'
    } catch {
      return false
    }
  })
  const toggleTwoViews = () =>
    setTwoViews((now) => {
      try {
        localStorage.setItem(TWO_VIEWS_KEY, String(!now))
      } catch {
        // Kept for this page only.
      }
      return !now
    })
  const [chosenSecond, setChosenSecond] = useState('top')
  const secondView = chosenSecond === view3d.view ? (view3d.view === 'top' ? 'activeCamera' : 'top') : chosenSecond
  /** Make Main: the second view goes to the viewer, the viewer's view here. */
  const swapViews = () => {
    const row = THREE_D_VIEWS.find(([, , each]) => each === secondView)
    if (!row) return
    setChosenSecond(view3d.view)
    void run(row[0], {})
  }
  // View ▸ Region of Interest: drawn with a drag over the composition; the viewer then renders
  // only inside it.
  const [region, setRegion] = useState<Region | null>(null)
  const [drawingRegion, setDrawingRegion] = useState(false)
  const setRegionOfInterest = (rect: Region | null) => {
    setDrawingRegion(false)
    void run('view.setRegionOfInterest', { rect }).then((done) => {
      if (done !== null) setRegion(rect)
    })
  }
  const toggleRegion = () => {
    if (region) setRegionOfInterest(null)
    else setDrawingRegion((now) => !now)
  }
  useEffect(() => {
    if (drawingRegion) workspace.current?.querySelector<HTMLElement>('[data-testid="motion-roi-layer"]')?.focus({ preventScroll: true })
  }, [drawingRegion])
  const purgeCache = () =>
    void session.doc
      ?.invoke('purgePreviewCache')
      .then(() => frameCache.refresh())
      .then(() => toast.success(t('cache.purged')))
  /** Commands the editor answers itself (the rest go to the engine as they are). */
  const own: Record<string, () => void> = {
    ...Object.fromEntries(Object.entries(OVERLAY_ROWS).map(([command, key]) => [command, () => toggleOverlay(key)])),
    ...Object.fromEntries(RESOLUTIONS.map(([command, , divisor]) => [command, () => setPreview(divisor)])),
    'playback.toggle': playback.toggle,
    [KIT.roi]: () => setDrawingRegion((now) => !now),
    [KIT.clearRoi]: () => setRegionOfInterest(null),
    [KIT.skipFrames]: toggleSkipFrames,
    [KIT.cacheAhead]: toggleCacheAhead,
    [KIT.twoViews]: toggleTwoViews,
    [KIT.purgeCache]: purgeCache,
    'file.exportMovie': () => {
      if (renderQueue.rendering) toast.info(t('queue.busy'))
      else if (state?.comp) setExporting('movie')
    },
    [KIT.addToQueue]: () => state?.comp && void addToQueue([state.comp.id]),
    [KIT.renderQueue]: () => setQueueOpen((now) => !now),
    [KIT.rove]: () => {
      if (keyState.rovable === false) toast.info(t('keyMenu.roveEnds'))
      else void run('keyframe.interpolation', { roving: !keyState.roving })
    },
    'kit:edit-key': () => void openKeyValue(),
    [KIT.collectFiles]: () => {
      const project = (host.init.document.path.split('/').pop() ?? 'Project').replace(/\.[^.]*$/, '')
      setDialog({ kind: 'collect', folder: t('collect.folderName', { name: project }), onCollect: (folder) => void collectFiles(folder) })
    },
    // Chosen before they can run, they say what they wait for (their menu rows say it too).
    [KIT.wiggler]: () => ((keyState.count ?? 0) >= 2 ? setDialog({ kind: 'wiggler', spatial: keyState.spatial }) : toast.info(phrase(ASSISTANT_NEEDS.wiggler))),
    [KIT.smoother]: () => ((keyState.count ?? 0) >= 3 ? setDialog({ kind: 'smoother' }) : toast.info(phrase(ASSISTANT_NEEDS.smoother))),
    // One undo step per property (each its own `prop.convertExpressionToKeyframes`).
    [KIT.convertExpression]: () => (convertible.length ? convertible.forEach((each) => void run('prop.convertExpressionToKeyframes', each)) : toast.info(phrase(expressionOff ? ASSISTANT_NEEDS.convertOff : ASSISTANT_NEEDS.convert))),
    'layer.revealSource': () => void revealSource(),
    'project.newFolder': () => void newFolder(),
    'file.removeUnusedFootage': () =>
      void run('file.removeUnusedFootage', {}).then((answer) => {
        const removed = parseAnswer<{ removed?: number }>(answer)
        if (removed) toast.success(t('project.removedUnused', { count: removed.removed ?? 0 }))
      }),
    'file.exportLottie': () => state?.comp && setExporting('lottie'),
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
    'keyframe.interpolation': () => void openInterpolation(),
    'keyframe.velocity': () => void openVelocity(),
    'layer.timeStretch': () => void openStretch(),
    'kit:sequence': () => state?.comp && setDialog({ kind: 'sequence', fps: state.comp.fps }),
    [KIT.alignTo]: () => setAlignTo((current) => (current === 'composition' ? 'selection' : 'composition')),
    'animation.savePreset': () => setDialog({ kind: 'savePreset', name: t('preset.defaultName'), folder: storedPresetFolder(), onSave: (name, inside) => void savePreset(name, inside) }),
    [KIT.gridSettings]: () => setDialog({ kind: 'grid', spacing: gridSpacing, onSave: setGridSpacing }),
    [KIT.newGuide]: () => state?.comp && setDialog({ kind: 'guide', comp: { width: state.comp.width, height: state.comp.height } }),
    'animation.applyPreset': () => void applyPreset(),
    ...Object.fromEntries(INTERPOLATIONS.map((row) => [row.command, () => void run('keyframe.interpolation', { interpolation: row.interpolation })])),
    'comp.settings': () => void openComp('settings'),
    'layer.newSolid': () => void openSolid('new'),
    // A new drop shadow is seen at once, even on a dark background (one already there is only turned on).
    'layer.style.dropShadow': () => void run('layer.style.dropShadow', styled.has('dropShadow') ? {} : { params: DROP_SHADOW }),
    'layer.newCamera': () => void run('layer.newCamera', { name: unusedLayerName(t('layerName.camera')) }),
    // A point light far in front of the composition, a little above its middle: its light reaches
    // every 3D layer from nearly straight on, so one turned 40° still shows (a light close in front
    // lit a turned layer off to one side almost black, a spot light off to one side even more).
    'layer.newLight': () => {
      const comp = state?.comp
      if (!comp) return
      void run('layer.newLight', { kind: 'Point', name: unusedLayerName(t('layerName.light')), position: [comp.width / 2, comp.height * 0.3, -comp.width * 2] })
    },
    'layer.newShape': () => void run('layer.newShape', { kind: 'rect', name: unusedLayerName(t('layerName.shape')) }),
    // A new text layer says “Text” in the editor's language (its name follows its text).
    'layer.newText': () => void run('layer.newText', { text: t('layerName.text') }),
    // Layer ▸ Label: the selected layers' colour label (one undo step).
    ...Object.fromEntries(
      LABELS.map(([name]) => [
        labelRow(name),
        () => {
          const layers = state?.layers.filter((layer) => layer.selected).map((layer) => layer.id) ?? []
          // The label they all have already (its row says “Current”): choosing it says so.
          if (state && currentLabel(state.layers) === name) toast.info(t('label.already', { name: t(`label.${name.replace(/ /g, '')}`) }))
          else if (layers.length) void run('edit.label', { label: name, layers, target: 'layers' })
        },
      ]),
    ),
    'layer.newNull': () => void run('layer.newNull', { name: unusedLayerName(t('layerName.null')) }),
    'layer.newAdjustment': () => void run('layer.newAdjustment', { name: unusedLayerName(t('layerName.adjustment')) }),
    'layer.settings': () => void openSolid('settings'),
    'time.set': () => state?.comp && setDialog({ kind: 'time', time: state.time, fps: state.comp.fps, duration: state.comp.duration }),
    'layer.rename': () => {
      const layer = state?.layers.find((item) => item.selected)
      if (layer) setDialog({ kind: 'rename', layer: layer.id, name: layer.name })
    },
  }
  const latestOwn = useRef(own)
  const latestAlign = useRef(alignTo)
  latestAlign.current = alignTo
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
      else if (KIT_RUNS[command]) void run(KIT_RUNS[command][0], { ...KIT_RUNS[command][1], ...(KIT_RUNS[command][0] === 'layer.align' ? { to: latestAlign.current } : {}) })
      else void run(command, {})
    },
    [run],
  )
  // Rows with nothing to act on are hidden; the predicate changes only when that does.
  const idle = useIdleRows(session.doc, state, playback.playing, alignTo)
  // The Align section's buttons are off where Layer ▸ Align's rows are hidden.
  const alignIdle = useMemo(() => {
    const off = new Set(idle)
    if (alignTo === 'selection' && (state?.layers.filter((layer) => layer.selected).length ?? 0) < 2) for (const command of Object.keys(KIT_RUNS)) if (command.startsWith('kit:align-')) off.add(command)
    return off
  }, [idle, alignTo, state?.layers])
  // The selected keyframes, for Rove Across Time: whether any moves through space, and whether they all rove.
  const keysSelected = editor.enabled('keyframe.interpolation')
  const [keyState, setKeyState] = useState<SelectedKeys>({ spatial: false, roving: false })
  useEffect(() => {
    if (!keysSelected || playback.playing) {
      setKeyState((before) => (before.spatial || before.roving || before.count ? { spatial: false, roving: false } : before))
      return undefined
    }
    let current = true
    void query<Array<SelectedKey & { spatial?: string | null; roving?: boolean }>>('keyframe.info', {}).then(async (answer) => {
      if (!current || !Array.isArray(answer)) return
      const keys = distinctKeys(answer)
      const moving = keys.filter((key) => typeof key.spatial === 'string')
      const spatial = moving.length > 0
      const roving = spatial && moving.every((key) => key.roving === true)
      // Roving on a first or last keyframe changes nothing (the engine tries it on a copy).
      const rovable = !spatial || roving ? true : await (session.doc?.invoke<boolean>('wouldChange', 'keyframe.interpolation', JSON.stringify({ roving: true })) ?? Promise.resolve(true)).catch(() => true)
      if (!current) return
      const next = { spatial, roving, rovable, count: keys.length }
      setKeyState((before) => (before.spatial === next.spatial && before.roving === next.roving && before.rovable === next.rovable && before.count === next.count ? before : next))
    })
    return () => {
      current = false
    }
  }, [state, keysSelected, playback.playing, query])
  // Animation ▸ Keyframe Assistant ▸ Convert Expression to Keyframes: the selected properties whose expression it would turn into keyframes.
  const [convertible, setConvertible] = useState<Array<{ layer: number; path: string }>>([])
  // A selected property has an expression that is turned off: Convert says to turn it on first.
  const [expressionOff, setExpressionOff] = useState(false)
  useEffect(() => {
    const doc = session.doc
    const chosen = (state?.layers ?? []).flatMap((layer) => (layer.selected ? layer.selectedProps.map((path) => ({ layer: layer.id, path })) : []))
    if (!doc || playback.playing || !chosen.length) {
      setConvertible((before) => (before.length ? [] : before))
      setExpressionOff(false)
      return undefined
    }
    let current = true
    void Promise.all(chosen.map((each) => doc.invoke<boolean>('wouldChange', 'prop.convertExpressionToKeyframes', JSON.stringify(each)).catch(() => false))).then(async (answers) => {
      if (!current) return
      const next = chosen.filter((_, at) => answers[at])
      setConvertible((before) => (JSON.stringify(before) === JSON.stringify(next) ? before : next))
      if (next.length) {
        setExpressionOff(false)
        return
      }
      // None would change: is there an expression there, turned off?
      const reads = await Promise.all(chosen.map((each) => query<{ expression?: string | null }>('prop.get', each).catch(() => null)))
      if (current) setExpressionOff(reads.some((read) => (read?.expression ?? '').trim() !== ''))
    })
    return () => {
      current = false
    }
  }, [session.doc, state, playback.playing, query])
  const hiding = hiddenRow(state, lastEffect, { fitted: fitView === viewKey, actual: zoom === 1, preview, alignTo, threeD: view3d, region: region !== null, cached: frameCache.cached.length > 0, keys: keyState }, keysSelected, idle)
  const hidden = useMemo(() => hiding.hidden, [hiding.key]) // eslint-disable-line react-hooks/exhaustive-deps
  // The first selected layer's layer styles: Layer ▸ Layer Styles marks them (Added).
  const firstSelected = state?.layers.find((layer) => layer.selected)?.id
  const [styled, setStyled] = useState<ReadonlySet<string>>(() => new Set())
  useEffect(() => {
    if (firstSelected == null) {
      setStyled((before) => (before.size ? new Set() : before))
      return undefined
    }
    let current = true
    void query<{ styles?: Array<{ style?: string }> }>('layer.style.list', { layer: firstSelected }).then((answer) => {
      if (!current) return
      const next = new Set((answer?.styles ?? []).map((style) => style.style ?? ''))
      setStyled((before) => ([...before].sort().join() === [...next].sort().join() ? before : next))
    })
    return () => {
      current = false
    }
  }, [firstSelected, state, query])
  // Layer ▸ Time ▸ Enable or Disable Time Remapping, by the first selected layer.
  const remapOn = state?.layers.find((layer) => layer.selected)?.animated.some((property) => property.path === 'timeRemap') ?? false
  // The label every selected layer has (Layer ▸ Label ticks it and calls it current).
  const labelNow = state ? currentLabel(state.layers) : null
  const canConvert: Convertible = convertible.length > 0 ? true : expressionOff ? 'off' : false
  // The Animation menu's rows that cannot run now look it (greyed; a click still says why).
  const waitingStyles = useMemo(() => waitingRowStyles(waitingRows(keyState, canConvert)), [keyState, canConvert])
  const rows = useMemo(() => menuRows(effects, overlays, alignTo, skipFrames, remapOn, styled, keyState, cacheAhead, twoViews, canConvert, labelNow), [effects, overlays, alignTo, skipFrames, remapOn, styled, keyState, cacheAhead, twoViews, canConvert, labelNow])
  // Only the menus' engine commands (and a few more) run from their shortcuts.
  const allowed = useMemo(() => shortcutCommands(rows), [rows])
  const hiddenCommands = useCallback((command: string) => !allowed.has(command), [allowed])
  const wholeWorkArea = hiding.key.includes('comp.trimToWorkArea')
  // Layer ▸ Label ticks the label every selected layer has.
  const checked = useCallback((command: string) => (command.startsWith('kit:label-') ? labelNow !== null && labelRow(labelNow) === command : undefined), [labelNow])
  // Home first (its own ribbon; a drop-down panel in Menus), then the editor's menus.
  const tabs = useMemo<FrameTab[]>(
    () => [
      { id: 'home', label: t('homeTab'), ribbon: <HomeTab info={state} onCommand={dispatch} wholeWorkArea={wholeWorkArea} lastEffect={lastEffect} /> },
      ...catalogTabs(rows, { phrase, view: VIEW_MENUS, primary: MOTION_PRIMARY, hidden, checked, onCommand: dispatch, onKitCommand: dispatch }).map((tab) =>
        // The Effect tab's ribbon: a drop-down per category rather than a button per effect.
        tab.id === 'Effect' ? { ...tab, ribbon: <EffectRibbon effects={effects} onCommand={dispatch} lastEffect={lastEffect} /> } : tab,
      ),
    ],
    [dispatch, effects, hidden, checked, lastEffect, phrase, rows, t, wholeWorkArea, state],
  )

  // What the active tool says (its hints, translated; a refused step in the editor's words, the
  // engine's own to the console), else how to use the Pen.
  const toolSays = useMemo(() => (tools.status ? toolStatus(t, phrase, (text) => Object.hasOwn(PHRASES['zh-CN'] ?? {}, text), tools.status) : null), [tools.status, t, phrase])
  const anySelected = state?.layers.some((layer) => layer.selected) ?? false
  // On a selected shape layer the Pen draws a new path in it, or a mask with its option on.
  const shapeSelected = state?.layers.find((layer) => layer.selected)?.type === 'Shape'
  const penHint = !anySelected ? 'pen.hint.shape' : shapeSelected && !penMasks ? 'pen.hint.path' : 'pen.hint.mask'
  const toolHint = toolSays ? toolSays : tools.active === 'pen' && !host.readOnly ? t(penHint) : null

  // The timeline's own keys (Delete, Cmd+D, …) come first.
  const timelineKeys: TimelineKeys = useRef(null)
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!state) return false
    // Escape while paths are edited ends the editing (the status bar says so). The tools take
    // Escape only while busy or typing: picking the Selection tool again ends it in the engine.
    if (event.key === 'Escape' && !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey && tools.status === PATH_EDIT_HINT) {
      tools.select('selection')
      return true
    }
    const command = event.metaKey || event.ctrlKey
    const plain = !command && !event.altKey && !event.shiftKey
    // T follows where the person last pressed (or where the focus is): after a press on the
    // timeline it shows Opacity, after one on the composition it picks the Text tool.
    const focusIn = event.target instanceof Element && event.target.closest('.motion-timeline') != null
    const inTimeline = focusIn || lastArea.current === 'timeline'
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
    // The 3D views' keys that browsers leave alone (as F10, F11 and F12).
    const view3dKey = pressed(EDITOR_KEYS.view3dFront, event) ? 'view.threeD.front' : pressed(EDITOR_KEYS.view3dCustom, event) ? 'view.threeD.custom1' : pressed(EDITOR_KEYS.view3dCamera, event) ? 'view.threeD.activeCamera' : null
    if (view3dKey) {
      if (view3d.has3D) void run(view3dKey, {})
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
      <style>{LABEL_ROW_STYLES}</style>
      <style data-testid="motion-waiting-rows">{waitingStyles}</style>
      <EffectCatalog.Provider value={effects}>
      <SamplerContext.Provider value={sampler}>
      <WaveformContext.Provider value={waveform}>
      <EditorFrame
        tabs={tabs}
        defaultTab="home"
        actions={
          <>
            <Button variant="ghost" className="motion-toggle" aria-pressed={project.open} onClick={project.toggle} title={t('project.toggle')} data-testid="motion-project-toggle">
              <PanelLeftIcon size={14} aria-hidden="true" />
              {t('project.title')}
            </Button>
            <Button variant="ghost" disabled={!state?.comp} onClick={() => dispatch('file.exportMovie')} data-testid="motion-render">
              <Download size={14} aria-hidden="true" />
              {t('renderMovie')}
            </Button>
          </>
        }
        sidebar={
          state && (project.open || queueOpen) ? (
            <div className="motion-sidebar" style={{ width: project.width }}>
              {project.open ? (
                <ProjectPanel
                  info={state}
                  width={project.width}
                  onWidth={project.setWidth}
                  onImport={() => void footage.importPicked()}
                  onNewComp={() => void openComp('new')}
                  onPlace={(item) => placeItem(item)}
                  onReplace={(item: ProjectItem) => void footage.replacePicked(item.id, item.name)}
                  onInterpret={(item: ProjectItem) => setDialog({ kind: 'interpret', item })}
                  onQueue={(comps) => void addToQueue(comps)}
                  reveal={reveal}
                />
              ) : null}
              {queueOpen ? <RenderQueuePanel queue={renderQueue} onClose={() => setQueueOpen(false)} /> : null}
            </div>
          ) : null
        }
        canvas={canvas}
        rail={<EngineToolRail hidden={hiddenRail} />}
        zoom={zoom}
        onKeyDown={keyDown}
        hiddenCommands={hiddenCommands}
        status={
          <>
          {state?.comp ? (
            <Select
              className="motion-resolution"
              value={String(preview)}
              aria-label={t('resolution.label')}
              title={t('resolution.label')}
              onChange={(event) => setPreview(Number(event.target.value))}
              data-testid="motion-resolution"
            >
              {RESOLUTIONS.map(([command, label, divisor]) => (
                <option key={command} value={divisor}>
                  {phrase(label)}
                </option>
              ))}
            </Select>
          ) : null}
          {state?.comp ? (
            <Button
              variant="ghost"
              className="motion-toggle"
              aria-pressed={region !== null || drawingRegion}
              title={region ? t('roi.clear') : drawingRegion ? t('roi.hint') : t('roi.draw')}
              onClick={toggleRegion}
              data-testid="motion-roi"
              data-region={region ? region.join(' ') : undefined}
            >
              {t('roi.label')}
            </Button>
          ) : null}
          {state?.comp ? (
            <Button
              variant="ghost"
              className="motion-toggle"
              aria-pressed={cacheAhead}
              title={
                !cacheAhead
                  ? t('cacheAhead.off')
                  : ahead?.full
                    ? lowerResolution
                      ? t('cacheAhead.fullLower', { held, total: ahead.total, megabytes: Math.round((ahead.budget ?? 0) / 1048576), resolution: phrase(lowerResolution[1]) })
                      : t('cacheAhead.full', { held, total: ahead.total, megabytes: Math.round((ahead.budget ?? 0) / 1048576) })
                    : t('cacheAhead.on', { done: ahead?.done ?? 0, total: ahead?.total ?? 0 })
              }
              onClick={toggleCacheAhead}
              data-testid="motion-cache-ahead"
              data-done={ahead?.done}
              data-total={ahead?.total}
              data-full={ahead?.full || undefined}
              data-bytes={ahead?.bytes}
              data-budget={ahead?.budget}
            >
              {/* On: how far it got (N/M), or that the cache is full. */}
              {!cacheAhead || !ahead || !(ahead.total > 0) ? t('cacheAhead.label') : ahead.full ? t('cacheAhead.fullShort', { done: ahead.done, total: ahead.total }) : t('cacheAhead.progress', { done: ahead.done, total: ahead.total })}
            </Button>
          ) : null}
          {/* A full cache: one ▾ beside the cache button (a split button, "Cache full N/M ▾") opens what
              can be done about it, so the status bar keeps its zoom controls in a narrow window. */}
          {state?.comp && cacheAhead && ahead?.full ? (
            <CacheFullMenu
              held={held}
              total={ahead.total}
              megabytes={Math.round((ahead.budget ?? 0) / 1048576)}
              lower={lowerResolution ? { label: phrase(lowerResolution[1]), set: () => setPreview(lowerResolution[2]) } : null}
              loop={loopFrom !== null ? { on: loopCached, toggle: () => setLoopCached((on) => !on), area: cachedArea } : null}
              moreMemory={!memoryReported && !moreMemory ? { megabytes: MORE_MEMORY_MB, use: () => setMoreMemory(true) } : null}
            />
          ) : null}
          {state?.comp && view3d.has3D ? (
            <Select
              className="motion-3d-view"
              value={view3d.view}
              aria-label={t('view3d.label')}
              title={t('view3d.label')}
              onChange={(event) => {
                const row = THREE_D_VIEWS.find(([, , view]) => view === event.target.value)
                if (row) void run(row[0], {})
              }}
              data-testid="motion-3d-view"
            >
              {THREE_D_VIEWS.map(([command, label, view]) => (
                <option key={command} value={view}>
                  {phrase(label)}
                </option>
              ))}
            </Select>
          ) : null}
          {tools.active === 'pen' && shapeSelected && !host.readOnly ? (
            <Button variant="ghost" className="motion-toggle" aria-pressed={penMasks} title={t('pen.maskOption.hint')} onClick={() => setPenMasks((now) => !now)} data-testid="motion-pen-mask">
              {t('pen.maskOption')}
            </Button>
          ) : null}
          {toolHint ? (
            <span className="motion-tool-status" role="status" title={toolHint} data-testid="motion-tool-status">
              {toolHint}
            </span>
          ) : null}
          {pointer && state?.comp ? (
            <Button
              variant="ghost"
              className="motion-info"
              aria-pressed={percent}
              aria-label={t('info.label')}
              title={t('info.toggle', { units: t(percent ? 'info.units.bytes' : 'info.units.percent') })}
              onClick={togglePercent}
              data-testid="motion-info"
              data-x={pointer.x}
              data-y={pointer.y}
              data-rgba={pointer.rgba?.join(' ')}
            >
              <span>{t('info.position', { x: pointer.x, y: pointer.y })}</span>
              {pointer.rgba ? (
                <>
                  <span className="motion-info-swatch" style={{ background: `rgb(${pointer.rgba[0]} ${pointer.rgba[1]} ${pointer.rgba[2]})` }} aria-hidden="true" />
                  {/* R G B A, short so the tool's hint keeps its room (the tooltip names them). */}
                  <span>{(pointer.rgba ?? []).map((value) => (percent ? `${Math.round((value / 255) * 100)}%` : String(value))).join(' ')}</span>
                </>
              ) : null}
            </Button>
          ) : null}
          <span className="motion-summary" title={state?.comp ? t('summary', { name: state.comp.name, width: state.comp.width, height: state.comp.height, fps: Math.round(state.comp.fps * 100) / 100, layers: counted(t, 'count.layers', state.layers.length) }) : undefined} data-testid="motion-status">
            {state?.comp ? t('summary', { name: state.comp.name, width: state.comp.width, height: state.comp.height, fps: Math.round(state.comp.fps * 100) / 100, layers: counted(t, 'count.layers', state.layers.length) }) : t('noComp')}
          </span>
          </>
        }
        panels={
          state ? (
            <PropertiesPanel
              info={state}
              onAddEffect={() => setFindEffect(true)}
              typingOnCanvas={tools.active === 'text' || tools.wantsText}
              align={{ to: alignTo, onTo: setAlignTo, idle: alignIdle, onCommand: dispatch }}
            />
          ) : null
        }
      >
        <MotionDialogs dialog={dialog} onClose={() => setDialog(null)} onSeek={seek} />
        {exporting && state ? <ExportDialog info={state} doc={session.doc} lottie={exporting === 'lottie'} onClose={() => setExporting(false)} onSaveFrame={saveFrame} /> : null}
        {findEffect ? <FindEffectDialog effects={effects} onApply={(effect) => applyEffect(effect.id)} onClose={() => setFindEffect(false)} /> : null}
        <div
          className="kui-row"
          ref={workspace}
          data-testid="motion-workspace"
          data-picture={picture ? `${picture.width} ${picture.height}` : undefined}
          data-overlays={OVERLAY_KEYS.filter((key) => overlays[key]).join(' ')}
          data-resolution={preview}
          data-align-to={alignTo}
          data-view3d={view3d.has3D ? view3d.view : undefined}
          data-roi={region ? region.join(' ') : drawingRegion ? 'drawing' : undefined}
          data-skip-frames={skipFrames}
          data-loop={loop}
          data-cached={frameCache.cached.length}
          data-cache-ahead={cacheAhead}
          data-two-views={twoViews}
          data-preview={frameCache.stats}
        >
          <div
            className="motion-main"
            ref={main}
            style={{ '--motion-timeline-height': `${timelineHeight}px` } as CSSProperties}
            onKeyDown={keepTypedKeys}
            onPointerMoveCapture={(event) => {
              repaintAfterCamera(event)
              readPointer(event)
            }}
            onPointerUpCapture={repaintAfterCamera}
            // Gone straight to the status bar (to the readout, to switch its units), it holds the last reading.
            onPointerLeave={(event) => {
              if (!(event.relatedTarget instanceof Element && event.relatedTarget.closest('.kui-status-bar'))) clearPointer()
            }}
            onPointerDownCapture={(event) => {
              const box = event.currentTarget.getBoundingClientRect()
              lastPress.current = { x: event.clientX - box.left, y: event.clientY - box.top }
              lastArea.current = event.target instanceof Element && event.target.closest('.motion-timeline, .motion-transport, .motion-splitter') ? 'timeline' : 'canvas'
            }}
            onCompositionStart={() => setComposing('')}
            onCompositionUpdate={(event) => setComposing(event.data)}
            onCompositionEnd={() => setComposing(null)}
            data-drop={dropping || undefined}
            // Accepted on entering as well as over: WebKit makes an element the drop target only
            // when its dragenter is cancelled, so with dragover alone a drag that came in and was
            // let go without a second move dropped on nothing (the timeline has no drop target of
            // its own, as the canvas has).
            onDragEnter={acceptDrag}
            onDragOver={acceptDrag}
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
              // Dropped on the composition, the layer's centre goes where it was dropped, at the current
              // time; on the timeline, it starts at the time under the pointer, in the middle.
              const onCanvas = !(event.target instanceof Element && event.target.closest('.motion-timeline, .motion-transport, .motion-splitter'))
              const position = onCanvas ? compPoint(event.clientX, event.clientY) : undefined
              const onTimeline = event.target instanceof Element && event.target.closest('.motion-timeline') != null
              const time = (onTimeline ? timeUnder(event.clientX) : null) ?? state.time
              if (item) footage.place(item, { time, ...(position ? { position } : {}) })
              else void footage.importDropped(files, { time, ...(position ? { position } : {}) })
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
            {composing !== null && tools.wantsText ? (
              <div className="motion-preedit" style={{ left: lastPress.current.x, top: lastPress.current.y + 12 }} role="status" data-testid="motion-preedit">
                {composing}
              </div>
            ) : null}
            <div className="motion-views" data-testid="motion-views" data-two={twoViews && view3d.has3D ? true : undefined}>
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
              {state?.comp ? <CanvasGuides views={views} comp={state.comp} overlays={overlays} gridSpacing={gridSpacing} readOnly={host.readOnly === true} /> : null}
              {state?.comp ? <RegionOfInterest views={views} comp={state.comp} region={region} drawing={drawingRegion} onDrawn={setRegionOfInterest} onCancel={() => setDrawingRegion(false)} /> : null}
            </KitDocumentCanvas>
            {twoViews && view3d.has3D && state?.comp ? (
              <SecondView
                doc={session.doc}
                view={secondView}
                onView={setChosenSecond}
                main={view3d.view}
                onMakeMain={swapViews}
                onClose={toggleTwoViews}
                onSelect={(layer, add) => void run('layer.select', add ? { layers: [layer], add: true, toggle: true } : { layers: [layer] })}
                version={state}
                playing={playback.playing}
                compWidth={state.comp.width}
                compHeight={state.comp.height}
              />
            ) : null}
            </div>
            {/* The transport and the timeline are the kit's own bars: Focus shows the composition alone. */}
            {state ? (
              <FrameChrome>
                <Splitter height={timelineHeight} onHeight={setTimelineHeight} main={main} />
                <Transport
                  info={shownState ?? state}
                  playing={playback.playing}
                  onPlay={playback.toggle}
                  onSeek={seek}
                  onStep={step}
                  onCommand={dispatch}
                  trail={trail}
                  onBack={(comp) => void run('comp.open', { comp })}
                  playbackRate={{ fps: frameCache.fps, skip: skipFrames, onSkip: toggleSkipFrames }}
                  loop={{ on: loop, onToggle: toggleLoop }}
                  meters={<AudioMeters feed={meterFeed} playing={playback.playing} />}
                />
                <Timeline
                  info={shownState ?? state}
                  onSeek={(seconds) => {
                    playback.stop()
                    seek(seconds)
                  }}
                  keys={timelineKeys}
                  onOpenPrecomp={openPrecomp}
                  onPrecompose={() => void precompose()}
                  onCommand={dispatch}
                  onRevealSource={(layer) => void revealSource(layer)}
                  cached={frameCache.cached}
                />
              </FrameChrome>
            ) : null}
          </div>
        </div>
      </EditorFrame>
      </WaveformContext.Provider>
      </SamplerContext.Provider>
      </EffectCatalog.Provider>
    </KitEditorProvider>
  )
}
