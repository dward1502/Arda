import { beforeEach, describe, expect, it, vi } from 'vitest'
import { loadManweLiveSnapshot } from './manweLive'

const mockedFetch = vi.fn()

beforeEach(() => {
  vi.stubGlobal('fetch', mockedFetch)
  mockedFetch.mockReset()
})

describe('Manwe runtime projection browser development fallback', () => {
  it('preserves observed sources when one endpoint is unavailable', async () => {
    mockedFetch.mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url.endsWith('/providers/capabilities')) {
        throw new Error('capabilities offline')
      }
      return {
        ok: true,
        json: async () => url.endsWith('/healthz')
          ? { ok: true, providers_healthy: 3 }
          : { ok: true, promotion_guard: { candidates: [], generated_at_utc: '2026-08-11T10:00:00Z' } },
      } as Response
    })

    const projection = await loadManweLiveSnapshot()

    expect(projection.schemaVersion).toBe('arda.system-health.manwe.v1')
    expect(projection.state).toBe('partial')
    expect(projection.health?.providers_healthy).toBe(3)
    expect(projection.capabilities).toBeNull()
    expect(projection.providerCandidates?.ok).toBe(true)
    expect(projection.sources.find((source) => source.sourceId === 'capabilities')).toMatchObject({
      state: 'unavailable',
      error: 'capabilities offline',
    })
    expect(projection.recoveryAction).toMatch(/Restore the unavailable Manwe projection source/)
  })
})
