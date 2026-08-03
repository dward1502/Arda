// sigil: REPAIR
import type { BoardroomSceneSlotId } from '../../lib/boardroomSlotSettings'

export type BoardroomZoneKind =
  | 'upper_monitor'
  | 'desk_surface'
  | 'control_panel'
  | 'physical_button'
  | 'avatar_emitter'
  | 'world_window'
  | 'status_band'

export type BoardroomZoneInteraction =
  | 'open_workstation'
  | 'open_settings'
  | 'open_hermes'
  | 'transition_world'
  | 'presence_focus'
  | 'display_only'

export type BoardroomPreviewMode = 'monitor_surface' | 'desk_surface' | 'button' | 'portal' | 'presence'

export type BoardroomVec3 = [number, number, number]
export type BoardroomZonePositionOverrides = Record<string, BoardroomVec3>

export type BoardroomConsoleShellRole = 'outer_left' | 'inner_left' | 'center' | 'inner_right' | 'outer_right'

export interface BoardroomConsoleShellSegment {
  id: string
  role: BoardroomConsoleShellRole
  position: BoardroomVec3
  rotation: BoardroomVec3
  size: BoardroomVec3
  accent: string
}

export const BOARDROOM_CONSOLE_SHELL_SEGMENTS: BoardroomConsoleShellSegment[] = [
  {
    id: 'boardroom.console.outer_left',
    role: 'outer_left',
    position: [-3.3, -0.18, 1.32],
    rotation: [0, 0.64, 0],
    size: [1.9, 0.38, 1.64],
    accent: '#ffd37a',
  },
  {
    id: 'boardroom.console.inner_left',
    role: 'inner_left',
    position: [-1.65, -0.18, 0.56],
    rotation: [0, 0.28, 0],
    size: [1.92, 0.4, 1.9],
    accent: '#5defff',
  },
  {
    id: 'boardroom.console.center',
    role: 'center',
    position: [0, -0.18, 0.3],
    rotation: [0, 0, 0],
    size: [1.72, 0.42, 2.08],
    accent: '#d8e7ff',
  },
  {
    id: 'boardroom.console.inner_right',
    role: 'inner_right',
    position: [1.65, -0.18, 0.56],
    rotation: [0, -0.28, 0],
    size: [1.92, 0.4, 1.9],
    accent: '#8cffc7',
  },
  {
    id: 'boardroom.console.outer_right',
    role: 'outer_right',
    position: [3.3, -0.18, 1.32],
    rotation: [0, -0.64, 0],
    size: [1.9, 0.38, 1.64],
    accent: '#ffa6d9',
  },
]

export const BOARDROOM_KINETIC_CONTROL_PANEL = {
  id: 'boardroom.controls.kinetic_panel',
  position: [0, 0.355, 1.42] as BoardroomVec3,
  rotation: [0, 0, 0] as BoardroomVec3,
  size: [0.9, 0.04, 0.3] as BoardroomVec3,
  accent: '#5defff',
}

export interface BoardroomSpatialZone {
  id: string
  label: string
  kind: BoardroomZoneKind
  interaction: BoardroomZoneInteraction
  binding?: string
  assignmentSlotId?: BoardroomSceneSlotId
  assignmentIndex?: number
  position: BoardroomVec3
  rotation: BoardroomVec3
  size: BoardroomVec3
  color: string
  primary?: boolean
  previewMode: BoardroomPreviewMode
  detail?: string
}

