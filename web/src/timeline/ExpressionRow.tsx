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

export function explainError(message: string, t: (key: string, values?: Record<string, string | number>) => string) {
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
