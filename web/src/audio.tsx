import { useEffect, useRef } from 'react'
import { useKitEditor } from '@kition/sdk/editor'

/** How much sound (seconds) preview playback asks the engine for at a time, and how far ahead it keeps. */
const AUDIO_CHUNK = 0.5
const AUDIO_AHEAD = 1
/** The stretch of sound each meter reading covers, in seconds. */
const METER_WINDOW = 1 / 30
/** How long a peak's hold mark stays before it falls, in seconds. */
const HOLD_SECONDS = 1.5

/** The loudest sample of each channel over a stretch of composition time. */
type Reading = { time: number; left: number; right: number }

/**
 * What the meters read while the composition plays: the mixed sound's peaks by composition time,
 * written by `usePreviewAudio` as it mixes, read by `AudioMeters` at the playhead.
 */
export type MeterFeed = { readings: Reading[]; at: () => number | null }

export function createMeterFeed(): MeterFeed {
  return { readings: [], at: () => null }
}

type Doc = { invoke: <T>(name: string, ...args: unknown[]) => Promise<T> }

/**
 * The composition's sound while it plays: the engine mixes it (`mixAudio`, every audible layer
 * with its levels, mute and solo) half a second at a time, a little ahead, and the page plays it
 * from the time playback started, against the same clock. An edit while it plays (a layer muted,
 * soloed, its levels changed) is heard at once: what was mixed ahead is dropped and mixed again
 * from where playback is. Nothing is asked for again once the engine answers that nothing is
 * audible. `scheduled` counts the stretches handed to the speakers (the footage test reads it from
 * the workspace's `data-audio`); `feed` gets each stretch's peaks for the meters.
 */
export function usePreviewAudio(doc: Doc | null, playing: boolean, from: number | null, duration: number, scheduled: (count: number) => void, round: number, version: unknown, feed: MeterFeed) {
  const edits = useRef(0)
  const seen = useRef(version)
  if (seen.current !== version) {
    seen.current = version
    edits.current += 1
  }
  useEffect(() => {
    feed.readings = []
    if (!playing || !doc || from === null || typeof AudioContext === 'undefined') return undefined
    const context = new AudioContext()
    const started = context.currentTime
    const wall = performance.now()
    let stopped = false
    let mixed = 0
    let count = 0
    let version = edits.current
    const sources: Array<{ source: AudioBufferSourceNode; at: number }> = []
    // Where playback is, in seconds from `from`, by the wall clock (the speakers' clock may not run without a device).
    const elapsed = () => (performance.now() - wall) / 1000
    const pump = async () => {
      while (!stopped && from + mixed < duration - 1e-6) {
        if (version !== edits.current) {
          // An edit: what was mixed ahead goes, and the sound is mixed again from here.
          version = edits.current
          const now = Math.min(elapsed(), Math.max(0, duration - from))
          for (const { source, at } of sources.splice(0)) if (at + AUDIO_CHUNK > context.currentTime) source.stop()
          feed.readings = feed.readings.filter((reading) => reading.time < from + now)
          mixed = now
        }
        if (mixed - elapsed() > AUDIO_AHEAD) {
          await new Promise((resolve) => window.setTimeout(resolve, 100))
          continue
        }
        const length = Math.min(AUDIO_CHUNK, duration - (from + mixed))
        const samples = await doc.invoke<Float32Array>('mixAudio', from + mixed, length, context.sampleRate).catch(() => null)
        if (stopped || version !== edits.current) continue
        if (!samples?.length) return
        const frames = Math.floor(samples.length / 2)
        const buffer = context.createBuffer(2, frames, context.sampleRate)
        const left = buffer.getChannelData(0)
        const right = buffer.getChannelData(1)
        const span = Math.max(1, Math.round(METER_WINDOW * context.sampleRate))
        for (let start = 0; start < frames; start += span) {
          let l = 0
          let r = 0
          for (let index = start; index < Math.min(frames, start + span); index += 1) {
            l = Math.max(l, Math.abs(samples[index * 2]))
            r = Math.max(r, Math.abs(samples[index * 2 + 1]))
          }
          feed.readings.push({ time: from + mixed + start / context.sampleRate, left: l, right: r })
        }
        for (let index = 0; index < frames; index += 1) {
          left[index] = samples[index * 2]
          right[index] = samples[index * 2 + 1]
        }
        const source = context.createBufferSource()
        source.buffer = buffer
        source.connect(context.destination)
        // A stretch that arrives late starts part of the way in, so the sound keeps to the picture.
        const at = started + mixed
        const late = Math.max(0, context.currentTime - at)
        if (late < length) source.start(Math.max(at, context.currentTime), late)
        sources.push({ source, at })
        mixed += length
        count += 1
        scheduled(count)
      }
    }
    void pump()
    return () => {
      stopped = true
      feed.readings = []
      void context.close()
    }
  }, [doc, playing, from, duration, scheduled, round, feed])
}

/** A level (0 to 1) in decibels, floored at -60. */
const decibels = (level: number) => (level > 0.001 ? Math.max(-60, 20 * Math.log10(level)) : -60)
/** Where a level sits on a meter (0 to 1, -60 dB to 0 dB). */
const reach = (level: number) => (decibels(level) + 60) / 60

/**
 * The transport's audio meters while it plays (their place kept, hidden, while it is stopped): the left and right channels' peaks at the playhead
 * (-60 dB to 0 dB), each with a hold mark at the loudest peak of the last moment and a red end
 * when it clips. Drawn each frame from `feed` without the editor rendering again; its `data-left`,
 * `data-right` (linear peaks) and `data-hold-*` say what it shows.
 */
export function AudioMeters({ feed, playing }: { feed: MeterFeed; playing: boolean }) {
  const { t } = useKitEditor()
  const box = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!playing) return undefined
    const holds = [
      { level: 0, until: 0 },
      { level: 0, until: 0 },
    ]
    let frame = 0
    const draw = () => {
      frame = requestAnimationFrame(draw)
      const element = box.current
      const time = feed.at()
      if (!element || time === null) return
      // The reading at the playhead: the last one at or before it.
      let reading: Reading | undefined
      for (let index = feed.readings.length - 1; index >= 0; index -= 1) {
        if (feed.readings[index].time <= time + 1e-6) {
          reading = feed.readings[index]
          break
        }
      }
      const levels = reading && time - reading.time < 0.25 ? [reading.left, reading.right] : [0, 0]
      const now = performance.now() / 1000
      levels.forEach((level, channel) => {
        const hold = holds[channel]
        if (level >= hold.level || now > hold.until) {
          hold.level = level
          hold.until = now + HOLD_SECONDS
        }
        const bar = element.children[channel] as HTMLElement | undefined
        bar?.style.setProperty('--level', String(reach(level)))
        bar?.style.setProperty('--hold', String(reach(hold.level)))
        bar?.toggleAttribute('data-clip', hold.level >= 0.999)
      })
      element.dataset.left = levels[0].toFixed(3)
      element.dataset.right = levels[1].toFixed(3)
      element.dataset.holdLeft = holds[0].level.toFixed(3)
      element.dataset.holdRight = holds[1].level.toFixed(3)
    }
    frame = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(frame)
  }, [playing, feed])
  // Its place is kept while stopped (hidden), so the transport's buttons never move when it plays.
  return (
    <div ref={box} className="motion-meters" role="meter" aria-label={t('meter.label')} aria-hidden={!playing || undefined} title={t('meter.label')} data-idle={!playing || undefined} data-testid="motion-meters">
      <span className="motion-meter" data-channel="L" />
      <span className="motion-meter" data-channel="R" />
    </div>
  )
}
