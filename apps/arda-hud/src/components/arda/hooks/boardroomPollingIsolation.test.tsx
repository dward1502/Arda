// sigil: REPAIR
import { renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { loadManweLiveSnapshot } from '../../../lib/manweLive'
import { listenHudPulse, startHudPulseStream } from '../../../lib/weathertop'
import { useArdaRuntimePulse } from './useArdaRuntimePulse'
import { useManweLiveSnapshot } from './useManweLiveSnapshot'

vi.mock('../../../lib/manweLive', () => ({
  loadManweLiveSnapshot: vi.fn(),
}))

vi.mock('../../../lib/weathertop', () => ({
  listenHudPulse: vi.fn(),
  startHudPulseStream: vi.fn(),
  stopHudPulseStream: vi.fn(),
}))

describe('boardroom polling isolation', () => {
  it('does not poll Manwe while its detail surface is inactive', () => {
    renderHook(() => useManweLiveSnapshot(5000, false))
    expect(loadManweLiveSnapshot).not.toHaveBeenCalled()
  })

  it('does not attach the runtime pulse while its detail surface is inactive', () => {
    renderHook(() => useArdaRuntimePulse(false))
    expect(listenHudPulse).not.toHaveBeenCalled()
    expect(startHudPulseStream).not.toHaveBeenCalled()
  })
})