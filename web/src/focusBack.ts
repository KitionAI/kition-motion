/**
 * The editor keeps the keyboard when the element that had it goes away. A focused field of the
 * Properties panel or the timeline is taken out of the page when its layer goes (the editor's
 * Undo pressed in that field can remove the layer itself); the browser then gives the focus to
 * the page's `body`, where no shortcut of the editor arrives (they are the `.kui-app` element's
 * keys), so Cmd/Ctrl+Z, Redo and every other key did nothing until the person clicked the
 * editor again. This gives the focus to the editor's `.kui-app` (as a click on the canvas or the
 * timeline does) when, and only when, the element that had the focus left the page and nothing
 * took it since: the focus moved anywhere on purpose (another field, the host's page) is left
 * where it is.
 */
export function installFocusBack(target: Document): () => void {
  /** The element that had the focus last, while it has it. */
  let held: Element | null = null
  const back = () => {
    if (!held || held.isConnected) return
    held = null
    const now = target.activeElement
    if (now && now !== target.body && now !== target.documentElement) return
    // The person is in the host's page, not in this frame: theirs.
    if (!target.hasFocus()) return
    target.querySelector<HTMLElement>('.kui-app')?.focus({ preventScroll: true })
  }
  const focusin = (event: FocusEvent) => {
    held = event.target instanceof Element ? event.target : null
  }
  const focusout = (event: FocusEvent) => {
    if (event.relatedTarget) return
    // Taken out of the page (some browsers say so with a blur), or left for nothing on purpose:
    // the latter keeps where it went.
    window.setTimeout(() => {
      if (held === event.target && held instanceof Element && held.isConnected) held = null
      else back()
    }, 0)
  }
  // Browsers that remove a focused element without a blur: seen when the page changes.
  const watcher = new MutationObserver(back)
  watcher.observe(target.body, { childList: true, subtree: true })
  target.addEventListener('focusin', focusin)
  target.addEventListener('focusout', focusout)
  return () => {
    watcher.disconnect()
    target.removeEventListener('focusin', focusin)
    target.removeEventListener('focusout', focusout)
  }
}
