import type { KitEngineInfo, RasterRect } from '@kition/sdk/editor'

/** One animated property of a layer and its key times, in composition seconds. */
type Animated = { path: string; name: string; times: number[] }

/** A layer's switches, as the timeline's columns show them (`layer.setSwitch` names in the comments). */
export type MotionSwitches = {
  video: boolean
  audio: boolean
  solo: boolean
  /** `lock` */
  locked: boolean
  shy: boolean
  /** Collapse Transformations (precomps) or Continuously Rasterize (vector layers). */
  collapse: boolean
  /** On at Best quality, off at Draft (`quality`). */
  quality: boolean
  /** `fx` */
  effects: boolean
  /** On at Frame Mix (`frameBlend`). */
  frameBlend: boolean
  motionBlur: boolean
  adjustment: boolean
  threeD: boolean
}

/** A layer's track matte: the layer whose alpha or luma it shows through, inverted or not. */
export type MotionMatte = { layer: number; kind: 'alpha' | 'alphaInverted' | 'luma' | 'lumaInverted' }

/** The engine's matte labels (`Alpha Inverted Matte`) as `layer.setTrackMatte` takes them. */
function matteKind(label: string | undefined): MotionMatte['kind'] {
  const plain = (label ?? '').toLowerCase()
  const inverted = plain.includes('invert')
  return plain.includes('luma') ? (inverted ? 'lumaInverted' : 'luma') : inverted ? 'alphaInverted' : 'alpha'
}

/** One layer of the composition, top first: `in`, `out`, and `start` in composition seconds. */
export type MotionLayer = {
  id: number
  index: number
  name: string
  type: string
  in: number
  out: number
  start: number
  selected: boolean
  switches: MotionSwitches
  animated: Animated[]
  /** Its blending mode by name (`Normal`, `Screen`), as `layer.setBlendMode` takes it. */
  blendMode: string
  /** Its colour label by the engine's name (`Red`, `Sea Foam`, `None`). */
  label: string
  /** The id of the layer it follows, or null. */
  parent: number | null
  /** The paths of its properties selected in the timeline (a click on a property's name). */
  selectedProps: string[]
  /** Its track matte, or null. */
  matte: MotionMatte | null
}

/** One transform property of the selected layer at the current time. */
export type MotionProperty = { path: string; name: string; type: 'scalar' | 'vec2' | 'vec3' | string; value: number | number[]; animated: boolean }

/** What the editor reads from the engine's `inspect()`. */
export type MotionInfo = {
  /** The picture the canvas shows, in document pixels. Scrolling stops a margin past it. */
  picture: { width: number; height: number }
  comp: {
    id: number
    name: string
    width: number
    height: number
    fps: number
    duration: number
    workArea: [number, number]
    /** The composition's switches: Hide Shy Layers, and whether Motion Blur and Frame Blending render. */
    switches: { hideShy: boolean; motionBlur: boolean; frameBlending: boolean }
  } | null
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
    switches?: { hideShy?: boolean; motionBlur?: boolean; frameBlending?: boolean }
    layers?: Array<{
      id?: number
      index?: number
      name?: string
      type?: string
      in?: number
      out?: number
      start?: number
      selected?: boolean
      switches?: {
        video?: boolean
        audio?: boolean
        solo?: boolean
        locked?: boolean
        shy?: boolean
        collapse?: boolean
        quality?: string
        effects?: boolean
        frame_blend?: string
        motion_blur?: boolean
        adjustment?: boolean
        three_d?: boolean
      }
      trackMatte?: { layer?: number; kind?: string } | null
      animated?: Animated[]
      blendMode?: string
      label?: string
      parent?: number | null
      selectedProps?: string[]
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
          switches: {
            hideShy: inspected.info?.switches?.hideShy === true,
            motionBlur: inspected.info?.switches?.motionBlur !== false,
            frameBlending: inspected.info?.switches?.frameBlending !== false,
          },
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
      switches: {
        video: layer.switches?.video !== false,
        audio: layer.switches?.audio !== false,
        solo: layer.switches?.solo === true,
        locked: layer.switches?.locked === true,
        shy: layer.switches?.shy === true,
        collapse: layer.switches?.collapse === true,
        quality: (layer.switches?.quality ?? 'Best') === 'Best',
        effects: layer.switches?.effects !== false,
        frameBlend: (layer.switches?.frame_blend ?? 'Off') !== 'Off',
        motionBlur: layer.switches?.motion_blur === true,
        adjustment: layer.switches?.adjustment === true,
        threeD: layer.switches?.three_d === true,
      },
      matte: layer.trackMatte?.layer != null ? { layer: layer.trackMatte.layer, kind: matteKind(layer.trackMatte.kind) } : null,
      animated: layer.animated ?? [],
      blendMode: layer.blendMode ?? 'Normal',
      label: layer.label ?? 'None',
      parent: layer.parent ?? null,
      selectedProps: layer.selectedProps ?? [],
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
