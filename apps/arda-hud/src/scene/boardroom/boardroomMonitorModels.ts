// sigil: REPAIR
import type { BoardroomSpatialZone, BoardroomVec3 } from './boardroomSpatialLayout'

export interface BoardroomMonitorModelBinding {
  zoneId: string
  binding: string
  fitSize: BoardroomVec3
  surfaceOffset: BoardroomVec3
}

export function deriveBoardroomMonitorModelBinding(zone: BoardroomSpatialZone): BoardroomMonitorModelBinding | null {
  if (zone.kind !== 'upper_monitor' || !zone.binding) return null

  return {
    zoneId: zone.id,
    binding: zone.binding,
    fitSize: [...zone.size],
    surfaceOffset: [0, 0, 0],
  }
}
