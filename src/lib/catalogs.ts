import { bridge } from './bridge'
import type { ModelCatalog } from './types'

// Shared across panel mounts. Keep the last result visible during refresh and
// coalesce requests; creating many desks never means many discovery processes.
const catalogs = new Map<string, ModelCatalog>()
const pending = new Map<string, Promise<ModelCatalog>>()
export const catalogKey = (harness: string, cwd: string, profile: string) => JSON.stringify([harness, cwd, profile])
export const keptCatalog = (key: string) => catalogs.get(key)

export function readCatalog(harness: string, cwd: string, profile: string, refresh = false): Promise<ModelCatalog> {
  const key = catalogKey(harness, cwd, profile)
  const underway = pending.get(key)
  if (underway) return underway
  const request = bridge.modelCatalog(harness, cwd, profile, refresh).then(catalog => {
    if (catalogs.size >= 32 && !catalogs.has(key)) catalogs.delete(catalogs.keys().next().value!)
    catalogs.set(key, catalog)
    return catalog
  }).finally(() => pending.delete(key))
  pending.set(key, request)
  return request
}

export const lines = (text: string) => text.split(/\r?\n/).map(value => value.trim()).filter(Boolean)
