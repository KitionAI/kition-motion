import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { DownloadIcon as Download, LoaderCircleIcon as Loader2 } from '@kition/tessera/icons/react'
import {
  Button,
  EditorFrame,
  FrameChrome,
  KitDocumentCanvas,
  KitEditorProvider,
  HAND_TOOL,
  commandMenuTabs,
  createKitStrings,
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
import { MOTION_PRIMARY, MENU_ORDER, describe, picturePanBounds } from './motion'
import { CanvasScrollbars, createViewStore } from './CanvasScrollbars'
import { HomeTab } from './HomeTab'
import { Timeline, type TimelineKeys } from './timeline/Timeline'
import { Transport } from './Transport'
import { PropertiesPanel } from './PropertiesPanel'

/** File commands the host's own Save, Open, and Close already cover. */
const HOST_FILE_COMMANDS = ['file.new', 'file.open', 'file.close', 'file.save', 'file.saveAs']
/** Menus kept in a read-only document. */
const VIEW_MENUS = ['View', 'Window', 'Help']

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
  const { commands } = editor
  // Home first (its own ribbon; a drop-down panel in Menus), then the engine's menus.
  const tabs = useMemo<FrameTab[]>(
    () => [{ id: 'home', label: t('homeTab'), ribbon: <HomeTab /> }, ...commandMenuTabs(commands, { order: MENU_ORDER, exclude: HOST_FILE_COMMANDS, phrase, view: VIEW_MENUS, primary: MOTION_PRIMARY })],
    [commands, phrase, t],
  )

  // The timeline's own keys (Delete, Cmd+D, …) come first.
  const timelineKeys: TimelineKeys = useRef(null)
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!state) return false
    if (timelineKeys.current?.(event)) return true
    const command = event.metaKey || event.ctrlKey
    if (event.key === ' ' && !command) playback.toggle()
    else if (event.key === 'PageDown' || (event.key === 'ArrowRight' && command)) step(event.shiftKey ? 10 : 1)
    else if (event.key === 'PageUp' || (event.key === 'ArrowLeft' && command)) step(event.shiftKey ? -10 : -1)
    // Layer ▸ Arrange ▸ Bring Layer Forward / Send Layer Backward (with Shift: to the front / back).
    else if (command && !host.readOnly && (event.key === ']' || event.key === '}')) void editor.run('layer.arrange', { to: event.shiftKey ? 'front' : 'forward' })
    else if (command && !host.readOnly && (event.key === '[' || event.key === '{')) void editor.run('layer.arrange', { to: event.shiftKey ? 'back' : 'backward' })
    else if (event.key === 'Home') seek(0)
    else if (event.key === 'End') seek((state.comp?.duration ?? 0) - state.frame)
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
                <Transport info={state} playing={playback.playing} onPlay={playback.toggle} onSeek={seek} onStep={step} />
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
