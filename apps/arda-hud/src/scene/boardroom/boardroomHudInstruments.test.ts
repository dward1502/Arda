import { describe, expect, it } from 'vitest'
import { deriveFleetHudInstrument } from './boardroomHudInstruments'

describe('deriveFleetHudInstrument', () => {
  it('creates a nominal fleet instrument when targets are live', () => {
    const instrument = deriveFleetHudInstrument({
      liveTargets: 5,
      totalTargets: 5,
      routableProviders: 3,
      unexpectedOffline: 0,
      intentionalOffline: 0,
      runtimeDrift: { driftedNodes: 0, totalNodes: 5 },
    })

    expect(instrument.status).toBe('nominal')
    expect(instrument.tone).toBe('cyan')
    expect(instrument.glyph).toBe('5/5')
    expect(instrument.nodes.filter((node) => node.state === 'good')).toHaveLength(5)
  })

  it('marks unexpected offline targets as alert nodes', () => {
    const instrument = deriveFleetHudInstrument({
      liveTargets: 3,
      totalTargets: 6,
      routableProviders: 2,
      unexpectedOffline: 2,
      intentionalOffline: 1,
      runtimeDrift: { driftedNodes: 1, totalNodes: 6 },
    })

    expect(instrument.status).toBe('watch')
    expect(instrument.tone).toBe('gold')
    expect(instrument.nodes.filter((node) => node.state === 'alert')).toHaveLength(2)
    expect(instrument.nodes.filter((node) => node.state === 'warn')).toHaveLength(1)
  })
})
