import { invoke } from '@tauri-apps/api/core'
import { ingest } from './parser'
import type { ComponentType, ParsedData } from './types'

export type DataSourceType = 'file' | 'api' | 'tauri-command' | 'websocket'

export interface DataSourceConfig {
  type: DataSourceType
  endpoint: string
  pollInterval?: number
}

export interface TauriDataSource extends DataSourceConfig {
  type: 'tauri-command'
}

const COMPONENT_TYPES: ComponentType[] = [
  'kanban',
  'grid',
  'metric',
  'timeline',
  'graph3d',
  'document',
  'status-matrix',
  'stream-log',
  'chart',
  'list',
]

export function normalizeComponentType(value: string | null | undefined): ComponentType | 'auto' {
  if (!value) return 'auto'
  return COMPONENT_TYPES.includes(value as ComponentType) ? value as ComponentType : 'auto'
}

async function fetchFromTauriCommand(command: string, args?: Record<string, unknown>): Promise<string> {
  const result = await invoke<{ success: boolean; content?: string; error?: string }>(command, args)
  if (!result.success) {
    throw new Error(result.error || 'Command failed')
  }
  return result.content || '{}'
}

export async function loadFromFile(path: string, format?: 'auto' | 'json' | 'yaml' | 'markdown' | 'csv'): Promise<ParsedData> {
  const result = await invoke<{ success: boolean; content?: string; error?: string }>('read_file', { path })
  
  if (!result.success || !result.content) {
    throw new Error(result.error || 'Failed to read file')
  }
  
  return ingest({
    source: result.content,
    format: format || 'auto',
    componentType: 'auto',
  })
}

export async function loadFromTauri(command: string, componentType?: string): Promise<ParsedData> {
  const content = await fetchFromTauriCommand(command)
  
  return ingest({
    source: content,
    format: 'auto',
    componentType: normalizeComponentType(componentType),
  })
}

export async function loadFromUrl(url: string, componentType?: string): Promise<ParsedData> {
  return ingest({
    source: url,
    format: 'auto',
    componentType: normalizeComponentType(componentType),
  })
}

export function createPolledSource(
  sourceFn: () => Promise<ParsedData>,
  intervalMs: number
): { start: () => void; stop: () => void; onUpdate: (fn: (data: ParsedData) => void) => void } {
  let intervalId: ReturnType<typeof setInterval> | null = null
  const listeners: ((data: ParsedData) => void)[] = []
  
  return {
    start: () => {
      if (intervalId) return
      sourceFn().then(data => {
        listeners.forEach(fn => fn(data))
      })
      intervalId = setInterval(async () => {
        try {
          const data = await sourceFn()
          listeners.forEach(fn => fn(data))
        } catch (e) {
          console.error('Polled source error:', e)
        }
      }, intervalMs)
    },
    stop: () => {
      if (intervalId) {
        clearInterval(intervalId)
        intervalId = null
      }
    },
    onUpdate: (fn: (data: ParsedData) => void) => {
      listeners.push(fn)
    },
  }
}

export interface DataSourceManager {
  sources: Map<string, DataSourceConfig>
  register: (id: string, config: DataSourceConfig) => void
  unregister: (id: string) => void
  get: (id: string) => DataSourceConfig | undefined
  load: (id: string) => Promise<ParsedData>
}

export function createDataSourceManager(): DataSourceManager {
  const sources = new Map<string, DataSourceConfig>()
  
  return {
    sources,
    register: (id, config) => sources.set(id, config),
    unregister: (id) => sources.delete(id),
    get: (id) => sources.get(id),
    load: async (id) => {
      const config = sources.get(id)
      if (!config) throw new Error(`Unknown source: ${id}`)
      
      switch (config.type) {
        case 'file':
          return loadFromFile(config.endpoint)
        case 'api':
          return loadFromUrl(config.endpoint)
        case 'tauri-command':
          return loadFromTauri(config.endpoint)
        case 'websocket':
          throw new Error('WebSocket not implemented yet')
        default:
          throw new Error(`Unknown source type: ${config.type}`)
      }
    },
  }
}
