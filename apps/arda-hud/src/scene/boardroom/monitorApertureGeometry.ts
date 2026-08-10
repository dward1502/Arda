// sigil: REPAIR
import type { BoardroomPreviewMode, BoardroomVec3 } from './boardroomSpatialLayout'

export interface MonitorApertureGeometry {
  width: number
  height: number
  position: BoardroomVec3
  rotation: BoardroomVec3
}

export function resolveMonitorApertureGeometry(
  slotId: string,
  previewMode: BoardroomPreviewMode,
  size: BoardroomVec3,
): MonitorApertureGeometry {
  if (previewMode === 'monitor_surface') {
    return {
      width: size[0] * 0.92,
      height: size[1] * 0.88,
      position: [0, 0, size[2] / 2 + 0.15],
      rotation: [0, 0, 0],
    }
  }

  return {
    width: size[0] * 0.92,
    height: size[2] * 0.88,
    position: [0, 0.038, 0],
    rotation: [-Math.PI / 2, 0, 0],
  }
}
