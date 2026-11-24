import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { createPortal } from 'react-dom'
import { LinkIcon, SigmaIcon } from '@kition/tessera/icons/react'
import { MenuButton, MenuItem, SubMenu, useEngineQuery, useKitEditor } from '@kition/sdk/editor'

/** An expression failure as `expr.errors` lists it. */
export type ExprError = { layer: number; path: string; message: string }

/**
 * An expression error in the editor's language: the engine's message (from the expression
 * language, in English) reworded where its form is known, else kept after a translated headline.
 */
/**
 * What a syntax error means, in the person's words: the engine wraps the expression in braces
 * before parsing it, so a brace it did not expect at the end (or an unexpected end) means the
 * expression stops short; another unexpected token is named as the person typed it.
 */
function syntaxDetail(detail: string, t: (key: string, values?: Record<string, string | number>) => string) {
  if (/unexpected (token\s*'?\}'?|end)|end of (input|file)/i.test(detail)) return t('expr.syntax.incomplete')
  if (/unterminated (string|template)/i.test(detail)) return t('expr.syntax.string')
  const token = /unexpected token\s*'([^']+)'/i.exec(detail)?.[1]
  if (token && token !== '{') return t('expr.syntax.unexpected', { token })
  return t('expr.syntax.generic')
}

/** The pairs of brackets an expression's text must balance. */
const PAIRS: Record<string, string> = { '(': ')', '[': ']', '{': '}' }
const OPENERS: Record<string, string> = { ')': '(', ']': '[', '}': '{' }

/**
 * The first bracket or quote in `source` that is left open or closes nothing, with its line and
 * column (from 1), skipping text in quotes and comments; null when they all pair up.
 */
export function unbalanced(source: string): { kind: 'unclosed' | 'extra' | 'quote'; open: string; close: string; line: number; column: number } | null {
  const stack: Array<{ char: string; line: number; column: number }> = []
  let line = 1
  let column = 0
  let quote: { char: string; line: number; column: number } | null = null
  for (let at = 0; at < source.length; at += 1) {
    const char = source[at]
    if (char === '\n') {
      line += 1
      column = 0
      // A quote ' or " ends with its line; a template string goes on.
      if (quote && quote.char !== '`') return { kind: 'quote', open: quote.char, close: quote.char, line: quote.line, column: quote.column }
      continue
    }
    column += 1
    if (quote) {
      if (char === '\\') {
        at += 1
        column += 1
      } else if (char === quote.char) quote = null
      continue
    }
    if (char === '/' && source[at + 1] === '/') {
      const end = source.indexOf('\n', at)
      if (end < 0) break
      column += end - at - 1
      at = end - 1
      continue
    }
    if (char === '/' && source[at + 1] === '*') {
      const end = source.indexOf('*/', at + 2)
      const skipped = source.slice(at, end < 0 ? source.length : end + 2)
      const lines = skipped.split('\n')
      if (lines.length > 1) {
        line += lines.length - 1
        column = lines[lines.length - 1].length
      } else column += skipped.length - 1
      at += skipped.length - 1
      continue
    }
    if (char === '"' || char === "'" || char === '`') quote = { char, line, column }
    else if (PAIRS[char]) stack.push({ char, line, column })
    else if (OPENERS[char]) {
      const open = stack.pop()
      if (!open) return { kind: 'extra', open: OPENERS[char], close: char, line, column }
      if (open.char !== OPENERS[char]) return { kind: 'unclosed', open: open.char, close: PAIRS[open.char], line: open.line, column: open.column }
    }
  }
  if (quote) return { kind: 'quote', open: quote.char, close: quote.char, line: quote.line, column: quote.column }
  const open = stack.pop()
  return open ? { kind: 'unclosed', open: open.char, close: PAIRS[open.char], line: open.line, column: open.column } : null
}

