import { ArrowLeftIcon as Back, ChevronRightIcon as Into, PauseIcon as Pause, PlayIcon as Play, PlusIcon as Plus, SkipBackIcon as SkipBack, SkipForwardIcon as SkipForward, StepBackIcon as StepBack, StepForwardIcon as StepForward } from '@kition/tessera/icons/react'
import { Button, IconButton, MenuButton, MenuItem, useEngineQuery, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { timecode, type MotionInfo } from './motion'
import { CompSwitcher } from './CompSwitcher'

/** The layers the add menu makes, as engine commands. */
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
}) {
  const { run, t } = useKitEditor()
  const readOnly = useReadOnly()
  const fps = info.comp?.fps ?? 30
  const end = Math.max(0, (info.comp?.duration ?? 0) - info.frame)
  return (
    <div className="kui-bar motion-transport" data-testid="motion-transport">
      <MenuButton
        trigger={({ toggle }) => (
          <Button variant="ghost" onClick={toggle} disabled={readOnly} data-testid="motion-new-layer">
            <Plus size={14} aria-hidden="true" />
            {t('newLayer')}
          </Button>
        )}
      >
        {NEW_LAYERS.map((layer) => (
          <MenuItem key={layer.id} label={t(`layerKind.${layer.id}`)} onSelect={() => (layer.command === 'layer.newSolid' ? onCommand(layer.command) : void run(layer.command, { ...layer.params }))} testId={`motion-new-${layer.id}`} />
        ))}
      </MenuButton>
      <CompPath trail={trail} onBack={onBack} />
      <CompSwitcher info={info} />
      <span style={{ flex: 1 }} />
      <IconButton icon={SkipBack} label={t('goToStart')} onClick={() => onSeek(0)} />
      <IconButton icon={StepBack} label={t('stepBack')} onClick={() => onStep(-1)} />
      <Button variant="primary" onClick={onPlay} aria-label={playing ? t('pause') : t('play')} title={`${playing ? t('pause') : t('play')} (Space)`} data-testid="motion-play">
        {playing ? <Pause size={15} aria-hidden="true" /> : <Play size={15} aria-hidden="true" />}
      </Button>
      <IconButton icon={StepForward} label={t('stepForward')} onClick={() => onStep(1)} />
      <IconButton icon={SkipForward} label={t('goToEnd')} onClick={() => onSeek(end)} />
      <span className="motion-timecode" data-testid="motion-timecode">
        {timecode(info.time, fps)}
      </span>
      <span className="motion-timecode kui-muted">/ {timecode(info.comp?.duration ?? 0, fps)}</span>
      {playbackRate ? (
        <>
          <Button
            variant="ghost"
            aria-pressed={playbackRate.skip}
            title={playbackRate.skip ? t('skipFrames.on') : t('skipFrames.off')}
            onClick={playbackRate.onSkip}
            data-testid="motion-skip-frames"
          >
            {t('skipFrames.label')}
          </Button>
          {playing && playbackRate.fps !== null ? (
            <span className="motion-fps kui-muted" role="status" data-testid="motion-fps" data-fps={playbackRate.fps.toFixed(2)}>
              {t('fps.shown', { fps: playbackRate.fps.toFixed(1), of: Math.round(fps * 100) / 100 })}
            </span>
          ) : null}
        </>
      ) : null}
      <span style={{ flex: 1 }} />
    </div>
  )
}
