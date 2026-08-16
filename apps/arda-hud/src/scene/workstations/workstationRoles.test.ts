// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  WORKSTATION_ROLE_IDS,
  WORKSTATION_ROLE_DEFINITIONS,
  getWorkstationRoleDefinition,
} from './workstationRoles'

describe('workstationRoles', () => {
  it('defines every V1 universal workstation role', () => {
    expect(WORKSTATION_ROLE_IDS).toEqual([
      'fleet',
      'routing',
      'work',
      'decisions',
      'knowledge',
      'evidence',
      'settings',
    ])
    expect(WORKSTATION_ROLE_DEFINITIONS.map((role) => role.id)).toEqual(WORKSTATION_ROLE_IDS)
  })

  it('keeps role ids stable and unique', () => {
    const ids = WORKSTATION_ROLE_DEFINITIONS.map((role) => role.id)

    expect(new Set(ids).size).toBe(ids.length)
    for (const id of ids) {
      expect(getWorkstationRoleDefinition(id)?.id).toBe(id)
    }
  })

  it('does not enable raw debug surfaces for normal operator roles by default', () => {
    for (const role of WORKSTATION_ROLE_DEFINITIONS.filter((definition) => definition.id !== 'settings')) {
      expect(role.debugRawAllowed).toBe(false)
      expect(role.defaultPresentationModes).toContain('in_scene')
    }
  })

  it('carries defining role metadata for implementation and UX authoring', () => {
    for (const role of WORKSTATION_ROLE_DEFINITIONS) {
      expect(role.description).toBeTruthy()
      expect(role.purpose).toBeTruthy()
      expect(role.operatorQuestion).toBeTruthy()
      expect(role.previewKinds.length).toBeGreaterThan(0)
      expect(role.focusedCapabilities.length).toBeGreaterThan(0)
      expect(role.previewKinds.every((kind) => typeof kind === 'string' && kind.length > 0)).toBe(true)
      expect(role.focusedCapabilities.every((capability) => typeof capability === 'string' && capability.length > 0)).toBe(true)
    }
  })
})
