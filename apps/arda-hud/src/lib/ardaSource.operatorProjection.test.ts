import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it, vi } from 'vitest'
import { createCoreStateSource } from './ardaSource'
import { readFile, fetchInventoryTree } from './weathertop'

vi.mock('./ardaHudSettings', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./ardaHudSettings')>()
  return {
    ...actual,
    loadArdaHudSettings: vi.fn(async () => ({
      rootPath: '/arda',
      settingsPath: '/arda/config/arda_hud.settings.json',
      settings: actual.DEFAULT_ARDA_HUD_SETTINGS,
    })),
  }
})

vi.mock('./weathertop', () => ({ readFile: vi.fn(), fetchInventoryTree: vi.fn() }))

const mockedReadFile = vi.mocked(readFile)
const mockedFetchInventoryTree = vi.mocked(fetchInventoryTree)

function result(path: string, content: string | null) {
  return { success: content !== null, content, error: content === null ? 'not found' : null, path }
}

describe('ArdaBundle canonical operator projection', () => {
  it('loads and validates the owned read-only projection instead of deriving parallel HUD truth', async () => {
    const fixturePath = resolve(process.cwd(), '../../spec/operator-projection/v1/fixtures/valid-operator-projection.json')
    const fixture = readFileSync(fixturePath, 'utf8')
    mockedFetchInventoryTree.mockResolvedValue(result('/arda/tree', JSON.stringify({
      name: 'empty', relative_path: 'empty', path: '/arda/empty', is_dir: true, children: [],
    })))
    mockedReadFile.mockImplementation(async (path: string) => {
      if (path === '/arda/core/state/operator_projection.json') return result(path, fixture)
      if (path === '/arda/core/projects/tasks/queue.jsonl') return result(path, '')
      return result(path, null)
    })

    const bundle = await createCoreStateSource().loadBundle()

    expect(mockedReadFile).toHaveBeenCalledWith('/arda/core/state/operator_projection.json')
    expect(bundle.operatorProjection?.projection_id).toBe('projection-p9-fixture')
    expect(bundle.operatorProjection?.authority).toBe('read_only')
  })
})
