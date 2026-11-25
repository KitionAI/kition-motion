import { useCallback, useEffect, useRef, useState } from 'react'
import { DiamondIcon as Diamond } from '@kition/tessera/icons/react'
import { Field, Select, useKitEditor } from '@kition/sdk/editor'
import type { MotionInfo, MotionLayer } from '../motion'
import { editableNumbers, type TreeNode } from '../timeline/tree'
import { cameraOrLight } from '../timeline/SwitchCells'
import { NumberField } from '../ValueField'
import { blendModes } from './LayerHeader'
import { nodeName, propMeta } from './meta'
import { Section } from './Section'

/** One property every selected layer has: its name and each layer's numbers (parts shown). */
type Common = { path: string; name: string; values: Array<{ layer: number; parts: number[]; whole: number[] | number; /** Whether the layer's property has keyframes. */ animated: boolean }>; decimals: number; unit?: string; min?: number; max?: number }

/** The Transform properties several layers are edited by together, in the panel's order. */
const ORDER = ['anchor', 'position', 'scale', 'orientation', 'rotationX', 'rotationY', 'rotation', 'opacity']
/** The ones only a 3D layer has a use for: shown when every selected layer is 3D. */
const THREE_D_ONLY = new Set(['orientation', 'rotationX', 'rotationY'])
/** Units by property, when its own say nothing: Anchor Point and Position in pixels, Orientation in degrees. */
const UNITS: Record<string, string> = { anchor: 'px', position: 'px', orientation: '°' }

/** Whether every value of part `index` is the same (to the field's decimals). */
const same = (values: Common['values'], index: number, decimals: number) => {
  const scale = 10 ** decimals
  const first = values[0]?.parts[index]
  return values.every((value) => Math.round((value.parts[index] ?? NaN) * scale) === Math.round((first ?? NaN) * scale))
}

let edits = 0

/**
 * The Properties panel with several layers selected: what they have in common, edited on all of
 * them at once. Blending mode and parent (a menu showing "mixed" where they differ); Transform's
 * properties every selected layer has (Anchor Point, Position, Scale, Rotation, Opacity, and a 3D
 * layer's Orientation and X/Y Rotation, with Rotation named Z Rotation, when all are 3D), each
 * number empty with a dash where the layers differ. A typed number goes to every layer (one undo step); a scrub moves each layer's
 * value by the same amount from where it was (one undo step per drag). Each change goes to the
 * engine as `prop.set` per layer with one `merge` key, so the history has one step.
 */
