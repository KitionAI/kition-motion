import type { ReactNode } from 'react'
import { ArrowLeftIcon as Back, RepeatIcon as Repeat, ChevronRightIcon as Into, PauseIcon as Pause, PlayIcon as Play, PlusIcon as Plus, SkipBackIcon as SkipBack, SkipForwardIcon as SkipForward, StepBackIcon as StepBack, StepForwardIcon as StepForward } from '@kition/tessera/icons/react'
import { Button, IconButton, MenuButton, MenuItem, useEngineQuery, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { timecode, type MotionInfo } from './motion'
import { CompSwitcher } from './CompSwitcher'

/** The layers the add menu makes, as engine commands. */
/** The new layers the editor makes itself (`ownLayer`): a solid asks in a dialog; a camera and a light get Chinese names, the light a sensible kind and place. */
export const OWN_NEW_LAYERS: ReadonlySet<string> = new Set(['layer.newText', 'layer.newSolid', 'layer.newCamera', 'layer.newLight', 'layer.newShape', 'layer.newNull', 'layer.newAdjustment'])
export const NEW_LAYERS = [
  { id: 'text', command: 'layer.newText', params: { text: 'Text' } },
  { id: 'shape', command: 'layer.newShape', params: { kind: 'rect' } },
  { id: 'solid', command: 'layer.newSolid', params: {} },
  { id: 'null', command: 'layer.newNull', params: {} },
  { id: 'adjustment', command: 'layer.newAdjustment', params: {} },
  { id: 'camera', command: 'layer.newCamera', params: {} },
  { id: 'light', command: 'layer.newLight', params: {} },
] as const

/**
 * The compositions the person went into from precomp layers, outermost first, each a button back
 * to it, with a Back button to the one just outside; the drop-down beside them is the one open now.
 */
function CompPath({ trail, onBack }: { trail: readonly number[]; onBack: (comp: number) => void }) {
  const { t } = useKitEditor()
  const summary = useEngineQuery<{ items?: Array<{ id: number; name: string }> }>('project.summary', {})
  const name = (id: number) => summary?.items?.find((item) => item.id === id)?.name ?? String(id)
  const parent = trail[trail.length - 1]
  if (parent === undefined) return null
  return (
    <nav className="motion-comp-path" aria-label={t('compPath')} data-testid="motion-comp-path">
      <IconButton icon={Back} label={t('compPath.back', { name: name(parent) })} onClick={() => onBack(parent)} data-testid="motion-comp-back" />
      {trail.map((comp) => (
        <span key={comp} className="motion-comp-crumb">
          <Button variant="ghost" onClick={() => onBack(comp)} data-testid={`motion-comp-crumb-${comp}`}>
            {name(comp)}
          </Button>
          <Into size={12} aria-hidden="true" className="kui-muted" />
        </span>
      ))}
    </nav>
  )
}

/** Add a layer, play, step, and jump, under the composition viewer; the way back out of a precomp before the composition's name. */
export function Transport({
  info,
  playing,
  onPlay,
  onSeek,
  onStep,
  onCommand,
  trail,
  onBack,
  playbackRate,
  loop,
  meters,
}: {
  info: MotionInfo
  playing: boolean
  onPlay: () => void
  onSeek: (seconds: number) => void
  onStep: (frames: number) => void
  onCommand: (command: string) => void
  trail: readonly number[]
  onBack: (comp: number) => void
  /** The frames per second playback shows (null before it has measured), and Skip Frames to Keep Real Time. */
  playbackRate?: { fps: number | null; skip: boolean; onSkip: () => void }
  /** Loop: playback goes round the work area (on by default). */
  loop?: { on: boolean; onToggle: () => void }
  /** The audio meters, shown while it plays. */
  meters?: ReactNode
}) {
  const { run, t } = useKitEditor()
  const readOnly = useReadOnly()
  const fps = info.comp?.fps ?? 30
  const end = Math.max(0, (info.comp?.duration ?? 0) - info.frame)
  return (
    // A size container: as the bar narrows (the Project and Properties panels open beside it), the
    // meters and the frame rate give up their room first, then the labels, never the controls
    // (app.css, `@container motion-transport`).
    <div className="kui-bar motion-transport" data-testid="motion-transport">
      <MenuButton
        placement="auto"
        trigger={({ toggle }) => (
          <Button variant="ghost" onClick={toggle} disabled={readOnly} aria-label={t('newLayer')} title={t('newLayer')} data-testid="motion-new-layer">
            <Plus size={14} aria-hidden="true" />
            <span className="motion-transport-label">{t('newLayer')}</span>
          </Button>
        )}
      >
        {NEW_LAYERS.map((layer) => (
          <MenuItem key={layer.id} label={t(`layerKind.${layer.id}`)} onSelect={() => (OWN_NEW_LAYERS.has(layer.command) ? onCommand(layer.command) : void run(layer.command, { ...layer.params }))} testId={`motion-new-${layer.id}`} />
        ))}
      </MenuButton>
      <CompPath trail={trail} onBack={onBack} />
      <CompSwitcher info={info} />
      <span className="motion-transport-lead" />
      <IconButton icon={SkipBack} label={t('goToStart')} onClick={() => onSeek(0)} />
      <IconButton icon={StepBack} label={t('stepBack')} onClick={() => onStep(-1)} />
      <Button variant="primary" onClick={onPlay} aria-label={playing ? t('pause') : t('play')} title={`${playing ? t('pause') : t('play')} (Space)`} data-testid="motion-play">
        {playing ? <Pause size={15} aria-hidden="true" /> : <Play size={15} aria-hidden="true" />}
      </Button>
      <IconButton icon={StepForward} label={t('stepForward')} onClick={() => onStep(1)} />
      <IconButton icon={SkipForward} label={t('goToEnd')} onClick={() => onSeek(end)} />
      {/* The time and the duration on one line: they never wrap however narrow the bar. */}
      <span className="motion-timecodes">
        <span className="motion-timecode" data-testid="motion-timecode">
          {timecode(info.time, fps)}
        </span>
        <span className="motion-timecode motion-duration kui-muted" data-testid="motion-duration">
          / {timecode(info.comp?.duration ?? 0, fps)}
        </span>
      </span>
      {loop ? (
        <Button variant="ghost" className="motion-toggle" aria-pressed={loop.on} aria-label={t('loop.label')} title={loop.on ? t('loop.on') : t('loop.off')} onClick={loop.onToggle} data-testid="motion-loop">
          <Repeat size={14} aria-hidden="true" />
          <span className="motion-transport-label">{t('loop.label')}</span>
        </Button>
      ) : null}
      {meters}
      {playbackRate ? (
        <>
          <Button
            variant="ghost"
            className="motion-toggle"
            aria-pressed={playbackRate.skip}
            title={playbackRate.skip ? t('skipFrames.on') : t('skipFrames.off')}
            onClick={playbackRate.onSkip}
            data-testid="motion-skip-frames"
          >
            {t('skipFrames.label')}
          </Button>
          {/* Its room is kept while stopped, so the buttons before it do not move when it shows. */}
          <span className="motion-fps kui-muted" role="status" data-testid="motion-fps" data-fps={playing && playbackRate.fps !== null ? playbackRate.fps.toFixed(2) : undefined} data-shown={playing && playbackRate.fps !== null ? true : undefined}>
            {playing && playbackRate.fps !== null ? t('fps.shown', { fps: playbackRate.fps.toFixed(1), of: Math.round(fps * 100) / 100 }) : ''}
          </span>
        </>
      ) : null}
      <span className="motion-transport-tail" />
    </div>
  )
}
