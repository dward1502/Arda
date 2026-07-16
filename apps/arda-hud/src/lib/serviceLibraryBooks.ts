// sigil: REPAIR
export type ServiceLibraryBookFreshness = 'fresh' | 'stale' | 'unknown'

export interface ServiceLibraryBook {
  schema_version: 'arda.arda.service_library_book.v1'
  id: string
  title: string
  provider: string
  canonical_url: string
  researched_by: 'athena' | 'operator' | 'imported'
  remembered_by: 'mnemosyne' | 'none'
  updated_at_utc: string
  freshness: ServiceLibraryBookFreshness
  summary: string
  adapter_hints: {
    source_zone_id: string
    suggested_preview_rows: Array<{ label: string; value: string }>
    supported_surfaces: Array<'monitor' | 'desk' | 'workstation' | 'world_window'>
    embed_policy: 'external_only' | 'inline_allowed' | 'blocked'
  }
  provenance: Array<{
    label: string
    url: string
    captured_at_utc: string
  }>
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value)
}

function stringValue(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string' && item.length > 0) : []
}

function previewRows(value: unknown): Array<{ label: string; value: string }> {
  return Array.isArray(value)
    ? value.map((row) => {
      const record = isRecord(row) ? row : {}
      return {
        label: stringValue(record.label),
        value: stringValue(record.value),
      }
    }).filter((row) => row.label.length > 0 && row.value.length > 0)
    : []
}

export function parseServiceLibraryBook(value: unknown): ServiceLibraryBook | null {
  if (!isRecord(value)) return null
  if (value.schema_version !== 'arda.arda.service_library_book.v1') return null
  const adapterHints = isRecord(value.adapter_hints) ? value.adapter_hints : {}
  const provenance = Array.isArray(value.provenance) ? value.provenance : []

  return {
    schema_version: 'arda.arda.service_library_book.v1',
    id: stringValue(value.id),
    title: stringValue(value.title),
    provider: stringValue(value.provider),
    canonical_url: stringValue(value.canonical_url),
    researched_by: value.researched_by === 'athena' || value.researched_by === 'operator' || value.researched_by === 'imported' ? value.researched_by : 'operator',
    remembered_by: value.remembered_by === 'mnemosyne' ? 'mnemosyne' : 'none',
    updated_at_utc: stringValue(value.updated_at_utc),
    freshness: value.freshness === 'fresh' || value.freshness === 'stale' || value.freshness === 'unknown' ? value.freshness : 'unknown',
    summary: stringValue(value.summary),
    adapter_hints: {
      source_zone_id: stringValue(adapterHints.source_zone_id),
      suggested_preview_rows: previewRows(adapterHints.suggested_preview_rows),
      supported_surfaces: stringArray(adapterHints.supported_surfaces).filter((surface): surface is 'monitor' | 'desk' | 'workstation' | 'world_window' => (
        surface === 'monitor' || surface === 'desk' || surface === 'workstation' || surface === 'world_window'
      )),
      embed_policy: adapterHints.embed_policy === 'inline_allowed' || adapterHints.embed_policy === 'blocked' ? adapterHints.embed_policy : 'external_only',
    },
    provenance: provenance.map((entry) => {
      const record = isRecord(entry) ? entry : {}
      return {
        label: stringValue(record.label),
        url: stringValue(record.url),
        captured_at_utc: stringValue(record.captured_at_utc),
      }
    }).filter((entry) => entry.label.length > 0 && entry.url.length > 0),
  }
}
