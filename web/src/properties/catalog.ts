import { createContext, useContext } from 'react'
import type { MotionEffect } from '../effects'

/** Every effect the engine has (`effect.list`), with its parameters: the panel's and the timeline's ranges and drop-down labels. */
export const EffectCatalog = createContext<readonly MotionEffect[]>([])

/** An effect (its id and parameters) by its id, or undefined when the catalog does not have it. */
export function useEffectOf() {
  const effects = useContext(EffectCatalog)
  return (id: string | undefined) => (id ? effects.find((effect) => effect.id === id) : undefined)
}
