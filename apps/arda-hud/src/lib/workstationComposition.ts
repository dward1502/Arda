import type { ModuleId } from '../components/arda/core/types'
import type { ArdaWorkstationManifest } from './ardaBundleTypes'

export type WorkstationPresentationMode = 'in_scene' | 'native_window'

interface WorkstationComposition {
  title: string
  moduleIds: ModuleId[]
  sourcePanelIds: string[]
  presentationModes: WorkstationPresentationMode[]
}

export interface WorkstationCompositionResolution {
  title: string
  moduleIds: ModuleId[]
  presentationModes: WorkstationPresentationMode[]
  rejectedPanelIds: string[]
  adapted: boolean
}

export type FocusedWorkstationKind = 'fleet' | 'routing' | 'continuity'

const FOCUSED_WORKSTATION_KINDS: Record<string, FocusedWorkstationKind> = {
  fleet_and_backbone: 'fleet',
  systems_health: 'fleet',
  routing_and_comms: 'routing',
  human_business_personal: 'continuity',
}

const WORKSTATION_COMPOSITIONS: Record<string, WorkstationComposition> = {
  sovereign_world: {
    title: 'Sovereign World',
    moduleIds: ['executive_overview', 'systems'],
    sourcePanelIds: ['3d_world', 'executive_overview'],
    presentationModes: ['in_scene', 'native_window'],
  },
  governance_guardhouse: {
    title: 'Governance + Guardhouse',
    moduleIds: ['governance_controls'],
    sourcePanelIds: ['security_posture', 'edge_guardhouse', 'policy_authority'],
    presentationModes: ['in_scene', 'native_window'],
  },
  fleet_and_backbone: {
    title: 'Fleet + Backbone',
    moduleIds: ['systems', 'operations_and_packages'],
    sourcePanelIds: ['fleet_health', 'node_inventory', 'model_inventory', 'hardware_inventory', 'backbone_topology'],
    presentationModes: ['in_scene', 'native_window'],
  },
  routing_and_comms: {
    title: 'Routing + Communications',
    moduleIds: ['systems'],
    sourcePanelIds: ['boardroom', 'inference_router', 'interrupts'],
    presentationModes: ['in_scene', 'native_window'],
  },
  human_business_personal: {
    title: 'Human + Business + Personal',
    moduleIds: ['human_realm'],
    sourcePanelIds: ['human_notes', 'business_ops', 'personal_growth'],
    presentationModes: ['in_scene', 'native_window'],
  },
  business_ops: {
    title: 'Business Operations',
    moduleIds: ['business'],
    sourcePanelIds: ['business_ops', 'boardroom', 'settings'],
    presentationModes: ['in_scene', 'native_window'],
  },
  personal_growth: {
    title: 'Personal Growth',
    moduleIds: ['personal_growth'],
    sourcePanelIds: ['personal_growth', 'human_notes', 'boardroom'],
    presentationModes: ['in_scene', 'native_window'],
  },
  knowledge_and_reasoning: {
    title: 'Knowledge + Research',
    moduleIds: ['research', 'human_realm', 'section_focus'],
    sourcePanelIds: ['research', 'memory', 'knowledge_triage'],
    presentationModes: ['in_scene', 'native_window'],
  },
  planning_and_queue: {
    title: 'Planning + Queue',
    moduleIds: ['planning', 'operating_surface'],
    sourcePanelIds: ['task_board', 'plan_progress', 'escalation_queue'],
    presentationModes: ['in_scene', 'native_window'],
  },
  memory_and_continuity: {
    title: 'Memory + Continuity',
    moduleIds: ['section_focus', 'human_realm'],
    sourcePanelIds: ['memory', 'identity_continuity', 'memory_activity', 'memory_scope_map'],
    presentationModes: ['in_scene', 'native_window'],
  },
  settings: {
    title: 'Settings',
    moduleIds: ['settings'],
    sourcePanelIds: ['settings'],
    presentationModes: ['in_scene', 'native_window'],
  },
  hermes_runtime: {
    title: 'Hermes Dashboard',
    moduleIds: ['hermes_dashboard', 'operations_and_packages'],
    sourcePanelIds: ['hermes_dashboard'],
    presentationModes: ['in_scene', 'native_window'],
  },
}

const HUD_MODULE_IDS = new Set<ModuleId>([
  'executive_overview',
  'operating_surface',
  'research',
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

const STATIC_MANIFESTS: Record<string, { id: string; entryAnchorId: string }> = {
  settings: { id: 'settings_workstation', entryAnchorId: 'settings_workstation_entry' },
  hermes_runtime: { id: 'hermes_dashboard_workstation', entryAnchorId: 'hermes_dashboard_entry' },
}

const STATIC_SOURCE_ZONE_IDS = Object.fromEntries(
  Object.entries(STATIC_MANIFESTS).map(([sourceZoneId, manifest]) => [manifest.id, sourceZoneId]),
)

export function resolveWorkstationComposition(
  sourceZoneId: string,
  sourcePanelIds: string[],
): WorkstationCompositionResolution {
  const composition = WORKSTATION_COMPOSITIONS[sourceZoneId]
  if (composition) {
    const accepted = new Set(composition.sourcePanelIds)
    return {
      title: composition.title,
      moduleIds: [...composition.moduleIds],
      presentationModes: [...composition.presentationModes],
      rejectedPanelIds: sourcePanelIds.filter((panelId) => !accepted.has(panelId)),
      adapted: true,
    }
  }

  const moduleIds = sourcePanelIds.filter((panelId): panelId is ModuleId => HUD_MODULE_IDS.has(panelId as ModuleId))
  return {
    title: sourceZoneId,
    moduleIds,
    presentationModes: ['in_scene', 'native_window'],
    rejectedPanelIds: sourcePanelIds.filter((panelId) => !HUD_MODULE_IDS.has(panelId as ModuleId)),
    adapted: false,
  }
}

export function getFocusedWorkstationKind(sourceZoneId: string | null): FocusedWorkstationKind | null {
  return sourceZoneId ? FOCUSED_WORKSTATION_KINDS[sourceZoneId] ?? null : null
}

export function getStaticWorkstationManifest(sourceZoneId: string | null): ArdaWorkstationManifest | null {
  if (!sourceZoneId) return null
  const staticManifest = STATIC_MANIFESTS[sourceZoneId]
  if (!staticManifest) return null
  const composition = resolveWorkstationComposition(sourceZoneId, [])
  return {
    id: staticManifest.id,
    title: composition.title,
    source_zone_id: sourceZoneId,
    entry_anchor_id: staticManifest.entryAnchorId,
    module_ids: composition.moduleIds,
    presentation_modes: composition.presentationModes,
  }
}

export function getStaticWorkstationManifestById(manifestId: string | null): ArdaWorkstationManifest | null {
  if (!manifestId) return null
  return getStaticWorkstationManifest(STATIC_SOURCE_ZONE_IDS[manifestId] ?? null)
}
