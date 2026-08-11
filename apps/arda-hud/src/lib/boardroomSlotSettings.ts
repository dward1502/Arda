// sigil: REPAIR
import { readFile, writeScopedFile, type FileReadResult } from './weathertop'
import { loopbackUrl } from './endpointConfig'
import { parseJsonOrDefault, parseJsonOrNull } from './jsonParse'
import { coerceRuntimeMonitorRegistry, toRuntimeMonitorRegistry } from './monitorSurfaceRegistryBridge'
import type { MonitorSessionRegistryDescriptor as CanonicalMonitorRegistry } from './monitorSurfaceContract'
import {
  defaultBoardroomVisualizationSelection,
  isBoardroomVisualizationSelection,
  resolveBoardroomVisualizationSelection,
  type BoardroomVisualizationSelection,
} from '../scene/boardroom/boardroomVisualizationPresets'

export const ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH = 'core/state/arda_boardroom_slots.json'
export const ARDA_BOARDROOM_SLOT_STORAGE_KEY = 'arda.boardroom.scene_slots.v2'

export const UPPER_MONITOR_SLOT_IDS = [
  'monitor_1',
  'monitor_2',
  'monitor_3',
  'monitor_4',
  'monitor_5',
] as const
export type UpperMonitorSlotId = typeof UPPER_MONITOR_SLOT_IDS[number]

export const BOARDROOM_MONITOR_SLOT_IDS = [...UPPER_MONITOR_SLOT_IDS] as const
export const BOARDROOM_CONTROL_SLOT_IDS = ['view_desk_l', 'view_desk_control_panel', 'view_desk_r', 'view_desk_aux'] as const
export const BOARDROOM_SCENE_SLOT_IDS = [...BOARDROOM_MONITOR_SLOT_IDS, ...BOARDROOM_CONTROL_SLOT_IDS] as const

export const LEGACY_MONITOR_SLOT_ALIASES: Record<string, UpperMonitorSlotId> = {
  monitor_left_1: 'monitor_1',
  monitor_left_2: 'monitor_2',
  monitor_left_3: 'monitor_4',
  monitor_left_4: 'monitor_5',
}

export type BoardroomSceneSlotId = typeof BOARDROOM_SCENE_SLOT_IDS[number]

export function resolveBoardroomSceneSlotId(raw: string): BoardroomSceneSlotId | null {
  const normalized = LEGACY_MONITOR_SLOT_ALIASES[raw] ?? raw
  return BOARDROOM_SCENE_SLOT_IDS.includes(normalized as BoardroomSceneSlotId)
    ? (normalized as BoardroomSceneSlotId)
    : null
}
export type BoardroomSceneSlotAssignments = Record<BoardroomSceneSlotId, string>
export type BoardroomWorkstationRoleId = 'fleet' | 'work' | 'decisions' | 'knowledge' | 'evidence' | 'settings'
export type BoardroomSurfaceAdapterType = 'component_grid' | 'external_url' | 'service_embed' | 'media_viewer' | 'streaming_text' | 'remote_desktop' | 'agent_activity'
export type BoardroomSurfacePreviewMode = 'component_grid' | 'service_status' | 'inline_embed' | 'media_thumbnail' | 'stream_feed' | 'remote_preview' | 'agent_activity'
export type BoardroomSurfaceFocusMode = 'in_scene_workstation' | 'native_window' | 'external_browser' | 'inline_embed' | 'remote_preview'
export type BoardroomSurfaceWidgetKind =
  | 'metric_strip'
  | 'particle_stream'
  | 'sparkline'
  | 'status_grid'
  | 'agent_comms'
  | 'media_tile'
  | 'iframe_preview'
  | 'markdown_doc'
  | 'pdf_doc'
  | 'image_asset'
  | 'video_asset'
  | 'document_asset'
  | 'data_stream'
  | 'remote_session'

export interface BoardroomSurfaceWidget {
  id: string
  kind: BoardroomSurfaceWidgetKind
  title: string
  data_binding: string
  grid_area: string
}

export interface BoardroomSurfaceLayout {
  enabled: boolean
  adapter_type: BoardroomSurfaceAdapterType
  preview: {
    mode: BoardroomSurfacePreviewMode
    refresh_ms: number
    widgets: BoardroomSurfaceWidget[]
  }
  focus: {
    mode: BoardroomSurfaceFocusMode
    target: string
    refresh_ms: number
  }
  embed: {
    url: string | null
    allow_inline: boolean
  }
}

export interface BoardroomAgentClaim {
  owner: string
  activity_kind: 'agent_activity' | 'streaming_text' | 'remote_session' | 'iframe_preview'
  payload_binding: string
  fallback_preview: BoardroomSurfaceLayout['preview']
  lease_expires_at_utc: string
}

export interface BoardroomSlotAssignmentRecord {
  slot_id: BoardroomSceneSlotId
  role_id?: BoardroomWorkstationRoleId
  component_id: string
  source_zone_id: string
  title: string
  module_ids: string[]
  presentation_modes: string[]
  surface_layout: BoardroomSurfaceLayout
  visualization: BoardroomVisualizationSelection
  agent_claims?: BoardroomAgentClaim[]
  updated_at_utc: string
}

export interface BoardroomRoleAssignmentProfile {
  role_id: BoardroomWorkstationRoleId
  label: string
  source_zone_id: string
  component_id: string
  title: string
  module_ids: string[]
  presentation_modes: string[]
}

export interface BoardroomSlotSettingsDocument {
  schema_version: 'arda.arda_boardroom_slots.v1' | 'arda.arda_boardroom_slots.v2'
  authority: 'core/state/arda_boardroom_slots.json'
  operator_profile_id: string | null
  updated_at_utc: string
  assignments: BoardroomSlotAssignmentRecord[]
}

export type BoardroomSlotAssignmentMode = 'workspace' | 'local' | 'fallback'

export interface BoardroomSlotSettingsLoadResult {
  mode: BoardroomSlotAssignmentMode
  assignments: BoardroomSceneSlotAssignments
  document: BoardroomSlotSettingsDocument
  message: string
}

export const DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS: BoardroomSceneSlotAssignments = {
  monitor_1: 'service_warp_dev',
  monitor_2: 'routing_and_comms',
  monitor_3: 'memory_and_continuity',
  monitor_4: 'planning_and_queue',
  monitor_5: 'human_business_personal',
  view_desk_l: 'governance_guardhouse',
  view_desk_control_panel: 'fleet_and_backbone',
  view_desk_r: 'routing_and_comms',
  view_desk_aux: 'human_business_personal',
}

