import { useCallback } from 'react'
import { toast, type HostConnection } from '@kition/sdk/editor'
import { failed } from './errors'

/**
 * Importing footage. Files go only through the host: File ▸ Import asks the host to pick one in
 * the workspace (`host/pick`); a file dropped from the computer is first written beside the
 * project through the host (`host/write`), since a kit never names a path outside the workspace.
 * The engine then imports it by its path from the project's folder (`file.import`) and reads it
 * as a lent file (`kition_kit::host`).
 */

/** What Import offers: the pictures, movies and sounds the web build reads. */
export const IMPORT_TYPES = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'mp4', 'mov', 'm4v', 'webm', 'mp3', 'wav']

/** The MIME type a project item carries while it is dragged onto the timeline or the composition. */
export const ITEM_MIME = 'application/x-kition-motion-item'

/** A footage item's file and how it is interpreted (File ▸ Interpret Footage), as `project.summary` lists it. */
export type FootageInfo = {
  path: string
  missing: boolean
  hasVideo: boolean
  hasAudio: boolean
  alpha: 'straight' | 'premultiplied' | 'ignore'
  loop: number
  pixelAspect: number
  /** The file's own frame rate. */
  nativeFrameRate: number
  /** Whether it plays at another rate than the file's (`frameRate`). */
  conformed: boolean
}

/** A project item as `project.summary` lists it; `uses` counts the layers (in every composition) using it. */
export type ProjectItem = { id: number; name: string; type: string; parent: number | null; /** Its colour label by the engine's name. */ label?: string; size?: [number, number]; duration?: number; frameRate?: number; layers?: number; uses?: number; footage?: FootageInfo }

/** `selection`: the Project panel's selected items (the engine's, `project.select`). */
export type ProjectSummary = { activeComp?: number | null; items?: ProjectItem[]; selection?: number[] }

/** The MIME type project items carry while they are dragged inside the Project panel (into a folder): their ids, as JSON. */
export const ITEMS_MIME = 'application/x-kition-motion-items'

/** Items a composition can hold as layers (the rest are folders). */
export const isLayerSource = (item: ProjectItem) => item.type !== 'Folder'

/** `to` named from `folder` (both workspace paths): `footage/a.png`, `../b.mov`. */
export function relativePath(folder: string, to: string) {
  const parts = (path: string) => path.replace(/\\/g, '/').split('/').filter(Boolean)
  const from = parts(folder)
  const target = parts(to)
  let shared = 0
  while (shared < from.length && shared < target.length - 1 && from[shared] === target[shared]) shared += 1
  return [...from.slice(shared).map(() => '..'), ...target.slice(shared)].join('/')
}

/** A command's answer (JSON text), or null when it is not JSON. */
export function parseAnswer<T>(text: string | null): T | null {
  if (!text) return null
  try {
    return JSON.parse(text) as T
  } catch {
    return null
  }
}

const extension = (name: string) => name.split('.').pop()?.toLowerCase() ?? ''

/** Where `layer.addItem` puts a new layer: its in point, and a point in the composition. */
export type Placement = { time?: number; position?: [number, number] }

type ImportAnswer = { items?: number[]; errors?: string[]; warnings?: string[] }

/**
 * Import from the host's picker, dropped files, and project items placed as layers, each with a
 * message the person can act on: what was imported, or why a file was not (missing, a format this
 * build cannot read).
 */
export function useFootage({ host, run, t }: { host: HostConnection; run: (command: string, params?: Record<string, unknown>) => Promise<string | null>; t: (key: string, values?: Record<string, string | number>) => string }) {
  const folder = host.init.document.path.split('/').slice(0, -1).join('/')

  /** Imports workspace `paths`; with `placement`, each becomes a layer of the active composition too. */
  const importPaths = useCallback(
    async (paths: string[], placement?: Placement) => {
      const names = paths.map((path) => path.split('/').pop() ?? path)
      const text = await run('file.import', { paths: paths.map((path) => relativePath(folder, path)) })
      if (!text) return []
      const answer = parseAnswer<ImportAnswer>(text) ?? {}
      const items = answer.items ?? []
      for (const error of answer.errors ?? []) {
        const name = names.find((each) => error.includes(each)) ?? error.split(':')[0]
        const known = IMPORT_TYPES.includes(extension(name))
        // A movie in a codec the build names but cannot decode (VP8) says so, with what to do.
        const codec = /cannot be decoded/.test(error) ? /\b(VP8)\b/.exec(error)?.[1] : undefined
        failed(t, t(codec ? 'import.codec' : known ? 'import.unreadable' : 'import.unsupported', { name, codec: codec ?? '' }), error)
      }
      if (placement) for (const item of items) await run('layer.addItem', { item, ...placement })
      if (items.length) toast.success(items.length === 1 ? t('import.done', { name: names[0] }) : t('import.doneMany', { count: items.length }))
      return items
    },
    [folder, run, t],
  )

  /** File ▸ Import: a file the person picks in the workspace. */
  const importPicked = useCallback(async () => {
    if (!host.init.capabilities.includes('pick-file')) {
      toast.error(t('import.noPicker'))
      return
    }
    let path: string | null = null
    try {
      path = await host.pick('import', IMPORT_TYPES)
    } catch (error) {
      failed(t, t('import.failed'), error)
      return
    }
    if (path) await importPaths([path])
  }, [host, importPaths, t])

  /** Files dropped from the computer: written beside the project through the host, then imported (as layers when `placement` is given). */
  const importDropped = useCallback(
    async (files: readonly File[], placement?: Placement) => {
      const paths: string[] = []
      for (const file of files) {
        if (!IMPORT_TYPES.includes(extension(file.name))) {
          toast.error(t('import.unsupported', { name: file.name }), { description: t('import.types', { types: IMPORT_TYPES.join(', ') }) })
          continue
        }
        try {
          const bytes = new Uint8Array(await file.arrayBuffer())
          paths.push(await host.exportFile([folder, file.name].filter(Boolean).join('/'), bytes))
        } catch (error) {
          failed(t, t('import.failedFile', { name: file.name }), error)
        }
      }
      if (paths.length) await importPaths(paths, placement)
    },
    [folder, host, importPaths, t],
  )

  /** A project item as a new layer of the active composition (one undo step). */
  const place = useCallback((item: number, placement: Placement) => void run('layer.addItem', { item, ...placement }), [run])

  /**
   * Replace Footage ▸ File…: a file the person picks in the workspace takes footage `item`'s
   * place, in every layer using it (one undo step). Answers whether it was replaced.
   */
  const replacePicked = useCallback(
    async (item: number, name: string) => {
      if (!host.init.capabilities.includes('pick-file')) {
        toast.error(t('import.noPicker'))
        return false
      }
      let path: string | null = null
      try {
        path = await host.pick('replace', IMPORT_TYPES)
      } catch (error) {
        failed(t, t('replace.failed', { name }), error)
        return false
      }
      if (!path) return false
      const done = (await run('file.replaceFootage', { item, path: relativePath(folder, path) })) !== null
      if (done) toast.success(t('replace.done', { name, file: path.split('/').pop() ?? path }))
      return done
    },
    [folder, host, run, t],
  )

  return { importPicked, importDropped, importPaths, place, replacePicked }
}
