// sigil: REPAIR
import { Html } from '@react-three/drei'
import type { AgentPresenceState, SovereignAgentId } from '../systems/presenceTypes'
import type { SceneZoneDefinition } from '../systems/runtimeTypes'

export interface WorldDistrictPresenceCueDefinition {
  districtId: string
  label: string
  detail: string
  position: [number, number, number]
  color: string
  eventId?: string
}

interface WorldDistrictPresenceCueProps {
  presenceState: AgentPresenceState
  zones: SceneZoneDefinition[]
}

const AGENT_DISTRICT_COLORS: Record<SovereignAgentId, string> = {
  arandur: '#dffcff',
  athena: '#b98cff',
  hermes: '#6ea8ff',
  charon: '#42f5d7',
  warden: '#ffb14a',
  oracle: '#ffd76a',
  plutus: '#68f58f',
  apollo: '#ff8ed6',
  mnemosyne: '#9cc7ff',
  prometheus: '#ff6e6e',
  hades: '#8f9bb3',
}

function titleAgent(agent: string): string {
  return agent.toUpperCase()
}

function normalizeWorldTarget(anchorTarget: string | undefined): string | null {
  if (!anchorTarget?.startsWith('world.')) return null
  const targetId = anchorTarget.slice('world.'.length).trim()
  return targetId.length > 0 ? targetId : null
}

function resolveTargetDistrict(targetId: string, zones: SceneZoneDefinition[]): { zone: SceneZoneDefinition; index: number } | null {
  const districts = zones.filter((zone) => zone.scene === 'world')
  const index = districts.findIndex((zone) => zone.id === targetId || zone.anchorIds.includes(targetId))
  if (index < 0) return null
  const zone = districts[index]
  return zone ? { zone, index } : null
}

function districtCuePosition(index: number): [number, number, number] {
  const columnHeight = 2.8 + (index % 4) * 1.35
  return [-10 + index * 3.2, columnHeight - 1.2, -2 - (index % 3) * 2.8]
}

export function resolveWorldDistrictPresenceCue(
  state: AgentPresenceState,
  zones: SceneZoneDefinition[],
): WorldDistrictPresenceCueDefinition | null {
  if (!state.missionId) return null
  const targetId = normalizeWorldTarget(state.anchorTarget)
  if (!targetId) return null
  const district = resolveTargetDistrict(targetId, zones)
  if (!district) return null

  return {
    districtId: district.zone.id,
    label: 'District presence',
    detail: `${titleAgent(state.primaryAgent)} · ${state.missionId}`,
    position: districtCuePosition(district.index),
    color: AGENT_DISTRICT_COLORS[state.primaryAgent],
    eventId: state.eventId,
  }
}

export default function WorldDistrictPresenceCue({ presenceState, zones }: WorldDistrictPresenceCueProps) {
  const cue = resolveWorldDistrictPresenceCue(presenceState, zones)
  if (!cue) return null

  return (
    <group
      position={cue.position}
      userData={{ sceneDistrictPresenceCueId: cue.districtId, sceneDistrictPresenceCueEventId: cue.eventId }}
    >
      <mesh rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.44, 0.58, 64]} />
        <meshStandardMaterial
          color={cue.color}
          emissive={cue.color}
          emissiveIntensity={1.1}
          transparent
          opacity={0.72}
        />
      </mesh>
      <mesh position={[0, 0.015, 0]}>
        <cylinderGeometry args={[0.34, 0.34, 0.03, 48]} />
        <meshStandardMaterial
          color={cue.color}
          emissive={cue.color}
          emissiveIntensity={0.72}
          transparent
          opacity={0.34}
        />
      </mesh>
      <pointLight position={[0, 0.28, 0]} color={cue.color} intensity={0.9} distance={3.2} />
      <Html position={[0, 0.72, 0]} center distanceFactor={9.5}>
        <div className="world-district-presence-cue" title={cue.eventId ?? cue.detail}>
          <span className="world-district-presence-cue__label">{cue.label}</span>
          <span className="world-district-presence-cue__detail">{cue.detail}</span>
        </div>
      </Html>
    </group>
  )
}
