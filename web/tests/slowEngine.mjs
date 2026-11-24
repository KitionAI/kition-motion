// An opt-in check that a test waits for the engine's answers: with ENGINE_DELAY_MS=<ms> in the
// environment, every message the editor's engine worker sends arrives that many milliseconds late
// (in order), as on a slow machine; without it nothing changes. Each browser script calls
// `delayEngineReplies(context)` on the contexts it opens.
//   ENGINE_DELAY_MS=40 node tests/canvas.mjs chromium

/** Wraps `Worker` in every page and frame of `context` so its messages arrive late. */
export async function delayEngineReplies(context) {
  const delay = Number(process.env.ENGINE_DELAY_MS ?? 0)
  if (!(delay > 0)) return
  await context.addInitScript((ms) => {
    const Native = globalThis.Worker
    if (typeof Native !== 'function') return
    globalThis.Worker = class DelayedWorker extends Native {
      constructor(...args) {
        super(...args)
        const passed = new WeakSet()
        // Registered first, so it runs before the page's own listeners and its onmessage.
        super.addEventListener('message', (event) => {
          if (passed.has(event)) return
          event.stopImmediatePropagation()
          setTimeout(() => {
            const late = new MessageEvent('message', { data: event.data, ports: [...event.ports] })
            passed.add(late)
            this.dispatchEvent(late)
          }, ms)
        })
      }
    }
  }, delay)
  console.log(`(engine replies delayed by ${delay} ms)`)
}
