import { useEffect, useRef, type KeyboardEvent, type RefObject } from 'react'

/** One focus session of a field: the texts it held, from the one it showed when it took the focus, and where Undo stands in them. */
type Session = { texts: string[]; at: number }

/**
 * A text field's own Undo and Redo while it has the focus (Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z,
 * Ctrl+Y, and the browser's own Undo and Redo of the field): they step through what was typed
 * since the field took the focus, one change at a time, down to the text it showed then, and stop
 * there. The browser's own undo history is never used: a field the editor reuses for another
 * layer (the Properties panel's) would otherwise undo into that layer's typing, and it can bring
 * back a value from before the last change was saved.
 *
 * `element` is the field. `begin` on focus (with the text shown), `record` on each change, `key`
 * first in `onKeyDown` (true when it was one of these keys), `end` on blur. `apply` puts a text
 * back in the field as if it had been typed.
 */
export function useTypingUndo(apply: (text: string) => void, element: RefObject<HTMLElement | null>) {
  const session = useRef<Session | null>(null)
  const applyRef = useRef(apply)
  applyRef.current = apply
  // A key just handled: the browser's undo it may still start (a `beforeinput` of `historyUndo`) is only stopped.
  const keyed = useRef(false)
  const step = (by: -1 | 1) => {
    const now = session.current
    if (!now) return
    const at = Math.min(now.texts.length - 1, Math.max(0, now.at + by))
    if (at === now.at) return
    now.at = at
    applyRef.current(now.texts[at] ?? '')
  }
  const stepRef = useRef(step)
  stepRef.current = step
  useEffect(() => {
    const field = element.current
    if (!field) return undefined
    const native = (event: InputEvent) => {
      if (event.inputType !== 'historyUndo' && event.inputType !== 'historyRedo') return
      event.preventDefault()
      if (keyed.current) {
        keyed.current = false
        return
      }
      stepRef.current(event.inputType === 'historyUndo' ? -1 : 1)
    }
    field.addEventListener('beforeinput', native)
    return () => field.removeEventListener('beforeinput', native)
  }, [element])
  return {
    begin(text: string) {
      session.current = { texts: [text], at: 0 }
    },
    record(text: string) {
      const now = session.current
      if (!now || now.texts[now.at] === text) return
      now.texts = [...now.texts.slice(0, now.at + 1), text]
      now.at = now.texts.length - 1
    },
    end() {
      session.current = null
    },
    key(event: KeyboardEvent<HTMLElement>): boolean {
      const by = undoStep(event)
      if (by === 0) return false
      // Never the browser's own history, and never the editor's Undo while typing here.
      event.preventDefault()
      event.stopPropagation()
      keyed.current = true
      window.setTimeout(() => (keyed.current = false), 0)
      step(by)
      return true
    },
  }
}

/** -1 for Undo (Cmd/Ctrl+Z), 1 for Redo (Cmd/Ctrl+Shift+Z, Ctrl+Y), 0 for any other key. */
function undoStep(event: KeyboardEvent<HTMLElement>): -1 | 0 | 1 {
  if (event.altKey || !(event.metaKey || event.ctrlKey) || event.nativeEvent.isComposing) return 0
  const code = event.code
  const key = event.key.toLowerCase()
  if (code === 'KeyZ' || key === 'z') return event.shiftKey ? 1 : -1
  if ((code === 'KeyY' || key === 'y') && event.ctrlKey && !event.metaKey && !event.shiftKey) return 1
  return 0
}