/** Messages of the expression language and the engine the editor words itself (`expr.msg.*`); a group is `{n}`. */
const KNOWN_ERRORS: ReadonlyArray<readonly [RegExp, string]> = [
  [/^(?:TypeError:\s*)?cannot convert '?null'? or '?undefined'? to object/i, 'expr.msg.empty'],
  [/^(?:TypeError:\s*)?cannot read propert(?:y|ies)\b.*\b(?:null|undefined)/i, 'expr.msg.empty'],
  [/^(?:TypeError:\s*)?not a callable function/i, 'expr.msg.notCallable'],
  [/^(?:TypeError:\s*)?not a constructor/i, 'expr.msg.notConstructor'],
  [/^expression result is undefined/i, 'expr.msg.noResult'],
  [/^expression result is not a finite number/i, 'expr.msg.notFinite'],
  [/^expression result must be a number/i, 'expr.msg.notNumber'],
  [/^expression result must be of dimension (\d+)/i, 'expr.msg.dimension'],
  [/^expression result array must contain numbers/i, 'expr.msg.arrayNumbers'],
  [/^expression result must be a path/i, 'expr.msg.path'],
  [/reference each other too deeply/i, 'expr.msg.circular'],
  [/too many property reads/i, 'expr.msg.reads'],
  [/nested too deeply/i, 'expr.msg.nested'],
  [/maximum (?:call )?stack size/i, 'expr.msg.stack'],
  [/^Layer named "(.*)" does not exist/i, 'expr.msg.noLayer'],
  [/^Layer index (\S+) out of range/i, 'expr.msg.layerIndex'],
  [/^Comp named "(.*)" does not exist/i, 'expr.msg.noComp'],
  [/^Footage named "(.*)" does not exist/i, 'expr.msg.noFootage'],
  [/^Effect "?(.*?)"? not found on layer/i, 'expr.msg.noEffect'],
  [/^Layer ".*" has no parent/i, 'expr.msg.noParent'],
  [/^Layer ".*" has no property "(.*)"/i, 'expr.msg.noProperty'],
  [/^Property "?(.*?)"? not found in/i, 'expr.msg.noProperty'],
  [/^Key index (\S+) out of range/i, 'expr.msg.keyIndex'],
  [/^Property has no keyframes/i, 'expr.msg.noKeys'],
  [/^(?:Marker .* (?:not found|out of range)|No markers)/i, 'expr.msg.noMarker'],
  [/has no active camera/i, 'expr.msg.noCamera'],
]

/** `source`: the expression's text; a syntax error from a bracket or a quote left open says which one and where. */
export function explainError(message: string, t: (key: string, values?: Record<string, string | number>) => string, source?: string) {
  // The engine says where: `Error at line 3: ReferenceError: foo is not defined`, or `Error: …`
  // when it cannot tell the line.
  const located = /^Error at line (\d+):\s*(.*)$/is.exec(message.trim())
  const line = located ? Number(located[1]) : null
  const body = (located ? located[2] : message.trim().replace(/^Error:\s*/i, '')).trim()
  const rules: Array<[RegExp, string]> = [
    [/^ReferenceError:\s*(.+?) is not defined/i, 'expr.error.undefined'],
    [/^SyntaxError:\s*(.*)$/is, 'expr.error.syntax'],
    [/^TypeError:\s*(.+?) is not a function/i, 'expr.error.notFunction'],
    [/^TypeError:\s*(.*)$/is, 'expr.error.type'],
  ]
  // A bracket or quote left open (or one too many): named with its place, which the engine's
  // message does not give (it fails where the text ends).
  if (/^SyntaxError:/i.test(body) && source !== undefined) {
    const found = unbalanced(source)
    if (found) return t('expr.error.syntax', { detail: t(`expr.syntax.${found.kind}`, { open: found.open, close: found.close, line: found.line, column: found.column }) })
  }
  let said = t('expr.error.other', { detail: body })
  // What the expression language and the engine say in English, in the person's words; anything
  // else keeps the engine's words on a second line under a translated headline.
  const known = KNOWN_ERRORS.find(([pattern]) => pattern.test(body))
  if (known) {
    const n = known[0].exec(body)?.[1] ?? ''
    said = t('expr.error.known', { detail: t(known[1], { n }) })
    return line !== null && Number.isFinite(line) ? t('expr.error.atLine', { line, message: said }) : said
  }
  for (const [pattern, key] of rules) {
    const found = pattern.exec(body)
    if (found) {
      said = t(key, { name: found[1] ?? '', detail: key === 'expr.error.syntax' ? syntaxDetail(found[1] ?? '', t) : (found[1] ?? '') })
      break
    }
  }
  return line !== null && Number.isFinite(line) ? t('expr.error.atLine', { line, message: said }) : said
}

