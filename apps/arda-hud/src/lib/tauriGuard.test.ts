// sigil: REPAIR
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { requireTauri, safeTauriInvoke, TauriUnavailableError } from './tauriGuard'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)

beforeEach(() => {
  mockedInvoke.mockReset()
  delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
})

describe('tauriGuard', () => {
  it('throws structured TauriUnavailableError without console warnings outside Tauri', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined)

    expect(() => requireTauri('open settings')).toThrow(TauriUnavailableError)
    await expect(safeTauriInvoke('open_settings_window')).rejects.toMatchObject({
      name: 'TauriUnavailableError',
      code: 'TAURI_UNAVAILABLE',
      operation: 'safeTauriInvoke',
      command: 'open_settings_window',
      environment: 'browser',
    })
    expect(warnSpy).not.toHaveBeenCalled()

    warnSpy.mockRestore()
  })

  it('delegates to Tauri invoke when desktop internals are present', async () => {
    ;(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {}
    mockedInvoke.mockResolvedValueOnce({ ok: true })

    await expect(safeTauriInvoke('read_status', { path: 'core/state/status.json' })).resolves.toEqual({ ok: true })
    expect(mockedInvoke).toHaveBeenCalledWith('read_status', { path: 'core/state/status.json' })
  })
})
