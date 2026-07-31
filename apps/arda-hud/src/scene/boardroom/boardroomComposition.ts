// sigil: REPAIR

export type BoardroomCompositionVec3 = [number, number, number]

export const BOARDROOM_CAMERA_COMPOSITION = {
  position: [0, 3.65, 8.8] as BoardroomCompositionVec3,
  target: [0, 2.02, 0.12] as BoardroomCompositionVec3,
  fov: 30,
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
    ringRadius: radius,
    ringTubeRadius: radius * 0.078,
    baseTopRadius: radius * 0.83,
    baseBottomRadius: radius * 0.97,
    coreTopRadius: radius * 0.41,
    coreBottomRadius: radius * 0.59,
    lightDistance: radius * 6.4,
  }
}
