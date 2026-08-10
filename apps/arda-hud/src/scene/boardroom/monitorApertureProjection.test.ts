import { describe, expect, it } from 'vitest'
import { resolveMonitorApertureGeometry } from './monitorApertureGeometry'
import type { BoardroomPreviewMode, BoardroomVec3 } from './boardroomSpatialLayout'

describe('monitor aperture projection', () => {
  const monitorSize = [1.63, 0.8, 0.08] as BoardroomVec3
  const deskSize = [1.58, 0.04, 0.72] as BoardroomVec3

  it('derives full-aperture geometry for upper monitors', () => {
    const geometry = resolveMonitorApertureGeometry('boardroom.monitor.center', 'monitor_surface', monitorSize)
    expect(geometry.width).toBeCloseTo(1.4996, 3)
    expect(geometry.height).toBeCloseTo(0.704, 2)
    expect(geometry.rotation).toEqual([0, 0, 0])
    expect(geometry.position).toEqual([0, 0, 0.19])
  })

  it('derives desk aperture geometry with rotated plane', () => {
    const geometry = resolveMonitorApertureGeometry('boardroom.lower.left_wrap', 'desk_surface', deskSize)
    expect(geometry.width).toBeCloseTo(1.4536, 3)
    expect(geometry.height).toBeCloseTo(0.6336, 3)
    expect(geometry.rotation).toEqual([-Math.PI / 2, 0, 0])
    expect(geometry.position).toEqual([0, 0.038, 0])
  })

  it('defaults unknown slots to desk surface geometry', () => {
    const geometry = resolveMonitorApertureGeometry('unknown.surface', 'desk_surface', deskSize)
    expect(geometry.width).toBeCloseTo(1.4536, 3)
    expect(geometry.height).toBeCloseTo(0.6336, 3)
  })
})
