export type BoardroomRenderProfileId = 'full' | 'low-power' | 'reduced-motion' | 'inactive'

export interface BoardroomRenderCapabilities {
  active: boolean
  prefersReducedMotion: boolean
  hardwareConcurrency?: number
  deviceMemoryGb?: number
}

export interface BoardroomRenderProfile {
  id: BoardroomRenderProfileId
  dpr: [number, number]
  frameloop: 'always' | 'demand' | 'never'
  shadows: boolean
  environmentEnabled: boolean
  motionEnabled: boolean
  postProcessingEnabled: false
}

const STATIC_PROFILE = {
  dpr: [1, 1.25] as [number, number],
  frameloop: 'demand' as const,
  shadows: false,
  environmentEnabled: false,
  motionEnabled: false,
  postProcessingEnabled: false as const,
}

export function resolveBoardroomRenderProfile(
  capabilities: BoardroomRenderCapabilities,
): BoardroomRenderProfile {
  if (!capabilities.active) {
    return { ...STATIC_PROFILE, id: 'inactive', frameloop: 'never' }
  }
  if (capabilities.prefersReducedMotion) {
    return { ...STATIC_PROFILE, id: 'reduced-motion' }
  }
  if (
    (capabilities.hardwareConcurrency !== undefined && capabilities.hardwareConcurrency <= 4)
    || (capabilities.deviceMemoryGb !== undefined && capabilities.deviceMemoryGb <= 4)
  ) {
    return { ...STATIC_PROFILE, id: 'low-power' }
  }
  return {
    id: 'full',
    dpr: [1, 1.5],
    frameloop: 'always',
    shadows: true,
    environmentEnabled: true,
    motionEnabled: true,
    postProcessingEnabled: false,
  }
}
