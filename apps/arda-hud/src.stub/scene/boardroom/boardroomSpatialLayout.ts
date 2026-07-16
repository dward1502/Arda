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
    id: 'boardroom.monitor.left',
    label: 'Monitor 01',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    binding: 'upper_monitor_1',
    assignmentSlotId: 'monitor_left_1',
    assignmentIndex: 0,
    position: [-3.35, 2.48, -2.7],
    rotation: [0, 0.34, 0],
    size: [1.62, 0.96, 0.16],
    color: '#5defff',
    previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.monitor.center_left',
    label: 'Monitor 02',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    binding: 'upper_monitor_2',
    assignmentSlotId: 'monitor_left_2',
    assignmentIndex: 1,
    position: [-1.16, 2.68, -3.06],
    rotation: [0, 0.12, 0],
    size: [1.62, 0.96, 0.16],
    color: '#5defff',
    previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.monitor.center_right',
    label: 'Monitor 03',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    binding: 'upper_monitor_3',
    assignmentSlotId: 'monitor_left_3',
    assignmentIndex: 2,
    position: [1.16, 2.68, -3.06],
    rotation: [0, -0.12, 0],
    size: [1.62, 0.96, 0.16],
    color: '#5defff',
    previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.monitor.right',
    label: 'Monitor 04',
    kind: 'upper_monitor',
    interaction: 'open_workstation',
    binding: 'upper_monitor_4',
    assignmentSlotId: 'monitor_left_4',
    assignmentIndex: 3,
    position: [3.35, 2.48, -2.7],
    rotation: [0, -0.34, 0],
    size: [1.62, 0.96, 0.16],
    color: '#5defff',
    previewMode: 'monitor_surface',
  },
  {
    id: 'boardroom.lower.left_wrap',
    label: 'Governance Console',
    kind: 'desk_surface',
    interaction: 'open_workstation',
    binding: 'governance_control',
    assignmentSlotId: 'view_desk_l',
    assignmentIndex: 0,
    position: [-3.28, 0.62, 2.02],
    rotation: [-0.28, 0.92, -0.05],
    size: [1.28, 0.24, 0.86],
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
    position: [-1.58, 0.6, 1.48],
    rotation: [-0.24, 0.42, -0.02],
    size: [1.22, 0.24, 0.84],
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
    position: [1.58, 0.6, 1.48],
    rotation: [-0.24, -0.42, 0.02],
    size: [1.22, 0.24, 0.84],
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
    position: [3.28, 0.62, 2.02],
    rotation: [-0.28, -0.92, 0.05],
    size: [1.28, 0.24, 0.86],
    color: '#ffa6d9',
    previewMode: 'desk_surface',
  },
  {
    id: 'boardroom.control.center',
    label: 'Control Core',
    kind: 'control_panel',
    interaction: 'open_settings',
    binding: 'settings_control',
    position: [0, 0.58, 1.14],
    rotation: [-0.22, 0, 0],
    size: [1.7, 0.26, 0.96],
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
    position: [2.92, 0.86, 1.2],
    rotation: [-0.22, -0.52, 0.02],
    size: [1.0, 0.22, 0.38],
    color: '#a855f7',
    primary: true,
    previewMode: 'button',
    detail: 'Tools + Abilities',
  },
  {
    id: 'boardroom.button.settings',
    label: 'Settings',
    kind: 'physical_button',
    interaction: 'open_settings',
    binding: 'settings_control',
    position: [-0.9, 0.74, 1.86],
    rotation: [-0.18, 0, 0],
    size: [0.96, 0.2, 0.34],
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
    size: [1.16, 0.2, 1.16],
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
