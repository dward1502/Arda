import type { HudInstrumentModel } from './boardroomHudInstruments'

export type LowerInstrumentRole = 'governance' | 'systems' | 'routing' | 'human'
export type LowerInstrumentTopology = 'decision_lattice' | 'reactor_spine' | 'route_orbits' | 'living_contours'

export interface LowerInstrumentSignal {
  role: LowerInstrumentRole
  topology: LowerInstrumentTopology
  activity: number
  pressure: number
  coherence: number
  cadence: number
  seed: number
  accent: string
  secondary: string
  warning: string
}

const ROLE_VISUALS: Record<LowerInstrumentRole, Pick<LowerInstrumentSignal, 'topology' | 'accent' | 'secondary' | 'warning'>> = {
  governance: { topology: 'decision_lattice', accent: '#ffd36a', secondary: '#ff629c', warning: '#ff315f' },
  systems: { topology: 'reactor_spine', accent: '#4df4ff', secondary: '#a5ff68', warning: '#ffba5d' },
  routing: { topology: 'route_orbits', accent: '#65ffc7', secondary: '#6ba7ff', warning: '#ff5fa2' },
  human: { topology: 'living_contours', accent: '#ff91d5', secondary: '#ad8cff', warning: '#ffd66b' },
}

const SLOT_ROLES: Record<string, LowerInstrumentRole> = {
  'boardroom.lower.left_wrap': 'governance',
  'boardroom.lower.left_inner': 'systems',
  'boardroom.lower.right_inner': 'routing',
  'boardroom.lower.right_wrap': 'human',
}

const clamp01 = (value: number): number => Math.max(0, Math.min(1, value))

export function resolveLowerInstrumentRole(slotId: string): LowerInstrumentRole | null {
  return SLOT_ROLES[slotId] ?? null
}

export function deriveLowerInstrumentSignal(
  role: LowerInstrumentRole,
  model: HudInstrumentModel,
): LowerInstrumentSignal {
  const total = Math.max(1, model.nodes.length)
  const alertCount = model.nodes.filter((node) => node.state === 'alert').length
  const warnCount = model.nodes.filter((node) => node.state === 'warn').length
  const activeCount = model.nodes.filter((node) => node.state === 'good' || node.state === 'warn').length
  const statusPressure = model.status === 'offline' ? 0.78 : model.status === 'watch' ? 0.42 : 0.08
  const statusCoherence = model.status === 'offline' ? 0.2 : model.status === 'watch' ? 0.58 : 0.92
  const seed = [...`${role}:${model.glyph}:${model.title}`].reduce((sum, character) => sum + character.charCodeAt(0), 0)

  return {
    role,
    ...ROLE_VISUALS[role],
    activity: clamp01(0.18 + activeCount / total * 0.62 + warnCount / total * 0.16),
    pressure: clamp01(statusPressure + alertCount / total * 0.48 + warnCount / total * 0.18),
    coherence: clamp01(statusCoherence - alertCount / total * 0.44 - warnCount / total * 0.12),
    cadence: 0.42 + (seed % 29) / 38,
    seed,
  }
}

export function sampleLowerInstrumentSequence(
  signal: LowerInstrumentSignal,
  time: number,
  sampleCount: number,
): number[] {
  if (sampleCount <= 0) return []
  return Array.from({ length: sampleCount }, (_, index) => {
    const progress = sampleCount === 1 ? 0 : index / (sampleCount - 1)
    const primary = Math.sin(progress * Math.PI * (3.2 + signal.activity * 3.8) + time * signal.cadence * 2.2)
    const harmonic = Math.sin(progress * Math.PI * 13 + signal.seed * 0.021 - time * 0.74) * (0.13 + signal.pressure * 0.24)
    const interference = Math.sin((progress + 0.17) * (signal.seed % 17 + 9) + time * 1.7) * (1 - signal.coherence) * 0.34
    return Math.max(-1, Math.min(1, primary * (0.42 + signal.activity * 0.25) + harmonic + interference))
  })
}
