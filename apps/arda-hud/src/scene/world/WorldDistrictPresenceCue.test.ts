// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { resolveWorldDistrictPresenceCue } from './WorldDistrictPresenceCue'
import type { AgentPresenceState } from '../systems/presenceTypes'
import type { SceneZoneDefinition } from '../systems/runtimeTypes'

const BASE_STATE: AgentPresenceState = {
  scenario: 'routing',
  phase: 'agent_arrival',
  primaryAgent: 'athena',
  supportAgents: ['hermes'],
  focus: ['athena', 'hermes'],
  urgency: 'normal',
  source: 'arda',
  timestamp: '2026-05-21T00:00:00.000Z',
}

const ZONES: SceneZoneDefinition[] = [
  {
    id: 'district_command',
    title: 'Command District',
    scene: 'world',
    owner: 'prometheus',
    status: 'ready',
    anchorIds: ['district_command_anchor'],
    surfaceIds: [],
    workstationIds: [],
    sourceIds: [],
  },
  {
    id: 'district_knowledge',
    title: 'Knowledge District',
    scene: 'world',
    owner: 'athena',
    status: 'active',
    anchorIds: ['district_knowledge_anchor'],
    surfaceIds: [],
    workstationIds: [],
    sourceIds: [],
  },
  {
    id: 'boardroom_only',
    title: 'Boardroom Only',
    scene: 'boardroom',
    owner: 'operator',
    status: 'hidden',
    anchorIds: ['district_boardroom_anchor'],
    surfaceIds: [],
    workstationIds: [],
    sourceIds: [],
  },
]

describe('world district presence cue binding', () => {
  it('anchors a mission cue to the targeted world district surface', () => {
    expect(resolveWorldDistrictPresenceCue({
      ...BASE_STATE,
      eventId: 'world-district-cue',
      missionId: 'mission-arandur-world-presence',
      anchorTarget: 'world.district_knowledge',
    }, ZONES)).toEqual({
      districtId: 'district_knowledge',
      label: 'District presence',
      detail: 'ATHENA · mission-arandur-world-presence',
      position: [-6.8, 2.95, -4.8],
      color: '#b98cff',
      eventId: 'world-district-cue',
    })
  })

  it('resolves world targets through district anchor ids', () => {
    expect(resolveWorldDistrictPresenceCue({
      ...BASE_STATE,
      missionId: 'mission-anchor-target',
      anchorTarget: 'world.district_command_anchor',
    }, ZONES)?.districtId).toBe('district_command')
  })

  it('stays silent when no mission or matching world district target is available', () => {
    expect(resolveWorldDistrictPresenceCue(BASE_STATE, ZONES)).toBeNull()
    expect(resolveWorldDistrictPresenceCue({
      ...BASE_STATE,
      missionId: 'mission-boardroom-target',
      anchorTarget: 'boardroom.view_desk_control_panel',
    }, ZONES)).toBeNull()
    expect(resolveWorldDistrictPresenceCue({
      ...BASE_STATE,
      missionId: 'mission-missing-world-target',
      anchorTarget: 'world.unknown_district',
    }, ZONES)).toBeNull()
    expect(resolveWorldDistrictPresenceCue({
      ...BASE_STATE,
      missionId: 'mission-non-world-target',
      anchorTarget: 'world.district_boardroom_anchor',
    }, ZONES)).toBeNull()
  })
})
