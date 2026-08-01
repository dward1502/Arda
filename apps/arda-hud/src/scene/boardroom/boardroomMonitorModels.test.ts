// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { BOARDROOM_CONTROL_ZONES, BOARDROOM_MONITOR_ZONES } from './boardroomSpatialLayout'
import { deriveBoardroomMonitorModelBinding } from './boardroomMonitorModels'

describe('boardroom monitor model bindings', () => {
  it('maps every upper monitor surface to its accepted GLB scene binding', () => {
    expect(BOARDROOM_MONITOR_ZONES.map((zone) => deriveBoardroomMonitorModelBinding(zone))).toEqual(
      BOARDROOM_MONITOR_ZONES.map((zone) => ({
        zoneId: zone.id,
        binding: zone.binding,
        fitSize: zone.size,
        surfaceOffset: [0, 0, 0],
      })),
    )
    expect(BOARDROOM_MONITOR_ZONES.map((zone) => zone.binding)).toEqual([
      'upper_monitor_1',
      'upper_monitor_2',
      'upper_monitor_3',
      'upper_monitor_3',
      'upper_monitor_4',
    ])
  })

  it('keeps model bindings separate from workstation assignment semantics', () => {
    for (const zone of BOARDROOM_MONITOR_ZONES) {
      const modelBinding = deriveBoardroomMonitorModelBinding(zone)

      expect(modelBinding?.binding).toBe(zone.binding)
      expect(modelBinding?.binding).not.toBe(zone.assignmentSlotId)
      expect(modelBinding?.zoneId).toBe(zone.id)
    }
  })

  it('maps every lower desk surface to its authored control housing', () => {
    const lowerSurfaces = BOARDROOM_CONTROL_ZONES.filter((zone) => zone.kind === 'desk_surface')

    expect(lowerSurfaces.map((zone) => deriveBoardroomMonitorModelBinding(zone))).toEqual(
      lowerSurfaces.map((zone) => ({
        zoneId: zone.id,
        binding: zone.binding,
        fitSize: zone.size,
        surfaceOffset: [0, 0, 0],
      })),
    )
  })
})
