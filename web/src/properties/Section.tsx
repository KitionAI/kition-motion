import { useState, type ReactNode } from 'react'
import { ChevronRightIcon as Chevron } from '@kition/tessera/icons/react'

const KEY = 'kition-motion.sections'

/** Which sections and groups the person opened or closed, per layer type, for the session. */
function remembered(): Record<string, boolean> {
  try {
    return JSON.parse(globalThis.sessionStorage?.getItem(KEY) ?? '{}') as Record<string, boolean>
  } catch {
    return {}
  }
}

/** Opens or closes the section or group `key` of layer type `scope`, as a click on it would. */
export function remember(key: string, open: boolean) {
  const all = remembered()
  all[key] = open
  try {
    sessionStorage.setItem(KEY, JSON.stringify(all))
  } catch {
    // Storage refused (private mode): the panel forgets on the next layer.
  }
}

/**
 * Whether the section or group `key` is open, remembered per layer type (`Text`, `Shape`) for the
 * session; `byDefault` until the person toggles it.
 */
export function useOpen(scope: string, key: string, byDefault: boolean) {
  const id = `${scope}:${key}`
  const [, redraw] = useState(0)
  const stored = remembered()[id]
  const open = stored ?? byDefault
  const set = (next: boolean) => {
    remember(id, next)
    redraw((n) => n + 1)
  }
  return [open, set] as const
}

/**
 * A collapsible section of the Properties panel, drawn as the SDK's panel sections, its state
 * remembered per layer type. `actions` sit at the right of its header.
 */
export function Section({ scope, id, title, children, byDefault = true, actions, testId }: { scope: string; id: string; title: ReactNode; children: ReactNode; byDefault?: boolean; actions?: ReactNode; testId?: string }) {
  const [open, setOpen] = useOpen(scope, id, byDefault)
  return (
    <section className="kui-panel-section motion-section" data-open={open} data-testid={testId}>
      <div className="kui-panel-section-header">
        <button type="button" className="kui-panel-section-toggle" aria-expanded={open} onClick={() => setOpen(!open)} data-testid={testId ? `${testId}-toggle` : undefined}>
          <Chevron size={13} aria-hidden="true" className="kui-panel-section-chevron" />
          {title}
        </button>
        {actions ? <span className="motion-section-actions">{actions}</span> : null}
      </div>
      {open ? <div className="kui-panel-section-body">{children}</div> : null}
    </section>
  )
}