export const BOARDROOM_SPATIAL_ZONES: BoardroomSpatialZone[] = [
  {
    id: 'boardroom.monitor.left', label: 'Monitor 00', kind: 'upper_monitor', interaction: 'open_workstation',
    binding: 'upper_monitor_1', assignmentSlotId: 'monitor_left_1', assignmentIndex: 0,
    position: [-3.8, 1.48, -0.62], rotation: [0, 0.415, 0], size: [1.63, 0.8, 0.08],
    color: '#5defff', previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.monitor.center_left', label: 'Monitor 01', kind: 'upper_monitor', interaction: 'open_workstation',
    binding: 'upper_monitor_2', assignmentSlotId: 'monitor_left_2', assignmentIndex: 1,
    position: [-1.9, 1.54, -0.78], rotation: [0, 0.218, 0], size: [1.63, 0.8, 0.08],
    color: '#5defff', previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.monitor.center', label: 'Monitor 02', kind: 'upper_monitor', interaction: 'open_workstation',
    binding: 'upper_monitor_3', assignmentIndex: 2, position: [0, 1.57, -0.84], rotation: [0, 0, 0],
    size: [1.63, 0.8, 0.08], color: '#5defff', primary: true, previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.monitor.center_right', label: 'Monitor 03', kind: 'upper_monitor', interaction: 'open_workstation',
    binding: 'upper_monitor_3', assignmentSlotId: 'monitor_left_3', assignmentIndex: 2,
    position: [1.9, 1.54, -0.78], rotation: [0, -0.218, 0], size: [1.63, 0.8, 0.08],
    color: '#5defff', previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.monitor.right', label: 'Monitor 04', kind: 'upper_monitor', interaction: 'open_workstation',
    binding: 'upper_monitor_4', assignmentSlotId: 'monitor_left_4', assignmentIndex: 3,
    position: [3.8, 1.48, -0.62], rotation: [0, -0.415, 0], size: [1.63, 0.8, 0.08],
    color: '#5defff', previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.lower.left_wrap',
    label: 'Governance Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    binding: 'governance_control',
    assignmentSlotId: 'view_desk_l',
    assignmentIndex: 0,
    position: [-3.8, 0.65, 0.05],
    rotation: [0.506, 0.44, 0],
    size: [1.58, 0.04, 0.72],
    color: '#ffd37a',
    previewMode: 'desk_surface',
  },
  {
    id: 'boardroom.lower.left_inner',
    label: 'Systems Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    binding: 'systems_control',
    assignmentSlotId: 'view_desk_control_panel',
    assignmentIndex: 1,
    position: [-1.9, 0.67, -0.08],
    rotation: [0.506, 0.17, 0],
    size: [1.24, 0.04, 0.62],
    color: '#5defff',
    previewMode: 'desk_surface',
  },
  {
    id: 'boardroom.lower.right_inner',
    label: 'Network Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    binding: 'network_control',
    assignmentSlotId: 'view_desk_r',
    assignmentIndex: 2,
    position: [1.9, 0.67, -0.08],
    rotation: [0.506, -0.17, 0],
    size: [1.24, 0.04, 0.62],
    color: '#8cffc7',
    previewMode: 'desk_surface',
  },
  {
    id: 'boardroom.lower.right_wrap',
    label: 'Human Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    binding: 'human_control',
    assignmentSlotId: 'view_desk_aux',
    assignmentIndex: 3,
    position: [3.8, 0.65, 0.05],
    rotation: [0.506, -0.44, 0],
    size: [1.58, 0.04, 0.72],
    color: '#ffa6d9',
    previewMode: 'desk_surface',
  },
  {
    id: 'boardroom.control.center',
    label: 'Control Core',
    kind: 'control_panel',
    interaction: 'open_workstation',
    binding: 'settings_control',
    position: [0, 0.68, -0.12],
    rotation: [0.506, 0, 0],
    size: [1.58, 0.04, 0.74],
    color: '#d8e7ff',
    primary: true,
    previewMode: 'desk_surface',
    detail: 'Command Core',
  },
  {
    id: 'boardroom.button.hermes',
    label: 'Hermes Dashboard',
    kind: 'physical_button',
    interaction: 'open_hermes',
    binding: 'human_control',
    position: [0.3, 0.414, 1.42],
    rotation: [0, 0, 0],
    size: [0.14, 0.045, 0.16],
    color: '#a855f7',
    primary: true,
    previewMode: 'button',
    detail: 'Tools + Abilities',
  },
  {
    id: 'boardroom.button.hermes_cli',
    label: 'Hermes CLI',
    kind: 'physical_button',
    interaction: 'open_hermes',
    binding: 'human_control',
    position: [0.1, 0.414, 1.42],
    rotation: [0, 0, 0],
    size: [0.14, 0.045, 0.16],
    color: '#22d3ee',
    primary: false,
    previewMode: 'button',
    detail: 'Open Hermes CLI',
  },
  {
    id: 'boardroom.button.service_health',
    label: 'Service Health',
    kind: 'physical_button',
    interaction: 'open_workstation',
    binding: 'fleet_and_backbone',
    position: [-0.3, 0.414, 1.42],
    rotation: [0, 0, 0],
    size: [0.14, 0.045, 0.16],
    color: '#5defff',
    previewMode: 'button',
    detail: 'Read-only fleet status',
  },
  {
    id: 'boardroom.button.settings',
    label: 'Settings',
    kind: 'physical_button',
    interaction: 'open_settings',
    binding: 'settings_control',
    position: [-0.1, 0.414, 1.42],
    rotation: [0, 0, 0],
    size: [0.14, 0.045, 0.16],
    color: '#d8e7ff',
    previewMode: 'button',
  },
  {
    id: 'boardroom.avatar.emitter',
    label: 'Avatar Emitter',
    kind: 'avatar_emitter',
    interaction: 'presence_focus',
    binding: 'hologram_anchor',
    position: [0, 0.38, 0.22],
    rotation: [0, 0, 0],
    size: [1.45, 0.2, 1.45],
    color: '#b98cff',
    previewMode: 'presence',
  },
  {
    id: 'boardroom.world.window',
    label: 'Enter World',
    kind: 'world_window',
    interaction: 'transition_world',
    binding: 'world_gate',
    position: [0, 2.85, -4.74],
    rotation: [0, 0, 0],
    size: [2.9, 2.05, 0.22],
    color: '#4fe6ff',
    primary: true,
    previewMode: 'portal',
    detail: 'City Window',
  },
]

