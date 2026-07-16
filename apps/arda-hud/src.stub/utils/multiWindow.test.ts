// sigil: REPAIR
import { afterEach, describe, expect, it, vi } from 'vitest'
import { getStoredWorkstationState, syncWorkstationState } from './multiWindow'

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}))

vi.mock('../lib/tauriGuard', () => ({
  safeTauriInvoke: vi.fn(() => Promise.reject(new Error('not tauri'))),
}))

const localStorageDescriptor = Object.getOwnPropertyDescriptor(window, 'localStorage')

afterEach(() => {
  if (localStorageDescriptor) {
    Object.defineProperty(window, 'localStorage', localStorageDescriptor)
  }
  window.localStorage.clear()
})

describe('multiWindow workstation storage bridge', () => {
  it('persists and reads workstation state when localStorage is available', () => {
    syncWorkstationState({
      workstationId: 'fleet-workstation',
      sourceZoneId: 'systems_health',
      activeModuleId: 'systems',
    })

    expect(getStoredWorkstationState('fleet-workstation')).toMatchObject({
      workstationId: 'fleet-workstation',
      sourceZoneId: 'systems_health',
      activeModuleId: 'systems',
    })
  })

  it('degrades without throwing when localStorage is restricted', () => {
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      get() {
        throw new Error('localStorage unavailable')
      },
    })

    expect(() => syncWorkstationState({ workstationId: 'restricted-storage' })).not.toThrow()
    expect(getStoredWorkstationState('restricted-storage')).toBeNull()
  })
})
