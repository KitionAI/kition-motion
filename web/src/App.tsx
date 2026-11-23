import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { DownloadIcon as Download, LoaderCircleIcon as Loader2 } from '@kition/tessera/icons/react'
import {
  Button,
  EditorFrame,
  FrameChrome,
  KitDocumentCanvas,
  KitEditorProvider,
  HAND_TOOL,
  catalogTabs,
  createKitStrings,
  shortcutMatches,
  toast,
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
import { ARRANGE, MENU_ROWS, MOTION_PRIMARY, VIEW_MENUS, hiddenRow } from './menus'
import { CanvasScrollbars, createViewStore } from './CanvasScrollbars'
import { HomeTab } from './HomeTab'
import { MotionDialogs, toHex, type MotionDialog, type Rgb } from './dialogs'
import { Timeline, type TimelineKeys } from './timeline/Timeline'
import { Transport } from './Transport'
import { PropertiesPanel } from './PropertiesPanel'

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

/** Commands the editor asks values for in its own dialogs. */
const DIALOG_COMMANDS = ['comp.new', 'comp.settings', 'layer.newSolid', 'layer.settings', 'layer.rename', 'time.set']

export function App({ host, phrase }: { host: HostConnection; phrase: Phrases }) {
  const t = useMemo(() => createKitStrings(CATALOGS, host.init.locale), [host])
  const manifest = useMemo(engineManifest, [])
  const session = useKitSession(host, manifest, describe)
  const editor = useKitEditorState(session, host, t, 'motion', phrase)
  const { state } = session
  const canvas = useRef<KitCanvasHandle>(null)
  const viewMemory = useRef<RasterView | null>(null)
  const [zoom, setZoom] = useState<number | null>(null)
  // Scrolling stops a margin past the picture: the view never leaves it for empty canvas.
  const picture = state?.picture
  const panBounds = useMemo(() => picturePanBounds(picture?.width ?? 0, picture?.height ?? 0, zoom), [picture?.width, picture?.height, zoom])
  const workspace = useRef<HTMLDivElement>(null)
  const views = useMemo(createViewStore, [])
  // Moving through time is not an edit: it goes to the engine without the editor's notices.
  const runQuietly = session.run
  const playback = usePlayback(runQuietly, state?.time ?? 0, state?.comp?.duration ?? 0)
  const movie = useMovieRender(session.doc, host, t)
  const seek = useCallback((seconds: number) => void runQuietly('time.set', { time: Math.max(0, seconds) }).catch(() => undefined), [runQuietly])
  const step = (frames: number) => void runQuietly('time.step', { frames }).catch(() => undefined)
  const { run, query } = editor
  const [dialog, setDialog] = useState<MotionDialog | null>(null)
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
  latestOwn.current = own
  /** A menu row, a ribbon button, or a shortcut: the editor's own handling first, else the engine. */
  const dispatch = useCallback(
    (command: string) => {
      const handler = latestOwn.current[command]
      if (handler) handler()
      else if (ARRANGE[command]) void run('layer.arrange', { to: ARRANGE[command] })
      else void run(command, {})
    },
    [run],
  )
  // Rows with nothing to act on are hidden; the predicate changes only when that does.
  const hiding = hiddenRow(state)
  const hidden = useMemo(() => hiding.hidden, [hiding.key]) // eslint-disable-line react-hooks/exhaustive-deps
  const wholeWorkArea = hiding.key.includes('comp.trimToWorkArea')
  // Home first (its own ribbon; a drop-down panel in Menus), then the editor's menus.
  const tabs = useMemo<FrameTab[]>(
    () => [
      { id: 'home', label: t('homeTab'), ribbon: <HomeTab onCommand={dispatch} wholeWorkArea={wholeWorkArea} /> },
      ...catalogTabs(MENU_ROWS, { phrase, view: VIEW_MENUS, primary: MOTION_PRIMARY, hidden, onCommand: dispatch, onKitCommand: dispatch }),
    ],
    [dispatch, hidden, phrase, t, wholeWorkArea],
  )

  // The timeline's own keys (Delete, Cmd+D, …) come first.
  const timelineKeys: TimelineKeys = useRef(null)
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!state) return false
    if (timelineKeys.current?.(event)) return true
    // The commands the editor asks values for take their menu shortcuts here (Cmd+N, Cmd+K, Cmd+Y…), before the engine would run them without.
    for (const id of DIALOG_COMMANDS) {
      const shortcut = editor.byId.get(id)?.shortcut
      if (shortcut && shortcutMatches(shortcut, event) && (editor.enabled(id) || id === 'comp.new')) {
        if (!host.readOnly || id === 'time.set') dispatch(id)
        return true
      }
    }
    const command = event.metaKey || event.ctrlKey
    if (event.key === ' ' && !command) playback.toggle()
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

  return (
    <KitEditorProvider editor={editor}>
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
        zoom={zoom}
        onKeyDown={keyDown}
        status={
          <span data-testid="motion-status">
            {state?.comp ? t('summary', { name: state.comp.name, width: state.comp.width, height: state.comp.height, fps: Math.round(state.comp.fps * 100) / 100, layers: state.layers.length }) : t('noComp')}
          </span>
        }
        panels={state ? <PropertiesPanel info={state} /> : null}
      >
        <MotionDialogs dialog={dialog} onClose={() => setDialog(null)} onSeek={seek} />
        <div className="kui-row" ref={workspace} data-testid="motion-workspace" data-picture={picture ? `${picture.width} ${picture.height}` : undefined}>
          <div className="motion-main">
            <KitDocumentCanvas
              canvas={canvas}
              viewMemory={viewMemory}
              panBounds={panBounds}
              tool={HAND_TOOL}
              onView={(view) => {
                setZoom(view.zoom)
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
    </KitEditorProvider>
  )
}
