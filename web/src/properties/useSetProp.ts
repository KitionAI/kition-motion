import { useCallback, useRef } from 'react'
import { useKitEditor } from '@kition/sdk/editor'

/**
 * Setting one property of a layer at a composition time. `set(value)` is one undo step;
 * `set(value, gesture)` during a drag sends only the latest value while the engine is busy, and
 * every value of one gesture folds into one undo step (`prop.set`'s `merge`).
 */
export function useSetProp(layer: number, path: string, time: number) {
  const { run } = useKitEditor()
  const latest = useRef({ layer, path, time })
  latest.current = { layer, path, time }
  const busy = useRef(false)
  const pending = useRef<{ value: unknown; gesture?: string } | null>(null)
  const flush = useCallback(async () => {
    if (busy.current) return
    busy.current = true
    while (pending.current) {
      const { value, gesture } = pending.current
      pending.current = null
      const { layer: id, path: at, time: when } = latest.current
      await run('prop.set', { layer: id, path: at, value, time: when, timeBase: 'comp', ...(gesture ? { merge: gesture } : {}) }).catch(() => null)
    }
    busy.current = false
  }, [run])
  return useCallback(
    (value: unknown, gesture?: string) => {
      pending.current = { value, gesture }
      void flush()
    },
    [flush],
  )
}
