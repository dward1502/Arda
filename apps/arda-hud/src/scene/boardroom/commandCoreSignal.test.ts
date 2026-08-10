import { describe, expect, it } from 'vitest'
import type { HudInstrumentModel } from './boardroomHudInstruments'
import {
  deriveCommandCoreSignal,
  resolveCommandCoreFrameTime,
  sampleCommandCoreWave,
} from './commandCoreSignal'

function instrument(status: HudInstrumentModel['status'], states: HudInstrumentModel['nodes'][number]['state'][]): HudInstrumentModel {
  return {
    title: 'ignored',
    eyebrow: 'ignored',
    tone: 'violet',
    status,
    glyph: 'ignored',
    preset: 'pulse',
    nodes: states.map((state, index) => ({ id: `n-${index}`, x: index * 10, y: 50, state })),
    links: [],
    rings: [],
  }
}

describe('command core signal model', () => {
  it('maps runtime condition into bounded visual behavior rather than display text', () => {
    const nominal = deriveCommandCoreSignal(instrument('nominal', ['good', 'good', 'dim']))
    const attention = deriveCommandCoreSignal(instrument('watch', ['good', 'warn', 'alert']))
    const offline = deriveCommandCoreSignal(instrument('offline', ['dim', 'dim']))

    expect(nominal.coherence).toBeGreaterThan(attention.coherence)
    expect(attention.attention).toBeGreaterThan(nominal.attention)
    expect(offline.coherence).toBeLessThan(attention.coherence)
    for (const signal of [nominal, attention, offline]) {
      expect(signal.intensity).toBeGreaterThanOrEqual(0)
      expect(signal.intensity).toBeLessThanOrEqual(1)
      expect(signal.attention).toBeGreaterThanOrEqual(0)
      expect(signal.attention).toBeLessThanOrEqual(1)
      expect(signal.coherence).toBeGreaterThanOrEqual(0)
      expect(signal.coherence).toBeLessThanOrEqual(1)
    }
  })

  it('produces deterministic animated waveform samples with visible variation', () => {
    const signal = deriveCommandCoreSignal(instrument('watch', ['good', 'warn', 'alert']))
    const first = sampleCommandCoreWave(signal, 1.25, 48)
    const second = sampleCommandCoreWave(signal, 1.25, 48)

    expect(first).toEqual(second)
    expect(first).toHaveLength(48)
    expect(new Set(first.map((sample) => sample.toFixed(4))).size).toBeGreaterThan(20)
    expect(first.every((sample) => sample >= -1 && sample <= 1)).toBe(true)
  })

  it('freezes time for reduced motion without erasing the instrument', () => {
    expect(resolveCommandCoreFrameTime(12.5, false)).toBe(0.75)
    expect(resolveCommandCoreFrameTime(12.5, true)).toBe(12.5)
  })
})
