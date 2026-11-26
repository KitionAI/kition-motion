import { toast } from '@kition/sdk/editor'

type Strings = (key: string, values?: Record<string, string | number>) => string

/** Engine refusals people meet often, as the string key of a short sentence that says what to do. */
const KNOWN: ReadonlyArray<readonly [RegExp, string]> = [
  [/\block(ed)?\b/i, 'error.locked'],
  [/no (layers? )?(is |are )?selected|nothing (is )?selected|no selection|select (a|one|the|some)? ?(layer|layers)\b|needs? (a )?selected layer/i, 'error.select'],
  [/no effect (has been )?applied/i, 'error.noLastEffect'],
  // Wiggler and Smoother with too few keyframes of one property (keyframe.wiggle, keyframe.smooth).
  [/select at least 2 keyframes/i, 'error.twoKeys'],
  [/select at least 3 keyframes/i, 'error.threeKeys'],
  [/out of range|must be (positive|between|greater|less|at least|at most|finite)|not a (valid )?number|invalid value|too (large|small|long|short)/i, 'error.value'],
]

/** The engine's error text: an Error's message, or the value itself. */
export function errorText(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

/**
 * A failure the editor shows, in the interface's language only: the engine's (or the host's) own
 * text is English and may name a command, so it goes to the developer console, never into the
 * toast. The toast is `title` (what failed) and `detail`, a translated generic line (or the
 * message of a [`Said`], the editor's own words).
 */
export function failed(t: Strings, title: string, error: unknown, detail = t('error.details')) {
  console.warn(`${title}: ${errorText(error)}`)
  toast('error', title, error instanceof Said ? error.message : detail)
}

/** A failure the editor words itself, in the interface's language: [`failed`] shows its message. */
export class Said extends Error {}

/**
 * Where the SDK frame's failures go (`report`), as [`failed`] shows them. A refusal of a command
 * the editor knows reads as its short sentence alone (select a layer, unlock it, check the
 * value); any other refused command as "this did not finish" over "nothing was changed" (a
 * refused command changes nothing); a failed save or other step keeps its own title.
 */
export function motionReport(t: Strings, commandFailed: string) {
  return (title: string, error: unknown) => {
    const text = errorText(error)
    const known = title === commandFailed ? KNOWN.find(([pattern]) => pattern.test(text)) : undefined
    if (known) {
      console.warn(`${title}: ${text}`)
      toast('error', t(known[1]))
    } else if (title === commandFailed) failed(t, t('error.failed'), error, t('error.unchanged'))
    else failed(t, title, error)
  }
}
