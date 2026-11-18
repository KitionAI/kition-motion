import { useEffect, useState } from 'react'
import { DiamondIcon as Diamond } from '@kition/tessera-icons/react'
import { PanelSection, useKitEditor, useReadOnly } from '@kition/kit-editor'
import type { MotionInfo, MotionProperty } from './motion'

const round = (value: number) => Math.round(value * 100) / 100

/** One number of a property; commits on Enter or blur. */
function NumberField({ value, label, onCommit, testId, readOnly }: { value: number; label: string; onCommit: (value: number) => void; testId: string; readOnly: boolean }) {
  const [text, setText] = useState(String(round(value)))
  useEffect(() => setText(String(round(value))), [value])
  const commit = () => {
    const next = Number(text)
    if (Number.isFinite(next) && next !== round(value)) onCommit(next)
    else setText(String(round(value)))
  }
  return (
    <input
      className="kui-field motion-number"
      aria-label={label}
      value={text}
      inputMode="decimal"
      readOnly={readOnly}
      aria-readonly={readOnly}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === 'Enter') event.currentTarget.blur()
        if (event.key === 'Escape') {
          // The typed value is dropped: the editor (and Focus) does not hear this Escape as well.
          event.preventDefault()
          setText(String(round(value)))
          event.currentTarget.blur()
        }
      }}
      data-testid={testId}
    />
  )
}

/** How many numbers a property shows: 2D layers show x and y of a 3D value. */
const dims = (property: MotionProperty) => (Array.isArray(property.value) ? Math.min(2, property.value.length) : 1)

/**
 * The selected layer's transform at the current time, as in After Effects' Transform group: edit
 * a value, or add a keyframe here with the diamond. Once a property has keyframes, edits set a
 * keyframe at the current time.
 */
export function PropertiesPanel({ info }: { info: MotionInfo }) {
  const { run, t } = useKitEditor()
  // A viewer sees the values but cannot change them or add keyframes.
  const readOnly = useReadOnly()
  const transform = info.transform
  const layer = transform ? info.layers.find((item) => item.id === transform.layer) : null
  const set = (property: MotionProperty, index: number, next: number) => {
    const value = Array.isArray(property.value) ? property.value.map((part, at) => (at === index ? next : part)) : next
    void run('prop.set', { layer: transform?.layer, path: property.path, value, time: info.time, timeBase: 'comp' })
  }
  return (
    <aside className="kui-panel motion-side" data-side="right" data-testid="motion-properties-panel">
      <div className="kui-panel-body">
        <PanelSection title={layer ? `${t('properties')} · ${layer.name}` : t('properties')}>
          {transform && layer ? (
            <div className="motion-props">
              {transform.properties.map((property) => {
                const id = property.path.split('/').pop()
                return (
                  <div key={property.path} className="motion-prop" data-testid={`motion-prop-${id}`}>
                    <button
                      type="button"
                      className="motion-key-button"
                      data-animated={property.animated}
                      aria-label={t('addKey', { name: property.name })}
                      title={t('addKey', { name: property.name })}
                      disabled={readOnly}
                      onClick={() => void run('prop.addKey', { layer: transform.layer, path: property.path, time: info.time, timeBase: 'comp' })}
                      data-testid={`motion-add-key-${id}`}
                    >
                      <Diamond size={12} aria-hidden="true" />
                    </button>
                    <span className="motion-prop-name kui-muted">{property.name}</span>
                    <span className="motion-prop-values">
                      {Array.from({ length: dims(property) }, (_, index) => (
                        <NumberField
                          key={index}
                          value={Array.isArray(property.value) ? (property.value[index] ?? 0) : property.value}
                          label={`${property.name} ${index + 1}`}
                          onCommit={(next) => set(property, index, next)}
                          testId={`motion-value-${id}-${index}`}
                          readOnly={readOnly}
                        />
                      ))}
                    </span>
                  </div>
                )
              })}
            </div>
          ) : (
            <p className="kui-muted motion-empty">{t('selectLayer')}</p>
          )}
        </PanelSection>
      </div>
    </aside>
  )
}
