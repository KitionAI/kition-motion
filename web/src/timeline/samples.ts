import { createContext, useContext } from 'react'

/**
 * One property's keyframed values at `count` evenly spaced composition times from `from` to `to`
 * seconds (the engine's `curveSamples`): what the graph editor draws. Null without a document.
 */
export type CurveSampler = (layer: number, path: string, from: number, to: number, count: number) => Promise<unknown[] | null>

export const SamplerContext = createContext<CurveSampler>(async () => null)

export function useSampler() {
  return useContext(SamplerContext)
}
