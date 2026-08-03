// sigil: REPAIR
import { useEffect, useMemo, useState } from 'react'
import * as THREE from 'three'
import {
  loadStatefulPersona,
  type StatefulPersona,
} from '../../lib/statefulPersona'
import { DEFAULT_AGENT_PRESENCE_STATE, presenceVisualState, presenceSupportMarkers } from '../systems/presenceState'
import { ParticleOrb } from './ParticleOrb'
import type { AgentPresenceState, PresenceSupportMarker } from '../systems/presenceTypes'

interface AvatarPresenceLayerProps {
  presenceState?: AgentPresenceState
  particleCount?: number
  particleRadius?: number
  motionEnabled?: boolean
  rootPath?: string | null
  persona?: StatefulPersona
}

/**
 * AvatarPresenceLayer orchestrates the visual presence of an agent avatar.
 *
 * It sits above AvatarEmitterBase (the physical hologram_anchor stage) and
 * composes:
 *  - ParticleOrb — phase-driven particle system (Milestone 1)
 *  - SupportAgentMarkers — orbiting support agent indicators
 *
 * The visual system reads presenceVisualState for pulse, opacity, and intensity
 * values. If no presenceState is provided, it falls back to the default idle
 * state (zero regression).
 */
export function AvatarPresenceLayer({
  presenceState = DEFAULT_AGENT_PRESENCE_STATE,
  particleCount = 600,
  particleRadius = 0.55,
  motionEnabled = true,
  rootPath = null,
  persona,
}: AvatarPresenceLayerProps) {
  const [loadedPersona, setLoadedPersona] = useState<StatefulPersona | undefined>(persona)
  useEffect(() => {
    if (persona) {
      setLoadedPersona(persona)
      return
    }
    if (!rootPath) {
      setLoadedPersona(undefined)
      return
    }

    let cancelled = false
    void loadStatefulPersona(rootPath, presenceState.primaryAgent).then((projection) => {
      if (!cancelled) setLoadedPersona(projection)
    })
    return () => {
      cancelled = true
    }
  }, [persona, presenceState.primaryAgent, rootPath])

  const activePersona = persona
    ?? (loadedPersona?.actor === presenceState.primaryAgent ? loadedPersona : undefined)
  const visualState = useMemo(
    () => presenceVisualState(presenceState, activePersona),
    [activePersona, presenceState],
  )
  const markers = useMemo(() => presenceSupportMarkers(presenceState), [presenceState])

  return (
    <group name="arda-avatar-presence-layer">
      <ParticleOrb
        presenceState={presenceState}
        count={particleCount}
        radius={particleRadius}
        motionEnabled={motionEnabled}
        persona={activePersona}
      />
      {markers.length > 0 ? (
        <group name="arda-support-markers" position={[0, 0.94, 0]}>
          {markers.map((marker) => (
            <SupportAgentMarker key={marker.agent} marker={marker} visualState={visualState} />
          ))}
        </group>
      ) : null}
    </group>
  )
}

interface SupportAgentMarkerProps {
  marker: PresenceSupportMarker
  visualState: ReturnType<typeof presenceVisualState>
}

function SupportAgentMarker({ marker, visualState }: SupportAgentMarkerProps) {
  const ringOpacity = marker.isFocus ? 0.7 : 0.42

  return (
    <group position={[Math.cos(marker.angleRadians) * marker.radius, 0, Math.sin(marker.angleRadians) * marker.radius]} scale={visualState.supportMarkerScale}>
      <pointLight color={marker.color} intensity={0.18} distance={1.25} />
      <mesh>
        <sphereGeometry args={[0.075, 18, 18]} />
        <meshStandardMaterial
          color={marker.color}
          emissive={marker.color}
          emissiveIntensity={marker.isFocus ? 2.8 : 2.05}
          transparent
          opacity={marker.isFocus ? 0.82 : 0.66}
          depthWrite={false}
          roughness={0.2}
          blending={THREE.AdditiveBlending}
        />
      </mesh>
      <mesh rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[0.12, 0.006, 8, 32]} />
        <meshBasicMaterial
          color={marker.color}
          transparent
          opacity={ringOpacity}
          depthWrite={false}
          side={THREE.DoubleSide}
          blending={THREE.AdditiveBlending}
        />
      </mesh>
      <sprite scale={[0.42, 0.16, 1]} position={[0, 0.22, 0]}>
        <spriteMaterial
          color="#ffffff"
          transparent
          opacity={0.76}
          depthWrite={false}
          blending={THREE.AdditiveBlending}
        />
      </sprite>
    </group>
  )
}