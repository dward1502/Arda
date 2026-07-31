// sigil: REPAIR
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { writeScopedFile } from './weathertop'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)

describe('writeScopedFile', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
  })

  it('uses the native numenorPath command argument for the workspace root', async () => {
    mockedInvoke.mockResolvedValueOnce({ success: true, content: 'File written', error: null, path: '/arda/core/state/settings.json' })

    await writeScopedFile('/arda', 'core/state/settings.json', '{}\n')

    expect(mockedInvoke).toHaveBeenCalledWith('write_scoped_file', {
      numenorPath: '/arda',
      relativePath: 'core/state/settings.json',
      content: '{}\n',
    })
  })
})
