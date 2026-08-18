import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  ambientIdleFixture,
  continuityHandoffReadyFixture,
  systemDegradedFixture,
} from './fixtures'
import {
  deriveMirromereVisualModel,
  isMirromereInspectAllowed,
  resolveMirromereMotion,
  shouldRenderMirromereAperture,
} from './MirromereAperture'
import type { MirromereSurface } from './types'

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
    expect(resolveMirromereMotion(surface({}), true, false)).toBe(true)
    expect(resolveMirromereMotion(surface({}), true, true)).toBe(false)
    expect(resolveMirromereMotion(surface({}), false, false)).toBe(false)
    expect(resolveMirromereMotion(surface({
      accessibility: { ...ambientIdleFixture.accessibility, reduced_motion: 'simplify' },
    }), true, true)).toBe(false)
  })

  it('uses only monitor_3 ambient ownership and backend-allowlisted inspection', () => {
    const runtime = surface({})
    expect(shouldRenderMirromereAperture('monitor_3', 'ambient', runtime)).toBe(true)
    expect(shouldRenderMirromereAperture('monitor_3', 'claim', runtime)).toBe(false)
    expect(shouldRenderMirromereAperture('monitor_2', 'ambient', runtime)).toBe(false)
    expect(isMirromereInspectAllowed(runtime)).toBe(true)
    expect(isMirromereInspectAllowed(surface({ allowed_interactions: [] }))).toBe(false)
  })

  it('uses bounded frame cadence without polling the backend', () => {
    const source = readFileSync(resolve(process.cwd(), 'src/features/mirromere/MirromereAperture.tsx'), 'utf8')
    expect(source).toContain('shouldDrawInstrumentFrame')
    expect(source).not.toContain('setInterval(')
    expect(source).not.toContain('fetch(')
  })
})
