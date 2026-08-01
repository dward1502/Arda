import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { BOARDROOM_VISUAL_REGRESSION_SCENARIOS } from './boardroomVisualRegression'

describe('boardroom visual regression contract', () => {
  it('pins web full-fidelity and degraded-state captures at the target viewport', () => {
    expect(BOARDROOM_VISUAL_REGRESSION_SCENARIOS.map((scenario) => scenario.id)).toEqual([
      'web-full-fidelity',
      'web-degraded-state',
    ])
    for (const scenario of BOARDROOM_VISUAL_REGRESSION_SCENARIOS) {
      expect(scenario.viewport).toEqual({ width: 1278, height: 513 })
      expect(scenario.captureKind).toBe('webgl-canvas')
      expect(scenario.baselinePath).toMatch(/^__visual_baselines__\/.+\.png$/)
      expect(scenario.requiredLandmarks).toContain('five-upper-monitors')
      expect(scenario.requiredLandmarks).toContain('five-lower-console-zones')
      expect(scenario.requiredLandmarks).toContain('four-physical-controls')
    }
  })

  it('requires the fail-closed mesh state in the degraded WebGL capture', () => {
    const degraded = BOARDROOM_VISUAL_REGRESSION_SCENARIOS.find((scenario) => scenario.id === 'web-degraded-state')!
    expect(degraded.requiredLandmarks).toContain('fail-closed-control-state')
    expect(degraded.runtimeState).toBe('degraded')
  })

  it('ships valid, distinct PNG baselines at the contracted viewport', () => {
    const hashes = BOARDROOM_VISUAL_REGRESSION_SCENARIOS.map((scenario) => {
      const image = readFileSync(new URL(scenario.baselinePath, import.meta.url))
      expect(image.subarray(1, 4).toString('ascii')).toBe('PNG')
      expect({ width: image.readUInt32BE(16), height: image.readUInt32BE(20) }).toEqual(scenario.viewport)
      return createHash('sha256').update(image).digest('hex')
    })

    expect(new Set(hashes).size).toBe(hashes.length)
  })
})
