import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type RefObject } from 'react'
import { DownloadIcon as Download, LoaderCircleIcon as Loader2 } from '@kition/tessera/icons/react'
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
  type KitEngine,
  type Phrases,
  type RasterView,
} from '@kition/sdk/editor'
import { engineManifest } from './engine'
import { CATALOGS } from './strings'
import { describe, picturePanBounds } from './motion'
import { ARRANGE, KIT, MOTION_PRIMARY, VIEW_MENUS, hiddenRow, menuRows } from './menus'
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
  return { playing, toggle, stop }
}

/** Renders the composition as an H.264 MP4 beside the project (the host picks an unused name). */
function useMovieRender(doc: KitEngine | null, host: HostConnection, t: (key: string, values?: Record<string, string | number>) => string) {
  const [rendering, setRendering] = useState(false)
  const start = useCallback(async () => {
    if (!doc || rendering) return
    setRendering(true)
    try {
      const path = host.init.document.path.replace(/\.[^./]+$/, '') + '.mp4'
      const name = path.split('/').pop() ?? path
      const bytes = await doc.invoke<Uint8Array>('renderMovie', name, 'h264')
      const written = await host.exportFile(path, bytes)
      toast('success', t('rendered', { name: written.split('/').pop() ?? written }))
    } catch (error) {
      toast('error', t('renderFailed'), error instanceof Error ? error.message : String(error))
    } finally {
      setRendering(false)
    }
  }, [doc, host, rendering, t])
  return { rendering, start }
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

/** Commands the editor asks values for in its own dialogs. */
const DIALOG_COMMANDS = ['comp.new', 'comp.settings', 'layer.newSolid', 'layer.settings', 'layer.rename', 'time.set']

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
  const movie = useMovieRender(session.doc, host, t)
  const seek = useCallback((seconds: number) => void runQuietly('time.set', { time: Math.max(0, seconds) }).catch(() => undefined), [runQuietly])
  const step = (frames: number) => void runQuietly('time.step', { frames }).catch(() => undefined)
  const { run, query } = editor
  const [dialog, setDialog] = useState<MotionDialog | null>(null)
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
  /** Commands the editor answers itself (the rest go to the engine as they are). */
  const own: Record<string, () => void> = {
    'playback.toggle': playback.toggle,
    'file.exportMovie': () => void movie.start(),
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
      else void run(command, {})
    },
    [run],
  )
  // Rows with nothing to act on are hidden; the predicate changes only when that does.
  const hiding = hiddenRow(state, lastEffect, { fitted: fitView === viewKey, actual: zoom === 1 })
  const hidden = useMemo(() => hiding.hidden, [hiding.key]) // eslint-disable-line react-hooks/exhaustive-deps
  const rows = useMemo(() => menuRows(effects), [effects])
  const wholeWorkArea = hiding.key.includes('comp.trimToWorkArea')
  // Home first (its own ribbon; a drop-down panel in Menus), then the editor's menus.
  const tabs = useMemo<FrameTab[]>(
    () => [
      { id: 'home', label: t('homeTab'), ribbon: <HomeTab onCommand={dispatch} wholeWorkArea={wholeWorkArea} lastEffect={lastEffect} /> },
      ...catalogTabs(rows, { phrase, view: VIEW_MENUS, primary: MOTION_PRIMARY, hidden, onCommand: dispatch, onKitCommand: dispatch }).map((tab) =>
        // The Effect tab's ribbon: a drop-down per category rather than a button per effect.
        tab.id === 'Effect' ? { ...tab, ribbon: <EffectRibbon effects={effects} onCommand={dispatch} lastEffect={lastEffect} /> } : tab,
      ),
    ],
    [dispatch, effects, hidden, lastEffect, phrase, rows, t, wholeWorkArea],
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
    if (arrow && !command && !event.altKey && tools.active === 'selection' && !host.readOnly && state.layers.some((layer) => layer.selected)) {
      const by = event.shiftKey ? NUDGE.shift : NUDGE.plain
      void editor.run('layer.nudge', { x: arrow[0] * by, y: arrow[1] * by })
      return true
    }
    // The commands the editor asks values for take their menu shortcuts here (Cmd+N, Cmd+K, Cmd+Y…), before the engine would run them without.
    for (const id of DIALOG_COMMANDS) {
      const shortcut = editor.byId.get(id)?.shortcut
      if (shortcut && shortcutMatches(shortcut, event) && (editor.enabled(id) || id === 'comp.new')) {
        if (!host.readOnly || id === 'time.set') dispatch(id)
        return true
      }
    }
    // Space plays on its release (useSpaceTap); its press is taken here so nothing else runs.
    if (event.key === ' ' && !command) return true
    // View ▸ Zoom In and Zoom Out: . and , (Cmd or Ctrl with = and - zoom too, and Cmd+0 fits).
    else if ((event.key === '.' || event.key === ',') && !command && !event.altKey) canvas.current?.zoomBy(event.key === '.' ? ZOOM_STEP : 1 / ZOOM_STEP)
    else if (event.key === 'PageDown' || (event.key === 'ArrowRight' && command)) step(event.shiftKey ? 10 : 1)
    else if (event.key === 'PageUp' || (event.key === 'ArrowLeft' && command)) step(event.shiftKey ? -10 : -1)
    // Layer ▸ Arrange ▸ Bring Layer Forward / Send Layer Backward (with Shift: to the front / back).
    else if (command && !host.readOnly && (event.key === ']' || event.key === '}')) void editor.run('layer.arrange', { to: event.shiftKey ? 'front' : 'forward' })
    else if (command && !host.readOnly && (event.key === '[' || event.key === '{')) void editor.run('layer.arrange', { to: event.shiftKey ? 'back' : 'backward' })
    else if (event.key === 'Home') seek(0)
    else if (event.key === 'End') end()
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
      <EditorFrame
        tabs={tabs}
        defaultTab="home"
        actions={
          <Button variant="ghost" disabled={!state?.comp || movie.rendering} onClick={() => void movie.start()} data-testid="motion-render">
            {movie.rendering ? <Loader2 size={14} className="kui-spin" aria-hidden="true" /> : <Download size={14} aria-hidden="true" />}
            {movie.rendering ? t('rendering') : t('renderMovie')}
          </Button>
        }
        canvas={canvas}
        rail={<EngineToolRail />}
        zoom={zoom}
        onKeyDown={keyDown}
        status={
          <span data-testid="motion-status">
            {state?.comp ? t('summary', { name: state.comp.name, width: state.comp.width, height: state.comp.height, fps: Math.round(state.comp.fps * 100) / 100, layers: state.layers.length }) : t('noComp')}
          </span>
        }
        panels={state ? <PropertiesPanel info={state} onAddEffect={() => setFindEffect(true)} typingOnCanvas={tools.active === 'text' || tools.wantsText} /> : null}
      >
        <MotionDialogs dialog={dialog} onClose={() => setDialog(null)} onSeek={seek} />
        {findEffect ? <FindEffectDialog effects={effects} onApply={(effect) => applyEffect(effect.id)} onClose={() => setFindEffect(false)} /> : null}
        <div className="kui-row" ref={workspace} data-testid="motion-workspace" data-picture={picture ? `${picture.width} ${picture.height}` : undefined}>
          <div className="motion-main" ref={main} style={{ '--motion-timeline-height': `${timelineHeight}px` } as CSSProperties} onKeyDown={keepTypedKeys}>
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
                <Transport info={state} playing={playback.playing} onPlay={playback.toggle} onSeek={seek} onStep={step} onCommand={dispatch} />
                <Timeline
                  info={state}
                  onSeek={(seconds) => {
                    playback.stop()
                    seek(seconds)
                  }}
                  keys={timelineKeys}
                />
              </FrameChrome>
            ) : null}
          </div>
        </div>
      </EditorFrame>
      </EffectCatalog.Provider>
    </KitEditorProvider>
  )
}
