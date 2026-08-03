// sigil: REPAIR
import type { AgentPresenceState, PresenceVisualState } from '../systems/presenceTypes'

export interface ParticlePresenceModel {
  targetProgress: 0 | 1
  activeFraction: number
  rotationSpeed: number
  turbulence: number
  transitionRate: number
  dissolveBias: number
}

export function deriveParticlePresenceModel(
  state: AgentPresenceState,
  visualState?: PresenceVisualState,
): ParticlePresenceModel {
  const density = visualState?.particleDensity ?? 1
  const noise = visualState?.noiseMultiplier ?? 1
  const dissolveBias = visualState?.dissolveBias ?? 0
  if (state.phase === 'idle') {
    return {
      targetProgress: 0,
      activeFraction: 0,
      rotationSpeed: 0,
      turbulence: 0,
      transitionRate: 1.8,
      dissolveBias: 0,
    }
  }

  if (state.phase === 'resolved') {
    return {
      targetProgress: 0,
      activeFraction: 0.42,
      rotationSpeed: 0.16,
      turbulence: 0.01,
      transitionRate: 1.8,
      dissolveBias,
    }
  }

  const isAlert = state.urgency === 'high'
    || state.scenario === 'alert'
    || state.phase === 'alert'
    || state.phase === 'awaiting_user'

  if (isAlert) {
    return {
      targetProgress: 1,
      activeFraction: Math.min(1, density),
      rotationSpeed: 0.62,
      turbulence: 0.045 * noise,
      transitionRate: 3.2,
      dissolveBias,
    }
  }

  return {
    targetProgress: 1,
    activeFraction: Math.min(1, (state.phase === 'action_confirmed' ? 0.84 : 0.72) * density),
    rotationSpeed: state.phase === 'action_confirmed' ? 0.22 : 0.3,
    turbulence: (state.phase === 'action_confirmed' ? 0.012 : 0.018) * noise,
    transitionRate: 2.4,
    dissolveBias,
  }
}

export function stepMaterialization(
  current: number,
  target: 0 | 1,
  deltaSeconds: number,
  rate: number,
): number {
  const step = Math.max(0, deltaSeconds) * Math.max(0, rate)
  if (target > current) return Math.min(target, current + step)
  if (target < current) return Math.max(target, current - step)
  return target
}

export function shouldPauseParticleFrame(state: AgentPresenceState, progress: number): boolean {
  return (state.phase === 'idle' || state.phase === 'resolved') && progress <= 0
}
