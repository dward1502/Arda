// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { DEFAULT_AGENT_PRESENCE_STATE } from '../systems/presenceState'
import { presenceVisualState } from '../systems/presenceState'
import type { StatefulPersona } from '../../lib/statefulPersona'
import {
  deriveParticlePresenceModel,
  shouldPauseParticleFrame,
  stepMaterialization,
} from './particlePresence'

describe('particle presence animation model', () => {
  it('keeps idle fully dismissed and pauses only after dismissal completes', () => {
    const model = deriveParticlePresenceModel(DEFAULT_AGENT_PRESENCE_STATE)

    expect(model).toEqual({
      targetProgress: 0,
      activeFraction: 0,
      rotationSpeed: 0,
      turbulence: 0,
      transitionRate: 1.8,
      dissolveBias: 0,
    })
    expect(shouldPauseParticleFrame(DEFAULT_AGENT_PRESENCE_STATE, 0)).toBe(true)
    expect(shouldPauseParticleFrame(DEFAULT_AGENT_PRESENCE_STATE, 0.2)).toBe(false)
  })

  it('materializes on arrival and intensifies density and motion for alerts', () => {
    const arrival = deriveParticlePresenceModel({
      ...DEFAULT_AGENT_PRESENCE_STATE,
      scenario: 'briefing',
      phase: 'agent_arrival',
    })
    const alert = deriveParticlePresenceModel({
      ...DEFAULT_AGENT_PRESENCE_STATE,
      scenario: 'alert',
      phase: 'alert',
      urgency: 'high',
    })

    expect(arrival).toEqual({
      targetProgress: 1,
      activeFraction: 0.72,
      rotationSpeed: 0.3,
      turbulence: 0.018,
      transitionRate: 2.4,
      dissolveBias: 0,
    })
    expect(alert).toEqual({
      targetProgress: 1,
      activeFraction: 1,
      rotationSpeed: 0.62,
      turbulence: 0.045,
      transitionRate: 3.2,
      dissolveBias: 0,
    })
    expect(shouldPauseParticleFrame({
      ...DEFAULT_AGENT_PRESENCE_STATE,
      phase: 'agent_arrival',
    }, 1)).toBe(false)
  })

  it('dismisses resolved presence while action confirmation remains embodied', () => {
    expect(deriveParticlePresenceModel({
      ...DEFAULT_AGENT_PRESENCE_STATE,
      phase: 'resolved',
      scenario: 'recovery',
    }).targetProgress).toBe(0)
    expect(deriveParticlePresenceModel({
      ...DEFAULT_AGENT_PRESENCE_STATE,
      phase: 'action_confirmed',
      scenario: 'routing',
    }).targetProgress).toBe(1)
  })

  it('steps materialization deterministically without overshooting either target', () => {
    expect(stepMaterialization(0, 1, 0.1, 2.4)).toBeCloseTo(0.24)
    expect(stepMaterialization(0.95, 1, 0.1, 2.4)).toBe(1)
    expect(stepMaterialization(1, 0, 0.1, 1.8)).toBeCloseTo(0.82)
    expect(stepMaterialization(0.05, 0, 0.1, 1.8)).toBe(0)
  })

  it('applies persona valence to particle density, turbulence, and dissolve tendency', () => {
    const state = {
      ...DEFAULT_AGENT_PRESENCE_STATE,
      scenario: 'briefing' as const,
      phase: 'agent_arrival' as const,
    }
    const persona = (weightedValence: number): StatefulPersona => ({
      actor: 'arandur',
      status: 'ready',
      sourceRecordId: 'persona-arandur',
      traits: [],
      moodSummary: {
        asOf: '2026-08-03T00:00:00Z',
        weightedValence,
        sampleCount: 4,
        windowHours: 336,
      },
      message: 'ready',
    })
    const positive = deriveParticlePresenceModel(state, presenceVisualState(state, persona(0.8)))
    const negative = deriveParticlePresenceModel(state, presenceVisualState(state, persona(-0.8)))

    expect(positive.activeFraction).toBeGreaterThan(negative.activeFraction)
    expect(positive.turbulence).toBeLessThan(negative.turbulence)
    expect(positive.dissolveBias).toBe(0)
    expect(negative.dissolveBias).toBeGreaterThan(0)
  })
})
