// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { BOARDROOM_MONITOR_ZONES, getBoardroomSpatialZone } from './boardroomSpatialLayout'
import {
  BOARDROOM_CAMERA_COMPOSITION,
  deriveAvatarEmitterGeometry,
} from './boardroomComposition'

describe('boardroom operator composition', () => {
  it('keeps the five-monitor arc while framing the complete outer desk displays', () => {
    expect(BOARDROOM_MONITOR_ZONES).toHaveLength(5)
    expect(BOARDROOM_CAMERA_COMPOSITION.position[2]).toBeLessThan(10)
    expect(BOARDROOM_CAMERA_COMPOSITION.position[1]).toBeGreaterThan(3.35)
    expect(BOARDROOM_CAMERA_COMPOSITION.fov).toBe(32)
    expect(BOARDROOM_CAMERA_COMPOSITION.target[1]).toBeGreaterThanOrEqual(2)
  })

  it('keeps the emitter a desk-scaled puck beneath the center monitor', () => {
    const emitter = getBoardroomSpatialZone('boardroom.avatar.emitter')!
    const centerMonitor = getBoardroomSpatialZone('boardroom.monitor.center')!
    const geometry = deriveAvatarEmitterGeometry(emitter.size)

    // Emitter is intentionally smaller than the desk consoles (visual pass
    // 2026-08-22): reads as a projector puck, not a stage.
    expect(emitter.size[0]).toBeGreaterThanOrEqual(0.8)
    expect(emitter.size[0]).toBeLessThanOrEqual(1.0)
    expect(geometry.ringRadius).toBeLessThan(emitter.size[0] / 2)
    expect(geometry.baseBottomRadius).toBeLessThan(geometry.ringRadius * 1.1)
    expect(emitter.position[0]).toBe(centerMonitor.position[0])
    expect(emitter.position[1] + emitter.size[1] / 2).toBeLessThan(centerMonitor.position[1] - centerMonitor.size[1] / 2)
  })

  it('keeps the emitter light local enough to illuminate the console without washing the room', () => {
    const emitter = getBoardroomSpatialZone('boardroom.avatar.emitter')!
    const geometry = deriveAvatarEmitterGeometry(emitter.size)

    expect(geometry.lightDistance).toBeGreaterThan(emitter.size[0] * 2)
    expect(geometry.lightDistance).toBeLessThan(emitter.size[0] * 4)
  })
})
