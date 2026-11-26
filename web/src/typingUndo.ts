import { useEffect, useRef, type KeyboardEvent, type RefObject } from 'react'
import { useKitEditor } from '@kition/sdk/editor'

/**
 * One focus session of a field: the texts it held, from the one it showed when it took the focus,
 * and where Undo stands in them. `stale`: the editor's Undo or Redo ran from the field since, so
 * the texts are of a document that may have changed; the next key or typing starts again from
 * what the field shows then.
 */
type Session = { texts: string[]; at: number; stale: boolean }

export type TypingUndoOptions = {
  /**
   * Called just before Cmd/Ctrl+Z (or Redo) falls through to the editor's Undo (or Redo): the
   * field shows the document's value from then on (it has nothing typed to keep) and commits
   * nothing it showed before.
   */
  leave?: () => void
  /**
   * Falls through only when nothing at all was typed in this visit, not when the typing was
   * undone again: for a field whose typing is sent to the document as it is typed (one step per
   * visit), where that step is still on top of the history and its last send may be on its way.
   */
  untouchedOnly?: boolean
}

/** The field's text as it shows it now. */
function shown(field: HTMLElement | null): string | null {
  if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) return field.value
  return null
}

/**
 * A text field's own Undo and Redo while it has the focus (Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z,
 * Ctrl+Y, and the browser's own Undo and Redo of the field): they step through what was typed
 * since the field took the focus, one change at a time, down to the text it showed then. The
 * browser's own undo history is never used: a field the editor reuses for another layer (the
 * Properties panel's) would otherwise undo into that layer's typing, and it can bring back a value
 * from before the last change was saved.
 *
 * With nothing left to undo in the field (it shows the text it took the focus with), Cmd/Ctrl+Z
 * is the editor's Undo; with nothing typed and nothing to redo, Redo is the editor's Redo. The
 * field first stops keeping its own text (`options.leave`), so it shows what the document holds
 * after the step and commits nothing older; its history then starts again from that text.
 *
 * `element` is the field. `begin` on focus (with the text shown), `record` on each change, `key`
 * first in `onKeyDown` (true when it was one of these keys), `end` on blur. `apply` puts a text
 * back in the field as if it had been typed.
 */
export function useTypingUndo(apply: (text: string) => void, element: RefObject<HTMLElement | null>, options: TypingUndoOptions = {}) {
  const { run, enabled } = useKitEditor()
  const session = useRef<Session | null>(null)
  const applyRef = useRef(apply)
  applyRef.current = apply
  const optionsRef = useRef(options)
  optionsRef.current = options
  // A key just handled: the browser's undo it may still start (a `beforeinput` of `historyUndo`) is only stopped.
  const keyed = useRef(false)
  /** After the editor's step: the field's history starts again from what it shows now. */
  const fresh = (now: Session) => {
    if (!now.stale) return
    const text = shown(element.current)
    if (text === null) return
    session.current = { texts: [text], at: 0, stale: false }
  }
  /** Whether `by` falls through to the editor: the field has nothing (or must have nothing) to step. */
  const throughEditor = (now: Session | null, by: -1 | 1) => {
    if (!now) return false
    if (optionsRef.current.untouchedOnly) return now.texts.length === 1
    return by < 0 ? now.at === 0 : now.at === 0 && now.texts.length === 1
  }
  const step = (by: -1 | 1) => {
    if (session.current) fresh(session.current)
    const now = session.current
    if (!now) return
    if (throughEditor(now, by)) {
      const command = by < 0 ? 'edit.undo' : 'edit.redo'
      if (!enabled(command)) return
      now.stale = true
      optionsRef.current.leave?.()
      void run(command)
      return
    }
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
      if (event.inputType !== 'historyUndo' && event.inputType !== 'historyRedo') {
        // Typing after the editor's step: the field's history starts from the text it shows now.
        if (session.current) fresh(session.current)
        return
      }
      event.preventDefault()
      if (keyed.current) {
        keyed.current = false
        return
      }
      stepRef.current(event.inputType === 'historyUndo' ? -1 : 1)
    }
    field.addEventListener('beforeinput', native)
    return () => field.removeEventListener('beforeinput', native)
  }, [element]) // eslint-disable-line react-hooks/exhaustive-deps
  return {
    begin(text: string) {
      session.current = { texts: [text], at: 0, stale: false }
    },
    record(text: string) {
      const now = session.current
      if (!now || now.texts[now.at] === text) return
      // A change the `beforeinput` did not see after the editor's step: start from this text.
      if (now.stale) {
        session.current = { texts: [text], at: 0, stale: false }
        return
      }
      now.texts = [...now.texts.slice(0, now.at + 1), text]
      now.at = now.texts.length - 1
    },
    end() {
      session.current = null
    },
    key(event: KeyboardEvent<HTMLElement>): boolean {
      const by = undoStep(event)
      if (by === 0) return false
      // Never the browser's own history; the editor's Undo only through `step`.
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