/** The line (from 1) an expression error is on: where the engine says, or the bracket or quote left open; null when it cannot tell. */
export function errorLine(message: string, source?: string): number | null {
  const located = /^Error at line (\d+):/i.exec(message.trim())
  if (located) return Number(located[1])
  if (/SyntaxError:/i.test(message) && source !== undefined) return unbalanced(source)?.line ?? null
  return null
}

/**
 * The expression function menu: what each group inserts (the text, with the cursor after it).
 * Group names are translated (`expr.fn.<group>`); the entries are the expression language's own.
 */
export const EXPRESSION_FUNCTIONS: ReadonlyArray<readonly [group: string, entries: readonly string[]]> = [
  ['global', ['time', 'value', 'thisComp', 'thisLayer', 'thisProperty', 'comp("Comp 1")', 'posterizeTime(12)', 'degreesToRadians(90)', 'radiansToDegrees(1)']],
  ['vector', ['add(a, b)', 'sub(a, b)', 'mul(a, 2)', 'div(a, 2)', 'clamp(value, 0, 100)', 'dot(a, b)', 'cross(a, b)', 'normalize(a)', 'length(a, b)', 'lookAt(fromPoint, atPoint)']],
  ['random', ['seedRandom(1, true)', 'random()', 'random(100)', 'random(0, 100)', 'gaussRandom()', 'noise(time)']],
  ['interpolation', ['linear(time, 0, 1, 0, 100)', 'ease(time, 0, 1, 0, 100)', 'easeIn(time, 0, 1, 0, 100)', 'easeOut(time, 0, 1, 0, 100)']],
  ['layer', ['thisLayer.name', 'index', 'inPoint', 'outPoint', 'startTime', 'hasParent', 'parent', 'toComp(anchorPoint)', 'fromComp([0, 0])', 'sampleImage(anchorPoint, [0.5, 0.5])']],
  ['property', ['wiggle(2, 30)', 'loopOut("cycle")', 'loopIn("cycle")', 'valueAtTime(time - 0.5)', 'velocity', 'speed', 'velocityAtTime(time)', 'smooth(0.2, 5)']],
  ['key', ['numKeys', 'key(1).value', 'key(1).time', 'nearestKey(time).index']],
  ['time', ['time', 'timeToFrames(time)', 'framesToTime(10)', 'timeToTimecode(time)', 'thisComp.frameDuration']],
]

/** What a pick-whip was let go over: a property's row (`data-pick-path`) or a layer's (`data-pick-layer`). */
function pickTarget(x: number, y: number): { layer: number; path?: string; name?: string } | null {
  const under = document.elementFromPoint(x, y)
  const row = under?.closest<HTMLElement>('[data-pick-path], [data-pick-layer]')
  const layer = Number(row?.dataset.pickLayer)
  if (!row || !Number.isFinite(layer)) return null
  return { layer, ...(row.dataset.pickPath ? { path: row.dataset.pickPath } : {}), ...(row.dataset.pickName ? { name: row.dataset.pickName } : {}) }
}

/** Calls `onClosed` when a menu that was open closes. */
function MenuClosed({ open, onClosed }: { open: boolean; onClosed: () => void }) {
  const was = useRef(open)
  useEffect(() => {
    if (was.current && !open) onClosed()
    was.current = open
  }, [open, onClosed])
  return null
}

