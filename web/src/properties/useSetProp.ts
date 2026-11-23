import { useCallback, useRef } from 'react'
import { useKitEditor } from '@kition/sdk/editor'

/**
 * Running `command` with changing parameters: `send(params)` is one undo step; `send(params,
 * gesture)` during a drag or while typing sends only the latest parameters while the engine is
 * busy, and every call of one gesture folds into one undo step (the command's `merge`).
 */
export function useSender(command: string) {
  const { run } = useKitEditor()
  const busy = useRef(false)
  const pending = useRef<{ params: Record<string, unknown>; gesture?: string } | null>(null)
  const flush = useCallback(async () => {
    if (busy.current) return
    busy.current = true
    while (pending.current) {
      const { params, gesture } = pending.current
      pending.current = null
      await run(command, { ...params, ...(gesture ? { merge: gesture } : {}) }).catch(() => null)
    }
    busy.current = false
  }, [command, run])
  return useCallback(
    (params: Record<string, unknown>, gesture?: string) => {
      pending.current = { params, gesture }
      void flush()
    },
    [flush],
  )
}

/** Setting one property of a layer at a composition time (`prop.set`), as `useSender` sends. */
export function useSetProp(layer: number, path: string, time: number) {
  const send = useSender('prop.set')
  return useCallback((value: unknown, gesture?: string) => send({ layer, path, value, time, timeBase: 'comp' }, gesture), [layer, path, send, time])
}
