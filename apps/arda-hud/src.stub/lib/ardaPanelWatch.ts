// sigil: REPAIR
// Watcher layer: observes core state paths and emits typed panel events.

import { getNumenorPath } from './weathertop'
import type { ArdaHudSettings } from './ardaHudSettings'

export type ArdaPanel =
  | 'queue'
  | 'plan'
  | 'systems'
  | 'sourceFreshness'
  | 'runtime'
  | 'learningLoop'
  | 'autonomy'
  | 'athena'
  | 'charon'

export type ArdaPanelEventType =
  | 'path_changed'
  | 'path_missing'
  | 'bundle_updated'
  | 'projection_ready'
  | 'projection_error'

export interface ArdaPanelChangeEvent {
  type: ArdaPanelEventType
  panel: ArdaPanel
  path: string
  value: unknown
  tsUnixMs: number
}

export interface ArdaPanelWatchOptions {
  settings: ArdaHudSettings
  throttleMs?: number
}

type Listener = (event: ArdaPanelChangeEvent) => void

export function pathForSetting(settings: ArdaHudSettings, key: keyof ArdaHudSettings): string | null {
  const value = settings[key]
  if (typeof value !== 'string') return null
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

export const PANEL_PATH_REGISTRY: Array<{ panel: ArdaPanel; settingKeys: Array<keyof ArdaHudSettings> }> = [
  { panel: 'queue', settingKeys: ['queue_active_path', 'queue_summary_path', 'task_queue_path'] },
  { panel: 'plan', settingKeys: ['core_plan_index_path', 'human_plan_index_path', 'core_plan_root', 'human_plan_root'] },
  { panel: 'systems', settingKeys: ['operator_runtime_status_path', 'operator_actions_path', 'soterion_render_contract_path'] },
  { panel: 'sourceFreshness', settingKeys: ['arda_snapshot_path', 'arda_source_map_path', 'remote_confidence_snapshot_path'] },
  {
    panel: 'runtime',
    settingKeys: ['world_path', 'runtime_settings_path', 'business_runtime_path', 'personal_runtime_path'],
  },
  { panel: 'learningLoop', settingKeys: ['core_state_root'] },
  {
    panel: 'autonomy',
    settingKeys: [
      'autonomy_readiness_path',
      'autonomy_daily_eval_path',
      'autonomy_hold_reason_path',
      'storage_pressure_path',
    ],
  },
  {
    panel: 'athena',
    settingKeys: [
      'athena_runtime_path',
      'athena_digest_path',
      'athena_deep_graph_path',
      'athena_policy_readiness_path',
    ],
  },
  { panel: 'charon', settingKeys: ['charon_router_path', 'fleet_runtime_drift_path'] },
]

class ArdaPanelWatcher {
  private readonly settings: ArdaHudSettings
  private readonly throttleMs: number
  private readonly listeners: Set<Listener> = new Set()
  private readonly lastEmitByPanel = new Map<ArdaPanel, number>()
  private numenorPath: string | null = null
  private disposed = false

  constructor(options: ArdaPanelWatchOptions) {
    this.settings = options.settings
    this.throttleMs = options.throttleMs ?? 1000
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  async resolveRootPath(): Promise<string> {
    if (this.numenorPath) return this.numenorPath
    this.numenorPath = await getNumenorPath()
    return this.numenorPath
  }

  watchedPathsForPanel(panel: ArdaPanel): string[] {
    const entry = PANEL_PATH_REGISTRY.find((item) => item.panel === panel)
    if (!entry) return []
    return entry.settingKeys
      .map((key) => pathForSetting(this.settings, key))
      .filter((path): path is string => Boolean(path))
  }

  async checkPanel(panel: ArdaPanel): Promise<ArdaPanelChangeEvent | null> {
    if (this.disposed) return null
    const watched = this.watchedPathsForPanel(panel)
    if (watched.length === 0) return null

    const root = await this.resolveRootPath()
    const absoluteMissing = watched.every((path) => path.trim().length === 0)
    if (absoluteMissing) {
      return this.emitIfAllowed(panel, {
        type: 'path_missing',
        panel,
        path: watched[0],
        value: null,
        tsUnixMs: Date.now(),
      })
    }

    return null
  }

  async checkAll(): Promise<void> {
    const panels: ArdaPanel[] = [
      'queue',
      'plan',
      'systems',
      'sourceFreshness',
      'runtime',
      'learningLoop',
      'autonomy',
      'athena',
      'charon',
    ]

    await Promise.all(panels.map((panel) => this.checkPanel(panel)))
  }

  async refresh(): Promise<void> {
    await this.checkAll()
  }

  dispose(): void {
    this.disposed = true
    this.listeners.clear()
    this.lastEmitByPanel.clear()
  }

  private emitIfAllowed(panel: ArdaPanel, event: ArdaPanelChangeEvent): ArdaPanelChangeEvent {
    const now = Date.now()
    const last = this.lastEmitByPanel.get(panel)
    if (last && now - last < this.throttleMs) return event

    this.lastEmitByPanel.set(panel, now)
    for (const listener of this.listeners) {
      try {
        listener(event)
      } catch {
        // Keep watcher robust to bad listeners.
      }
    }
    return event
  }
}

let watcher: ArdaPanelWatcher | null = null

export function createPanelWatcher(options: ArdaPanelWatchOptions): ArdaPanelWatcher {
  if (watcher) {
    watcher.dispose()
  }
  watcher = new ArdaPanelWatcher(options)
  return watcher
}

export function getActivePanelWatcher(): ArdaPanelWatcher | null {
  return watcher
}
