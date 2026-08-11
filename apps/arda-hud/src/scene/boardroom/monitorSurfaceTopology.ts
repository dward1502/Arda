// sigil: REPAIR
import type { BoardroomSceneSlotId, UpperMonitorSlotId } from '../../lib/boardroomSlotSettings'
import { BOARDROOM_MONITOR_ZONES } from './boardroomSpatialLayout'

export interface MonitorSurfaceTopologySlot {
  slotId: UpperMonitorSlotId
  zoneId: string
  label: string
  position: [number, number, number]
  rotation: [number, number, number]
  size: [number, number, number]
  binding: string
}

export const UPPER_MONITOR_TOPOLOGY_SLOTS: MonitorSurfaceTopologySlot[] = BOARDROOM_MONITOR_ZONES.map((zone) => ({
  slotId: zone.assignmentSlotId as UpperMonitorSlotId,
  zoneId: zone.id,
  label: zone.label,
  position: zone.position,
  rotation: zone.rotation,
  size: zone.size,
  binding: zone.binding,
}))

export function getMonitorSurfaceTopologySlot(
  slotId: UpperMonitorSlotId,
): MonitorSurfaceTopologySlot | null {
  return UPPER_MONITOR_TOPOLOGY_SLOTS.find((slot) => slot.slotId === slotId) ?? null
}

export function getMonitorSurfaceTopologySlotByZoneId(zoneId: string): MonitorSurfaceTopologySlot | null {
  return UPPER_MONITOR_TOPOLOGY_SLOTS.find((slot) => slot.zoneId === zoneId) ?? null
}

export function resolveUpperMonitorSlotIdFromZoneId(zoneId: string): UpperMonitorSlotId | null {
  return UPPER_MONITOR_TOPOLOGY_SLOTS.find((slot) => slot.zoneId === zoneId)?.slotId ?? null
}

export function isUpperMonitorSlotId(value: unknown): value is UpperMonitorSlotId {
  return typeof value === 'string' && UPPER_MONITOR_TOPOLOGY_SLOTS.some((slot) => slot.slotId === value)
}

export function assertUniqueTopologySlots(): true {
  const seen = new Set<string>()
  for (const slot of UPPER_MONITOR_TOPOLOGY_SLOTS) {
    if (seen.has(slot.slotId)) {
      throw new Error(`Duplicate monitor topology slot: ${slot.slotId}`)
    }
    if (seen.has(slot.zoneId)) {
      throw new Error(`Duplicate monitor topology zone: ${slot.zoneId}`)
    }
    seen.add(slot.slotId)
    seen.add(slot.zoneId)
  }
  return true
}
