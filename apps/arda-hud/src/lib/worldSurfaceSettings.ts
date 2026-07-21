// sigil: REPAIR
import { readFile, writeScopedFile, type FileReadResult } from './weathertop'
import { parseJsonOrDefault, parseJsonOrNull } from './jsonParse'
import type {
  BoardroomSurfaceAdapterType,
  BoardroomSurfaceFocusMode,
  BoardroomSurfaceLayout,
  BoardroomSurfacePreviewMode,
  BoardroomSurfaceWidget,
  BoardroomSurfaceWidgetKind,
} from './boardroomSlotSettings'

export const ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH = 'core/state/arda_world_surfaces.json'
export const ARDA_WORLD_SURFACE_STORAGE_KEY = 'arda.world.scene_surfaces.v1'

export const WORLD_DISTRICT_SURFACE_IDS = [
  'district_command',
  'district_knowledge',
  'district_operations',
  'district_finance',
  'district_communications',
  'district_governance',
  'district_monitoring',
  'district_human_business',
] as const

export const WORLD_TERMINAL_SURFACE_IDS = [
  'terminal_queue',
  'terminal_tools',
  'terminal_status',
] as const

export const WORLD_SCENE_SURFACE_IDS = [...WORLD_DISTRICT_SURFACE_IDS, ...WORLD_TERMINAL_SURFACE_IDS] as const

export type WorldSceneSurfaceId = typeof WORLD_SCENE_SURFACE_IDS[number]
export type WorldSceneSurfaceAssignments = Record<WorldSceneSurfaceId, string>
export type WorldSurfaceLayout = BoardroomSurfaceLayout
export type WorldSurfaceAssignmentMode = 'workspace' | 'local' | 'fallback'

export interface WorldSurfaceAssignmentRecord {
  surface_id: WorldSceneSurfaceId
  component_id: string
  source_zone_id: string
  title: string
  role: 'district' | 'terminal'
  module_ids: string[]
  presentation_modes: string[]
  surface_layout: WorldSurfaceLayout
  updated_at_utc: string
}

export interface WorldSurfaceSettingsDocument {
  schema_version: 'arda.arda_world_surfaces.v1'
  authority: 'core/state/arda_world_surfaces.json'
  operator_profile_id: string | null
  updated_at_utc: string
  assignments: WorldSurfaceAssignmentRecord[]
}

export interface WorldSurfaceSettingsLoadResult {
  mode: WorldSurfaceAssignmentMode
  assignments: WorldSceneSurfaceAssignments
  document: WorldSurfaceSettingsDocument
  message: string
}

const DEFAULT_WORLD_SURFACE_ASSIGNMENTS: WorldSceneSurfaceAssignments = {
  district_command: 'sovereign_world',
  district_knowledge: 'knowledge_and_reasoning',
  district_operations: 'planning_and_queue',
  district_finance: 'lifecycle_execution_economics',
  district_communications: 'routing_and_comms',
  district_governance: 'governance_guardhouse',
  district_monitoring: 'systems_health',
  district_human_business: 'human_realm',
  terminal_queue: 'planning_and_queue',
  terminal_tools: 'systems_health',
  terminal_status: 'sovereign_world',
}

