import { describe, expect, it } from 'vitest'
import { shouldDrawInstrumentFrame } from './instrumentFrameCadence'

describe('instrument frame cadence', () => {
  it('draws immediately and then at the configured cadence', () => {
    expect(shouldDrawInstrumentFrame(0, Number.NEGATIVE_INFINITY)).toBe(true)
    expect(shouldDrawInstrumentFrame(20, 0)).toBe(false)
    expect(shouldDrawInstrumentFrame(124, 0)).toBe(false)
    expect(shouldDrawInstrumentFrame(125, 0)).toBe(true)
  })
})
