import { describe, expect, it } from 'vitest'
import { resolveBoardroomRenderProfile } from './boardroomPerformance'

describe('boardroom performance profile', () => {
  it('keeps bounded full-fidelity defaults on capable hardware', () => {
    expect(resolveBoardroomRenderProfile({
      active: true,
      prefersReducedMotion: false,
      hardwareConcurrency: 16,
      deviceMemoryGb: 16,
    })).toEqual({
      id: 'full',
      dpr: [1, 1.5],
      frameloop: 'always',
      shadows: true,
      environmentEnabled: true,
      motionEnabled: true,
      postProcessingEnabled: false,
    })
  })

  it('uses a static low-power profile on constrained hardware', () => {
    const profile = resolveBoardroomRenderProfile({
      active: true,
      prefersReducedMotion: false,
      hardwareConcurrency: 4,
      deviceMemoryGb: 4,
    })

    expect(profile.id).toBe('low-power')
    expect(profile.dpr[1]).toBeLessThanOrEqual(1.25)
    expect(profile.frameloop).toBe('demand')
    expect(profile.shadows).toBe(false)
    expect(profile.environmentEnabled).toBe(false)
    expect(profile.motionEnabled).toBe(false)
  })

  it('preserves motion while bounding expensive native WebKit effects', () => {
    expect(resolveBoardroomRenderProfile({
      active: true,
      prefersReducedMotion: false,
      hardwareConcurrency: 16,
      deviceMemoryGb: 16,
      nativeWebKit: true,
    })).toEqual({
      id: 'native',
      dpr: [1, 1.25],
      frameloop: 'always',
      shadows: false,
      environmentEnabled: false,
      motionEnabled: true,
      postProcessingEnabled: false,
    })
  })

  it('prevents continuous rendering in the explicit software compatibility path', () => {
    expect(resolveBoardroomRenderProfile({
      active: true,
      prefersReducedMotion: false,
      hardwareConcurrency: 16,
      deviceMemoryGb: 16,
      nativeWebKit: true,
      softwareRenderer: true,
    })).toMatchObject({
      id: 'compatibility',
      dpr: [1, 1],
      frameloop: 'demand',
      motionEnabled: false,
      shadows: false,
      environmentEnabled: false,
    })
  })

  it('honors reduced motion independently of hardware capacity', () => {
    expect(resolveBoardroomRenderProfile({
      active: true,
      prefersReducedMotion: true,
      hardwareConcurrency: 16,
      deviceMemoryGb: 16,
    })).toMatchObject({
      id: 'reduced-motion',
      frameloop: 'demand',
      motionEnabled: false,
    })
  })

  it('stops rendering when the scene is inactive', () => {
    expect(resolveBoardroomRenderProfile({
      active: false,
      prefersReducedMotion: false,
      hardwareConcurrency: 16,
      deviceMemoryGb: 16,
    })).toMatchObject({ id: 'inactive', frameloop: 'never', motionEnabled: false })
  })
})
