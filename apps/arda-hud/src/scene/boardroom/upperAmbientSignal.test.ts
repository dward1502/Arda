import { describe, expect, it } from 'vitest'
import {
  isUpperMonitorInteractive,
  resolveUpperAmbientIdentity,
  resolveUpperMonitorDisplayMode,
  sampleUpperAmbientField,
  UPPER_AMBIENT_IDENTITIES,
} from './upperAmbientSignal'

describe('upper monitor ambient identities', () => {
  it('assigns one stable and distinct ambient identity to each canonical monitor', () => {
    const identities = ['monitor_1', 'monitor_2', 'monitor_3', 'monitor_4', 'monitor_5']
      .map(resolveUpperAmbientIdentity)

    expect(identities).toEqual([
      'aurora_veil',
      'constellation_mesh',
      'signal_mandala',
      'vector_rain',
      'dream_horizon',
    ])
    expect(new Set(identities).size).toBe(5)
    expect(resolveUpperAmbientIdentity('view_desk_l')).toBeNull()
    expect(new Set(Object.values(UPPER_AMBIENT_IDENTITIES).map((identity) => identity.accent)).size).toBe(5)
  })

  it('always gives a live session or agent claim priority over ambient pixels', () => {
    expect(resolveUpperMonitorDisplayMode(true, true)).toBe('session')
    expect(resolveUpperMonitorDisplayMode(false, true)).toBe('claim')
    expect(resolveUpperMonitorDisplayMode(false, false)).toBe('ambient')
    expect(isUpperMonitorInteractive('session')).toBe(true)
    expect(isUpperMonitorInteractive('claim')).toBe(true)
    expect(isUpperMonitorInteractive('ambient')).toBe(false)
  })

  it('produces deterministic, non-flat ambient motion fields', () => {
    const identity = UPPER_AMBIENT_IDENTITIES.aurora_veil
    const first = sampleUpperAmbientField(identity, 2.5, 24)
    const second = sampleUpperAmbientField(identity, 2.5, 24)

    expect(first).toEqual(second)
    expect(new Set(first).size).toBeGreaterThan(8)
    first.forEach((value) => {
      expect(value).toBeGreaterThanOrEqual(-1)
      expect(value).toBeLessThanOrEqual(1)
    })
  })
})