export const BOARDROOM_WORKSTATION_ROLE_PROFILES: BoardroomRoleAssignmentProfile[] = [
  {
    role_id: 'fleet',
    label: 'Fleet',
    source_zone_id: 'systems_health',
    component_id: 'fleet-workstation',
    title: 'Fleet',
    module_ids: ['systems', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  {
    role_id: 'work',
    label: 'Work',
    source_zone_id: 'planning_and_queue',
    component_id: 'work-queue-workstation',
    title: 'Work Queue',
    module_ids: ['planning', 'learning_loop', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  {
    role_id: 'decisions',
    label: 'Decisions',
    source_zone_id: 'decisions',
    component_id: 'decisions-workstation',
    title: 'Decisions',
    module_ids: ['governance_controls', 'operating_surface'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  {
    role_id: 'knowledge',
    label: 'Knowledge',
    source_zone_id: 'memory_and_continuity',
    component_id: 'knowledge-workstation',
    title: 'Knowledge + Memory',
    module_ids: ['section_focus', 'human_realm'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  {
    role_id: 'evidence',
    label: 'Evidence',
    source_zone_id: 'evidence_trust',
    component_id: 'evidence-workstation',
    title: 'Evidence + Trust',
    module_ids: ['operating_surface', 'systems', 'human_realm'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  {
    role_id: 'settings',
    label: 'Settings',
    source_zone_id: 'settings',
    component_id: 'settings-workstation',
    title: 'Settings',
    module_ids: ['settings'],
    presentation_modes: ['in_scene'],
  },
]

const DEFAULT_ASSIGNMENT_METADATA: Record<BoardroomSceneSlotId, Omit<BoardroomSlotAssignmentRecord, 'slot_id' | 'surface_layout' | 'visualization' | 'updated_at_utc'>> = {
  monitor_1: {
    component_id: 'ambient-monitor-surface',
    source_zone_id: '',
    title: 'Ambient Monitor 1',
    module_ids: [],
    presentation_modes: ['in_scene'],
  },
  monitor_2: {
    component_id: 'ambient-monitor-surface',
    source_zone_id: '',
    title: 'Ambient Monitor 2',
    module_ids: [],
    presentation_modes: ['in_scene'],
  },
  monitor_3: {
    component_id: 'ambient-monitor-surface',
    source_zone_id: '',
    title: 'Ambient Monitor 3',
    module_ids: [],
    presentation_modes: ['in_scene'],
  },
  monitor_4: {
    component_id: 'ambient-monitor-surface',
    source_zone_id: '',
    title: 'Ambient Monitor 4',
    module_ids: [],
    presentation_modes: ['in_scene'],
  },
  monitor_5: {
    component_id: 'ambient-monitor-surface',
    source_zone_id: '',
    title: 'Ambient Monitor 5',
    module_ids: [],
    presentation_modes: ['in_scene'],
  },
  view_desk_l: {
    component_id: 'review-gates-workstation',
    source_zone_id: 'governance_guardhouse',
    title: 'Review Gates',
    module_ids: ['governance_controls', 'section_focus'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  view_desk_control_panel: {
    component_id: 'fleet-workstation',
    source_zone_id: 'fleet_and_backbone',
    title: 'Systems + Fleet',
    module_ids: ['systems', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  view_desk_r: {
    component_id: 'routing-comms-workstation',
    source_zone_id: 'routing_and_comms',
    title: 'Routing + Communications',
    module_ids: ['systems', 'operations_and_packages'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  view_desk_aux: {
    component_id: 'human-business-workstation',
    source_zone_id: 'human_business_personal',
    title: 'Human + Business',
    module_ids: ['human_realm', 'business'],
    presentation_modes: ['in_scene', 'native_window'],
  },
}

const LOCAL_SERVICE_EMBED_URLS: Record<string, string> = {
  service_beelink_grafana: 'http://100.103.125.88:3000',
  service_beelink_openwebui: 'http://100.103.125.88:8080',
}

function isBoardroomSurfaceWidgetKind(value: unknown): value is BoardroomSurfaceWidgetKind {
  return typeof value === 'string' && (
    value === 'metric_strip'
    || value === 'particle_stream'
    || value === 'sparkline'
    || value === 'status_grid'
    || value === 'agent_comms'
    || value === 'media_tile'
    || value === 'iframe_preview'
    || value === 'markdown_doc'
    || value === 'pdf_doc'
    || value === 'image_asset'
    || value === 'video_asset'
    || value === 'document_asset'
    || value === 'data_stream'
    || value === 'remote_session'
  )
}

function isBoardroomSceneSlotId(value: unknown): value is BoardroomSceneSlotId {
  return typeof value === 'string' && resolveBoardroomSceneSlotId(value) !== null
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string' && entry.trim().length > 0) : []
}

function isBoardroomWorkstationRoleId(value: unknown): value is BoardroomWorkstationRoleId {
  return typeof value === 'string' && BOARDROOM_WORKSTATION_ROLE_PROFILES.some((profile) => profile.role_id === value)
}

export function getBoardroomRoleProfileByRoleId(roleId: string | null | undefined): BoardroomRoleAssignmentProfile | null {
  if (!roleId) return null
  return BOARDROOM_WORKSTATION_ROLE_PROFILES.find((profile) => profile.role_id === roleId) ?? null
}

export function inferBoardroomRoleId(sourceZoneId: string | null | undefined): BoardroomWorkstationRoleId | undefined {
  if (!sourceZoneId) return undefined
  const profile = BOARDROOM_WORKSTATION_ROLE_PROFILES.find((item) => item.source_zone_id === sourceZoneId)
  return profile?.role_id
}

function resolveAssignmentProfile(sourceZoneId: string): BoardroomRoleAssignmentProfile | null {
  const roleId = inferBoardroomRoleId(sourceZoneId)
  return roleId ? getBoardroomRoleProfileByRoleId(roleId) : null
}

function createDefaultSurfaceLayout(slotId: BoardroomSceneSlotId, sourceZoneId: string, componentId: string): BoardroomSurfaceLayout {
  if (slotId.startsWith('monitor_') && !sourceZoneId) {
    return {
      enabled: true,
      adapter_type: 'agent_activity',
      preview: { mode: 'agent_activity', refresh_ms: 500, widgets: [] },
      focus: { mode: 'in_scene_workstation', target: '', refresh_ms: 500 },
      embed: { url: null, allow_inline: false },
    }
  }

  if (sourceZoneId === 'hermes_runtime') {
    return {
      enabled: true,
      adapter_type: 'service_embed',
      preview: {
        mode: 'stream_feed',
        refresh_ms: 2500,
        widgets: [
          { id: `${slotId}.terminal`, kind: 'agent_comms', title: 'Hermes terminal', data_binding: 'hermes.dashboard.status', grid_area: 'main' },
        ],
      },
      focus: { mode: 'native_window', target: sourceZoneId, refresh_ms: 1000 },
      embed: { url: loopbackUrl({ port: 9119 }), allow_inline: false },
    }
  }

  if (sourceZoneId.startsWith('service_')) {
    const localServiceUrl = LOCAL_SERVICE_EMBED_URLS[sourceZoneId] ?? null
    return {
      enabled: true,
      adapter_type: localServiceUrl ? 'service_embed' : 'external_url',
      preview: {
        mode: 'service_status',
        refresh_ms: 5000,
        widgets: [
          { id: `${slotId}.status`, kind: 'status_grid', title: 'Service status', data_binding: sourceZoneId, grid_area: 'main' },
        ],
      },
      focus: { mode: 'native_window', target: sourceZoneId, refresh_ms: 5000 },
      embed: { url: localServiceUrl, allow_inline: false },
    }
  }

  return {
    enabled: true,
    adapter_type: 'component_grid',
    preview: {
      mode: 'component_grid',
      refresh_ms: 3000,
      widgets: [
        { id: `${slotId}.metrics`, kind: 'metric_strip', title: 'Metrics', data_binding: `${sourceZoneId}.summary`, grid_area: 'top' },
        { id: `${slotId}.stream`, kind: 'particle_stream', title: 'Flow', data_binding: `${sourceZoneId}.health`, grid_area: 'main' },
        { id: `${slotId}.status`, kind: 'status_grid', title: 'Status', data_binding: `${sourceZoneId}.status`, grid_area: 'side' },
      ],
    },
    focus: {
      mode: componentId === 'command-podium-workstation' ? 'in_scene_workstation' : 'native_window',
      target: sourceZoneId,
      refresh_ms: 1000,
    },
    embed: { url: null, allow_inline: false },
  }
}

function defaultAssignmentForSlot(slotId: BoardroomSceneSlotId, updatedAtUtc: string): BoardroomSlotAssignmentRecord {
  const metadata = DEFAULT_ASSIGNMENT_METADATA[slotId]
  const roleId = inferBoardroomRoleId(metadata.source_zone_id) ?? undefined
  return {
    slot_id: slotId,
    ...(roleId ? { role_id: roleId } : {}),
    updated_at_utc: updatedAtUtc,
    ...metadata,
    surface_layout: createDefaultSurfaceLayout(slotId, metadata.source_zone_id, metadata.component_id),
    visualization: defaultBoardroomVisualizationSelection(metadata.source_zone_id),
  }
}

function parseVisualizationSelection(
  value: unknown,
  sourceZoneId: string,
  fallback: BoardroomVisualizationSelection,
): BoardroomVisualizationSelection {
  if (!isBoardroomVisualizationSelection(value)) return fallback
  return resolveBoardroomVisualizationSelection(sourceZoneId, value, fallback).selection
}

function parseSurfaceLayout(value: unknown, fallback: BoardroomSurfaceLayout): BoardroomSurfaceLayout {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return fallback
  const record = value as Record<string, unknown>
  const preview = record.preview && typeof record.preview === 'object' && !Array.isArray(record.preview) ? record.preview as Record<string, unknown> : {}
  const focus = record.focus && typeof record.focus === 'object' && !Array.isArray(record.focus) ? record.focus as Record<string, unknown> : {}
  const embed = record.embed && typeof record.embed === 'object' && !Array.isArray(record.embed) ? record.embed as Record<string, unknown> : {}
  const widgets = Array.isArray(preview.widgets)
    ? preview.widgets
      .map((widget, index) => {
        if (!widget || typeof widget !== 'object' || Array.isArray(widget)) return fallback.preview.widgets[index] ?? null
        const record = widget as Record<string, unknown>
        return {
          id: typeof record.id === 'string' ? record.id : fallback.preview.widgets[index]?.id ?? `widget.${index + 1}`,
          kind: typeof record.kind === 'string' && isBoardroomSurfaceWidgetKind(record.kind) ? record.kind : (fallback.preview.widgets[index]?.kind ?? 'metric_strip'),
          title: typeof record.title === 'string' ? record.title : fallback.preview.widgets[index]?.title ?? 'Widget',
          data_binding: typeof record.data_binding === 'string' ? record.data_binding : fallback.preview.widgets[index]?.data_binding ?? fallback.focus.target,
          grid_area: typeof record.grid_area === 'string' ? record.grid_area : fallback.preview.widgets[index]?.grid_area ?? 'main',
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

export function surfaceLayoutsFromDocument(document: BoardroomSlotSettingsDocument): Record<BoardroomSceneSlotId, BoardroomSurfaceLayout> {
  return BOARDROOM_SCENE_SLOT_IDS.reduce<Record<BoardroomSceneSlotId, BoardroomSurfaceLayout>>((layouts, slotId) => {
    const record = document.assignments.find((assignment) => assignment.slot_id === slotId)
    layouts[slotId] = record?.surface_layout ?? defaultAssignmentForSlot(slotId, document.updated_at_utc).surface_layout
    return layouts
  }, {} as Record<BoardroomSceneSlotId, BoardroomSurfaceLayout>)
}

export function createDefaultBoardroomSlotSettings(updatedAtUtc = new Date(0).toISOString()): BoardroomSlotSettingsDocument {
  return {
    schema_version: 'arda.arda_boardroom_slots.v1',
    authority: ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH,
    operator_profile_id: null,
    updated_at_utc: updatedAtUtc,
    assignments: BOARDROOM_SCENE_SLOT_IDS.map((slotId) => defaultAssignmentForSlot(slotId, updatedAtUtc)),
  }
}

export function migrateBoardroomSlotSettingsV1ToV2(document: BoardroomSlotSettingsDocument): BoardroomSlotSettingsDocument {
  const updatedAtUtc = document.updated_at_utc || new Date(0).toISOString()
  const defaults = createDefaultBoardroomSlotSettings(updatedAtUtc)
  const fallbackBySlot = new Map(defaults.assignments.map((assignment) => [assignment.slot_id, assignment]))

  const migratedAssignments = document.assignments.map((assignment) => {
    const canonicalSlotId = resolveBoardroomSceneSlotId(assignment.slot_id)
    if (!canonicalSlotId || canonicalSlotId === assignment.slot_id) {
      return assignment
    }

    const existing = document.assignments.find((candidate) => candidate.slot_id === canonicalSlotId)
    const fallback = fallbackBySlot.get(canonicalSlotId) ?? fallbackBySlot.get('monitor_1')!
    const target = existing ?? fallback

    return {
      ...target,
      slot_id: canonicalSlotId,
      agent_claims: assignment.agent_claims,
      updated_at_utc: updatedAtUtc,
    }
  })

  const byCanonicalSlot = new Map(migratedAssignments.map((assignment) => [assignment.slot_id, assignment]))

  return {
    ...document,
    schema_version: 'arda.arda_boardroom_slots.v2',
    assignments: BOARDROOM_SCENE_SLOT_IDS.map((slotId) => byCanonicalSlot.get(slotId) ?? defaults.assignments.find((candidate) => candidate.slot_id === slotId)!),
  }
}

export function parseBoardroomSlotSettings(value: unknown): BoardroomSlotSettingsDocument | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const record = value as Record<string, unknown>
  const schemaVersion = typeof record.schema_version === 'string' ? record.schema_version : ''
  if (schemaVersion !== 'arda.arda_boardroom_slots.v1' && schemaVersion !== 'arda.arda_boardroom_slots.v2') return null
  const rawAssignments = Array.isArray(record.assignments) ? record.assignments : []
  const updatedAtUtc = typeof record.updated_at_utc === 'string' ? record.updated_at_utc : new Date(0).toISOString()
  const defaults = createDefaultBoardroomSlotSettings(updatedAtUtc)
  const normalizedDefaults = new Map(defaults.assignments.map((assignment) => [assignment.slot_id, assignment]))
  const parsed = parseBoardroomSlotSettingsInternal(rawAssignments, normalizedDefaults, record, defaults, schemaVersion, updatedAtUtc)
  if (schemaVersion === 'arda.arda_boardroom_slots.v1') {
    return migrateBoardroomSlotSettingsV1ToV2(parsed)
  }
  return parsed
}

function parseBoardroomSlotSettingsInternal(
  rawAssignments: unknown[],
  normalizedDefaults: Map<string, BoardroomSlotAssignmentRecord>,
  record: Record<string, unknown>,
  defaults: BoardroomSlotSettingsDocument,
  schemaVersion: string,
  updatedAtUtc: string,
): BoardroomSlotSettingsDocument {
  const bySlot = new Map<BoardroomSceneSlotId, BoardroomSlotAssignmentRecord>()

  for (const rawAssignment of rawAssignments) {
    if (!rawAssignment || typeof rawAssignment !== 'object' || Array.isArray(rawAssignment)) continue
    const assignment = rawAssignment as Record<string, unknown>
    const canonicalSlotId = resolveBoardroomSceneSlotId(assignment.slot_id as string)
    if (!canonicalSlotId) continue
    const fallback = normalizedDefaults.get(canonicalSlotId)!
    const explicitRoleId = isBoardroomWorkstationRoleId(assignment.role_id) ? assignment.role_id as BoardroomWorkstationRoleId : null
    const roleProfile = explicitRoleId ? getBoardroomRoleProfileByRoleId(explicitRoleId) : null
    const sourceZoneId = typeof assignment.source_zone_id === 'string'
      ? assignment.source_zone_id
      : roleProfile?.source_zone_id ?? fallback.source_zone_id
    const inferredRoleId = explicitRoleId ?? inferBoardroomRoleId(sourceZoneId) ?? undefined
    const profile = roleProfile ?? resolveAssignmentProfile(sourceZoneId)
    const componentId = typeof assignment.component_id === 'string' && assignment.component_id.length > 0 ? assignment.component_id : profile?.component_id ?? fallback.component_id
    bySlot.set(canonicalSlotId, {
      slot_id: canonicalSlotId,
      ...(inferredRoleId ? { role_id: inferredRoleId } : {}),
      component_id: componentId,
      source_zone_id: sourceZoneId,
      title: typeof assignment.title === 'string' && assignment.title.length > 0 ? assignment.title : profile?.title ?? fallback.title,
      module_ids: stringArray(assignment.module_ids).length > 0 ? stringArray(assignment.module_ids) : profile?.module_ids ?? fallback.module_ids,
      presentation_modes: stringArray(assignment.presentation_modes).length > 0 ? stringArray(assignment.presentation_modes) : profile?.presentation_modes ?? fallback.presentation_modes,
      surface_layout: parseSurfaceLayout(assignment.surface_layout, createDefaultSurfaceLayout(canonicalSlotId, sourceZoneId, componentId)),
      visualization: parseVisualizationSelection(assignment.visualization, sourceZoneId, fallback.visualization),
      agent_claims: assignment.agent_claims ? parseBoardroomAgentClaims(assignment.agent_claims) : undefined,
      updated_at_utc: typeof assignment.updated_at_utc === 'string' && assignment.updated_at_utc.length > 0 ? assignment.updated_at_utc : fallback.updated_at_utc,
    })
  }

  return {
    schema_version: schemaVersion === 'arda.arda_boardroom_slots.v2' ? 'arda.arda_boardroom_slots.v2' : 'arda.arda_boardroom_slots.v1',
    authority: 'core/state/arda_boardroom_slots.json',
    operator_profile_id: typeof record.operator_profile_id === 'string' ? record.operator_profile_id : null,
    updated_at_utc: updatedAtUtc,
    assignments: BOARDROOM_SCENE_SLOT_IDS.map((slotId) => bySlot.get(slotId) ?? defaults.assignments.find((candidate) => candidate.slot_id === slotId)!),
  }
}

export function assignmentsFromDocument(document: BoardroomSlotSettingsDocument): BoardroomSceneSlotAssignments {
  return BOARDROOM_SCENE_SLOT_IDS.reduce<BoardroomSceneSlotAssignments>((assignments, slotId) => {
    const record = document.assignments.find((assignment) => resolveBoardroomSceneSlotId(assignment.slot_id) === slotId)
    assignments[slotId] = record?.source_zone_id ?? DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS[slotId]
    return assignments
  }, { ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS })
}

export function documentFromAssignments(
  assignments: BoardroomSceneSlotAssignments,
  updatedAtUtc = new Date().toISOString(),
  baseDocument?: BoardroomSlotSettingsDocument,
): BoardroomSlotSettingsDocument {
  return {
    ...createDefaultBoardroomSlotSettings(updatedAtUtc),
    updated_at_utc: updatedAtUtc,
    assignments: BOARDROOM_SCENE_SLOT_IDS.map((slotId) => {
      const fallback = DEFAULT_ASSIGNMENT_METADATA[slotId]
      const rawSourceZoneId = assignments[slotId]
      const sourceZoneId = typeof rawSourceZoneId === 'string' && rawSourceZoneId.length > 0 ? rawSourceZoneId : fallback.source_zone_id
      const roleId = inferBoardroomRoleId(sourceZoneId) ?? undefined
      const profile = resolveAssignmentProfile(sourceZoneId)
      const existing = baseDocument?.assignments.find((assignment) => assignment.slot_id === slotId)
      if (existing && existing.source_zone_id === sourceZoneId) {
        return {
          ...existing,
          ...(roleId ? { role_id: roleId } : {}),
          updated_at_utc: updatedAtUtc,
        }
      }
      return {
        slot_id: slotId,
        ...(roleId ? { role_id: roleId } : {}),
        component_id: profile?.component_id ?? fallback.component_id,
        source_zone_id: sourceZoneId,
        title: profile?.title ?? fallback.title,
        module_ids: profile?.module_ids ?? fallback.module_ids,
        presentation_modes: profile?.presentation_modes ?? fallback.presentation_modes,
        surface_layout: createDefaultSurfaceLayout(slotId, sourceZoneId, profile?.component_id ?? fallback.component_id),
        visualization: defaultBoardroomVisualizationSelection(sourceZoneId),
        updated_at_utc: updatedAtUtc,
      }
    }),
  }
}

export function normalizeLegacyAssignments(document: BoardroomSlotSettingsDocument | null | undefined): BoardroomSlotSettingsDocument {
  const base = document ?? createDefaultBoardroomSlotSettings()
  return {
    ...base,
    assignments: base.assignments.map((assignment) => {
      const roleId = assignment.role_id ?? inferBoardroomRoleId(assignment.source_zone_id) ?? undefined
      const profile = resolveAssignmentProfile(assignment.source_zone_id)
      const fallback = DEFAULT_ASSIGNMENT_METADATA[assignment.slot_id]
      const componentId = typeof assignment.component_id === 'string' && assignment.component_id.length > 0 ? assignment.component_id : profile?.component_id ?? fallback.component_id
      return {
        ...assignment,
        ...(roleId ? { role_id: roleId } : {}),
        component_id: componentId,
        title: typeof assignment.title === 'string' && assignment.title.length > 0 ? assignment.title : profile?.title ?? fallback.title,
        module_ids: stringArray(assignment.module_ids).length > 0 ? stringArray(assignment.module_ids) : profile?.module_ids ?? fallback.module_ids,
        presentation_modes: stringArray(assignment.presentation_modes).length > 0 ? stringArray(assignment.presentation_modes) : profile?.presentation_modes ?? fallback.presentation_modes,
        surface_layout: parseSurfaceLayout(assignment.surface_layout, createDefaultSurfaceLayout(assignment.slot_id, assignment.source_zone_id, componentId)),
        visualization: parseVisualizationSelection(assignment.visualization, assignment.source_zone_id, defaultBoardroomVisualizationSelection(assignment.source_zone_id)),
        updated_at_utc: typeof assignment.updated_at_utc === 'string' && assignment.updated_at_utc.length > 0 ? assignment.updated_at_utc : base.updated_at_utc,
      }
    }),
  }
}

export function documentWithSurfaceLayout(
  document: BoardroomSlotSettingsDocument,
  slotId: BoardroomSceneSlotId,
  surfaceLayout: BoardroomSurfaceLayout,
  updatedAtUtc = new Date().toISOString(),
): BoardroomSlotSettingsDocument {
  return {
    ...document,
    updated_at_utc: updatedAtUtc,
    assignments: document.assignments.map((assignment) => (
      assignment.slot_id === slotId
        ? { ...assignment, surface_layout: surfaceLayout, updated_at_utc: updatedAtUtc }
        : assignment
    )),
  }
}

export function documentWithVisualizationSelection(
  document: BoardroomSlotSettingsDocument,
  slotId: BoardroomSceneSlotId,
  requested: BoardroomVisualizationSelection,
  updatedAtUtc = new Date().toISOString(),
): { ok: boolean; document: BoardroomSlotSettingsDocument; message: string } {
  const assignment = document.assignments.find((candidate) => candidate.slot_id === slotId)
  if (!assignment) return { ok: false, document, message: `Unknown boardroom slot: ${slotId}` }
  const resolution = resolveBoardroomVisualizationSelection(
    assignment.source_zone_id,
    requested,
    assignment.visualization,
  )
  if (!resolution.ok) return { ok: false, document, message: resolution.message }
  return {
    ok: true,
    message: resolution.message,
    document: {
      ...document,
      updated_at_utc: updatedAtUtc,
      assignments: document.assignments.map((candidate) => candidate.slot_id === slotId
        ? { ...candidate, visualization: resolution.selection, updated_at_utc: updatedAtUtc }
        : candidate),
    },
  }
}

export function exportBoardroomProfile(document: BoardroomSlotSettingsDocument): string {
  return `${JSON.stringify(document, null, 2)}\n`
}

export function importBoardroomProfile(serialized: string): {
  ok: boolean
  document: BoardroomSlotSettingsDocument | null
  message: string
} {
  const parsedJson = parseJsonOrNull<unknown>(serialized)
  if (!parsedJson) return { ok: false, document: null, message: 'Boardroom profile is not valid JSON' }
  const document = parseBoardroomSlotSettings(parsedJson)
  if (!document) return { ok: false, document: null, message: 'Boardroom profile schema is invalid' }
  return { ok: true, document, message: `Imported ${document.assignments.length} boardroom slots` }
}

export function resetBoardroomProfile(updatedAtUtc = new Date().toISOString()): BoardroomSlotSettingsDocument {
  return createDefaultBoardroomSlotSettings(updatedAtUtc)
}

export function readLocalBoardroomSlotSettingsDocument(
  storage: Pick<Storage, 'getItem'> | null | undefined,
): BoardroomSlotSettingsDocument | null {
  try {
    const raw = storage?.getItem(ARDA_BOARDROOM_SLOT_STORAGE_KEY)
    return raw ? importBoardroomProfile(raw).document : null
  } catch {
    return null
  }
}

export function readLocalBoardroomSlotAssignments(storage: Pick<Storage, 'getItem'> | null | undefined): BoardroomSceneSlotAssignments {
  try {
    const raw = storage?.getItem(ARDA_BOARDROOM_SLOT_STORAGE_KEY)
    if (!raw) return { ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS }
    const parsed = parseJsonOrNull<unknown>(raw)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return { ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS }
    const document = parseBoardroomSlotSettings(parsed)
    if (document) return assignmentsFromDocument(document)
    const stored = parsed as Record<string, unknown>
    return BOARDROOM_SCENE_SLOT_IDS.reduce<BoardroomSceneSlotAssignments>((assignments, slotId) => {
      const value = stored[slotId]
      assignments[slotId] = typeof value === 'string' ? value : DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS[slotId]
      return assignments
    }, { ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS })
  } catch {
    return { ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS }
  }
}

export async function loadBoardroomSlotSettings(rootPath: string): Promise<BoardroomSlotSettingsLoadResult> {
  const settingsPath = `${rootPath}/${ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH}`
  const result = await readFile(settingsPath)
  if (!result.success || !result.content) {
    const document = createDefaultBoardroomSlotSettings()
    return {
      mode: 'fallback',
      assignments: assignmentsFromDocument(document),
      document,
      message: result.error ?? 'workspace boardroom slot settings unavailable',
    }
  }

  try {
    const parsed = parseBoardroomSlotSettings(parseJsonOrDefault<unknown>(result.content, null))
    if (!parsed) throw new Error('invalid boardroom slot settings schema')
    return {
      mode: 'workspace',
      assignments: assignmentsFromDocument(parsed),
      document: parsed,
      message: `loaded ${ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH}`,
    }
  } catch (error) {
    const document = createDefaultBoardroomSlotSettings()
    return {
      mode: 'fallback',
      assignments: assignmentsFromDocument(document),
      document,
      message: error instanceof Error ? error.message : 'invalid boardroom slot settings',
    }
  }
}

function isBoardroomAgentActivityKind(value: unknown): value is BoardroomAgentClaim['activity_kind'] {
  return value === 'agent_activity' || value === 'streaming_text' || value === 'remote_session' || value === 'iframe_preview'
}

function parseBoardroomAgentClaims(value: unknown): BoardroomAgentClaim[] {
  if (!Array.isArray(value)) return []
  return value
    .filter((entry): entry is Record<string, unknown> => entry && typeof entry === 'object' && !Array.isArray(entry))
    .map((claim): BoardroomAgentClaim | null => {
      const owner = typeof claim.owner === 'string' && claim.owner.length > 0 ? claim.owner : null
      if (!owner) return null
      const activityKind = isBoardroomAgentActivityKind(claim.activity_kind) ? claim.activity_kind : 'agent_activity'
      const payloadBinding = typeof claim.payload_binding === 'string' ? claim.payload_binding : ''
      const leaseExpiresAt = typeof claim.lease_expires_at_utc === 'string' ? claim.lease_expires_at_utc : new Date(0).toISOString()
      return {
        owner,
        activity_kind: activityKind,
        payload_binding: payloadBinding,
        fallback_preview: {
          mode: 'stream_feed' as const,
          refresh_ms: 2500,
          widgets: [
            { id: `${owner}.activity`, kind: 'agent_comms', title: 'Agent activity', data_binding: payloadBinding || 'agent.live', grid_area: 'main' },
          ],
        },
        lease_expires_at_utc: leaseExpiresAt,
      }
    })
    .filter((claim): claim is BoardroomAgentClaim => claim !== null)
}

export interface BoardroomMonitorSlotSource {
  sourceZoneId: string
  assignment: BoardroomSlotAssignmentRecord
  claim: BoardroomAgentClaim | null
  active: boolean
}

export function resolveMonitorSlotSource(
  slotId: BoardroomSceneSlotId,
  document: BoardroomSlotSettingsDocument,
  nowUtc: string = new Date().toISOString(),
): BoardroomMonitorSlotSource | null {
  const assignment = document.assignments.find((candidate) => candidate.slot_id === slotId)
  if (!assignment) return null
  if (!BOARDROOM_MONITOR_SLOT_IDS.includes(slotId as typeof BOARDROOM_MONITOR_SLOT_IDS[number])) return null
  const claims = parseBoardroomAgentClaims(assignment.agent_claims)
  const now = new Date(nowUtc).getTime()
  const liveClaims = claims
    .filter((claim) => new Date(claim.lease_expires_at_utc).getTime() > now)
    .sort((left, right) => left.lease_expires_at_utc.localeCompare(right.lease_expires_at_utc))
  const activeClaim = liveClaims.length > 0 ? liveClaims[0] ?? null : null
  return {
    sourceZoneId: assignment.source_zone_id,
    assignment,
    claim: activeClaim,
    active: activeClaim !== null,
  }
}

export function claimMonitorSlot(
  document: BoardroomSlotSettingsDocument,
  slotId: BoardroomSceneSlotId,
  claim: BoardroomAgentClaim,
  nowUtc: string = new Date().toISOString(),
): BoardroomSlotSettingsDocument {
  return {
    ...document,
    updated_at_utc: nowUtc,
    assignments: document.assignments.map((assignment) => {
      if (assignment.slot_id !== slotId) return assignment
      const existing = parseBoardroomAgentClaims(assignment.agent_claims)
      const withoutOwner = existing.filter((existing) => existing.owner !== claim.owner)
      return {
        ...assignment,
        agent_claims: [...withoutOwner, claim],
        updated_at_utc: nowUtc,
      }
    }),
  }
}

export function releaseMonitorSlot(
  document: BoardroomSlotSettingsDocument,
  slotId: BoardroomSceneSlotId,
  owner: string,
  updatedAtUtc: string = new Date().toISOString(),
): BoardroomSlotSettingsDocument {
  return {
    ...document,
    updated_at_utc: updatedAtUtc,
    assignments: document.assignments.map((assignment) => {
      if (assignment.slot_id !== slotId) return assignment
      const existing = parseBoardroomAgentClaims(assignment.agent_claims)
      return {
        ...assignment,
        agent_claims: existing.filter((claim) => claim.owner !== owner),
        updated_at_utc: updatedAtUtc,
      }
    }),
  }
}

export function refreshMonitorSlot(
  document: BoardroomSlotSettingsDocument,
  slotId: BoardroomSceneSlotId,
  owner: string,
  leaseExpiresAtUtc: string,
  updatedAtUtc: string = new Date().toISOString(),
): BoardroomSlotSettingsDocument {
  const assignment = document.assignments.find((candidate) => candidate.slot_id === slotId)
  const existing = parseBoardroomAgentClaims(assignment?.agent_claims)
  if (!existing.some((claim) => claim.owner === owner)) {
    throw new Error(`Agent '${owner}' does not own monitor slot '${slotId}'`)
  }
  return {
    ...document,
    updated_at_utc: updatedAtUtc,
    assignments: document.assignments.map((candidate) => candidate.slot_id === slotId
      ? {
          ...candidate,
          agent_claims: existing.map((claim) => claim.owner === owner
            ? { ...claim, lease_expires_at_utc: leaseExpiresAtUtc }
            : claim),
          updated_at_utc: updatedAtUtc,
        }
      : candidate),
  }
}

export function resetMonitorSlot(
  document: BoardroomSlotSettingsDocument,
  slotId: BoardroomSceneSlotId,
  updatedAtUtc: string = new Date().toISOString(),
): BoardroomSlotSettingsDocument {
  return {
    ...document,
    updated_at_utc: updatedAtUtc,
    assignments: document.assignments.map((assignment) => {
      if (assignment.slot_id !== slotId) return assignment
      const fallback = createDefaultBoardroomSlotSettings(updatedAtUtc).assignments.find((candidate) => candidate.slot_id === slotId)!
      return {
        ...assignment,
        surface_layout: fallback.surface_layout,
        agent_claims: undefined,
        updated_at_utc: updatedAtUtc,
      }
    }),
  }
}

export async function saveBoardroomSlotSettings(
  rootPath: string,
  assignments: BoardroomSceneSlotAssignments,
): Promise<FileReadResult> {
  const document = documentFromAssignments(assignments)
  return saveBoardroomSlotSettingsDocument(rootPath, document)
}

export async function saveBoardroomSlotSettingsDocument(
  rootPath: string,
  document: BoardroomSlotSettingsDocument,
): Promise<FileReadResult> {
  const parsed = parseBoardroomSlotSettings(document)
  if (!parsed) {
    return { success: false, content: null, error: 'invalid boardroom slot settings document', path: ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH }
  }
  return writeScopedFile(rootPath, ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH, `${JSON.stringify(parsed, null, 2)}\n`)
}

export interface MonitorSurfaceRequest {
  slotId: string
  sourceZoneId: string
  focusMode: string
  title?: string
  width?: number
  height?: number
}

export interface SurfaceBridgeResult {
  ok: boolean
  message: string
  windowLabel?: string
  sourceZoneId: string
  slotId: string
}

export async function createMonitorSurface(request: MonitorSurfaceRequest): Promise<SurfaceBridgeResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<SurfaceBridgeResult>('create_monitor_surface', { request: {
    slotId: request.slotId,
    sourceZoneId: request.sourceZoneId,
    focusMode: request.focusMode,
    title: request.title ?? null,
    width: request.width ?? null,
    height: request.height ?? null,
  } })
}

export async function dismissMonitorSurface(windowLabel: string): Promise<string> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<string>('dismiss_monitor_surface', { windowLabel })
}

export interface AgentClaimMonitorRequest {
  slotId: string
  owner: string
  activityKind: 'agent_activity' | 'streaming_text' | 'remote_session' | 'iframe_preview'
  payloadBinding: string
  focusMode: 'in_scene_workstation' | 'native_window' | 'external_browser' | 'inline_embed' | 'remote_preview'
  title?: string
  width?: number
  height?: number
}

export interface AgentClaimResult {
  ok: boolean
  message: string
  slotId: string
  windowLabel?: string
  active: boolean
  leaseExpiresAtUtc: string
}

export async function agentClaimMonitor(request: AgentClaimMonitorRequest): Promise<AgentClaimResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  const result = await invoke<AgentClaimResult>('claim_monitor_slot', { request: {
    slotId: request.slotId,
    owner: request.owner,
    activityKind: request.activityKind,
    payloadBinding: request.payloadBinding,
    focusMode: request.focusMode,
    title: request.title ?? null,
    width: request.width ?? null,
    height: request.height ?? null,
  } })
  return result
}

export async function agentReleaseMonitor(slotId: string, owner: string): Promise<AgentClaimResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<AgentClaimResult>('release_monitor_slot', { slotId, owner })
}

export async function agentRefreshMonitorLease(slotId: string, owner: string): Promise<AgentClaimResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<AgentClaimResult>('refresh_monitor_slot_lease', { request: {
    slotId,
    owner,
  } })
}

export interface AgentSurfacePayload {
  slotId: string
  owner: string
  payloadBinding: string
  content: string
  mime: string
}

export interface AgentSurfacePayloadResult {
  ok: boolean
  message: string
  slotId: string
}

export async function agentPushSurfacePayload(payload: AgentSurfacePayload): Promise<AgentSurfacePayloadResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  const result = await invoke<AgentSurfacePayloadResult>('push_surface_payload', { payload: {
    slotId: payload.slotId,
    owner: payload.owner,
    payloadBinding: payload.payloadBinding,
    content: payload.content,
    mime: payload.mime,
  } })
  return result
}

export interface MonitorSurfaceContentDescriptor {
  kind: 'web' | 'youtube' | 'video' | 'image' | 'document' | 'terminal' | 'component' | 'remote_session' | 'fallback'
  url?: string
  videoId?: string
  source?: { kind: 'local'; path: string } | { kind: 'remote'; url: string }
  mime?: string
  fit?: 'contain' | 'cover'
  loop?: boolean
  autoplay?: boolean
  muted?: boolean
  alt?: string
  documentKind?: 'pdf' | 'markdown' | 'text'
  page?: number
  sessionId?: string
  readOnly?: boolean
  theme?: string
  rendererId?: string
  props?: Record<string, unknown>
  streamUrl?: string
  transport?: 'webrtc' | 'hls' | 'mjpeg'
  reason?: string
  retryable?: boolean
  display?: 'inline' | 'capture_stream'
  sandboxProfile?: string
  startSeconds?: number
}

export interface MonitorSurfaceOwner {
  kind: 'agent' | 'operator' | 'system'
  name?: string
  id?: string
}

export interface MonitorSurfaceClaimRequest {
  slotId: string
  owner: MonitorSurfaceOwner
  initialContent: MonitorSurfaceContentDescriptor
  ttlMs: number
}

export interface MonitorSurfaceSessionRecord {
  slot_id: string
  session_id: string
  surface_session_id: string
  owner: string
  kind: string
  revision: number
  opened_at_utc: string
  lease_expires_at_utc: string
  content: MonitorSurfaceContentDescriptor
  playback?: import('./monitorSurfaceContract').MonitorPlaybackState
  workstation_handoff: { session_id: string; mode: string }
  created_at_utc: string
  updated_at_utc: string
}

export interface MonitorSurfaceRegistryDescriptor {
  schema_version: string
  updated_at_utc: string
  sessions: Record<string, MonitorSurfaceSessionRecord>
}

export interface MonitorSurfaceClaimResult {
  ok: boolean
  registry: MonitorSurfaceRegistryDescriptor
  session: MonitorSurfaceSessionRecord | null
  message: string
}

function normalizeMonitorSurfaceClaimResult(value: unknown): MonitorSurfaceClaimResult {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('monitor surface command returned an invalid result')
  }
  const raw = value as Record<string, unknown>
  const registry = coerceRuntimeMonitorRegistry(raw.registry)
  if (!registry) throw new Error('monitor surface command returned an invalid registry')
  const rawSession = raw.session && typeof raw.session === 'object' && !Array.isArray(raw.session)
    ? raw.session as Record<string, unknown>
    : null
  const surfaceSessionId = typeof rawSession?.surfaceSessionId === 'string'
    ? rawSession.surfaceSessionId
    : typeof rawSession?.surface_session_id === 'string' ? rawSession.surface_session_id : null
  const session = surfaceSessionId
    ? Object.values(registry.sessions).find((candidate) => candidate.surface_session_id === surfaceSessionId) ?? null
    : null
  return {
    ok: raw.ok === true,
    registry: registry as unknown as MonitorSurfaceRegistryDescriptor,
    session: session as unknown as MonitorSurfaceSessionRecord | null,
    message: typeof raw.message === 'string' ? raw.message : '',
  }
}

export async function agentClaimMonitorSurface(request: MonitorSurfaceClaimRequest): Promise<MonitorSurfaceClaimResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  const result = await invoke<unknown>('claim_monitor_surface', { request: {
    slotId: request.slotId,
    owner: serializeMonitorSurfaceOwner(request.owner),
    content: request.initialContent,
    workstationHandoff: { sessionId: `session-${request.slotId}-${Date.now()}`, mode: 'same_live_session' },
    ttlSecs: Math.max(1, Math.floor(request.ttlMs / 1000)),
  } })
  return normalizeMonitorSurfaceClaimResult(result)
}

export async function agentReleaseMonitorSurface(surfaceSessionId: string, owner: MonitorSurfaceOwner): Promise<MonitorSurfaceClaimResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  const result = await invoke<unknown>('release_monitor_surface', {
    surfaceSessionId,
    owner: serializeMonitorSurfaceOwner(owner),
  })
  return normalizeMonitorSurfaceClaimResult(result)
}

export async function agentRefreshMonitorSurfaceLease(surfaceSessionId: string, owner: MonitorSurfaceOwner, ttlMs: number): Promise<MonitorSurfaceClaimResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  const result = await invoke<unknown>('refresh_monitor_surface_lease', {
    surfaceSessionId,
    owner: serializeMonitorSurfaceOwner(owner),
    ttlSecs: Math.max(1, Math.floor(ttlMs / 1000)),
  })
  return normalizeMonitorSurfaceClaimResult(result)
}

export async function agentPatchMonitorSurfacePlayback(
  surfaceSessionId: string,
  owner: MonitorSurfaceOwner,
  expectedRevision: number,
  playback: import('./monitorSurfaceContract').MonitorPlaybackState | null,
): Promise<MonitorSurfaceClaimResult> {
  const { invoke } = await import('@tauri-apps/api/core')
  const result = await invoke<unknown>('patch_monitor_surface_playback', {
    request: {
      surfaceSessionId,
      owner: serializeMonitorSurfaceOwner(owner),
      expectedRevision,
      playback,
    },
  })
  return normalizeMonitorSurfaceClaimResult(result)
}

export async function agentGetMonitorSurfaceRegistry(): Promise<MonitorSurfaceRegistryDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  const result = await invoke<unknown>('get_monitor_surface_registry')
  const registry = coerceRuntimeMonitorRegistry(result)
  if (!registry) throw new Error('monitor surface command returned an invalid registry')
  return registry as unknown as MonitorSurfaceRegistryDescriptor
}

export async function agentRestoreMonitorSurfaceRegistry(registry: MonitorSurfaceRegistryDescriptor): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<void>('restore_monitor_surface_registry', {
    document: toRuntimeMonitorRegistry(registry as unknown as CanonicalMonitorRegistry),
  })
}

export const MONITOR_SURFACE_REGISTRY_STORAGE_KEY = 'arda.monitor-surface-registry.v1'

export async function persistMonitorSurfaceRegistry(registry: MonitorSurfaceRegistryDescriptor): Promise<void> {
  try {
    const raw = typeof window !== 'undefined' ? window.localStorage : null
    raw?.setItem(MONITOR_SURFACE_REGISTRY_STORAGE_KEY, JSON.stringify(registry))
  } catch {
    // Persistence is best-effort; the runtime state remains authoritative.
  }
}

export async function loadPersistedMonitorSurfaceRegistry(): Promise<MonitorSurfaceRegistryDescriptor | null> {
  try {
    const raw = typeof window !== 'undefined' ? window.localStorage : null
    const stored = raw?.getItem(MONITOR_SURFACE_REGISTRY_STORAGE_KEY)
    if (!stored) return null
    const parsed = JSON.parse(stored)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return null
    if (parsed.schema_version !== 'arda.monitor-session-registry.v2') return null
    return parsed as MonitorSurfaceRegistryDescriptor
  } catch {
    return null
  }
}

export async function rehydrateMonitorSurfaceRegistry(): Promise<MonitorSurfaceRegistryDescriptor | null> {
  const persisted = await loadPersistedMonitorSurfaceRegistry()
  if (!persisted) return null
  try {
    await agentRestoreMonitorSurfaceRegistry(persisted)
    return persisted
  } catch {
    return null
  }
}

function serializeMonitorSurfaceOwner(owner: MonitorSurfaceOwner): string {
  if (owner.kind === 'agent' && owner.name) return `agent:${owner.name}`
  if (owner.kind === 'operator' && owner.id) return `operator:${owner.id}`
  if (owner.kind === 'system' && owner.name) return `system:${owner.name}`
  return `${owner.kind}:unknown`
}
