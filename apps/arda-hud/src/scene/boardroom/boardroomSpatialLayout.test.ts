// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import * as boardroomSpatialLayout from './boardroomSpatialLayout'
import {
  BOARDROOM_CONTROL_ZONES,
  BOARDROOM_MONITOR_ZONES,
  BOARDROOM_SPATIAL_ZONES,
  getBoardroomSpatialZone,
  normalizeBoardroomZonePositionOverrides,
  serializeBoardroomZonePositionOverrides,
} from './boardroomSpatialLayout'

describe('boardroom spatial layout contract', () => {
  it('keeps every boardroom zone addressable with stable dimensions', () => {
    const ids = new Set<string>()

    for (const zone of BOARDROOM_SPATIAL_ZONES) {
      expect(ids.has(zone.id)).toBe(false)
      ids.add(zone.id)
      expect(zone.position).toHaveLength(3)
      expect(zone.rotation).toHaveLength(3)
      expect(zone.size).toHaveLength(3)
      expect(zone.size.every((axis) => axis > 0)).toBe(true)
      expect(zone.color).toMatch(/^#[0-9a-f]{6}$/i)
    }
  })

  it('defines configurable monitor and desk slots separately from ARDA data', () => {
    expect(BOARDROOM_MONITOR_ZONES).toHaveLength(5)
    expect(BOARDROOM_MONITOR_ZONES.filter((zone) => zone.assignmentSlotId).map((zone) => zone.assignmentSlotId)).toEqual([
      'monitor_left_1',
      'monitor_left_2',
      'monitor_left_3',
      'monitor_left_4',
    ])
    expect(BOARDROOM_CONTROL_ZONES.map((zone) => zone.assignmentSlotId)).toEqual([
      'view_desk_l',
      'view_desk_control_panel',
      'view_desk_r',
      'view_desk_aux',
    ])
  })

  it('exposes the primitive anchors the boardroom art pass can snap onto', () => {
    expect(getBoardroomSpatialZone('boardroom.monitor.left')?.binding).toBe('upper_monitor_1')
    expect(getBoardroomSpatialZone('boardroom.control.center')?.interaction).toBe('open_workstation')
    expect(getBoardroomSpatialZone('boardroom.button.settings')?.interaction).toBe('open_settings')
    expect(getBoardroomSpatialZone('boardroom.avatar.emitter')?.binding).toBe('hologram_anchor')
    expect(getBoardroomSpatialZone('boardroom.world.window')?.interaction).toBe('transition_world')
  })

  it('aligns the live surfaces to the five-monitor arc authored in the physical-stage GLB', () => {
    expect(BOARDROOM_MONITOR_ZONES.map((zone) => zone.position[0])).toEqual([
      -3.8, -1.9, 0, 1.9, 3.8,
    ])
    expect(BOARDROOM_MONITOR_ZONES.map((zone) => zone.rotation[1])).toEqual([
      0.415, 0.218, 0, -0.218, -0.415,
    ])
    expect(BOARDROOM_MONITOR_ZONES.every((zone) => zone.rotation[0] === 0)).toBe(true)
    for (let index = 1; index < BOARDROOM_MONITOR_ZONES.length; index += 1) {
      expect(BOARDROOM_MONITOR_ZONES[index].position[0] - BOARDROOM_MONITOR_ZONES[index - 1].position[0]).toBeLessThan(2)
    }
  })

  it('embeds the physical controls as a compact non-overlapping desk panel', () => {
    const panel = boardroomSpatialLayout.BOARDROOM_KINETIC_CONTROL_PANEL
    const buttons = [
      getBoardroomSpatialZone('boardroom.button.service_health')!,
      getBoardroomSpatialZone('boardroom.button.settings')!,
      getBoardroomSpatialZone('boardroom.button.hermes_cli')!,
      getBoardroomSpatialZone('boardroom.button.hermes')!,
    ]

    expect(buttons.every((button) => button.kind === 'physical_button')).toBe(true)
    for (let index = 1; index < buttons.length; index += 1) {
      const previousRightEdge = buttons[index - 1].position[0] + buttons[index - 1].size[0] / 2
      const currentLeftEdge = buttons[index].position[0] - buttons[index].size[0] / 2
      expect(currentLeftEdge).toBeGreaterThan(previousRightEdge)
    }
    expect(panel.size[0]).toBeLessThan(1.8)
    expect(buttons.every((button) => button.size[0] <= 0.3 && button.size[2] <= 0.3)).toBe(true)
    expect(buttons.every((button) => Math.abs(button.position[0] - panel.position[0]) < panel.size[0] / 2)).toBe(true)
    expect(buttons.every((button) => Math.abs(button.position[2] - panel.position[2]) < panel.size[2] / 2)).toBe(true)
  })

  it('wraps the lower desk surfaces around the operator position', () => {
    expect(getBoardroomSpatialZone('boardroom.lower.left_wrap')?.rotation[1]).toBeGreaterThan(0.25)
    expect(getBoardroomSpatialZone('boardroom.lower.left_inner')?.rotation[1]).toBeGreaterThan(0.1)
    expect(getBoardroomSpatialZone('boardroom.lower.right_inner')?.rotation[1]).toBeLessThan(-0.1)
    expect(getBoardroomSpatialZone('boardroom.lower.right_wrap')?.rotation[1]).toBeLessThan(-0.25)
    expect(getBoardroomSpatialZone('boardroom.lower.left_wrap')?.rotation[2]).toBeLessThan(0)
    expect(getBoardroomSpatialZone('boardroom.lower.left_inner')?.rotation[2]).toBeLessThan(0)
    expect(getBoardroomSpatialZone('boardroom.lower.right_inner')?.rotation[2]).toBeGreaterThan(0)
    expect(getBoardroomSpatialZone('boardroom.lower.right_wrap')?.rotation[2]).toBeGreaterThan(0)
  })

  it('gives the five lower surfaces a deliberate tactical-to-command hierarchy', () => {
    const leftOuter = getBoardroomSpatialZone('boardroom.lower.left_wrap')!
    const leftInner = getBoardroomSpatialZone('boardroom.lower.left_inner')!
    const center = getBoardroomSpatialZone('boardroom.control.center')!
    const rightInner = getBoardroomSpatialZone('boardroom.lower.right_inner')!
    const rightOuter = getBoardroomSpatialZone('boardroom.lower.right_wrap')!

    expect(leftOuter.size).toEqual(rightOuter.size)
    expect(leftInner.size).toEqual(rightInner.size)
    expect(leftOuter.size[0]).toBeGreaterThan(leftInner.size[0])
    expect(center.size[0]).toBeGreaterThan(leftInner.size[0])
    expect(center.primary).toBe(true)
    expect(Math.abs(leftOuter.rotation[1])).toBeGreaterThan(Math.abs(leftInner.rotation[1]))
  })

  it('tilts every lower display surface toward the operator while preserving mirrored wrap', () => {
    const [leftWrap, leftInner, rightInner, rightWrap] = BOARDROOM_CONTROL_ZONES.filter(
      (zone) => zone.kind === 'desk_surface',
    )

    expect([leftWrap, leftInner, rightInner, rightWrap].every((zone) => zone.rotation[0] > 0)).toBe(true)
    expect(leftWrap.rotation[0]).toBeGreaterThan(leftInner.rotation[0])
    expect(rightWrap.rotation[0]).toBeGreaterThan(rightInner.rotation[0])
    expect(leftWrap.rotation[0]).toBeCloseTo(rightWrap.rotation[0], 6)
    expect(leftWrap.rotation[1]).toBeCloseTo(-rightWrap.rotation[1], 3)
    expect(leftInner.rotation[1]).toBeCloseTo(-rightInner.rotation[1], 3)
  })

  it('defines a mirrored five-segment command-console shell that wraps around the operator', () => {
    const segments = (boardroomSpatialLayout as typeof boardroomSpatialLayout & {
      BOARDROOM_CONSOLE_SHELL_SEGMENTS?: Array<{
        id: string
        role: string
        position: [number, number, number]
        rotation: [number, number, number]
        size: [number, number, number]
      }>
    }).BOARDROOM_CONSOLE_SHELL_SEGMENTS ?? []

    expect(segments.map((segment) => segment.role)).toEqual([
      'outer_left',
      'inner_left',
      'center',
      'inner_right',
      'outer_right',
    ])
    expect(segments.map((segment) => segment.id)).toEqual([
      'boardroom.console.outer_left',
      'boardroom.console.inner_left',
      'boardroom.console.center',
      'boardroom.console.inner_right',
      'boardroom.console.outer_right',
    ])

    const [outerLeft, innerLeft, center, innerRight, outerRight] = segments
    expect(outerLeft.position[0]).toBe(-outerRight.position[0])
    expect(innerLeft.position[0]).toBe(-innerRight.position[0])
    expect(outerLeft.rotation[1]).toBe(-outerRight.rotation[1])
    expect(innerLeft.rotation[1]).toBe(-innerRight.rotation[1])
    expect(Math.abs(outerLeft.rotation[1])).toBeGreaterThan(Math.abs(innerLeft.rotation[1]))
    expect(center.position[0]).toBe(0)
    expect(segments.every((segment) => segment.size.every((axis) => axis > 0))).toBe(true)
  })

  it('normalizes edit-mode position overrides to known zones and finite coordinates', () => {
    expect(normalizeBoardroomZonePositionOverrides({
      'boardroom.monitor.left': [1.23456, 2, -3],
      'unknown.zone': [9, 9, 9],
      'boardroom.monitor.right': [1, Number.NaN, 3],
      'boardroom.control.center': [1, 2],
    })).toEqual({
      'boardroom.monitor.left': [1.235, 2, -3],
    })
  })

  it('serializes edit-mode position overrides as a copyable layout promotion snippet', () => {
    const serialized = serializeBoardroomZonePositionOverrides({
      'boardroom.monitor.left': [1.2, 2.3456, -3],
      'not.a.zone': [4, 5, 6],
    })

    expect(serialized).toContain('acceptedBoardroomZonePositions')
    expect(serialized).toContain('"boardroom.monitor.left": [1.200, 2.346, -3.000]')
    expect(serialized).not.toContain('not.a.zone')
  })
})
