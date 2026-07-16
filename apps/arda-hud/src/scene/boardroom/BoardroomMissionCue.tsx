// sigil: REPAIR
import { Html } from '@react-three/drei'
import type { AgentPresenceState } from '../systems/presenceTypes'

export interface BoardroomMissionCueDefinition {
  anchorId: string
  label: string
  detail: string
  position: [number, number, number]
  color: string
  eventId?: string
}

interface BoardroomMissionCueProps {
  presenceState: AgentPresenceState
}

const BOARDROOM_MISSION_CUE_ANCHORS: Record<string, [number, number, number]> = {
  view_desk_l: [-2.6, 0.84, 1.16],
  view_desk_control_panel: [-0.9, 0.82, 1.34],
  view_desk_r: [0.9, 0.82, 1.34],
  view_desk_aux: [2.6, 0.84, 1.16],
}

function normalizeBoardroomAnchor(anchorTarget: string | undefined): string | null {
  if (!anchorTarget?.startsWith('boardroom.')) return null
  const anchorId = anchorTarget.slice('boardroom.'.length)
  return Object.prototype.hasOwnProperty.call(BOARDROOM_MISSION_CUE_ANCHORS, anchorId) ? anchorId : null
}

function titleAgent(agent: string): string {
  return agent.toUpperCase()
}

export function resolveBoardroomMissionCue(state: AgentPresenceState): BoardroomMissionCueDefinition | null {
  if (!state.missionId) return null
  const anchorId = normalizeBoardroomAnchor(state.anchorTarget)
  if (!anchorId) return null

  return {
    anchorId,
    label: 'Mission cue',
    detail: `${titleAgent(state.primaryAgent)} · ${state.missionId}`,
    position: BOARDROOM_MISSION_CUE_ANCHORS[anchorId],
    color: state.urgency === 'high' ? '#ff8a6b' : '#b98cff',
    eventId: state.eventId,
  }
}

export default function BoardroomMissionCue({ presenceState }: BoardroomMissionCueProps) {
  const cue = resolveBoardroomMissionCue(presenceState)
  if (!cue) return null

  return (
    <group position={cue.position} userData={{ sceneMissionCueAnchorId: cue.anchorId, sceneMissionCueEventId: cue.eventId }}>
      <mesh>
        <ringGeometry args={[0.2, 0.3, 48]} />
        <meshStandardMaterial
          color={cue.color}
          emissive={cue.color}
          emissiveIntensity={0.9}
          transparent
          opacity={0.7}
        />
      </mesh>
      <mesh position={[0, 0.02, 0]}>
        <sphereGeometry args={[0.055, 24, 16]} />
        <meshStandardMaterial color={cue.color} emissive={cue.color} emissiveIntensity={1.3} />
      </mesh>
      <Html position={[0, 0.28, 0]} center distanceFactor={7.6}>
        <div className="boardroom-mission-cue" title={cue.eventId ?? cue.detail}>
          <span className="boardroom-mission-cue__label">{cue.label}</span>
          <span className="boardroom-mission-cue__detail">{cue.detail}</span>
        </div>
      </Html>
    </group>
  )
}
