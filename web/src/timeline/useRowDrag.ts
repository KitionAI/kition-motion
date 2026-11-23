import { useState, type RefObject } from 'react'

/** Where a dragged row would land: before the `gap`-th layer (the end for the last), and the line's place in the rows box. */
export type RowDrop = { layer: number; gap: number; top: number }

/**
 * Reordering by dragging a row: the gap between layer rows nearest the pointer, shown as a line,
 * and on release the layer's new 0-based stack position (`index` for `layer.arrange`), or null
 * when it would not move.
 */
export function useRowDrag(rows: RefObject<HTMLDivElement | null>, order: readonly number[]) {
  const [drop, setDrop] = useState<RowDrop | null>(null)
  const dropAt = (layer: number, y: number): RowDrop | null => {
    const box = rows.current
    if (!box) return null
    const blocks = [...box.querySelectorAll<HTMLElement>('[data-layer-block]')]
    if (!blocks.length) return null
    const outer = box.getBoundingClientRect()
    // A layer's block is its row and its open properties; the gap is decided on the layer row.
    let gap = blocks.findIndex((block) => {
      const head = block.firstElementChild?.getBoundingClientRect() ?? block.getBoundingClientRect()
      return y < head.top + head.height / 2
    })
    if (gap < 0) gap = blocks.length
    const edge = gap < blocks.length ? blocks[gap].getBoundingClientRect().top : blocks[blocks.length - 1].getBoundingClientRect().bottom
    return { layer, gap, top: edge - outer.top + box.scrollTop }
  }
  const drag = (layer: number, y: number, phase: 'move' | 'end' | 'cancel'): number | null => {
    if (phase === 'move') {
      setDrop(dropAt(layer, y))
      return null
    }
    setDrop(null)
    const landed = phase === 'end' ? dropAt(layer, y) : null
    const from = order.indexOf(layer)
    if (!landed || from < 0) return null
    const to = landed.gap > from ? landed.gap - 1 : landed.gap
    return to === from ? null : to
  }
  return { drop, drag }
}
