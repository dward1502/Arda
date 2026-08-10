// sigil: REPAIR
import { parseJsonOrNull } from './jsonParse'
import { readFile, writeScopedFile, type FileReadResult } from './weathertop'
import {
  createEmptyBoardroomSessionRegistry,
  parseBoardroomSessionRegistry,
  type BoardroomSessionRegistry,
} from './boardroomSessionRegistry'

export const ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH = 'core/state/arda_boardroom_session_registry.json'
export const ARDA_BOARDROOM_SESSION_STORAGE_KEY = 'arda.boardroom.session_registry.v1'

export function readLocalBoardroomSessionRegistryDocument(storage: Pick<Storage, 'getItem'> | null | undefined) {
  try {
    const raw = storage?.getItem(ARDA_BOARDROOM_SESSION_STORAGE_KEY)
    return raw ? parseJsonOrNull<unknown>(raw) : null
  } catch {
    return null
  }
}

export function readLocalBoardroomSessionRegistry(storage: Pick<Storage, 'getItem'> | null | undefined) {
  const raw = readLocalBoardroomSessionRegistryDocument(storage)
  const parsed = parseBoardroomSessionRegistry(raw)
  return parsed ?? createEmptyBoardroomSessionRegistry()
}

export async function loadBoardroomSessionRegistry(rootPath: string) {
  const settingsPath = `${rootPath}/${ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH}`
  const result = await readFile(settingsPath)
  if (!result.success || !result.content) {
    return {
      mode: 'fallback' as const,
      registry: createEmptyBoardroomSessionRegistry(),
      message: result.error ?? 'workspace boardroom session registry unavailable',
    }
  }

  try {
    const parsed = parseBoardroomSessionRegistry(parseJsonOrNull<unknown>(result.content))
    if (!parsed) throw new Error('invalid boardroom session registry schema')
    return {
      mode: 'workspace' as const,
      registry: parsed,
      message: `loaded ${ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH}`,
    }
  } catch (error) {
    return {
      mode: 'fallback' as const,
      registry: createEmptyBoardroomSessionRegistry(),
      message: error instanceof Error ? error.message : 'invalid boardroom session registry',
    }
  }
}

export async function saveBoardroomSessionRegistryDocument(rootPath: string, registry: BoardroomSessionRegistry) {
  return writeScopedFile(rootPath, ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH, JSON.stringify(registry, null, 2))
}
