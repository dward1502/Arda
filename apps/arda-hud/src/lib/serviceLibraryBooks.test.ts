// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { parseServiceLibraryBook } from './serviceLibraryBooks'

describe('service library books', () => {
  it('parses ATHENA/Mnemosyne service adapter context', () => {
    const parsed = parseServiceLibraryBook({
      schema_version: 'arda.arda.service_library_book.v1',
      id: 'factory-ai-2026-05-22',
      title: 'Factory AI',
      provider: 'Factory',
      canonical_url: 'https://factory.ai/',
      researched_by: 'athena',
      remembered_by: 'mnemosyne',
      updated_at_utc: '2026-05-22T00:00:00Z',
      freshness: 'fresh',
      summary: 'Agentic development platform reference.',
      adapter_hints: {
        source_zone_id: 'service_factory_ai',
        suggested_preview_rows: [{ label: 'type', value: 'agent platform' }],
        supported_surfaces: ['monitor', 'workstation'],
        embed_policy: 'external_only',
      },
      provenance: [{ label: 'Factory', url: 'https://factory.ai/', captured_at_utc: '2026-05-22T00:00:00Z' }],
    })

    expect(parsed?.researched_by).toBe('athena')
    expect(parsed?.remembered_by).toBe('mnemosyne')
    expect(parsed?.adapter_hints.source_zone_id).toBe('service_factory_ai')
  })

  it('rejects unknown schemas', () => {
    expect(parseServiceLibraryBook({ schema_version: 'other' })).toBeNull()
  })
})
