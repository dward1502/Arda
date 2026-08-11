import type { HudInstrumentModel } from './boardroomHudInstruments'

export interface CommandCoreSignal {
  intensity: number
  attention: number
  coherence: number
  cadence: number
  seed: number
  accent: string
  secondary: string
  warning: string
}

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value))
}

export function deriveCommandCoreSignal(model: HudInstrumentModel): CommandCoreSignal {
  const total = Math.max(1, model.nodes.length)
  const alerts = model.nodes.filter((node) => node.state === 'alert').length
  const warnings = model.nodes.filter((node) => node.state === 'warn').length
  const active = model.nodes.filter((node) => node.state !== 'dim').length
  const attention = clamp01((alerts * 1.8 + warnings * 0.85) / total)
  const statusCoherence = model.status === 'nominal'
    ? 0.94
    : model.status === 'watch'
      ? 0.68
      : model.status === 'external'
        ? 0.76
        : 0.28
  const coherence = clamp01(statusCoherence - alerts * 0.055)
  const intensity = clamp01(0.22 + (active / total) * 0.42 + attention * 0.36)
  const cadence = 0.42 + intensity * 0.78 + attention * 0.62
  const seed = model.nodes.reduce((value, node, index) => value + node.x * (index + 3) + node.y * (index + 7), model.rings.length * 31)

  return {
    intensity,
    attention,
    coherence,
    cadence,
    seed,
    accent: model.status === 'offline' ? '#ff4d8f' : '#68f7ff',
    secondary: model.status === 'watch' ? '#ffcf66' : '#a77cff',
    warning: '#ff4d8f',
  }
}

export function sampleCommandCoreWave(
  signal: CommandCoreSignal,
  time: number,
  sampleCount: number,
): number[] {
  const count = Math.max(2, Math.floor(sampleCount))
  const phase = time * signal.cadence * Math.PI * 2
  return Array.from({ length: count }, (_, index) => {
    const position = index / (count - 1)
    const carrier = Math.sin(position * Math.PI * (4.5 + signal.intensity * 3.5) + phase)
    const harmonic = Math.sin(position * Math.PI * 13 - phase * 0.57 + signal.seed * 0.003) * 0.34
    const interference = Math.sin(position * Math.PI * 31 + phase * 1.7 + signal.seed) * (1 - signal.coherence) * 0.3
    const envelope = 0.42 + Math.sin(position * Math.PI) * 0.58
    return Math.min(1, Math.max(-1, (carrier * 0.62 + harmonic + interference) * envelope))
  })
}

export function resolveCommandCoreFrameTime(elapsedSeconds: number, motionEnabled: boolean): number {
  return motionEnabled ? elapsedSeconds : 0.75
}
