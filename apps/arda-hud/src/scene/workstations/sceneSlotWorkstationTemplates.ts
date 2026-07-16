// sigil: REPAIR
import type { ArdaWorkstationManifest } from '../../lib/ardaSource'
import {
  BOARDROOM_SCENE_SLOT_IDS,
  type BoardroomSceneSlotId,
} from '../../lib/boardroomSlotSettings'

export const SCENE_SLOT_ZONE_PREFIX = 'scene_slot:'
const SCENE_SLOT_WORKSTATION_PREFIX = 'scene_slot_'
const SCENE_SLOT_WORKSTATION_SUFFIX = '_workstation'

interface SceneSlotWorkstationTemplate {
  title: string
  moduleIds: string[]
  presentationModes: Array<'in_scene' | 'native_window'>
}

const SCENE_SLOT_WORKSTATION_TEMPLATES: Record<BoardroomSceneSlotId, SceneSlotWorkstationTemplate> = {
  monitor_left_1: {
    title: 'Monitor Left 1 Service Template',
    moduleIds: ['service_embed', 'systems'],
    presentationModes: ['in_scene', 'native_window'],
  },
  monitor_left_2: {
    title: 'Monitor Left 2 Routing Template',
    moduleIds: ['operations_and_packages', 'systems', 'governance_controls'],
    presentationModes: ['in_scene', 'native_window'],
  },
  monitor_left_3: {
    title: 'Monitor Left 3 Knowledge Template',
    moduleIds: ['section_focus', 'human_realm'],
    presentationModes: ['in_scene', 'native_window'],
  },
  monitor_left_4: {
    title: 'Monitor Left 4 Planning Template',
    moduleIds: ['planning', 'section_focus'],
    presentationModes: ['in_scene', 'native_window'],
  },
  view_desk_l: {
    title: 'Desk Left Review Template',
    moduleIds: ['governance_controls', 'section_focus'],
    presentationModes: ['in_scene', 'native_window'],
  },
  view_desk_control_panel: {
    title: 'Desk Control Command Template',
    moduleIds: ['operating_surface', 'executive_overview', 'systems'],
    presentationModes: ['in_scene'],
  },
  view_desk_r: {
    title: 'Desk Right Human Realm Template',
    moduleIds: ['human_realm', 'business', 'personal_growth'],
    presentationModes: ['in_scene', 'native_window'],
  },
  view_desk_aux: {
    title: 'Desk Aux Hermes Template',
    moduleIds: ['hermes_dashboard', 'operations_and_packages'],
    presentationModes: ['in_scene', 'native_window'],
  },
}

function isBoardroomSceneSlotId(value: string): value is BoardroomSceneSlotId {
  return BOARDROOM_SCENE_SLOT_IDS.includes(value as BoardroomSceneSlotId)
}

function manifestIdForSlot(slotId: string): string {
  return `${SCENE_SLOT_WORKSTATION_PREFIX}${slotId}${SCENE_SLOT_WORKSTATION_SUFFIX}`
}

export function sceneSlotZoneIdForSlot(slotId: string): string {
  return `${SCENE_SLOT_ZONE_PREFIX}${slotId}`
}

export function getSceneSlotWorkstationTemplate(slotId: string): SceneSlotWorkstationTemplate | null {
  return isBoardroomSceneSlotId(slotId) ? SCENE_SLOT_WORKSTATION_TEMPLATES[slotId] : null
}

export function getSceneSlotWorkstationManifestByZoneId(zoneId: string | null): ArdaWorkstationManifest | null {
  if (!zoneId?.startsWith(SCENE_SLOT_ZONE_PREFIX)) return null
  const slotId = zoneId.slice(SCENE_SLOT_ZONE_PREFIX.length)
  const template = getSceneSlotWorkstationTemplate(slotId)
  if (!template) return null
  return {
    id: manifestIdForSlot(slotId),
    title: template.title,
    source_zone_id: zoneId,
    entry_anchor_id: `${slotId}_template_entry`,
    module_ids: template.moduleIds,
    presentation_modes: template.presentationModes,
  }
}

export function getSceneSlotWorkstationManifestById(manifestId: string | null): ArdaWorkstationManifest | null {
  if (!manifestId?.startsWith(SCENE_SLOT_WORKSTATION_PREFIX) || !manifestId.endsWith(SCENE_SLOT_WORKSTATION_SUFFIX)) return null
  const slotId = manifestId.slice(
    SCENE_SLOT_WORKSTATION_PREFIX.length,
    -SCENE_SLOT_WORKSTATION_SUFFIX.length,
  )
  return getSceneSlotWorkstationManifestByZoneId(sceneSlotZoneIdForSlot(slotId))
}

export function getSceneSlotWorkstationTemplates(): Record<BoardroomSceneSlotId, SceneSlotWorkstationTemplate> {
  return { ...SCENE_SLOT_WORKSTATION_TEMPLATES }
}
