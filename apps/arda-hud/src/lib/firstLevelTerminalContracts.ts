import type { ModuleId } from '../components/arda/core/types'

export const FIRST_LEVEL_TERMINALS = [
  { deskId: 'desk_1', zoneId: 'boardroom.lower.left_wrap', slotId: 'view_desk_l', role: 'governance_decisions', configurable: true },
  { deskId: 'desk_2', zoneId: 'boardroom.lower.left_inner', slotId: 'view_desk_control_panel', role: 'systems_fleet', configurable: true },
  { deskId: 'desk_3', zoneId: 'boardroom.control.center', slotId: null, role: 'command_core_now', configurable: false },
  { deskId: 'desk_4', zoneId: 'boardroom.lower.right_inner', slotId: 'view_desk_r', role: 'routing_communications', configurable: true },
  { deskId: 'desk_5', zoneId: 'boardroom.lower.right_wrap', slotId: 'view_desk_aux', role: 'human_business', configurable: true },
] as const

interface WorkstationProfile {
  moduleIds: ModuleId[]
  sourcePanelIds: string[]
}

const WORKSTATION_PROFILES: Record<string, WorkstationProfile> = {
  sovereign_world: {
    moduleIds: ['executive_overview', 'systems'],
    sourcePanelIds: ['3d_world', 'executive_overview'],
  },
  governance_guardhouse: {
    moduleIds: ['governance_controls', 'section_focus'],
    sourcePanelIds: ['security_posture', 'edge_guardhouse', 'policy_authority'],
  },
  fleet_and_backbone: {
    moduleIds: ['systems', 'operations_and_packages'],
    sourcePanelIds: ['fleet_health', 'node_inventory', 'model_inventory', 'hardware_inventory', 'backbone_topology'],
  },
  routing_and_comms: {
    moduleIds: ['systems', 'operations_and_packages'],
    sourcePanelIds: ['boardroom', 'inference_router', 'interrupts'],
  },
  human_business_personal: {
    moduleIds: ['human_realm', 'business'],
    sourcePanelIds: ['human_notes', 'business_ops', 'personal_growth'],
  },
  business_ops: {
    moduleIds: ['business'],
    sourcePanelIds: ['business_ops', 'boardroom', 'settings'],
  },
  personal_growth: {
    moduleIds: ['personal_growth'],
    sourcePanelIds: ['personal_growth', 'human_notes', 'boardroom'],
  },
  planning_and_queue: {
    moduleIds: ['planning', 'operating_surface'],
    sourcePanelIds: ['task_board', 'plan_progress', 'escalation_queue'],
  },
  memory_and_continuity: {
    moduleIds: ['section_focus', 'human_realm'],
    sourcePanelIds: ['memory', 'identity_continuity', 'memory_activity', 'memory_scope_map'],
  },
}

const HUD_MODULE_IDS = new Set<ModuleId>([
  'executive_overview',
  'operating_surface',
  'section_focus',
  'human_realm',
  'systems',
  'governance_controls',
  'operations_and_packages',
  'hermes_dashboard',
  'planning',
  'learning_loop',
  'business',
  'personal_growth',
  'culture_and_art',
  'service_embed',
  'media_library',
  'settings',
])

export interface WorkstationProfileResolution {
  moduleIds: ModuleId[]
  rejectedPanelIds: string[]
  adapted: boolean
}

export function resolveWorkstationProfile(sourceZoneId: string, sourcePanelIds: string[]): WorkstationProfileResolution {
  const profile = WORKSTATION_PROFILES[sourceZoneId]
  if (profile) {
    const accepted = new Set(profile.sourcePanelIds)
    return {
      moduleIds: [...profile.moduleIds],
      rejectedPanelIds: sourcePanelIds.filter((panelId) => !accepted.has(panelId)),
      adapted: true,
    }
  }

  const moduleIds = sourcePanelIds.filter((panelId): panelId is ModuleId => HUD_MODULE_IDS.has(panelId as ModuleId))
  return {
    moduleIds,
    rejectedPanelIds: sourcePanelIds.filter((panelId) => !HUD_MODULE_IDS.has(panelId as ModuleId)),
    adapted: false,
  }
}