/** A value as the row shows it: numbers to two decimals, several joined by commas. */
function shownValue(value: unknown): string | null {
  const one = (part: unknown) => (typeof part === 'number' ? String(Math.round(part * 100) / 100) : null)
  if (typeof value === 'number') return one(value)
  if (Array.isArray(value)) {
    const parts = value.map(one)
    return parts.every((part) => part !== null) ? parts.join(', ') : null
  }
  return typeof value === 'string' ? value : null
}

/**
 * A property's expression on the timeline, on the row under the property: the text (committed
 * when the field is left or with Enter; Shift+Enter starts a new line, Escape goes back), a switch
 * that turns it on and off, Remove, and, when it fails, why. Each change is one undo step
 * (`prop.setExpression`).
 */
export function ExpressionRow({ layer, path, name, text, enabled, error, line, depth, time, testId, readOnly }: { layer: number; path: string; name: string; text: string; enabled: boolean; error: string | null; /** The line the error is on (from 1), highlighted in the field. */ line?: number | null; depth: number; time: number; testId: string; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  const [draft, setDraft] = useState(text)
  const dropped = useRef(false)
  const field = useRef<HTMLTextAreaElement>(null)
  const [scroll, setScroll] = useState(0)
  // Where the cursor was in the field while it is being edited (what a pick-whip or a function goes
  // in place of); null when the field is not being edited: a pick-whip then replaces the whole text.
  const caret = useRef<[number, number] | null>(null)
  // The draft is held (not committed when the field loses the focus) while a pick-whip is dragged
  // or the function menu is open: what they insert goes into the draft, and the draft with it is
  // one undo step when it is committed.
  const held = useRef(false)
  // A pick-whip let go into the text being edited: where the reference went, to put the cursor after
  // it when the engine's text comes back.
  const whipped = useRef<{ from: number; tail: number } | null>(null)
  // A pick-whip's link on its way to the engine: until it is written the draft (the text before
  // it) is not committed, or a click elsewhere in that moment would write the old text over the link.
  const linking = useRef(false)
  const link = (command: string, params: Record<string, unknown>) => {
    linking.current = true
    void run(command, params)
      .then((answer) => {
        // The engine's new text at once (its document update may come a moment later).
        const written = typeof answer === 'string' ? answer : (answer as { expression?: unknown } | null)?.expression
        if (typeof written === 'string') setDraft(written)
      })
      .finally(() => {
        linking.current = false
      })
  }
  useEffect(() => {
    setDraft(text)
    const insert = whipped.current
    whipped.current = null
    const element = field.current
    if (insert && element && document.activeElement === element) {
      const at = Math.max(insert.from, text.length - insert.tail)
      caret.current = [at, at]
      element.setSelectionRange(at, at)
    }
  }, [text])
  const remember = () => {
    const element = field.current
    if (element && document.activeElement === element) caret.current = [element.selectionStart, element.selectionEnd]
  }
  /** Gives the focus back to the field after a held draft was used (or nothing was chosen). */
  const refocus = (at?: number) => {
    // After the menu has given the focus back to its button.
    window.setTimeout(() => {
      held.current = false
      const element = field.current
      if (!element) return
      element.focus({ preventScroll: true })
      const [from, to] = at !== undefined ? [at, at] : (caret.current ?? [element.value.length, element.value.length])
      element.setSelectionRange(from, to)
    }, 0)
  }
  /** Puts `insert` where the cursor was (in place of the whole text when the field was not being edited), and puts the cursor after it. */
  const insertText = (insert: string) => {
    const [from, to] = caret.current ?? [0, draft.length]
    const next = draft.slice(0, from) + insert + draft.slice(to)
    setDraft(next)
    const at = from + insert.length
    caret.current = [at, at]
    refocus(at)
  }
  // The pick-whip being dragged: from its handle to the pointer, the row under it lit.
  const [whip, setWhip] = useState<{ from: [number, number]; to: [number, number] } | null>(null)
  const lit = useRef<HTMLElement | null>(null)
  const light = (row: HTMLElement | null) => {
    if (lit.current === row) return
    lit.current?.removeAttribute('data-pick-over')
    row?.setAttribute('data-pick-over', 'true')
    lit.current = row
  }
  const whipDown = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (readOnly || event.button !== 0) return
    event.preventDefault()
    // The text being typed stays a draft while the whip is out (it is not committed on the way).
    held.current = true
    remember()
    event.stopPropagation()
    event.currentTarget.setPointerCapture(event.pointerId)
    const box = event.currentTarget.getBoundingClientRect()
    setWhip({ from: [box.left + box.width / 2, box.top + box.height / 2], to: [event.clientX, event.clientY] })
  }
  const whipMove = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (!whip) return
    setWhip({ ...whip, to: [event.clientX, event.clientY] })
    const under = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-pick-path], [data-pick-layer]') ?? null
    light(under)
  }
  /** Let go over a property: the engine writes the reference where the cursor was (one undo step); over a layer, `thisComp.layer("name")`. */
  const whipUp = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (!whip) return
    setWhip(null)
    light(null)
    const target = pickTarget(event.clientX, event.clientY)
    // In the text being edited the reference goes at the cursor (in place of the selection);
    // otherwise it replaces the whole expression.
    const editingText = caret.current !== null
    const [from, to] = caret.current ?? [0, draft.length]
    const unchanged = !target || (target.path !== undefined && target.layer === layer && target.path === path)
    if (unchanged) {
      if (editingText) refocus()
      else held.current = false
      return
    }
    if (editingText) whipped.current = { from, tail: draft.length - to }
    if (editingText) refocus()
    else held.current = false
    if (target.path) {
      // The engine counts characters, not UTF-16 units.
      const chars = (end: number) => [...draft.slice(0, end)].length
      link('prop.pickWhip', { layer, path, target: { layer: target.layer, path: target.path }, expression: draft, range: [chars(from), chars(to)] })
      return
    }
    const reference = target.layer === layer ? 'thisLayer' : `thisComp.layer(${JSON.stringify(target.name ?? '')})`
    const next = draft.slice(0, from) + reference + draft.slice(to)
    link('prop.setExpression', { layer, path, expression: next })
  }
  // What the expression makes of the property now (what renders), as `prop.get` evaluates it.
  const read = useEngineQuery<{ evaluated?: unknown }>('prop.get', { layer, path }, `${text} ${enabled} ${time}`)
  const result = shownValue(read?.evaluated)
  const commit = () => {
    caret.current = null
    // The link being written replaces the draft (the engine's text comes back with it).
    if (linking.current) return
    if (dropped.current) {
      dropped.current = false
      setDraft(text)
      return
    }
    if (draft !== text) void run('prop.setExpression', { layer, path, expression: draft.trim() ? draft : text })
  }
  return (
    <div className="motion-row motion-prop-row motion-expr-row" data-failed={error ? true : undefined} data-testid={testId}>
      <div className="motion-timeline-names motion-prop-names motion-expr-names" style={{ paddingLeft: 36 + depth * 14 }}>
        <button
          type="button"
          className="motion-expr-switch"
          aria-pressed={enabled}
          title={t(enabled ? 'expr.disable' : 'expr.enable', { name })}
          aria-label={t(enabled ? 'expr.disable' : 'expr.enable', { name })}
          disabled={readOnly}
          onClick={() => void run('prop.setExpression', { layer, path, enabled: !enabled })}
          data-testid={`${testId}-switch`}
        >
          =
        </button>
        <span className="motion-row-name kui-muted">{t('expr.label')}</span>
        <button type="button" className="motion-expr-remove" disabled={readOnly} title={t('expr.remove')} onClick={() => void run('prop.setExpression', { layer, path, expression: '' })} data-testid={`${testId}-remove`}>
          {t('expr.removeShort')}
        </button>
      </div>
      <div className="motion-row-track motion-expr-track">
        <div className="motion-expr-edit">
          <button
            type="button"
            className="motion-expr-whip"
            title={t('expr.whip', { name })}
            aria-label={t('expr.whip', { name })}
            disabled={readOnly}
            onPointerDown={whipDown}
            onPointerMove={whipMove}
            onPointerUp={whipUp}
            onPointerCancel={() => {
              setWhip(null)
              light(null)
              if (caret.current !== null) refocus()
              else held.current = false
            }}
            // The field keeps the focus (and its cursor) while the whip is pressed.
            onMouseDown={(event) => event.preventDefault()}
            data-testid={`${testId}-whip`}
          >
            <LinkIcon size={13} aria-hidden="true" />
          </button>
          <MenuButton
            placement="auto"
            align="start"
            trigger={({ open, toggle }) => (
              <>
                <button
                  type="button"
                  className="motion-expr-whip"
                  aria-expanded={open}
                  title={t('expr.functions')}
                  aria-label={t('expr.functions')}
                  disabled={readOnly}
                  // Opening the menu does not commit the text being typed: what it inserts goes into the draft.
                  onPointerDown={() => {
                    remember()
                    if (document.activeElement === field.current) held.current = true
                  }}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => {
                    if (!open && document.activeElement === field.current) {
                      remember()
                      held.current = true
                    }
                    toggle()
                  }}
                  data-testid={`${testId}-functions`}
                >
                  <SigmaIcon size={13} aria-hidden="true" />
                </button>
                <MenuClosed open={open} onClosed={() => (held.current ? refocus() : undefined)} />
              </>
            )}
          >
            {EXPRESSION_FUNCTIONS.map(([group, entries]) => (
              <SubMenu key={group} label={t(`expr.fn.${group}`)} path={`${testId}/fn/${group}`} testId={`${testId}-fn-${group}`}>
                {entries.map((entry) => (
                  <MenuItem key={entry} label={entry} onSelect={() => insertText(entry)} testId={`${testId}-fn-${group}-${entry.replace(/[^a-zA-Z0-9]+/g, '-')}`} />
                ))}
              </SubMenu>
            ))}
          </MenuButton>
          <span className="motion-expr-box">
            {/* The line the error is on, lit behind the text (the field scrolls over it). */}
            {line != null && line > 0 && error ? <span className="motion-expr-line" style={{ top: `calc(4px + ${line - 1} * 1.4em - ${scroll}px)` }} aria-hidden="true" data-line={line} data-testid={`${testId}-error-line`} /> : null}
        <textarea
          ref={field}
          className="kui-field motion-expr-field"
          value={draft}
          // As tall as its text, up to eight lines (it does not jump when clicked: the rows under it
          // stay where they are for a pick-whip); a person can drag it taller.
          rows={Math.min(8, Math.max(1, draft.split('\n').length))}
          spellCheck={false}
          readOnly={readOnly}
          aria-label={t('expr.field', { name })}
          onChange={(event) => setDraft(event.target.value)}
          onSelect={remember}
          onKeyUp={remember}
          onScroll={(event) => setScroll(event.currentTarget.scrollTop)}
          onBlur={() => {
            if (held.current) return
            commit()
          }}
          onKeyDown={(event) => {
            event.stopPropagation()
            if (event.key === 'Enter' && !event.shiftKey) {
              event.preventDefault()
              event.currentTarget.blur()
            }
            if (event.key === 'Escape') {
              event.preventDefault()
              dropped.current = true
              event.currentTarget.blur()
            }
          }}
          data-testid={`${testId}-field`}
        />
          </span>
        </div>
        {whip
          ? createPortal(
              <svg className="motion-whip-line" aria-hidden="true">
                <line x1={whip.from[0]} y1={whip.from[1]} x2={whip.to[0]} y2={whip.to[1]} />
              </svg>,
              document.body,
            )
          : null}
        {result !== null && !error ? (
          <span className="motion-expr-result kui-muted" data-testid={`${testId}-result`}>
            {t('expr.result', { value: result })}
          </span>
        ) : null}
        {error ? (
          <span className="motion-expr-error" role="alert" data-testid={`${testId}-error`}>
            {error}
          </span>
        ) : null}
      </div>
    </div>
  )
}