export function MultiLayers({ info, layers, readOnly }: { info: MotionInfo; layers: MotionLayer[]; readOnly: boolean }) {
  const { run, query, t, phrase, byId } = useKitEditor()
  const ids = layers.map((layer) => layer.id).join(',')
  const [common, setCommon] = useState<Common[]>([])
  // Each selected layer's Transform, read again whenever the document or the time changes.
  useEffect(() => {
    let current = true
    void Promise.all(layers.map((layer) => query<{ properties?: TreeNode }>('layer.tree', { layer: layer.id, depth: 3, time: info.time }).then((tree) => ({ layer, tree })))).then((trees) => {
      if (!current) return
      const groups = trees.map(({ layer, tree }) => ({ layer, transform: tree?.properties?.children?.find((node) => node.match === 'transform') }))
      if (groups.some((group) => !group.transform)) {
        setCommon([])
        return
      }
      const rows: Common[] = []
      const allThreeD = layers.every((layer) => layer.switches.threeD)
      for (const match of ORDER) {
        if (THREE_D_ONLY.has(match) && !allThreeD) continue
        const nodes = groups.map(({ layer, transform }) => ({ layer, node: transform?.children?.find((child) => child.match === match) }))
        if (nodes.some((each) => !each.node)) continue
        const values = nodes.map(({ layer, node }) => ({ layer: layer.id, parts: (node && editableNumbers(node, layer)) ?? [], whole: node?.value as number[] | number, animated: (node?.keys?.length ?? 0) > 0 }))
        const count = Math.min(...values.map((value) => value.parts.length))
        if (!(count > 0)) continue
        const first = nodes[0].node as TreeNode
        // Its Transform group as the parent: the units and decimals the single-layer panel shows.
        const meta = propMeta(first, groups[0].transform, undefined)
        // With every selected layer 3D, Rotation (beside X and Y Rotation) reads Z Rotation, as for one layer.
        rows.push({ path: first.path, name: nodeName(first, undefined, null, allThreeD, phrase, t), values: values.map((value) => ({ ...value, parts: value.parts.slice(0, count) })), decimals: meta?.decimals ?? 2, unit: meta?.unit ?? UNITS[match], min: meta?.min, max: meta?.max })
      }
      setCommon((before) => (JSON.stringify(before) === JSON.stringify(rows) ? before : rows))
    })
    return () => {
      current = false
    }
  }, [ids, info, query, phrase, t]) // eslint-disable-line react-hooks/exhaustive-deps

  // Every layer's new value of `row`, sent one after another under one merge key (one undo step);
  // while the engine is busy only the latest values wait.
  const pending = useRef<{ sets: Array<{ layer: number; path: string; value: number[] | number }>; merge: string } | null>(null)
  const busy = useRef(false)
  const flush = useCallback(async () => {
    if (busy.current) return
    busy.current = true
    while (pending.current) {
      const { sets, merge } = pending.current
      pending.current = null
      for (const set of sets) await run('prop.set', { layer: set.layer, path: set.path, value: set.value, time: info.time, timeBase: 'comp', merge }).catch(() => null)
    }
    busy.current = false
  }, [run, info.time])
  const send = (sets: Array<{ layer: number; path: string; value: number[] | number }>, merge: string) => {
    pending.current = { sets, merge }
    void flush()
  }
  /** A layer's value with part `index` set to `next` (a single number stays one). */
  const withPart = (value: Common['values'][number], index: number, next: number) => (Array.isArray(value.whole) ? value.whole.map((part, at) => (at === index ? next : part)) : next)
  // A scrub's starting values, kept for its gesture.
  const starts = useRef<{ gesture: string; values: Common['values'] } | null>(null)

  const modes = blendModes(byId.get('layer.setBlendMode')?.schema)
  const pixels = layers.filter((layer) => !cameraOrLight(layer))
  const mode = pixels.length && pixels.every((layer) => layer.blendMode === pixels[0].blendMode) ? pixels[0].blendMode : ''
  const parent = layers.every((layer) => layer.parent === layers[0].parent) ? (layers[0].parent == null ? '' : String(layers[0].parent)) : '*'
  const chosen = new Set(layers.map((layer) => layer.id))
  return (
    <>
      <div className="motion-layer-header" data-testid="motion-layer-header">
        <p className="motion-many kui-muted" data-testid="motion-properties-many">
          {t('props.manyEdit', { count: layers.length })}
        </p>
        <div className="motion-form-pair">
          {pixels.length ? (
            <Field label={t('props.blendMode')}>
              {({ id }) => (
                <Select id={id} value={mode} disabled={readOnly} onChange={(event) => event.target.value && void run('layer.setBlendMode', { layers: pixels.map((layer) => layer.id), mode: event.target.value })} data-testid="motion-multi-blend-mode">
                  {mode === '' ? <option value="">{t('props.mixed')}</option> : null}
                  {modes.map((each) => (
                    <option key={each} value={each}>
                      {phrase(each)}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          ) : null}
          <Field label={t('props.parent')}>
            {({ id }) => (
              <Select
                id={id}
                value={parent}
                disabled={readOnly}
                onChange={(event) => event.target.value !== '*' && void run('layer.setParent', { layers: layers.map((layer) => layer.id), parent: event.target.value === '' ? null : Number(event.target.value) })}
                data-testid="motion-multi-parent"
              >
                {parent === '*' ? <option value="*">{t('props.mixed')}</option> : null}
                <option value="">{t('props.noParent')}</option>
                {info.layers
                  .filter((other) => !chosen.has(other.id))
                  .map((other) => (
                    <option key={other.id} value={String(other.id)}>
                      {other.index}. {other.name}
                    </option>
                  ))}
              </Select>
            )}
          </Field>
        </div>
      </div>
      <Section scope="multi" id="transform" title={t('props.transformMany', { count: layers.length })} testId="motion-section-transform">
        <div className="motion-props">
          {common.map((row) => {
            const id = row.path.replace(/^transform\//, '')
            const parts = row.values[0]?.parts ?? []
            return (
              <div key={row.path} className="motion-prop" data-testid={`motion-multi-${id}`}>
                <MultiStopwatch row={row} readOnly={readOnly} testId={`motion-multi-${id}-key`} />
                <span className="motion-prop-name kui-muted">{row.name}</span>
                <span className="motion-prop-values">
                  {parts.map((first, index) => {
                    const mixed = !same(row.values, index, row.decimals)
                    return (
                      <NumberField
                        key={index}
                        value={first}
                        mixed={mixed}
                        label={parts.length > 1 ? `${row.name} ${'XYZ'[index]}` : row.name}
                        decimals={row.decimals}
                        min={row.min}
                        max={row.max}
                        // The unit once, after the last number.
                        unit={index === parts.length - 1 ? row.unit : undefined}
                        readOnly={readOnly}
                        onCommit={(next) => {
                          edits += 1
                          send(
                            row.values.map((value) => ({ layer: value.layer, path: row.path, value: withPart(value, index, next) })),
                            `multi-${Date.now()}-${edits}`,
                          )
                        }}
                        onScrub={(next, final, gesture) => {
                          if (starts.current?.gesture !== gesture) starts.current = { gesture, values: row.values }
                          const from = starts.current.values
                          const delta = next - (from[0]?.parts[index] ?? 0)
                          send(
                            from.map((value) => ({ layer: value.layer, path: row.path, value: withPart(value, index, (value.parts[index] ?? 0) + delta) })),
                            gesture,
                          )
                          if (final) starts.current = null
                        }}
                        testId={`motion-multi-${id}-${index}`}
                      />
                    )
                  })}
                </span>
              </div>
            )
          })}
          {common.length === 0 ? <p className="kui-muted motion-empty">{t('props.nothingCommon')}</p> : null}
        </div>
      </Section>
    </>
  )
}

/**
 * A row's stopwatch with several layers selected: on (accent) when every layer's property is
 * animated; a click turns animation on for all of them (a keyframe here where there was none), or
 * off for all when all were on, as one undo step (`prop.toggleAnimation` per layer, one `merge`).
 */
function MultiStopwatch({ row, readOnly, testId }: { row: Common; readOnly: boolean; testId: string }) {
  const { run, t } = useKitEditor()
  const all = row.values.length > 0 && row.values.every((value) => value.animated)
  const some = row.values.some((value) => value.animated)
  const label = t(all ? 'props.stopAnimatingMany' : 'props.animateMany', { name: row.name, count: row.values.length })
  return (
    <button
      type="button"
      className="motion-key-button"
      data-animated={all}
      data-mixed={!all && some ? true : undefined}
      aria-label={label}
      title={label}
      disabled={readOnly}
      onClick={() => {
        edits += 1
        const merge = `multi-key-${Date.now()}-${edits}`
        void (async () => {
          for (const value of row.values) await run('prop.toggleAnimation', { layer: value.layer, path: row.path, value: !all, merge }).catch(() => null)
        })()
      }}
      data-testid={testId}
    >
      <Diamond size={12} aria-hidden="true" />
    </button>
  )
}
