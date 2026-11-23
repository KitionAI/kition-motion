import type { MotionLayer } from '../motion'

/** One keyframe: its layer, its property's path, and its time in composition seconds. */
export type KeyRef = { layer: number; path: string; time: number }

/** A keyframe's identity, the same for a ref drawn and a ref the engine reports selected. */
export function keyId(key: KeyRef) {
  return `${key.layer}|${key.path}|${Math.round(key.time * 1e6)}`
}

/** A property's keyframe times in composition seconds, as the editor's state reports them. */
export function keyTimes(layer: MotionLayer, path: string): number[] {
  return layer.animated.find((property) => property.path === path)?.times ?? []
}

/** A collapsed layer's keyframes: one mark per time, standing for every property's keyframe there. */
export function layerKeyMarks(layer: MotionLayer): Array<{ time: number; keys: KeyRef[] }> {
  const byTime = new Map<number, KeyRef[]>()
  for (const property of layer.animated) {
    for (const time of property.times) {
      const at = Math.round(time * 1e6)
      const list = byTime.get(at) ?? []
      list.push({ layer: layer.id, path: property.path, time })
      byTime.set(at, list)
    }
  }
  return [...byTime.values()].map((keys) => ({ time: keys[0].time, keys })).sort((a, b) => a.time - b.time)
}

/** The keys a mark element stands for (its `data-keys`). */
export function keysOf(element: Element): KeyRef[] {
  try {
    return JSON.parse(element.getAttribute('data-keys') ?? '[]') as KeyRef[]
  } catch {
    return []
  }
}

/** What `keys.info` answers per selected keyframe (the fields the timeline reads). */
export type SelectedKey = { layer: number; path: string; compTime: number }
