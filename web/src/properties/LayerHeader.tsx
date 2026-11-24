import { cameraOrLight } from '../timeline/SwitchCells'
import { useEffect, useRef, useState } from 'react'
import { Field, Select, TextInput, useKitEditor } from '@kition/sdk/editor'
import type { MotionInfo, MotionLayer } from '../motion'

/**
 * The top of the Properties panel: the layer's name (typed in place), its type, its blending
 * mode, and the layer it follows (its parent), each change one undo step.
 */
/** Where the blending-mode menu starts a new group (the modes themselves come from the engine). */
const GROUP_STARTS = new Set(['Darken', 'Add', 'Overlay', 'Difference', 'Hue', 'Stencil Alpha'])

/** The blending modes `layer.setBlendMode` takes, from its parameter schema, in the engine's order. */
export function blendModes(schema: Record<string, unknown> | null | undefined): string[] {
  const mode = (schema as Record<string, { enum?: unknown }> | null | undefined)?.mode
  return Array.isArray(mode?.enum) ? mode.enum.filter((value): value is string => typeof value === 'string') : []
}

export function LayerHeader({ info, layer, readOnly, count }: { info: MotionInfo; layer: MotionLayer; readOnly: boolean; count: number }) {
  const { run, t, phrase, byId } = useKitEditor()
  const modes = blendModes(byId.get('layer.setBlendMode')?.schema)
  const [name, setName] = useState(layer.name)
  const dropped = useRef(false)
  useEffect(() => setName(layer.name), [layer.name, layer.id])
  const others = info.layers.filter((other) => other.id !== layer.id)
  return (
    <div className="motion-layer-header" data-testid="motion-layer-header">
      {count > 1 ? (
        <p className="motion-many kui-muted" data-testid="motion-properties-many">
          {t('props.many', { count })}
        </p>
      ) : null}
      <div className="motion-layer-title">
        <TextInput
          value={name}
          aria-label={t('props.layerName')}
          readOnly={readOnly}
          onChange={(event) => setName(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') event.currentTarget.blur()
            if (event.key === 'Escape') {
              event.preventDefault()
              dropped.current = true
              setName(layer.name)
              event.currentTarget.blur()
            }
          }}
          onBlur={() => {
            const next = name.trim()
            if (!dropped.current && next && next !== layer.name) void run('layer.rename', { layer: layer.id, name: next })
            else setName(layer.name)
            dropped.current = false
          }}
          data-testid="motion-layer-name"
        />
        <span className="motion-layer-type kui-muted" data-testid="motion-layer-type">
          {phrase(layer.type)}
        </span>
      </div>
      <div className="motion-form-pair">
        {cameraOrLight(layer) ? null : (
          <Field label={t('props.blendMode')}>
            {({ id }) => (
              <Select id={id} value={layer.blendMode} disabled={readOnly} onChange={(event) => void run('layer.setBlendMode', { layers: [layer.id], mode: event.target.value })} data-testid="motion-blend-mode">
                {modes.flatMap((mode) => [
                  ...(GROUP_STARTS.has(mode)
                    ? [
                        <option key={`--${mode}`} disabled>
                          ──────
                        </option>,
                      ]
                    : []),
                  <option key={mode} value={mode}>
                    {phrase(mode)}
                  </option>,
                ])}
              </Select>
            )}
          </Field>
        )}
        <Field label={t('props.parent')}>
          {({ id }) => (
            <Select
              id={id}
              value={layer.parent == null ? '' : String(layer.parent)}
              disabled={readOnly}
              onChange={(event) => void run('layer.setParent', { layers: [layer.id], parent: event.target.value === '' ? null : Number(event.target.value) })}
              data-testid="motion-parent"
            >
              <option value="">{t('props.noParent')}</option>
              {others.map((other) => (
                <option key={other.id} value={String(other.id)}>
                  {other.index}. {other.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
      </div>
    </div>
  )
}
