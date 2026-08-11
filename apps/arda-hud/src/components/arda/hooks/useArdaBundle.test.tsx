// sigil: REPAIR
import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ArdaBundle, ArdaDataSource } from '../../../lib/ardaBundleTypes'
import { useArdaBundle } from './useArdaBundle'

function createSource() {
  const bundle = { sections: [{ id: 'now' }] } as unknown as ArdaBundle
  const loadBundle = vi.fn().mockResolvedValue(bundle)
  return { bundle, loadBundle, source: { loadBundle } as unknown as ArdaDataSource }
}

describe('useArdaBundle', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('loads once by default instead of polling the complete projection inventory', async () => {
    vi.useFakeTimers()
    const { loadBundle, source } = createSource()
    renderHook(() => useArdaBundle({ source }))

    await act(async () => Promise.resolve())
    expect(loadBundle).toHaveBeenCalledTimes(1)

    await act(async () => vi.advanceTimersByTimeAsync(20_000))
    expect(loadBundle).toHaveBeenCalledTimes(1)
  })

  it('supports an explicit periodic refresh for consumers that need it', async () => {
    const { bundle, loadBundle, source } = createSource()
    const { result } = renderHook(() => useArdaBundle({ source, refreshIntervalMs: 25 }))

    await waitFor(() => expect(result.current.bundle).toBe(bundle))
    await waitFor(() => expect(loadBundle.mock.calls.length).toBeGreaterThan(1))
  })
})