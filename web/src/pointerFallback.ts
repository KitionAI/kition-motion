/**
 * WebKit (seen with Playwright's WebKit; Safari shares the engine) delivers no `pointerdown` for
 * the first press after an HTML5 drag-and-drop: the pointer that began the drag never got its
 * `pointerup` or `pointercancel`, so the next press arrives only as `mousedown` (followed by
 * `pointerup`, `mouseup` and `click`). Every press handler here (the ruler, the layer bars,
 * keyframes, the canvas tools) listens for `pointerdown`, so that press did nothing. This turns
 * such a `mousedown` into the `pointerdown` it should have been, on the same element, before any
 * handler sees it; pointer capture then works as for a real press (the pointer is still active in
 * WebKit's eyes). A press that had its `pointerdown` is left alone.
 */
export function installPointerdownFallback(target: Document): () => void {
  let pressed = false
  const pointerdown = (event: PointerEvent) => {
    if (event.isTrusted) pressed = true
  }
  const mouseup = () => {
    pressed = false
  }
  const mousedown = (event: MouseEvent) => {
    if (pressed || event.button !== 0 || !(event.target instanceof Element)) return
    pressed = true
    event.target.dispatchEvent(
      new PointerEvent('pointerdown', {
        bubbles: true,
        cancelable: true,
        composed: true,
        clientX: event.clientX,
        clientY: event.clientY,
        screenX: event.screenX,
        screenY: event.screenY,
        button: 0,
        buttons: event.buttons || 1,
        pointerId: 1,
        pointerType: 'mouse',
        isPrimary: true,
        shiftKey: event.shiftKey,
        altKey: event.altKey,
        metaKey: event.metaKey,
        ctrlKey: event.ctrlKey,
      }),
    )
  }
  // A press ends with its mouseup, or, when it began a drag-and-drop, with the drag's end.
  const ends = ['mouseup', 'dragend', 'drop'] as const
  target.addEventListener('pointerdown', pointerdown, true)
  target.addEventListener('mousedown', mousedown, true)
  for (const type of ends) target.addEventListener(type, mouseup, true)
  return () => {
    target.removeEventListener('pointerdown', pointerdown, true)
    target.removeEventListener('mousedown', mousedown, true)
    for (const type of ends) target.removeEventListener(type, mouseup, true)
  }
}
