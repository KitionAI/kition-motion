import type { KitEngineInfo } from '@kition/sdk/editor'

/** One animated property of a layer and its key times, in composition seconds. */
type Animated = { path: string; name: string; times: number[] }

/** One layer of the composition, top first. */
export type MotionLayer = { id: number; index: number; name: string; type: string; in: number; out: number; selected: boolean; animated: Animated[] }

/** One transform property of the selected layer at the current time. */
export type MotionProperty = { path: string; name: string; type: 'scalar' | 'vec2' | 'vec3' | string; value: number | number[]; animated: boolean }

/** What the editor reads from the engine's `inspect()`. */
export type MotionInfo = {
  comp: { id: number; name: string; width: number; height: number; fps: number; duration: number } | null
  time: number
  /** One frame, in seconds. */
  frame: number
  layers: MotionLayer[]
  /** The first selected layer's transform, for the Properties panel. */
  transform: { layer: number; properties: MotionProperty[] } | null
}

type Inspected = {
  comp?: { id?: number; name?: string; width?: number; height?: number; frameRate?: number; duration?: number } | null
  info?: {
    layers?: Array<{ id?: number; index?: number; name?: string; type?: string; in?: number; out?: number; selected?: boolean; animated?: Animated[] }>
    transform?: { layer?: number; properties?: MotionProperty[] }
  } | null
  time?: number
}

export function describe(info: KitEngineInfo): MotionInfo {
  const inspected = info.inspected as Inspected
  const comp = inspected.comp
  const fps = comp?.frameRate || 30
  return {
    comp: comp ? { id: comp.id ?? 0, name: comp.name ?? '', width: comp.width ?? info.width, height: comp.height ?? info.height, fps, duration: comp.duration ?? 0 } : null,
    time: inspected.time ?? 0,
    frame: 1 / fps,
    layers: (inspected.info?.layers ?? []).map((layer) => ({
      id: layer.id ?? 0,
      index: layer.index ?? 0,
      name: layer.name ?? '',
      type: layer.type ?? '',
      in: layer.in ?? 0,
      out: layer.out ?? 0,
      selected: layer.selected === true,
      animated: layer.animated ?? [],
    })),
    transform: inspected.info?.transform?.layer != null ? { layer: inspected.info.transform.layer, properties: inspected.info.transform.properties ?? [] } : null,
  }
}

/** `seconds` as `0:00:01:12` at `fps` frames per second. */
export function timecode(seconds: number, fps: number) {
  const rate = Math.max(1, Math.round(fps))
  const frames = Math.max(0, Math.round(seconds * fps))
  const pad = (value: number) => String(value).padStart(2, '0')
  const total = Math.floor(frames / rate)
  return `${Math.floor(total / 3600)}:${pad(Math.floor(total / 60) % 60)}:${pad(total % 60)}:${pad(frames % rate)}`
}

export const MENU_ORDER = ['File', 'Edit', 'Composition', 'Layer', 'Effect', 'Animation', 'View', 'Window', 'Help']

/** The engine commands each menu tab shows in Single line's row (After Effects' most-used), by menu. */
export const MOTION_PRIMARY: Record<string, readonly string[]> = {
  File: ['file.exportLottie', 'file.projectSettings', 'file.newCompFromSelection'],
  Edit: ['edit.undo', 'edit.redo', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.duplicate', 'edit.splitLayer', 'edit.clear'],
  Composition: ['comp.new', 'comp.settings', 'renderQueue.add', 'comp.trimToWorkArea', 'playback.toggle'],
  Layer: ['layer.newText', 'layer.newSolid', 'layer.newShape', 'layer.newNull', 'layer.newAdjustment', 'layer.precompose', 'layer.addMask', 'layer.settings'],
  Effect: ['effect.manage', 'effect.applyLast', 'effect.removeAll'],
  Animation: ['anim.addKeyframe', 'keys.easyEase', 'keys.toggleHold', 'anim.applyPreset', 'track.motion', 'prop.setExpression'],
  View: ['view.zoomIn', 'view.zoomOut', 'view.grid', 'view.guides', 'view.rulers', 'view.res.full', 'view.res.half'],
}
