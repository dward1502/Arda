import { describe, expect, it } from 'vitest'
import { BOARDROOM_CONTROL_SLOT_IDS } from './boardroomSlotSettings'
import { FIRST_LEVEL_TERMINALS, resolveWorkstationProfile } from './firstLevelTerminalContracts'

describe('first-level terminal contracts', () => {
  it('formalizes four configurable desks plus one fixed command core', () => {
    expect(FIRST_LEVEL_TERMINALS).toHaveLength(5)
    expect(FIRST_LEVEL_TERMINALS.filter((terminal) => terminal.configurable)).toHaveLength(4)
    expect(FIRST_LEVEL_TERMINALS.find((terminal) => terminal.deskId === 'desk_3')).toMatchObject({
      zoneId: 'boardroom.control.center',
      slotId: null,
      role: 'command_core_now',
      configurable: false,
    })
    expect(FIRST_LEVEL_TERMINALS.flatMap((terminal) => terminal.slotId ? [terminal.slotId] : [])).toEqual([
      ...BOARDROOM_CONTROL_SLOT_IDS,
    ])
  })

  it('adapts source-map taxonomy into bounded HUD module IDs', () => {
    expect(resolveWorkstationProfile('governance_guardhouse', [
      'security_posture',
      'edge_guardhouse',
      'policy_authority',
    ])).toEqual({
      moduleIds: ['governance_controls'],
      rejectedPanelIds: [],
      adapted: true,
    })

    expect(resolveWorkstationProfile('human_business_personal', [
      'human_notes',
      'business_ops',
      'personal_growth',
    ]).moduleIds).toEqual(['human_realm', 'business', 'personal_growth'])
  })

  it('reports unknown source panel labels instead of silently creating an empty workstation', () => {
    expect(resolveWorkstationProfile('unregistered_domain', ['unknown_panel', 'systems'])).toEqual({
      moduleIds: ['systems'],
      rejectedPanelIds: ['unknown_panel'],
      adapted: false,
    })
  })
})
