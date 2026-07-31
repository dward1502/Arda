export type BoardroomVisualizationPresetId =
  | 'standby'
  | 'topology'
  | 'routes'
  | 'lanes'
  | 'constellation'
  | 'pulse'

export type BoardroomVisualizationInputKind =
  | 'service'
  | 'routing'
  | 'knowledge'
  | 'planning'
  | 'governance'
  | 'systems'
  | 'human'
  | 'command'
  | 'unknown'

export type BoardroomVisualizationDensity = 'low' | 'medium' | 'high'

export interface BoardroomVisualizationConfig {
  density: BoardroomVisualizationDensity
  timespan_minutes: number
  alert_threshold: number | null
}

export interface BoardroomVisualizationSelection {
  preset_id: BoardroomVisualizationPresetId
  config: BoardroomVisualizationConfig
}

export interface BoardroomVisualizationPreset {
  id: BoardroomVisualizationPresetId
  label: string
  description: string
  compatible_input_kinds: BoardroomVisualizationInputKind[]
}

export interface BoardroomVisualizationResolution {
  ok: boolean
  selection: BoardroomVisualizationSelection
  message: string
}

const ALL_INPUT_KINDS: BoardroomVisualizationInputKind[] = [
  'service',
  'routing',
  'knowledge',
  'planning',
  'governance',
  'systems',
  'human',
  'command',
  'unknown',
]

export const BOARDROOM_VISUALIZATION_PRESETS: BoardroomVisualizationPreset[] = [
  {
    id: 'standby',
    label: 'Standby',
    description: 'Bounded source identity and freshness without decorative telemetry.',
    compatible_input_kinds: ALL_INPUT_KINDS,
  },
  {
    id: 'topology',
    label: 'Topology',
    description: 'Node and connection topology for fleet and routing sources.',
    compatible_input_kinds: ['routing', 'systems'],
  },
  {
    id: 'routes',
    label: 'Routes',
    description: 'Provider and communication route projection.',
    compatible_input_kinds: ['routing'],
  },
  {
    id: 'lanes',
    label: 'Lanes',
    description: 'Queue, decision, and command lane projection.',
    compatible_input_kinds: ['planning', 'governance', 'command'],
  },
  {
    id: 'constellation',
    label: 'Constellation',
    description: 'Evidence and continuity relationship projection.',
    compatible_input_kinds: ['knowledge', 'human'],
  },
  {
    id: 'pulse',
    label: 'Pulse',
    description: 'Bounded activity and state pulse history.',
    compatible_input_kinds: ['service', 'routing', 'systems', 'human', 'command'],
  },
]

const DEFAULT_PRESET_BY_INPUT_KIND: Record<BoardroomVisualizationInputKind, BoardroomVisualizationPresetId> = {
  service: 'standby',
  routing: 'routes',
  knowledge: 'constellation',
  planning: 'lanes',
  governance: 'lanes',
  systems: 'topology',
  human: 'pulse',
  command: 'pulse',
  unknown: 'standby',
}

export const DEFAULT_BOARDROOM_VISUALIZATION_CONFIG: BoardroomVisualizationConfig = {
  density: 'medium',
  timespan_minutes: 15,
  alert_threshold: null,
}

export function boardroomVisualizationInputKind(sourceZoneId: string): BoardroomVisualizationInputKind {
  if (sourceZoneId.startsWith('service_') || sourceZoneId === 'hermes_runtime' || sourceZoneId === 'hermes_dashboard') return 'service'
  if (sourceZoneId === 'routing_and_comms' || sourceZoneId === 'routing_health') return 'routing'
  if (sourceZoneId === 'memory_and_continuity' || sourceZoneId === 'evidence_trust') return 'knowledge'
  if (sourceZoneId === 'planning_and_queue' || sourceZoneId === 'workbench') return 'planning'
  if (sourceZoneId === 'governance_guardhouse' || sourceZoneId === 'decisions') return 'governance'
  if (sourceZoneId === 'systems_health' || sourceZoneId === 'sovereign_world') return 'systems'
  if (sourceZoneId === 'human_realm') return 'human'
  if (sourceZoneId === 'now_command') return 'command'
  return 'unknown'
}

export function compatibleBoardroomVisualizationPresets(sourceZoneId: string): BoardroomVisualizationPreset[] {
  const inputKind = boardroomVisualizationInputKind(sourceZoneId)
  return BOARDROOM_VISUALIZATION_PRESETS.filter((preset) => preset.compatible_input_kinds.includes(inputKind))
}

export function defaultBoardroomVisualizationSelection(sourceZoneId: string): BoardroomVisualizationSelection {
  return {
    preset_id: DEFAULT_PRESET_BY_INPUT_KIND[boardroomVisualizationInputKind(sourceZoneId)],
    config: { ...DEFAULT_BOARDROOM_VISUALIZATION_CONFIG },
  }
}

function presetById(presetId: string): BoardroomVisualizationPreset | null {
  return BOARDROOM_VISUALIZATION_PRESETS.find((preset) => preset.id === presetId) ?? null
}

export function isBoardroomVisualizationSelection(value: unknown): value is BoardroomVisualizationSelection {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const selection = value as Record<string, unknown>
  const config = selection.config
  if (!presetById(String(selection.preset_id ?? ''))) return false
  if (!config || typeof config !== 'object' || Array.isArray(config)) return false
  const configRecord = config as Record<string, unknown>
  return (configRecord.density === 'low' || configRecord.density === 'medium' || configRecord.density === 'high')
    && typeof configRecord.timespan_minutes === 'number'
    && Number.isFinite(configRecord.timespan_minutes)
    && configRecord.timespan_minutes > 0
    && (configRecord.alert_threshold === null
      || (typeof configRecord.alert_threshold === 'number'
        && Number.isFinite(configRecord.alert_threshold)
        && configRecord.alert_threshold >= 0
        && configRecord.alert_threshold <= 1))
}

export function resolveBoardroomVisualizationSelection(
  sourceZoneId: string,
  requested: BoardroomVisualizationSelection,
  previous = defaultBoardroomVisualizationSelection(sourceZoneId),
): BoardroomVisualizationResolution {
  const inputKind = boardroomVisualizationInputKind(sourceZoneId)
  const requestedPreset = presetById(requested.preset_id)
  const previousPreset = presetById(previous.preset_id) ?? presetById(DEFAULT_PRESET_BY_INPUT_KIND[inputKind])!
  if (requestedPreset?.compatible_input_kinds.includes(inputKind) && isBoardroomVisualizationSelection(requested)) {
    return {
      ok: true,
      selection: requested,
      message: `${requestedPreset.label} is compatible with ${inputKind} input`,
    }
  }
  return {
    ok: false,
    selection: previous,
    message: `${requestedPreset?.label ?? requested.preset_id} is incompatible with ${inputKind} input; retained ${previousPreset.label}`,
  }
}
