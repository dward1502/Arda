// sigil: REPAIR
import { readFile, writeScopedFile, type FileReadResult } from './weathertop'
import { loopbackUrl } from './endpointConfig'
import { parseJsonOrDefault, parseJsonOrNull } from './jsonParse'
import {
  defaultBoardroomVisualizationSelection,
  isBoardroomVisualizationSelection,
  resolveBoardroomVisualizationSelection,
  type BoardroomVisualizationSelection,
} from '../scene/boardroom/boardroomVisualizationPresets'

export const ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH = 'core/state/arda_boardroom_slots.json'
export const ARDA_BOARDROOM_SLOT_STORAGE_KEY = 'arda.boardroom.scene_slots.v1'

export const BOARDROOM_MONITOR_SLOT_IDS = ['monitor_left_1', 'monitor_left_2', 'monitor_left_3', 'monitor_left_4'] as const
export const BOARDROOM_CONTROL_SLOT_IDS = ['view_desk_l', 'view_desk_control_panel', 'view_desk_r', 'view_desk_aux'] as const
export const BOARDROOM_SCENE_SLOT_IDS = [...BOARDROOM_MONITOR_SLOT_IDS, ...BOARDROOM_CONTROL_SLOT_IDS] as const

export type BoardroomSceneSlotId = typeof BOARDROOM_SCENE_SLOT_IDS[number]
export type BoardroomSceneSlotAssignments = Record<BoardroomSceneSlotId, string>
export type BoardroomWorkstationRoleId = 'fleet' | 'work' | 'decisions' | 'knowledge' | 'evidence' | 'settings'
export type BoardroomSurfaceAdapterType = 'component_grid' | 'external_url' | 'service_embed' | 'media_viewer' | 'streaming_text' | 'remote_desktop' | 'agent_activity'
export type BoardroomSurfacePreviewMode = 'component_grid' | 'service_status' | 'inline_embed' | 'media_thumbnail' | 'stream_feed' | 'remote_preview' | 'agent_activity'
export type BoardroomSurfaceFocusMode = 'in_scene_workstation' | 'native_window' | 'external_browser' | 'inline_embed'
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
  schema_version: 'arda.arda_boardroom_slots.v1'
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
  monitor_left_1: 'service_warp_dev',
  monitor_left_2: 'routing_and_comms',
  monitor_left_3: 'memory_and_continuity',
  monitor_left_4: 'planning_and_queue',
  view_desk_l: 'governance_guardhouse',
  view_desk_control_panel: 'sovereign_world',
  view_desk_r: 'human_realm',
  view_desk_aux: 'now_command',
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
  monitor_left_1: {
    component_id: 'warp-dev-service-surface',
    source_zone_id: 'service_warp_dev',
    title: 'Warp Surface',
    module_ids: ['service_embed'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  monitor_left_2: {
    component_id: 'routing-providers-workstation',
    source_zone_id: 'routing_and_comms',
    title: 'Routing Providers',
    module_ids: ['operations_and_packages', 'governance_controls'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  monitor_left_3: {
    component_id: 'knowledge-workstation',
    source_zone_id: 'memory_and_continuity',
    title: 'Knowledge + Memory',
    module_ids: ['section_focus', 'human_realm'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  monitor_left_4: {
    component_id: 'queue-plans-workstation',
    source_zone_id: 'planning_and_queue',
    title: 'Queue + Plans',
    module_ids: ['planning', 'section_focus'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  view_desk_l: {
    component_id: 'review-gates-workstation',
    source_zone_id: 'governance_guardhouse',
    title: 'Review Gates',
    module_ids: ['governance_controls', 'section_focus'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  view_desk_control_panel: {
    component_id: 'command-podium-workstation',
    source_zone_id: 'sovereign_world',
    title: 'Command Podium',
    module_ids: ['executive_overview', 'systems'],
    presentation_modes: ['in_scene'],
  },
  view_desk_r: {
    component_id: 'human-business-workstation',
    source_zone_id: 'human_realm',
    title: 'Human + Business',
    module_ids: ['human_realm', 'business'],
    presentation_modes: ['in_scene', 'native_window'],
  },
  view_desk_aux: {
    component_id: 'daily-command-workstation',
    source_zone_id: 'now_command',
    title: 'Daily Command',
    module_ids: ['operating_surface', 'executive_overview'],
    presentation_modes: ['in_scene', 'native_window'],
  },
}

const LOCAL_SERVICE_EMBED_URLS: Record<string, string> = {
  service_beelink_grafana: 'http://100.103.125.88:3000',
  service_beelink_openwebui: 'http://100.103.125.88:8080',
}

function isBoardroomSceneSlotId(value: unknown): value is BoardroomSceneSlotId {
  return typeof value === 'string' && BOARDROOM_SCENE_SLOT_IDS.includes(value as BoardroomSceneSlotId)
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

export function inferBoardroomRoleId(sourceZoneId: string | null | undefined): BoardroomWorkstationRoleId | null {
  if (!sourceZoneId) return null
  if (sourceZoneId === 'systems_health' || sourceZoneId === 'routing_health' || sourceZoneId === 'sovereign_world') return 'fleet'
  return BOARDROOM_WORKSTATION_ROLE_PROFILES.find((profile) => profile.source_zone_id === sourceZoneId)?.role_id ?? null
}

function resolveAssignmentProfile(sourceZoneId: string): BoardroomRoleAssignmentProfile | null {
  const roleId = inferBoardroomRoleId(sourceZoneId)
  return roleId ? getBoardroomRoleProfileByRoleId(roleId) : null
}

function createDefaultSurfaceLayout(slotId: BoardroomSceneSlotId, sourceZoneId: string, componentId: string): BoardroomSurfaceLayout {
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
          kind: typeof record.kind === 'string' ? record.kind as BoardroomSurfaceWidgetKind : fallback.preview.widgets[index]?.kind ?? 'metric_strip',
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

export function parseBoardroomSlotSettings(value: unknown): BoardroomSlotSettingsDocument | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const record = value as Record<string, unknown>
  if (record.schema_version !== 'arda.arda_boardroom_slots.v1') return null
  const rawAssignments = Array.isArray(record.assignments) ? record.assignments : []
  const defaults = createDefaultBoardroomSlotSettings(typeof record.updated_at_utc === 'string' ? record.updated_at_utc : new Date(0).toISOString())
  const bySlot = new Map<BoardroomSceneSlotId, BoardroomSlotAssignmentRecord>()

  for (const rawAssignment of rawAssignments) {
    if (!rawAssignment || typeof rawAssignment !== 'object' || Array.isArray(rawAssignment)) continue
    const assignment = rawAssignment as Record<string, unknown>
    if (!isBoardroomSceneSlotId(assignment.slot_id)) continue
    const slotId = assignment.slot_id
    const fallback = defaults.assignments.find((candidate) => candidate.slot_id === slotId)
    if (!fallback) continue
    const explicitRoleId = isBoardroomWorkstationRoleId(assignment.role_id) ? assignment.role_id : null
    const roleProfile = explicitRoleId ? getBoardroomRoleProfileByRoleId(explicitRoleId) : null
    const sourceZoneId = typeof assignment.source_zone_id === 'string'
      ? assignment.source_zone_id
      : roleProfile?.source_zone_id ?? fallback.source_zone_id
    const inferredRoleId = explicitRoleId ?? inferBoardroomRoleId(sourceZoneId) ?? undefined
    const profile = roleProfile ?? resolveAssignmentProfile(sourceZoneId)
    const componentId = typeof assignment.component_id === 'string' ? assignment.component_id : profile?.component_id ?? fallback.component_id
    bySlot.set(slotId, {
      slot_id: slotId,
      ...(inferredRoleId ? { role_id: inferredRoleId } : {}),
      component_id: componentId,
      source_zone_id: sourceZoneId,
      title: typeof assignment.title === 'string' ? assignment.title : profile?.title ?? fallback.title,
      module_ids: stringArray(assignment.module_ids).length > 0 ? stringArray(assignment.module_ids) : profile?.module_ids ?? fallback.module_ids,
      presentation_modes: stringArray(assignment.presentation_modes).length > 0 ? stringArray(assignment.presentation_modes) : profile?.presentation_modes ?? fallback.presentation_modes,
      surface_layout: parseSurfaceLayout(assignment.surface_layout, createDefaultSurfaceLayout(slotId, sourceZoneId, componentId)),
      visualization: parseVisualizationSelection(assignment.visualization, sourceZoneId, fallback.visualization),
      updated_at_utc: typeof assignment.updated_at_utc === 'string' ? assignment.updated_at_utc : fallback.updated_at_utc,
    })
  }

  return {
    schema_version: 'arda.arda_boardroom_slots.v1',
    authority: 'core/state/arda_boardroom_slots.json',
    operator_profile_id: typeof record.operator_profile_id === 'string' ? record.operator_profile_id : null,
    updated_at_utc: typeof record.updated_at_utc === 'string' ? record.updated_at_utc : defaults.updated_at_utc,
    assignments: BOARDROOM_SCENE_SLOT_IDS.map((slotId) => bySlot.get(slotId) ?? defaults.assignments.find((candidate) => candidate.slot_id === slotId)!),
  }
}

export function assignmentsFromDocument(document: BoardroomSlotSettingsDocument): BoardroomSceneSlotAssignments {
  return BOARDROOM_SCENE_SLOT_IDS.reduce<BoardroomSceneSlotAssignments>((assignments, slotId) => {
    const record = document.assignments.find((assignment) => assignment.slot_id === slotId)
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
