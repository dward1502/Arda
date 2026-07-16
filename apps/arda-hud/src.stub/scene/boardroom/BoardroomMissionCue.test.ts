// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { resolveBoardroomMissionCue } from './BoardroomMissionCue'
import type { AgentPresenceState } from '../systems/presenceTypes'

const BASE_STATE: AgentPresenceState = {
  scenario: 'routing',
  phase: 'agent_arrival',
  primaryAgent: 'athena',
  supportAgents: ['hermes'],
  focus: ['athena', 'hermes'],
  urgency: 'normal',
  source: 'arda',
  timestamp: '2026-05-18T12:00:00.000Z',
}

describe('boardroom mission cue binding', () => {
  it('anchors a mission cue to the referenced desk control surface instead of floating at the hologram', () => {
    expect(resolveBoardroomMissionCue({
      ...BASE_STATE,
      eventId: 'presence-mission-cue',
      missionId: 'mission-arandur-presence-scene',
      anchorTarget: 'boardroom.view_desk_control_panel',
      banner: 'ATHENA coordinating with focused attention at boardroom.view_desk_control_panel',
    })).toEqual({
      anchorId: 'view_desk_control_panel',
      label: 'Mission cue',
      detail: 'ATHENA · mission-arandur-presence-scene',
      position: [-0.9, 0.82, 1.34],
      color: '#b98cff',
      eventId: 'presence-mission-cue',
    })
  })

  it('stays silent when no mission or explicit boardroom anchor is available', () => {
    expect(resolveBoardroomMissionCue(BASE_STATE)).toBeNull()
    expect(resolveBoardroomMissionCue({
      ...BASE_STATE,
      missionId: 'mission-without-slot',
      anchorTarget: 'boardroom.unknown_slot',
    })).toBeNull()
  })
})