export const BOARDROOM_MONITOR_ZONES = BOARDROOM_SPATIAL_ZONES.filter((zone) => zone.kind === 'upper_monitor')
export const BOARDROOM_CONTROL_ZONES = BOARDROOM_SPATIAL_ZONES.filter((zone) => zone.kind === 'desk_surface' && zone.assignmentSlotId)

export function getBoardroomSpatialZone(id: string): BoardroomSpatialZone | null {
  return BOARDROOM_SPATIAL_ZONES.find((zone) => zone.id === id) ?? null
}

export function normalizeBoardroomZonePositionOverrides(
  raw: unknown,
  zones: BoardroomSpatialZone[] = BOARDROOM_SPATIAL_ZONES,
): BoardroomZonePositionOverrides {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {}

  const validZoneIds = new Set(zones.map((zone) => zone.id))
  return Object.entries(raw as Record<string, unknown>).reduce<BoardroomZonePositionOverrides>((overrides, [zoneId, value]) => {
    if (!validZoneIds.has(zoneId)) return overrides
    if (!Array.isArray(value) || value.length !== 3) return overrides
    if (!value.every((axis) => typeof axis === 'number' && Number.isFinite(axis))) return overrides
    overrides[zoneId] = value.map((axis) => Number(axis.toFixed(3))) as BoardroomVec3
    return overrides
  }, {})
}

export function serializeBoardroomZonePositionOverrides(
  overrides: BoardroomZonePositionOverrides,
  zones: BoardroomSpatialZone[] = BOARDROOM_SPATIAL_ZONES,
): string {
  const normalized = normalizeBoardroomZonePositionOverrides(overrides, zones)
  const zoneById = new Map(zones.map((zone) => [zone.id, zone]))
  const lines = Object.keys(normalized)
    .sort((a, b) => (zoneById.get(a)?.label ?? a).localeCompare(zoneById.get(b)?.label ?? b))
    .map((zoneId) => {
      const position = normalized[zoneId]
      return `  ${JSON.stringify(zoneId)}: [${position.map((axis) => axis.toFixed(3)).join(', ')}],`
    })

  return [
    '// Paste these accepted edit-mode positions back into BOARDROOM_SPATIAL_ZONES.',
    '// Values are filtered to known boardroom zone ids and rounded to 3 decimals.',
    'const acceptedBoardroomZonePositions = {',
    ...lines,
    '} as const',
    '',
  ].join('\n')
}
