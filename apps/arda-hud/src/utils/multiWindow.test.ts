// sigil: REPAIR
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  getStoredWorkstationState,
  syncWorkstationState,
  windowManager,
} from './multiWindow'
import { safeTauriInvoke } from '../lib/tauriGuard'

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
  vi.clearAllMocks()
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

  it('wraps native workstation fields under the Rust request argument', async () => {
    vi.mocked(safeTauriInvoke).mockResolvedValueOnce('monitor-workstation-session-1')

    windowManager.open({
      id: 'monitor-workstation-session-1',
      title: 'Monitor session',
      windowRole: 'workstation',
      workstationId: 'monitor-session:session-1',
      sourceZoneId: 'monitor_1',
      originAnchorId: 'monitor_1',
      presentationMode: 'native_window',
      width: 1280,
      height: 800,
    })
    await Promise.resolve()

    expect(safeTauriInvoke).toHaveBeenCalledWith('open_workstation_window', {
      request: expect.objectContaining({
        window_label: 'monitor-workstation-session-1',
        workstation_id: 'monitor-session:session-1',
        source_zone_id: 'monitor_1',
      }),
    })
  })
})
