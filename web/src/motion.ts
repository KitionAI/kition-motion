import type { KitEngineInfo, RasterRect } from '@kition/sdk/editor'

/** One animated property of a layer and its key times, in composition seconds. */
type Animated = { path: string; name: string; times: number[] }

/** A layer's switches the timeline shows: visible (video), solo, locked, and 3D. */
export type MotionSwitches = { video: boolean; solo: boolean; locked: boolean; threeD: boolean }

/** One layer of the composition, top first: `in`, `out`, and `start` in composition seconds. */
export type MotionLayer = { id: number; index: number; name: string; type: string; in: number; out: number; start: number; selected: boolean; switches: MotionSwitches; animated: Animated[] }

/** One transform property of the selected layer at the current time. */
export type MotionProperty = { path: string; name: string; type: 'scalar' | 'vec2' | 'vec3' | string; value: number | number[]; animated: boolean }

/** What the editor reads from the engine's `inspect()`. */
export type MotionInfo = {
  /** The picture the canvas shows, in document pixels. Scrolling stops a margin past it. */
  picture: { width: number; height: number }
  comp: { id: number; name: string; width: number; height: number; fps: number; duration: number; workArea: [number, number] } | null
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
    workArea?: [number, number]
    layers?: Array<{
      id?: number
      index?: number
      name?: string
      type?: string
      in?: number
      out?: number
      start?: number
      selected?: boolean
      switches?: { video?: boolean; solo?: boolean; locked?: boolean; three_d?: boolean }
      animated?: Animated[]
    }>
    transform?: { layer?: number; properties?: MotionProperty[] }
  } | null
  time?: number
}

export function describe(info: KitEngineInfo): MotionInfo {
  const inspected = info.inspected as Inspected
  const comp = inspected.comp
  const fps = comp?.frameRate || 30
  return {
    picture: { width: info.width, height: info.height },
    comp: comp
      ? {
          id: comp.id ?? 0,
          name: comp.name ?? '',
          width: comp.width ?? info.width,
          height: comp.height ?? info.height,
          fps,
          duration: comp.duration ?? 0,
          workArea: inspected.info?.workArea ?? [0, comp.duration ?? 0],
        }
      : null,
    time: inspected.time ?? 0,
    frame: 1 / fps,
    layers: (inspected.info?.layers ?? []).map((layer) => ({
      id: layer.id ?? 0,
      index: layer.index ?? 0,
      name: layer.name ?? '',
      type: layer.type ?? '',
      in: layer.in ?? 0,
      out: layer.out ?? 0,
      start: layer.start ?? 0,
      selected: layer.selected === true,
      switches: { video: layer.switches?.video !== false, solo: layer.switches?.solo === true, locked: layer.switches?.locked === true, threeD: layer.switches?.three_d === true },
      animated: layer.animated ?? [],
    })),
    transform: inspected.info?.transform?.layer != null ? { layer: inspected.info.transform.layer, properties: inspected.info.transform.properties ?? [] } : null,
  }
}

/** How far past the picture's edge the view scrolls, in screen pixels: the margin a fitted picture has, so a fitted picture does not scroll. */
export const SCROLL_MARGIN = 32

/**
 * Where scrolling stops: the picture and `SCROLL_MARGIN` screen pixels of canvas around it, in
 * document pixels at `zoom`. A view larger than that keeps the picture centred.
 */
export function picturePanBounds(width: number, height: number, zoom: number | null): RasterRect | null {
  if (!zoom || !(width > 0) || !(height > 0)) return null
  const margin = SCROLL_MARGIN / zoom
  return { x0: -margin, y0: -margin, x1: width + margin, y1: height + margin }
}

/** `seconds` as `0:00:01:12` at `fps` frames per second. */
export function timecode(seconds: number, fps: number) {
  const rate = Math.max(1, Math.round(fps))
  const frames = Math.max(0, Math.round(seconds * fps))
  const pad = (value: number) => String(value).padStart(2, '0')
  const total = Math.floor(frames / rate)
  return `${Math.floor(total / 3600)}:${pad(Math.floor(total / 60) % 60)}:${pad(total % 60)}:${pad(frames % rate)}`
}
