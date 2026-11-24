import { useEffect, useRef, useState } from 'react'
import { useEngineQuery, useKitEditor } from '@kition/sdk/editor'

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
  for (const [pattern, key] of rules) {
    const found = pattern.exec(body)
    if (found) {
      said = t(key, { name: found[1] ?? '', detail: key === 'expr.error.syntax' ? syntaxDetail(found[1] ?? '', t) : (found[1] ?? '') })
      break
    }
  }
  return line !== null && Number.isFinite(line) ? t('expr.error.atLine', { line, message: said }) : said
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
export function ExpressionRow({ layer, path, name, text, enabled, error, depth, time, testId, readOnly }: { layer: number; path: string; name: string; text: string; enabled: boolean; error: string | null; depth: number; time: number; testId: string; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  const [draft, setDraft] = useState(text)
  const dropped = useRef(false)
  useEffect(() => setDraft(text), [text])
  // What the expression makes of the property now (what renders), as `prop.get` evaluates it.
  const read = useEngineQuery<{ evaluated?: unknown }>('prop.get', { layer, path }, `${text} ${enabled} ${time}`)
  const result = shownValue(read?.evaluated)
  const commit = () => {
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
        <textarea
          className="kui-field motion-expr-field"
          value={draft}
          rows={Math.min(4, Math.max(1, draft.split('\n').length))}
          spellCheck={false}
          readOnly={readOnly}
          aria-label={t('expr.field', { name })}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commit}
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
