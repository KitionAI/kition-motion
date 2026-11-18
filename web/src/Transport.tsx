import { PauseIcon as Pause, PlayIcon as Play, PlusIcon as Plus, SkipBackIcon as SkipBack, SkipForwardIcon as SkipForward, StepBackIcon as StepBack, StepForwardIcon as StepForward } from '@kition/tessera-icons/react'
import { Button, IconButton, MenuButton, MenuItem, useKitEditor, useReadOnly } from '@kition/kit-editor'
import { timecode, type MotionInfo } from './motion'

/** The layers the add menu makes, as engine commands. */
export const NEW_LAYERS = [
  { id: 'text', command: 'layer.newText', params: { text: 'Text' } },
  { id: 'shape', command: 'layer.newShape', params: { kind: 'rect' } },
  { id: 'solid', command: 'layer.newSolid', params: {} },
  { id: 'null', command: 'layer.newNull', params: {} },
  { id: 'adjustment', command: 'layer.newAdjustment', params: {} },
] as const

/** Add a layer, play, step, and jump, under the composition viewer. */
export function Transport({ info, playing, onPlay, onSeek, onStep }: { info: MotionInfo; playing: boolean; onPlay: () => void; onSeek: (seconds: number) => void; onStep: (frames: number) => void }) {
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
          <MenuItem key={layer.id} label={t(`layerKind.${layer.id}`)} onSelect={() => void run(layer.command, { ...layer.params })} testId={`motion-new-${layer.id}`} />
        ))}
      </MenuButton>
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
      <span style={{ flex: 1 }} />
    </div>
  )
}
