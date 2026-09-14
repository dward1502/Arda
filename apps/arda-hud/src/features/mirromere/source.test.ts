import { describe, expect, it, vi } from 'vitest'
import { ambientIdleFixture } from './fixtures'
import { loadMirromereSurface } from './source'

const fixtureNow = new Date('2026-08-17T12:01:00Z')

describe('Mirromere runtime source', () => {
  it('invokes the bounded backend projection and parses runtime provenance', async () => {
    const invoke = vi.fn().mockResolvedValue({ ...ambientIdleFixture, source_mode: 'runtime' })
    const surface = await loadMirromereSurface(invoke, fixtureNow)
    expect(invoke).toHaveBeenCalledWith('get_mirromere_surface', { displayRole: 'hud_aperture' })
    expect(surface.source_mode).toBe('runtime')
  })


  it('never promotes a fixture response into runtime mode', async () => {
    const invoke = vi.fn().mockResolvedValue(ambientIdleFixture)
    await expect(loadMirromereSurface(invoke, fixtureNow)).rejects.toThrow(/fixture/i)
  })

  it('preserves backend failure instead of fabricating a fixture fallback', async () => {
    const invoke = vi.fn().mockRejectedValue(new Error('backend unavailable'))
    await expect(loadMirromereSurface(invoke, fixtureNow)).rejects.toThrow('backend unavailable')
  })
})
