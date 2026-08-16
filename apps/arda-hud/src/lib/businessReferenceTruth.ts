import type { JsonRecord } from './ardaBundleTypes'

function asRecord(value: unknown): JsonRecord | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as JsonRecord : null
}

export function reconcileBusinessRuntimeReferences(snapshot: JsonRecord, derived: JsonRecord): JsonRecord {
  const highlights = asRecord(derived.highlights)
  const livePaths = new Set(Array.isArray(highlights?.client_paths)
    ? highlights.client_paths.filter((path): path is string => typeof path === 'string')
    : [])
  const records = Array.isArray(snapshot.client_records) ? snapshot.client_records : []
  const companyOps = asRecord(snapshot.company_ops)
  const liveProjectPaths = new Set(Array.isArray(highlights?.project_paths)
    ? highlights.project_paths.filter((path): path is string => typeof path === 'string')
    : [])
  const projects = Array.isArray(companyOps?.projects) ? companyOps.projects : []
  return {
    ...snapshot,
    client_records: records.map((entry) => {
      const record = asRecord(entry)
      if (!record) return entry
      const path = typeof record.path === 'string' ? record.path : ''
      return { ...record, exists: path.length > 0 && livePaths.has(path) }
    }),
    ...(companyOps ? {
      company_ops: {
        ...companyOps,
        projects: projects.map((entry) => {
          const record = asRecord(entry)
          if (!record) return entry
          const path = typeof record.path === 'string' ? record.path : ''
          return path ? { ...record, exists: liveProjectPaths.has(path) } : record
        }),
      },
    } : {}),
  }
}