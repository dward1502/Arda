export type UpperAmbientIdentityId =
  | 'aurora_veil'
  | 'constellation_mesh'
  | 'signal_mandala'
  | 'vector_rain'
  | 'dream_horizon'

export type UpperMonitorDisplayMode = 'session' | 'claim' | 'ambient'

export interface UpperAmbientIdentity {
  id: UpperAmbientIdentityId
  accent: string
  secondary: string
  cadence: number
  seed: number
}

export const UPPER_AMBIENT_IDENTITIES: Record<UpperAmbientIdentityId, UpperAmbientIdentity> = {
  aurora_veil: { id: 'aurora_veil', accent: '#ff72bd', secondary: '#5df4ff', cadence: 0.24, seed: 17 },
  constellation_mesh: { id: 'constellation_mesh', accent: '#f7c95f', secondary: '#bb8cff', cadence: 0.2, seed: 31 },
  signal_mandala: { id: 'signal_mandala', accent: '#6ef5ff', secondary: '#ff5fa8', cadence: 0.27, seed: 47 },
  vector_rain: { id: 'vector_rain', accent: '#80ffb5', secondary: '#59a9ff', cadence: 0.34, seed: 61 },
  dream_horizon: { id: 'dream_horizon', accent: '#be8cff', secondary: '#ff91cf', cadence: 0.18, seed: 79 },
}

const SLOT_IDENTITIES: Record<string, UpperAmbientIdentityId> = {
  monitor_1: 'aurora_veil',
  monitor_2: 'constellation_mesh',
  monitor_3: 'signal_mandala',
  monitor_4: 'vector_rain',
  monitor_5: 'dream_horizon',
}

export function resolveUpperAmbientIdentity(slotId: string): UpperAmbientIdentityId | null {
  return SLOT_IDENTITIES[slotId] ?? null
}

export function resolveUpperMonitorDisplayMode(
  hasSessionRecord: boolean,
  hasActiveClaim: boolean,
): UpperMonitorDisplayMode {
  if (hasSessionRecord) return 'session'
  if (hasActiveClaim) return 'claim'
  return 'ambient'
}

export function isUpperMonitorInteractive(mode: UpperMonitorDisplayMode): boolean {
  return mode !== 'ambient'
}

export function sampleUpperAmbientField(
  identity: UpperAmbientIdentity,
  time: number,
  sampleCount: number,
): number[] {
  if (sampleCount <= 0) return []
  return Array.from({ length: sampleCount }, (_, index) => {
    const progress = sampleCount === 1 ? 0 : index / (sampleCount - 1)
    const primary = Math.sin(progress * Math.PI * 3.6 + time * identity.cadence * 3 + identity.seed * 0.04) * 0.62
    const harmonic = Math.sin(progress * Math.PI * 11 + time * 0.37 + identity.seed) * 0.24
    const drift = Math.cos(progress * Math.PI * 1.8 - time * identity.cadence + identity.seed * 0.11) * 0.14
    return Math.max(-1, Math.min(1, primary + harmonic + drift))
  })
}
