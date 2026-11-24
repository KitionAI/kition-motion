import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react'
import { TrashIcon } from '@kition/tessera/icons/react'
import { IconButton, useKitEditor } from '@kition/sdk/editor'
import { NumberField } from '../ValueField'
import { ColorSwatch, hexOf } from './ColorSwatch'

/** One colour stop: where it is (0–1), its colour, and its opacity (0–1). */
type Stop = { at: number; hex: string; alpha: number }

/** A gradient as `layer.tree` gives it (`{t: 'Gradient', v: {colors, opacities}}`), as stops. */
export function gradientStops(value: unknown): Stop[] {
  const raw = (value && typeof value === 'object' && 'v' in value ? (value as { v: unknown }).v : value) as { colors?: unknown; opacities?: unknown } | null
  const colors = Array.isArray(raw?.colors) ? raw.colors : []
  const opacities = (Array.isArray(raw?.opacities) ? raw.opacities : []).filter((one): one is [number, number] => Array.isArray(one) && typeof one[0] === 'number' && typeof one[1] === 'number')
  const alphaAt = (at: number) => {
    if (!opacities.length) return 1
    const sorted = [...opacities].sort((a, b) => a[0] - b[0])
    if (at <= sorted[0][0]) return sorted[0][1]
    for (let i = 1; i < sorted.length; i += 1) {
      const [p0, a0] = sorted[i - 1]
      const [p1, a1] = sorted[i]
      if (at <= p1) return p1 > p0 ? a0 + ((a1 - a0) * (at - p0)) / (p1 - p0) : a1
    }
    return sorted[sorted.length - 1][1]
  }
  const stops = colors
    .filter((one): one is [number, unknown] => Array.isArray(one) && typeof one[0] === 'number')
    .map(([at, color]) => ({ at: Math.min(1, Math.max(0, at)), hex: hexOf(color), alpha: alphaAt(at) }))
    .sort((a, b) => a.at - b.at)
  return stops.length >= 2 ? stops : [{ at: 0, hex: '#ffffff', alpha: 1 }, { at: 1, hex: '#000000', alpha: 1 }]
}

/** Stops as `prop.set` takes a gradient: colours and opacities at their places. */
function gradientValue(stops: readonly Stop[]) {
  const sorted = [...stops].sort((a, b) => a.at - b.at)
  return { colors: sorted.map((stop) => [stop.at, stop.hex]), opacities: sorted.map((stop) => [stop.at, stop.alpha]) }
}

/** The colour between stops at `at`, for a stop added there. */
function sample(stops: readonly Stop[], at: number): Stop {
  const sorted = [...stops].sort((a, b) => a.at - b.at)
  const after = sorted.findIndex((stop) => stop.at >= at)
  const b = sorted[after < 0 ? sorted.length - 1 : after]
  const a = sorted[Math.max(0, (after < 0 ? sorted.length : after) - 1)]
  const f = b.at > a.at ? (at - a.at) / (b.at - a.at) : 0
  const channel = (hex: string, i: number) => parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16) || 0
  const mix = [0, 1, 2].map((i) => Math.round(channel(a.hex, i) + (channel(b.hex, i) - channel(a.hex, i)) * f))
  return { at, hex: `#${mix.map((v) => v.toString(16).padStart(2, '0')).join('')}`, alpha: a.alpha + (b.alpha - a.alpha) * f }
}

let gestures = 0

/**
 * A gradient's stops: the gradient as a bar, its stops as markers under it. A click on the bar
 * adds a stop there; a marker is picked by a click and dragged along (or moved with the arrow
 * keys, Shift ten times as far); the picked stop's colour, place and opacity are edited below it,
 * and Delete (or the bin) removes it while three or more remain. Each change is one undo step
 * (`set` with a gesture name for a drag).
 */
