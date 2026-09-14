import type { ModuleId } from '../components/arda/core/types'
import { resolveWorkstationComposition } from './workstationComposition'

export const FIRST_LEVEL_TERMINALS = [
  { deskId: 'desk_1', zoneId: 'boardroom.lower.left_wrap', slotId: 'view_desk_l', role: 'governance_decisions', configurable: true },
  { deskId: 'desk_2', zoneId: 'boardroom.lower.left_inner', slotId: 'view_desk_control_panel', role: 'systems_fleet', configurable: true },
  { deskId: 'desk_3', zoneId: 'boardroom.control.center', slotId: null, role: 'command_core_now', configurable: false },
  { deskId: 'desk_4', zoneId: 'boardroom.lower.right_inner', slotId: 'view_desk_r', role: 'routing_communications', configurable: true },
  { deskId: 'desk_5', zoneId: 'boardroom.lower.right_wrap', slotId: 'view_desk_aux', role: 'human_business', configurable: true },
] as const

export interface WorkstationProfileResolution {
  moduleIds: ModuleId[]
  rejectedPanelIds: string[]
  adapted: boolean
}

export function resolveWorkstationProfile(sourceZoneId: string, sourcePanelIds: string[]): WorkstationProfileResolution {
  const composition = resolveWorkstationComposition(sourceZoneId, sourcePanelIds)
  return {
    moduleIds: composition.moduleIds,
    rejectedPanelIds: composition.rejectedPanelIds,
    adapted: composition.adapted,
  }
}
