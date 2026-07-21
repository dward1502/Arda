// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { BOARDROOM_MONITOR_ZONES } from './boardroomSpatialLayout'
import { deriveBoardroomMonitorModelBinding } from './boardroomMonitorModels'

describe('boardroom monitor model bindings', () => {
  it('maps every upper monitor surface to its accepted GLB scene binding', () => {
    expect(BOARDROOM_MONITOR_ZONES.map((zone) => deriveBoardroomMonitorModelBinding(zone))).toEqual([
      {
        zoneId: 'boardroom.monitor.left',
        binding: 'upper_monitor_1',
        fitSize: [1.62, 0.96, 0.16],
        surfaceOffset: [0, 0, 0],
      },
      {
        zoneId: 'boardroom.monitor.center_left',
        binding: 'upper_monitor_2',
        fitSize: [1.62, 0.96, 0.16],
        surfaceOffset: [0, 0, 0],
      },
      {
        zoneId: 'boardroom.monitor.center_right',
        binding: 'upper_monitor_3',
        fitSize: [1.62, 0.96, 0.16],
        surfaceOffset: [0, 0, 0],
      },
      {
        zoneId: 'boardroom.monitor.right',
        binding: 'upper_monitor_4',
        fitSize: [1.62, 0.96, 0.16],
        surfaceOffset: [0, 0, 0],
      },
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
})
