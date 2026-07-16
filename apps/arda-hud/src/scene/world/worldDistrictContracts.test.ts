// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  WORLD_DISTRICT_CONTRACTS,
  assertWorldDistrictContractsComplete,
  getWorldDistrictContract,
} from './worldDistrictContracts'

describe('world district contracts', () => {
  it('defines every milestone 3 operating district with owner, sources, triggers, and targets', () => {
    expect(WORLD_DISTRICT_CONTRACTS.length).toBeGreaterThanOrEqual(8)
    expect(assertWorldDistrictContractsComplete()).toEqual([])
  })

  it('keeps mutating world actions draft-only or gated rather than executable', () => {
    for (const contract of WORLD_DISTRICT_CONTRACTS) {
      for (const action of contract.gatedActions) {
        expect(['gated', 'draft_only']).toContain(action.guard)
      }
      expect(contract.safeActions.every((action) => action.guard === 'safe')).toBe(true)
    }
  })

  it('resolves contracts by district id or scene zone id', () => {
    expect(getWorldDistrictContract('district_communications')?.ownerAgent).toContain('CHARON')
    expect(getWorldDistrictContract('systems_health')?.districtId).toBe('district_monitoring')
  })
})
