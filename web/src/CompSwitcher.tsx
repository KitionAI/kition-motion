import { Select, useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import type { MotionInfo } from './motion'

type Summary = { activeComp?: number | null; items?: Array<{ id: number; name: string; type: string }> }

/** The active composition's name as a drop-down of every composition in the project; choosing one opens it. */
export function CompSwitcher({ info }: { info: MotionInfo }) {
  const { run, t } = useKitEditor()
  const summary = useEngineQuery<Summary>('project.summary', {})
  const comps = (summary?.items ?? []).filter((item) => item.type === 'Composition')
  const active = info.comp?.id
  if (!info.comp) return null
  return (
    <Select
      className="motion-comp-switcher"
      aria-label={t('compSwitcher')}
      title={t('compSwitcher')}
      value={String(active)}
      onChange={(event) => void run('comp.open', { comp: Number(event.target.value) })}
      data-testid="motion-comp-switcher"
    >
      {/* Until the project's list arrives, the active composition alone. */}
      {(comps.length ? comps : [{ id: info.comp.id, name: info.comp.name }]).map((comp) => (
        <option key={comp.id} value={String(comp.id)}>
          {comp.name}
        </option>
      ))}
    </Select>
  )
}
