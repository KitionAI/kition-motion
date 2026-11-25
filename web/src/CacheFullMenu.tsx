import { useState } from 'react'
import { Button, Popover, PopoverContent, PopoverTrigger, useKitEditor } from '@kition/sdk/editor'

/**
 * What can be done about a full preview cache, behind one ▾ beside the cache button (together
 * they read "Cache full N/M ▾"): what the cache holds, then Set preview to Half (Third, Quarter),
 * Use more memory (1 GB) when the browser does not say how much it has, and Loop only the cached
 * part. One small button instead of three keeps the status bar's zoom controls in view in a
 * narrow window.
 */
export function CacheFullMenu({
  held,
  total,
  megabytes,
  lower,
  loop,
  moreMemory,
}: {
  held: string
  total: number
  megabytes: number
  lower: { label: string; set: () => void } | null
  loop: { on: boolean; toggle: () => void; area: readonly number[] | null | undefined } | null
  moreMemory: { megabytes: number; use: () => void } | null
}) {
  const { t } = useKitEditor()
  const [open, setOpen] = useState(false)
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="ghost" className="motion-cache-full-menu" aria-label={t('cacheAhead.fullMenu')} title={t('cacheAhead.fullMenu')} aria-expanded={open} data-testid="motion-cache-full-menu">
          ▾
        </Button>
      </PopoverTrigger>
      <PopoverContent
        className="motion-cache-full-popover"
        align="end"
        side="top"
        onEscapeKeyDown={(event) => event.stopPropagation()}
        data-testid="motion-cache-full-popover"
      >
        <div className="motion-cache-full-title">{t('cacheAhead.fullMenuTitle')}</div>
        <p className="motion-cache-full-text kui-muted" data-testid="motion-cache-full-text">
          {t('cacheAhead.fullExplain', { held, total, megabytes })}
        </p>
        <div className="motion-cache-full-actions">
          {lower ? (
            <Button variant="ghost" title={t('cacheAhead.lowerHint', { resolution: lower.label })} onClick={lower.set} data-testid="motion-cache-lower">
              {t('cacheAhead.lower', { resolution: lower.label })}
            </Button>
          ) : null}
          {moreMemory ? (
            <Button variant="ghost" title={t('cacheAhead.moreMemoryHint', { megabytes: moreMemory.megabytes })} onClick={moreMemory.use} data-testid="motion-cache-more-memory">
              {t('cacheAhead.moreMemory')}
            </Button>
          ) : null}
          {loop ? (
            <Button
              variant="ghost"
              className="motion-toggle"
              aria-pressed={loop.on}
              title={t('cacheAhead.loopCachedHint', { held })}
              onClick={loop.toggle}
              data-testid="motion-cache-loop-cached"
              data-area={loop.area ? loop.area.map((each) => each.toFixed(3)).join(' ') : undefined}
            >
              {t('cacheAhead.loopCached')}
            </Button>
          ) : null}
        </div>
      </PopoverContent>
    </Popover>
  )
}