export function GradientEditor({ value, name, testId, readOnly, set }: { value: unknown; name: string; testId: string; readOnly: boolean; set: (value: unknown, gesture?: string) => void }) {
  const { t } = useKitEditor()
  const stops = gradientStops(value)
  const [picked, setPicked] = useState(0)
  // While a marker is dragged, the stops as the drag has them (the engine's copy catches up).
  const [moving, setMoving] = useState<Stop[] | null>(null)
  const bar = useRef<HTMLDivElement>(null)
  const drag = useRef<{ pointerId: number; index: number; gesture: string; moved: boolean; x: number } | null>(null)
  const shown = moving ?? stops
  // A stop just added on the bar takes the focus once the engine has it, so Delete and the
  // arrow keys act on it.
  const focusAt = useRef<number | null>(null)
  const stopsRef = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const at = focusAt.current
    if (at === null) return
    const index = shown.findIndex((one) => Math.abs(one.at - at) < 1e-6)
    if (index < 0) return
    focusAt.current = null
    setPicked(index)
    stopsRef.current?.querySelectorAll<HTMLButtonElement>('.motion-gradient-stop')[index]?.focus({ preventScroll: true })
  })
  const current = Math.min(picked, shown.length - 1)
  const stop = shown[current]
  const css = `linear-gradient(to right, ${[...shown].sort((a, b) => a.at - b.at).map((one) => `${one.hex}${Math.round(one.alpha * 255).toString(16).padStart(2, '0')} ${(one.at * 100).toFixed(2)}%`).join(', ')})`
  const atX = (clientX: number) => {
    const box = bar.current?.getBoundingClientRect()
    return box && box.width > 0 ? Math.min(1, Math.max(0, (clientX - box.left) / box.width)) : 0
  }
  const change = (next: Stop[], gesture?: string) => set(gradientValue(next), gesture)
  const update = (index: number, patch: Partial<Stop>, gesture?: string) => change(shown.map((one, at) => (at === index ? { ...one, ...patch } : one)), gesture)
  const remove = (index: number) => {
    if (shown.length <= 2 || readOnly) return
    change(shown.filter((_, at) => at !== index))
    setPicked(Math.max(0, index - 1))
  }
  const markerKey = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    if (readOnly) return
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault()
      event.stopPropagation()
      remove(index)
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault()
      event.stopPropagation()
      const by = (event.key === 'ArrowLeft' ? -1 : 1) * (event.shiftKey ? 0.1 : 0.01)
      update(index, { at: Math.min(1, Math.max(0, Math.round((shown[index].at + by) * 1000) / 1000)) })
    }
  }
  return (
    <div
      className="motion-gradient"
      data-testid={testId}
      onKeyDown={(event) => {
        // Delete and Backspace inside the gradient editor are its own: never the layer's.
        if (event.key !== 'Delete' && event.key !== 'Backspace') return
        event.stopPropagation()
        if (event.target instanceof Element && event.target.closest('input, textarea, select, [contenteditable="true"]')) return
        event.preventDefault()
        remove(current)
      }}
    >
      <div
        ref={bar}
        className="motion-gradient-bar"
        style={{ backgroundImage: `${css}, repeating-conic-gradient(#999 0 25%, #fff 0 50%)`, backgroundSize: 'auto, 10px 10px' }}
        role="presentation"
        tabIndex={-1}
        title={t('gradient.addHint')}
        onClick={(event) => {
          if (readOnly) return
          const at = Math.round(atX(event.clientX) * 1000) / 1000
          const next = [...shown, sample(shown, at)].sort((a, b) => a.at - b.at)
          change(next)
          setPicked(next.findIndex((one) => one.at === at))
          focusAt.current = at
        }}
        data-testid={`${testId}-bar`}
      />
      <div className="motion-gradient-stops" ref={stopsRef}>
        {shown.map((one, index) => (
          <button
            key={index}
            type="button"
            className="motion-gradient-stop"
            style={{ left: `${one.at * 100}%`, background: one.hex }}
            aria-pressed={index === current}
            aria-label={t('gradient.stop', { index: index + 1, at: Math.round(one.at * 100) })}
            disabled={readOnly}
            onKeyDown={(event) => markerKey(event, index)}
            onPointerDown={(event: ReactPointerEvent<HTMLButtonElement>) => {
              if (event.button !== 0 || readOnly) return
              setPicked(index)
              event.currentTarget.setPointerCapture(event.pointerId)
              gestures += 1
              drag.current = { pointerId: event.pointerId, index, gesture: `gradient-${Date.now()}-${gestures}`, moved: false, x: event.clientX }
            }}
            onPointerMove={(event) => {
              const now = drag.current
              if (!now || now.pointerId !== event.pointerId) return
              if (!now.moved && Math.abs(event.clientX - now.x) < 3) return
              now.moved = true
              const next = (moving ?? stops).map((each, at) => (at === now.index ? { ...each, at: Math.round(atX(event.clientX) * 1000) / 1000 } : each))
              setMoving(next)
              change(next, now.gesture)
            }}
            onPointerUp={(event) => {
              const now = drag.current
              if (!now || now.pointerId !== event.pointerId) return
              drag.current = null
              event.currentTarget.releasePointerCapture(event.pointerId)
              if (now.moved && moving) change(moving, now.gesture)
              setMoving(null)
            }}
            onPointerCancel={() => {
              drag.current = null
              setMoving(null)
            }}
            data-testid={`${testId}-stop-${index}`}
          />
        ))}
      </div>
      {stop ? (
        <div className="motion-gradient-edit">
          <ColorSwatch value={stop.hex} label={t('gradient.color', { name })} readOnly={readOnly} onChange={(hex, final) => update(current, { hex }, final ? undefined : `gradient-color-${current}`)} testId={`${testId}-color`} />
          <NumberField value={Math.round(stop.at * 1000) / 10} label={t('gradient.location')} min={0} max={100} decimals={1} unit="%" readOnly={readOnly} onCommit={(next) => update(current, { at: next / 100 })} testId={`${testId}-location`} />
          <NumberField value={Math.round(stop.alpha * 1000) / 10} label={t('gradient.opacity')} min={0} max={100} decimals={1} unit="%" readOnly={readOnly} onCommit={(next) => update(current, { alpha: next / 100 })} testId={`${testId}-opacity`} />
          <IconButton icon={TrashIcon} label={t('gradient.remove')} disabled={readOnly || shown.length <= 2} onClick={() => remove(current)} data-testid={`${testId}-remove`} />
        </div>
      ) : null}
    </div>
  )
}
