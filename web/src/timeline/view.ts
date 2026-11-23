/** The stretch of the composition the timeline shows, in seconds: its start and its length. */
export type TimeView = { start: number; span: number }

/** How close (CSS pixels) a dragged time comes to the playhead before it snaps to it. */
export const SNAP_PX = 8

/** The shortest stretch the timeline zooms in to, in frames. */
const MIN_FRAMES = 8

/** `view` kept inside the composition: no longer than it, no shorter than a few frames. */
export function clampView(view: TimeView, duration: number, frame: number): TimeView {
  const span = Math.min(duration, Math.max(frame * MIN_FRAMES, view.span))
  const start = Math.min(Math.max(0, duration - span), Math.max(0, view.start))
  return { start, span }
}

/** `view` zoomed by `factor` (above 1 zooms in) with the time `anchor` staying where it is. */
export function zoomView(view: TimeView, factor: number, anchor: number, duration: number, frame: number): TimeView {
  const span = view.span / factor
  return clampView({ start: anchor - ((anchor - view.start) * span) / view.span, span }, duration, frame)
}

/** Where `seconds` is along a track, as a CSS percentage of the track's width (outside 0–100% when out of view). */
export function leftOf(view: TimeView, seconds: number) {
  return `${((seconds - view.start) / view.span) * 100}%`
}

/** The time under `clientX` on a track whose box is `box`. */
export function timeAtX(view: TimeView, clientX: number, box: { left: number; width: number }) {
  return view.start + ((clientX - box.left) / Math.max(1, box.width)) * view.span
}

/** Seconds a horizontal drag of `dx` CSS pixels covers on a track `width` pixels wide. */
export function secondsFor(view: TimeView, dx: number, width: number) {
  return (dx * view.span) / Math.max(1, width)
}

/** `seconds` on a frame, or on the playhead when it is within `SNAP_PX` of it. */
export function snapTime(seconds: number, frame: number, playhead: number, secondsPerPx: number) {
  if (Math.abs(seconds - playhead) <= SNAP_PX * secondsPerPx) return playhead
  return Math.round(seconds / frame) * frame
}

/** The step between labelled ruler ticks: at least `minPx` apart, in frames when zoomed in. */
export function rulerStep(pxPerSecond: number, fps: number, minPx = 64): { seconds: number; frames: boolean } {
  const rate = Math.max(1, Math.round(fps))
  for (const frames of [1, 2, 5, 10, 15]) {
    if (frames < rate && (frames / rate) * pxPerSecond >= minPx) return { seconds: frames / rate, frames: true }
  }
  for (const seconds of [1, 2, 5, 10, 15, 30, 60, 120, 300, 600]) {
    if (seconds * pxPerSecond >= minPx) return { seconds, frames: false }
  }
  return { seconds: 600, frames: false }
}

/** A ruler label: `2s` on whole seconds, `1:15` (second and frame) between them. */
export function rulerLabel(seconds: number, fps: number) {
  const rate = Math.max(1, Math.round(fps))
  const frames = Math.round(seconds * rate)
  const whole = Math.floor(frames / rate)
  const rest = frames % rate
  return rest === 0 ? `${whole}s` : `${whole}:${String(rest).padStart(2, '0')}`
}
