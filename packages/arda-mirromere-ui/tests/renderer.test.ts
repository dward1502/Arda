import { describe, expect, it } from 'vitest'
import {
  ambientIdleFixture,
  continuityHandoffReadyFixture,
  systemDegradedFixture,
} from './fixtures'
import {
  deriveMirromereVisualModel,
  isMirromereInspectAllowed,
  resolveMirromereMotion,
} from '../src/renderer'
import type { MirromereSurface } from '../src/contract'

function surface(overrides: Partial<MirromereSurface>): MirromereSurface {
  return { ...ambientIdleFixture, source_mode: 'runtime', ...overrides }
}

describe('MirromereAperture visual contract', () => {
  it('maps idle, degraded, handoff, veil, and stale truth without card labels', () => {
    expect(deriveMirromereVisualModel(surface({}))).toMatchObject({ mode: 'wave', tone: 'cyan', glyph: '∿' })
    expect(deriveMirromereVisualModel(surface({
      ...systemDegradedFixture,
      source_mode: 'runtime',
    }))).toMatchObject({ mode: 'radar', tone: 'amber', truthState: 'stale' })
    expect(deriveMirromereVisualModel(surface({
      ...continuityHandoffReadyFixture,
      source_mode: 'runtime',
    }))).toMatchObject({ mode: 'handoff', tone: 'violet', glyph: '⇄' })
    expect(deriveMirromereVisualModel(surface({
      scene: { ...ambientIdleFixture.scene, scene_id: 'privacy.veil' },
    }))).toMatchObject({ mode: 'veil', glyph: '◫' })
    expect(deriveMirromereVisualModel(surface({ freshness: 'stale' })).truthState).toBe('stale')
  })

  it('honors contract and operator reduced-motion controls', () => {
    expect(resolveMirromereMotion(surface({
      accessibility: { ...ambientIdleFixture.accessibility, reduced_motion: 'none' },
    }), true, false)).toBe(true)
    expect(resolveMirromereMotion(surface({}), true, true)).toBe(false)
    expect(resolveMirromereMotion(surface({}), false, false)).toBe(false)
    expect(resolveMirromereMotion(surface({
      accessibility: { ...ambientIdleFixture.accessibility, reduced_motion: 'simplify' },
    }), true, true)).toBe(false)
  })


})