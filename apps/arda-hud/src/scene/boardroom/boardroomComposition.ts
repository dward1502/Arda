// sigil: REPAIR

export type BoardroomCompositionVec3 = [number, number, number]

export const BOARDROOM_CAMERA_COMPOSITION = {
  position: [0, 3.65, 8.8] as BoardroomCompositionVec3,
  target: [0, 2.02, 0.12] as BoardroomCompositionVec3,
  fov: 32,
}

export interface AvatarEmitterGeometry {
  ringRadius: number
  ringTubeRadius: number
  baseTopRadius: number
  baseBottomRadius: number
  coreTopRadius: number
  coreBottomRadius: number
  lightDistance: number
}

export function deriveAvatarEmitterGeometry(size: BoardroomCompositionVec3): AvatarEmitterGeometry {
  const radius = size[0] / 2
  return {
    ringRadius: radius * 0.68,
    ringTubeRadius: radius * 0.026,
    baseTopRadius: radius * 0.58,
    baseBottomRadius: radius * 0.72,
    coreTopRadius: radius * 0.24,
    coreBottomRadius: radius * 0.4,
    lightDistance: radius * 4.8,
  }
}
