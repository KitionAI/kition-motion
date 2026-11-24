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

/** A layer's sound as peaks (0 to 1) from `from` to `to` composition seconds (the engine's `layerWaveform`); null without a document. */
export type Waveform = { from: number; to: number; peaks: number[] }
export type WaveformReader = (layer: number, count: number) => Promise<Waveform | null>

export const WaveformContext = createContext<WaveformReader>(async () => null)

export function useWaveformReader() {
  return useContext(WaveformContext)
}
