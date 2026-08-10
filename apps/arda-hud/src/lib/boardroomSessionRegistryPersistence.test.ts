// sigil: REPAIR
import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  loadBoardroomSessionRegistry,
  readLocalBoardroomSessionRegistry,
  readLocalBoardroomSessionRegistryDocument,
  saveBoardroomSessionRegistryDocument,
} from './boardroomSessionRegistryPersistence'
import { createEmptyBoardroomSessionRegistry } from './boardroomSessionRegistry'
import { readFile, writeScopedFile } from './weathertop'

vi.mock('./weathertop', () => ({
  readFile: vi.fn(),
  writeScopedFile: vi.fn(),
}))

const mockedReadFile = vi.mocked(readFile)
const mockedWriteScopedFile = vi.mocked(writeScopedFile)

describe('boardroom session registry persistence', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('reads browser-local session registry document defensively', () => {
    const storage = { getItem: () => JSON.stringify({ schema_version: 'arda.boardroom.session_registry.v1', sessions: {} }) }
    const document = readLocalBoardroomSessionRegistryDocument(storage)
    expect(document?.schema_version).toBe('arda.boardroom.session_registry.v1')
    expect(readLocalBoardroomSessionRegistry({ getItem: () => '{broken' })).toEqual(createEmptyBoardroomSessionRegistry())
  })

  it('loads workspace session registry when core state file is available', async () => {
    const registry = { ...createEmptyBoardroomSessionRegistry('2026-07-30T15:00:00.000Z'), sessions: { monitor_1: { slot_id: 'monitor_1', kind: 'monitor', owner: 'hermes', opened_at_utc: '2026-07-30T15:00:00.000Z', lease_expires_at_utc: '2026-07-30T15:10:00.000Z', metadata: {} } } }
    mockedReadFile.mockResolvedValueOnce({ success: true, content: JSON.stringify(registry), error: null, path: 'core/state/arda_boardroom_session_registry.json' })
    const result = await loadBoardroomSessionRegistry('/arda')
    expect(result.mode).toBe('workspace')
    expect(result.registry.sessions.monitor_1.owner).toBe('hermes')
  })

  it('falls back when session registry core state file is unavailable', async () => {
    mockedReadFile.mockResolvedValueOnce({ success: false, content: null, error: 'missing core state file', path: 'core/state/arda_boardroom_session_registry.json' })
    const result = await loadBoardroomSessionRegistry('/arda')
    expect(result.mode).toBe('fallback')
    expect(Object.keys(result.registry.sessions)).toHaveLength(0)
  })

  it('saves session registry through scoped write contract', async () => {
    const registry = createEmptyBoardroomSessionRegistry('2026-07-30T16:00:00.000Z')
    mockedWriteScopedFile.mockResolvedValueOnce({ success: true, content: 'ok', error: null, path: 'core/state/arda_boardroom_session_registry.json' })
    await saveBoardroomSessionRegistryDocument('/arda', registry)
    expect(mockedWriteScopedFile).toHaveBeenCalledWith('/arda', 'core/state/arda_boardroom_session_registry.json', expect.stringContaining('arda.boardroom.session_registry.v1'))
  })
})
