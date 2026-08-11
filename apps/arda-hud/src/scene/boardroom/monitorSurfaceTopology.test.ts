// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  UPPER_MONITOR_TOPOLOGY_SLOTS,
  assertUniqueTopologySlots,
  getMonitorSurfaceTopologySlot,
  getMonitorSurfaceTopologySlotByZoneId,
  isUpperMonitorSlotId,
  type MonitorSurfaceTopologySlot,
} from './monitorSurfaceTopology'

describe('monitor surface topology', () => {
  it('pins five unique canonical slots in physical left-to-right order', () => {
    expect(UPPER_MONITOR_TOPOLOGY_SLOTS.map((slot) => slot.slotId)).toEqual([
      'monitor_1',
      'monitor_2',
      'monitor_3',
      'monitor_4',
      'monitor_5',
    ])
  })

  it('maps every physical upper monitor zone to exactly one canonical slot', () => {
    expect(UPPER_MONITOR_TOPOLOGY_SLOTS.map((slot) => slot.zoneId)).toEqual([
      'boardroom.monitor.left',
      'boardroom.monitor.center_left',
      'boardroom.monitor.center',
      'boardroom.monitor.center_right',
      'boardroom.monitor.right',
    ])
  })

  it('resolves slots and zones bidirectionally', () => {
    for (const slot of UPPER_MONITOR_TOPOLOGY_SLOTS) {
      expect(getMonitorSurfaceTopologySlot(slot.slotId)?.zoneId).toBe(slot.zoneId)
      expect(getMonitorSurfaceTopologySlotByZoneId(slot.zoneId)?.slotId).toBe(slot.slotId)
    }
  })

  it('preserves authored aperture metadata from the spatial layout', () => {
    const left = getMonitorSurfaceTopologySlot('monitor_1')!
    expect(left.binding).toBe('upper_monitor_1')
    expect(left.position[0]).toBeLessThan(getMonitorSurfaceTopologySlot('monitor_2')!.position[0])
    expect(getMonitorSurfaceTopologySlot('monitor_3')!.position[0]).toBe(0)
    expect(UPPER_MONITOR_TOPOLOGY_SLOTS.every((slot) => slot.size.every((axis) => axis > 0))).toBe(true)
  })

  it('rejects duplicate slot or zone ids at module load time', () => {
    expect(assertUniqueTopologySlots()).toBe(true)
  })

  it('rejects unknown slot ids', () => {
    expect(isUpperMonitorSlotId('monitor_1')).toBe(true)
    expect(isUpperMonitorSlotId('monitor_left_1')).toBe(false)
    expect(isUpperMonitorSlotId('desk_left')).toBe(false)
    expect(isUpperMonitorSlotId('')).toBe(false)
  })
})
