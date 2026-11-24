import { useCallback, useState } from 'react'
import type { MotionSwitches } from '../motion'

/**
 * The timeline's columns beside a layer's name: the A/V switches left of it, the layer switches,
 * the modes (blending mode and track matte) and the parent right of it. Each can be shown or
 * hidden from the column header's menu; the choice is kept in this browser.
 */

/** A switch column: which `MotionSwitches` field it shows and the `layer.setSwitch` name it sets. */
export type SwitchColumn = { id: SwitchId; field: keyof MotionSwitches; name: string; side: 'av' | 'switches' }

export type SwitchId = 'video' | 'audio' | 'solo' | 'lock' | 'shy' | 'collapse' | 'quality' | 'effects' | 'frameBlend' | 'motionBlur' | 'adjustment' | 'threeD'
export type ColumnId = SwitchId | 'mode' | 'matte' | 'parent'

export const SWITCH_COLUMNS: readonly SwitchColumn[] = [
  { id: 'video', field: 'video', name: 'video', side: 'av' },
  { id: 'audio', field: 'audio', name: 'audio', side: 'av' },
  { id: 'solo', field: 'solo', name: 'solo', side: 'av' },
  { id: 'lock', field: 'locked', name: 'lock', side: 'av' },
  { id: 'shy', field: 'shy', name: 'shy', side: 'switches' },
  { id: 'collapse', field: 'collapse', name: 'collapse', side: 'switches' },
  { id: 'quality', field: 'quality', name: 'quality', side: 'switches' },
  { id: 'effects', field: 'effects', name: 'fx', side: 'switches' },
  { id: 'frameBlend', field: 'frameBlend', name: 'frameBlend', side: 'switches' },
  { id: 'motionBlur', field: 'motionBlur', name: 'motionBlur', side: 'switches' },
  { id: 'adjustment', field: 'adjustment', name: 'adjustment', side: 'switches' },
  { id: 'threeD', field: 'threeD', name: 'threeD', side: 'switches' },
]

/** Every column in its menu's order. */
export const COLUMN_IDS: readonly ColumnId[] = [...SWITCH_COLUMNS.map((column) => column.id), 'mode', 'matte', 'parent']

/**
 * Shown until the person changes it: the A/V switches (the names column keeps the width it always
 * had); the layer switches, the modes and the parent are a right-click on the column header (or
 * the Columns button) away, and the last three are in the Properties panel too.
 */
const DEFAULT_SHOWN: readonly ColumnId[] = SWITCH_COLUMNS.filter((column) => column.side === 'av').map((column) => column.id)

/** Widths (CSS pixels) of a switch cell and of a drop-down cell; the name column's own width. */
export const SWITCH_WIDTH = 22
export const MENU_WIDTH = 104
export const NAMES_BASE = 232

const KEY = 'kition-motion.timeline-columns'

function stored(): ReadonlySet<ColumnId> {
  try {
    const saved = JSON.parse(globalThis.localStorage?.getItem(KEY) ?? 'null') as unknown
    if (Array.isArray(saved)) return new Set(saved.filter((id): id is ColumnId => COLUMN_IDS.includes(id as ColumnId)))
  } catch {
    // A bad entry: the defaults.
  }
  return new Set(DEFAULT_SHOWN)
}

/** The shown columns and a toggle for one of them, kept in this browser. */
export function useColumns() {
  const [shown, setShown] = useState<ReadonlySet<ColumnId>>(stored)
  const toggle = useCallback((id: ColumnId) => {
    setShown((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      try {
        localStorage.setItem(KEY, JSON.stringify(COLUMN_IDS.filter((each) => next.has(each))))
      } catch {
        // Kept for this page only.
      }
      return next
    })
  }, [])
  return { shown, toggle }
}

/** The names column's width with these columns shown. */
export function namesWidth(shown: ReadonlySet<ColumnId>) {
  const switches = SWITCH_COLUMNS.filter((column) => shown.has(column.id)).length
  const menus = (['mode', 'matte', 'parent'] as const).filter((id) => shown.has(id)).length
  return NAMES_BASE + switches * SWITCH_WIDTH + menus * MENU_WIDTH
}
