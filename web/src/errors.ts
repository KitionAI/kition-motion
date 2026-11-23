import { toast } from '@kition/sdk/editor'

type Strings = (key: string, values?: Record<string, string | number>) => string

/** Engine refusals people meet often, as the string key of a short sentence that says what to do. */
const KNOWN: ReadonlyArray<readonly [RegExp, string]> = [
  [/\block(ed)?\b/i, 'error.locked'],
  [/no (layers? )?(is |are )?selected|nothing (is )?selected|no selection|select (a|one|the|some)? ?(layer|layers)\b|needs? (a )?selected layer/i, 'error.select'],
  [/no effect (has been )?applied/i, 'error.noLastEffect'],
  [/out of range|must be (positive|between|greater|less|at least|at most|finite)|not a (valid )?number|invalid value|too (large|small|long|short)/i, 'error.value'],
]

/** The engine's error text: an Error's message, or the value itself. */
export function errorText(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

/**
 * Where the editor's failures go: the engine's English never stands alone. A refusal it knows reads
 * as a short sentence (select a layer, unlock it, check the value); anything else as "this did not
 * finish". The engine's own text stays in the details.
 */
export function motionReport(t: Strings, commandFailed: string) {
  return (title: string, error: unknown) => {
    const text = errorText(error)
    const known = KNOWN.find(([pattern]) => pattern.test(text))
    if (known) toast('error', t(known[1]), text)
    else toast('error', title === commandFailed ? t('error.failed') : title, text)
  }
}
