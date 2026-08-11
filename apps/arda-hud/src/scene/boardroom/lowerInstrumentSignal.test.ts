import { describe, expect, it } from 'vitest'
import type { HudInstrumentModel } from './boardroomHudInstruments'
import {
  deriveLowerInstrumentSignal,
  resolveLowerInstrumentRole,
  sampleLowerInstrumentSequence,
} from './lowerInstrumentSignal'

const baseModel: HudInstrumentModel = {
  title: 'Surface',
  eyebrow: 'Instrument',
  tone: 'cyan',
  status: 'nominal',
  glyph: 'SIG',
  preset: 'pulse',
  nodes: [
    { id: 'one', x: 20, y: 30, state: 'good' },
    { id: 'two', x: 50, y: 50, state: 'dim' },
    { id: 'three', x: 80, y: 70, state: 'warn' },
  ],
  links: [[0, 1], [1, 2]],
  rings: [20, 40],
}

describe('lower instrument signal language', () => {
  it('maps each physical desk to one stable and unique role', () => {
    const roles = [
      resolveLowerInstrumentRole('boardroom.lower.left_wrap'),
      resolveLowerInstrumentRole('boardroom.lower.left_inner'),
      resolveLowerInstrumentRole('boardroom.lower.right_inner'),
      resolveLowerInstrumentRole('boardroom.lower.right_wrap'),
    ]

    expect(roles).toEqual(['governance', 'systems', 'routing', 'human'])
    expect(new Set(roles).size).toBe(4)
    expect(resolveLowerInstrumentRole('boardroom.monitor.left')).toBeNull()
  })

  it('gives every role a distinct palette and visual topology', () => {
    const roles = ['governance', 'systems', 'routing', 'human'] as const
    const signals = roles.map((role) => deriveLowerInstrumentSignal(role, baseModel))

    expect(new Set(signals.map((signal) => signal.topology)).size).toBe(4)
    expect(new Set(signals.map((signal) => signal.accent)).size).toBe(4)
    signals.forEach((signal) => {
      expect(signal.activity).toBeGreaterThanOrEqual(0)
      expect(signal.activity).toBeLessThanOrEqual(1)
      expect(signal.pressure).toBeGreaterThanOrEqual(0)
      expect(signal.pressure).toBeLessThanOrEqual(1)
      expect(signal.coherence).toBeGreaterThanOrEqual(0)
      expect(signal.coherence).toBeLessThanOrEqual(1)
    })
  })

  it('projects degraded state into pressure and deterministic motion samples', () => {
    const nominal = deriveLowerInstrumentSignal('routing', baseModel)
    const degraded = deriveLowerInstrumentSignal('routing', {
      ...baseModel,
      status: 'offline',
      nodes: baseModel.nodes.map((node) => ({ ...node, state: 'alert' as const })),
    })

    expect(degraded.pressure).toBeGreaterThan(nominal.pressure)
    expect(degraded.coherence).toBeLessThan(nominal.coherence)
    expect(sampleLowerInstrumentSequence(degraded, 1.25, 12)).toEqual(
      sampleLowerInstrumentSequence(degraded, 1.25, 12),
    )
    expect(new Set(sampleLowerInstrumentSequence(degraded, 1.25, 12)).size).toBeGreaterThan(4)
  })
})
