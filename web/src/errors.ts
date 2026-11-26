import { toast } from '@kition/sdk/editor'

type Strings = (key: string, values?: Record<string, string | number>) => string

/** Engine refusals people meet often, as the string key of a short sentence that says what to do. */
const KNOWN: ReadonlyArray<readonly [RegExp, string]> = [
  // Layer ▸ Time ▸ Enable Time Remapping (Cmd/Ctrl+Alt+T) on a layer that has no source time (layer.enableTimeRemap).
  [/time remapping needs a footage or composition layer/i, 'error.timeRemap'],
  // Animation ▸ Add Expression (Alt+Shift+=) with no property selected (prop.setExpression).
  [/select a property in the timeline/i, 'error.selectProperty'],
  // Layer ▸ Mask ▸ New Mask on a camera, light, null or 3D model (layer.addMask, mask.new).
  [/can't have masks/i, 'error.noMasks'],
  // Layer ▸ Layer Styles on a camera, light, null or adjustment layer (layer.style.*).
  [/no layer that takes layer styles/i, 'error.noStyles'],
  [/\block(ed)?\b/i, 'error.locked'],
  [/no (layers? )?(is |are )?selected|nothing (is )?selected|no selection|select (a|one|the|some)? ?(layer|layers)\b|needs? (a )?selected layer/i, 'error.select'],
  [/no effect (has been )?applied/i, 'error.noLastEffect'],
  // Wiggler and Smoother with too few keyframes of one property (keyframe.wiggle, keyframe.smooth).
  [/select at least 2 keyframes/i, 'error.twoKeys'],
  [/select at least 3 keyframes/i, 'error.threeKeys'],
  [/out of range|must be (positive|between|greater|less|at least|at most|finite)|not a (valid )?number|invalid value|too (large|small|long|short)/i, 'error.value'],
]

/** The editor's short sentence for an engine refusal it knows (a string key), or undefined. */
export function knownRefusal(text: string): string | undefined {
  return KNOWN.find(([pattern]) => pattern.test(text))?.[1]
}

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
    const known = title === commandFailed ? knownRefusal(text) : undefined
    if (known) {
      console.warn(`${title}: ${text}`)
      toast('error', t(known))
    } else if (title === commandFailed) failed(t, t('error.failed'), error, t('error.unchanged'))
    else failed(t, title, error)
  }
}

/**
 * Why a job failed, as a Render Queue row says it (its "Failed: …"): the editor's own words (a
 * [`Said`]), else the translated line that the reason is in the developer console, where the
 * engine's (or the host's) own text goes with the job's name.
 */
export function failedReason(t: Strings, job: string, error: unknown) {
  console.warn(`${job}: ${errorText(error)}`)
  return error instanceof Said ? error.message : t('error.details')
}

/**
 * What the status bar shows for a canvas tool's message: its hints are the editor's phrases
 * (`known`: the phrase table has them), shown translated; anything else is a command of the tool
 * the engine refused, in its own English, which goes to the developer console while the status
 * bar says the editor's sentence for it, or that the step did not work.
 */
export function toolStatus(t: Strings, phrase: (text: string) => string, known: (text: string) => boolean, text: string) {
  if (!text || known(text)) return phrase(text)
  console.warn(`${t('error.failed')}: ${text}`)
  const said = knownRefusal(text)
  return said ? t(said) : t('error.tool')
}
