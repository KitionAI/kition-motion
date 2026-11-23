import { createContext, useContext } from 'react'
import type { MotionInfo } from '../motion'
import { snapTime, type TimeView } from './view'

/** What every part of the timeline reads: the composition, the stretch in view, and the track's width. */
export type TimelineFrame = {
  info: MotionInfo
  view: TimeView
  /** The tracks' width in CSS pixels (the ruler's), for snapping and ticks. */
  width: number
  readOnly: boolean
}

export const TimelineContext = createContext<TimelineFrame | null>(null)

export function useTimeline(): TimelineFrame {
  const frame = useContext(TimelineContext)
  if (!frame) throw new Error('useTimeline outside the Timeline')
  return frame
}

/** `seconds` snapped as a drag on the timeline snaps it: to a frame, or to the playhead within a few pixels. */
export function snapIn(frame: TimelineFrame, seconds: number) {
  return snapTime(seconds, frame.info.frame, frame.info.time, frame.view.span / Math.max(1, frame.width))
}