const DEFAULT_WORLD_SURFACE_METADATA: Record<WorldSceneSurfaceId, Omit<WorldSurfaceAssignmentRecord, 'surface_id' | 'surface_layout' | 'updated_at_utc'>> = {
  district_command: {
    component_id: 'world-command-district',
    source_zone_id: 'sovereign_world',
    title: 'Command District',
    role: 'district',
    module_ids: ['operating_surface', 'executive_overview', 'systems'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  district_knowledge: {
    component_id: 'world-knowledge-district',
    source_zone_id: 'knowledge_and_reasoning',
    title: 'Knowledge District',
    role: 'district',
    module_ids: ['section_focus', 'human_realm'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  district_operations: {
    component_id: 'world-operations-district',
    source_zone_id: 'planning_and_queue',
    title: 'Operations District',
    role: 'district',
    module_ids: ['planning', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  district_finance: {
    component_id: 'world-economy-district',
    source_zone_id: 'lifecycle_execution_economics',
    title: 'Economy District',
    role: 'district',
    module_ids: ['planning', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  district_communications: {
    component_id: 'world-routing-comms-district',
    source_zone_id: 'routing_and_comms',
    title: 'Routing & Comms District',
    role: 'district',
    module_ids: ['section_focus', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  district_governance: {
    component_id: 'world-governance-district',
    source_zone_id: 'governance_guardhouse',
    title: 'Governance District',
    role: 'district',
    module_ids: ['governance_controls', 'operating_surface'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  district_monitoring: {
    component_id: 'world-monitoring-district',
    source_zone_id: 'systems_health',
    title: 'Fleet & Monitoring District',
    role: 'district',
    module_ids: ['systems', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  district_human_business: {
    component_id: 'world-human-business-district',
    source_zone_id: 'human_realm',
    title: 'Human & Business District',
    role: 'district',
    module_ids: ['human_realm', 'business'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  terminal_queue: {
    component_id: 'world-queue-terminal',
    source_zone_id: 'planning_and_queue',
    title: 'Queue Terminal',
    role: 'terminal',
    module_ids: ['planning', 'section_focus'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  terminal_tools: {
    component_id: 'world-tools-terminal',
    source_zone_id: 'systems_health',
    title: 'Tools Terminal',
    role: 'terminal',
    module_ids: ['systems', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  terminal_status: {
    component_id: 'world-status-terminal',
    source_zone_id: 'sovereign_world',
    title: 'Status Terminal',
    role: 'terminal',
    module_ids: ['operating_surface', 'executive_overview'],
    presentation_modes: ['in_scene', 'native_window'],
  },
}

function isWorldSceneSurfaceId(value: unknown): value is WorldSceneSurfaceId {
  return typeof value === 'string' && WORLD_SCENE_SURFACE_IDS.includes(value as WorldSceneSurfaceId)
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string' && entry.trim().length > 0) : []
}

function createDefaultWorldSurfaceLayout(surfaceId: WorldSceneSurfaceId, sourceZoneId: string): WorldSurfaceLayout {
  const role = WORLD_TERMINAL_SURFACE_IDS.includes(surfaceId as typeof WORLD_TERMINAL_SURFACE_IDS[number]) ? 'terminal' : 'district'
  const widgetPrefix = surfaceId.replace(/_/g, '.')
  const widgets: BoardroomSurfaceWidget[] = role === 'terminal'
    ? [
      { id: `${surfaceId}.status`, kind: 'status_grid', title: 'Terminal Status', data_binding: `${sourceZoneId}.status`, grid_area: 'top' },
      { id: `${surfaceId}.feed`, kind: 'agent_comms', title: 'Activity Feed', data_binding: `${sourceZoneId}.feed`, grid_area: 'main' },
    ]
    : [
      { id: `${surfaceId}.pressure`, kind: 'particle_stream', title: 'Pressure', data_binding: `${widgetPrefix}.urgency`, grid_area: 'main' },
      { id: `${surfaceId}.metrics`, kind: 'metric_strip', title: 'Signals', data_binding: `${sourceZoneId}.summary`, grid_area: 'top' },
      { id: `${surfaceId}.state`, kind: 'status_grid', title: 'State', data_binding: `${sourceZoneId}.status`, grid_area: 'side' },
    ]

  return {
    enabled: true,
    adapter_type: role === 'terminal' ? 'streaming_text' : 'component_grid',
    preview: {
      mode: role === 'terminal' ? 'stream_feed' : 'component_grid',
      refresh_ms: role === 'terminal' ? 1500 : 3000,
      widgets,
    },
    focus: {
      mode: 'in_scene_workstation',
      target: sourceZoneId,
      refresh_ms: 1000,
    },
    embed: {
      url: null,
      allow_inline: false,
    },
  }
}

function parseSurfaceLayout(value: unknown, fallback: WorldSurfaceLayout): WorldSurfaceLayout {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return fallback
  const record = value as Record<string, unknown>
  const preview = record.preview && typeof record.preview === 'object' && !Array.isArray(record.preview) ? record.preview as Record<string, unknown> : {}
  const focus = record.focus && typeof record.focus === 'object' && !Array.isArray(record.focus) ? record.focus as Record<string, unknown> : {}
  const embed = record.embed && typeof record.embed === 'object' && !Array.isArray(record.embed) ? record.embed as Record<string, unknown> : {}
  const widgets = Array.isArray(preview.widgets)
    ? preview.widgets
      .map((widget, index) => {
        if (!widget || typeof widget !== 'object' || Array.isArray(widget)) return fallback.preview.widgets[index] ?? null
        const widgetRecord = widget as Record<string, unknown>
        return {
          id: typeof widgetRecord.id === 'string' ? widgetRecord.id : fallback.preview.widgets[index]?.id ?? `widget.${index + 1}`,
          kind: typeof widgetRecord.kind === 'string' ? widgetRecord.kind as BoardroomSurfaceWidgetKind : fallback.preview.widgets[index]?.kind ?? 'metric_strip',
          title: typeof widgetRecord.title === 'string' ? widgetRecord.title : fallback.preview.widgets[index]?.title ?? 'Widget',
          data_binding: typeof widgetRecord.data_binding === 'string' ? widgetRecord.data_binding : fallback.preview.widgets[index]?.data_binding ?? fallback.focus.target,
          grid_area: typeof widgetRecord.grid_area === 'string' ? widgetRecord.grid_area : fallback.preview.widgets[index]?.grid_area ?? 'main',
        }
      })
      .filter((widget): widget is BoardroomSurfaceWidget => widget !== null)
    : fallback.preview.widgets

  return {
    enabled: typeof record.enabled === 'boolean' ? record.enabled : fallback.enabled,
    adapter_type: typeof record.adapter_type === 'string' ? record.adapter_type as BoardroomSurfaceAdapterType : fallback.adapter_type,
    preview: {
      mode: typeof preview.mode === 'string' ? preview.mode as BoardroomSurfacePreviewMode : fallback.preview.mode,
      refresh_ms: typeof preview.refresh_ms === 'number' && Number.isFinite(preview.refresh_ms) ? preview.refresh_ms : fallback.preview.refresh_ms,
      widgets,
    },
    focus: {
      mode: typeof focus.mode === 'string' ? focus.mode as BoardroomSurfaceFocusMode : fallback.focus.mode,
      target: typeof focus.target === 'string' ? focus.target : fallback.focus.target,
      refresh_ms: typeof focus.refresh_ms === 'number' && Number.isFinite(focus.refresh_ms) ? focus.refresh_ms : fallback.focus.refresh_ms,
    },
    embed: {
      url: typeof embed.url === 'string' ? embed.url : fallback.embed.url,
      allow_inline: typeof embed.allow_inline === 'boolean' ? embed.allow_inline : fallback.embed.allow_inline,
    },
  }
}

function defaultAssignmentForSurface(surfaceId: WorldSceneSurfaceId, updatedAtUtc: string): WorldSurfaceAssignmentRecord {
  const metadata = DEFAULT_WORLD_SURFACE_METADATA[surfaceId]
  return {
    surface_id: surfaceId,
    updated_at_utc: updatedAtUtc,
    ...metadata,
    surface_layout: createDefaultWorldSurfaceLayout(surfaceId, metadata.source_zone_id),
  }
}

export function createDefaultWorldSurfaceSettings(updatedAtUtc = new Date(0).toISOString()): WorldSurfaceSettingsDocument {
  return {
    schema_version: 'arda.arda_world_surfaces.v1',
    authority: ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH,
    operator_profile_id: null,
    updated_at_utc: updatedAtUtc,
    assignments: WORLD_SCENE_SURFACE_IDS.map((surfaceId) => defaultAssignmentForSurface(surfaceId, updatedAtUtc)),
  }
}

export function parseWorldSurfaceSettings(value: unknown): WorldSurfaceSettingsDocument | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const record = value as Record<string, unknown>
  if (record.schema_version !== 'arda.arda_world_surfaces.v1') return null
  const rawAssignments = Array.isArray(record.assignments) ? record.assignments : []
  const defaults = createDefaultWorldSurfaceSettings(typeof record.updated_at_utc === 'string' ? record.updated_at_utc : new Date(0).toISOString())
  const bySurface = new Map<WorldSceneSurfaceId, WorldSurfaceAssignmentRecord>()

  for (const rawAssignment of rawAssignments) {
    if (!rawAssignment || typeof rawAssignment !== 'object' || Array.isArray(rawAssignment)) continue
    const assignment = rawAssignment as Record<string, unknown>
    if (!isWorldSceneSurfaceId(assignment.surface_id)) continue
    const surfaceId = assignment.surface_id
    const fallback = defaults.assignments.find((candidate) => candidate.surface_id === surfaceId)
    if (!fallback) continue
    const sourceZoneId = typeof assignment.source_zone_id === 'string' ? assignment.source_zone_id : fallback.source_zone_id
    bySurface.set(surfaceId, {
      surface_id: surfaceId,
      component_id: typeof assignment.component_id === 'string' ? assignment.component_id : fallback.component_id,
      source_zone_id: sourceZoneId,
      title: typeof assignment.title === 'string' ? assignment.title : fallback.title,
      role: assignment.role === 'terminal' ? 'terminal' : fallback.role,
      module_ids: stringArray(assignment.module_ids).length > 0 ? stringArray(assignment.module_ids) : fallback.module_ids,
      presentation_modes: stringArray(assignment.presentation_modes).length > 0 ? stringArray(assignment.presentation_modes) : fallback.presentation_modes,
      surface_layout: parseSurfaceLayout(assignment.surface_layout, createDefaultWorldSurfaceLayout(surfaceId, sourceZoneId)),
      updated_at_utc: typeof assignment.updated_at_utc === 'string' ? assignment.updated_at_utc : fallback.updated_at_utc,
    })
  }

  return {
    schema_version: 'arda.arda_world_surfaces.v1',
    authority: ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH,
    operator_profile_id: typeof record.operator_profile_id === 'string' ? record.operator_profile_id : null,
    updated_at_utc: typeof record.updated_at_utc === 'string' ? record.updated_at_utc : defaults.updated_at_utc,
    assignments: WORLD_SCENE_SURFACE_IDS.map((surfaceId) => bySurface.get(surfaceId) ?? defaults.assignments.find((candidate) => candidate.surface_id === surfaceId)!),
  }
}

export function worldSurfaceLayoutsFromDocument(document: WorldSurfaceSettingsDocument): Record<WorldSceneSurfaceId, WorldSurfaceLayout> {
  return WORLD_SCENE_SURFACE_IDS.reduce<Record<WorldSceneSurfaceId, WorldSurfaceLayout>>((layouts, surfaceId) => {
    const record = document.assignments.find((assignment) => assignment.surface_id === surfaceId)
    layouts[surfaceId] = record?.surface_layout ?? defaultAssignmentForSurface(surfaceId, document.updated_at_utc).surface_layout
    return layouts
  }, {} as Record<WorldSceneSurfaceId, WorldSurfaceLayout>)
}

export function worldSurfaceAssignmentsFromDocument(document: WorldSurfaceSettingsDocument): WorldSceneSurfaceAssignments {
  return WORLD_SCENE_SURFACE_IDS.reduce<WorldSceneSurfaceAssignments>((assignments, surfaceId) => {
    const record = document.assignments.find((assignment) => assignment.surface_id === surfaceId)
    assignments[surfaceId] = record?.source_zone_id ?? DEFAULT_WORLD_SURFACE_ASSIGNMENTS[surfaceId]
    return assignments
  }, { ...DEFAULT_WORLD_SURFACE_ASSIGNMENTS })
}

export function worldSurfaceDocumentWithLayout(
  document: WorldSurfaceSettingsDocument,
  surfaceId: WorldSceneSurfaceId,
  surfaceLayout: WorldSurfaceLayout,
  updatedAtUtc = new Date().toISOString(),
): WorldSurfaceSettingsDocument {
  return {
    ...document,
    updated_at_utc: updatedAtUtc,
    assignments: document.assignments.map((assignment) => (
      assignment.surface_id === surfaceId
        ? { ...assignment, surface_layout: surfaceLayout, updated_at_utc: updatedAtUtc }
        : assignment
    )),
  }
}

export function readLocalWorldSurfaceAssignments(storage: Pick<Storage, 'getItem'> | null | undefined): WorldSceneSurfaceAssignments {
  try {
    const raw = storage?.getItem(ARDA_WORLD_SURFACE_STORAGE_KEY)
    if (!raw) return { ...DEFAULT_WORLD_SURFACE_ASSIGNMENTS }
    const parsed = parseJsonOrNull<unknown>(raw)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return { ...DEFAULT_WORLD_SURFACE_ASSIGNMENTS }
    const stored = parsed as Record<string, unknown>
    return WORLD_SCENE_SURFACE_IDS.reduce<WorldSceneSurfaceAssignments>((assignments, surfaceId) => {
      const value = stored[surfaceId]
      assignments[surfaceId] = typeof value === 'string' ? value : DEFAULT_WORLD_SURFACE_ASSIGNMENTS[surfaceId]
      return assignments
    }, { ...DEFAULT_WORLD_SURFACE_ASSIGNMENTS })
  } catch {
    return { ...DEFAULT_WORLD_SURFACE_ASSIGNMENTS }
  }
}

export async function loadWorldSurfaceSettings(rootPath: string): Promise<WorldSurfaceSettingsLoadResult> {
  const settingsPath = `${rootPath}/${ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH}`
  const result = await readFile(settingsPath)
  if (!result.success || !result.content) {
    const document = createDefaultWorldSurfaceSettings()
    return {
      mode: 'fallback',
      assignments: worldSurfaceAssignmentsFromDocument(document),
      document,
      message: result.error ?? 'workspace world surface settings unavailable',
    }
  }

  try {
    const parsed = parseWorldSurfaceSettings(parseJsonOrDefault<unknown>(result.content, null))
    if (!parsed) throw new Error('invalid world surface settings schema')
    return {
      mode: 'workspace',
      assignments: worldSurfaceAssignmentsFromDocument(parsed),
      document: parsed,
      message: `loaded ${ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH}`,
    }
  } catch (error) {
    const document = createDefaultWorldSurfaceSettings()
    return {
      mode: 'fallback',
      assignments: worldSurfaceAssignmentsFromDocument(document),
      document,
      message: error instanceof Error ? error.message : 'invalid world surface settings',
    }
  }
}

export async function saveWorldSurfaceSettingsDocument(
  rootPath: string,
  document: WorldSurfaceSettingsDocument,
): Promise<FileReadResult> {
  const parsed = parseWorldSurfaceSettings(document)
  if (!parsed) {
    return { success: false, content: null, error: 'invalid world surface settings document', path: ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH }
  }
  return writeScopedFile(rootPath, ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH, `${JSON.stringify(parsed, null, 2)}\n`)
}
