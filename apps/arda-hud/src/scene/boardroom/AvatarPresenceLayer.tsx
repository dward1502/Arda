// sigil: REPAIR
import { useFrame } from '@react-three/fiber'
import { useEffect, useMemo, useRef, useState } from 'react'
import * as THREE from 'three'
import {
  loadStatefulPersona,
  type StatefulPersona,
} from '../../lib/statefulPersona'
import { DEFAULT_AGENT_PRESENCE_STATE, presenceVisualState, presenceSupportMarkers } from '../systems/presenceState'
import { ParticleOrb } from './ParticleOrb'
import PresenceParticleSystem from './PresenceParticleSystem'
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
      <HolographicAvatarForm
        presenceState={presenceState}
        motionEnabled={motionEnabled}
        visualState={visualState}
      />
      <ParticleOrb
        presenceState={presenceState}
        count={particleCount}
        radius={particleRadius * 0.78}
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

function HolographicAvatarForm({
  presenceState,
  motionEnabled,
  visualState,
}: {
  presenceState: AgentPresenceState
  motionEnabled: boolean
  visualState: ReturnType<typeof presenceVisualState>
}) {
  const groupRef = useRef<THREE.Group>(null)
  const isActive = presenceState.phase !== 'idle'
  const isAlert = presenceState.scenario === 'alert' || presenceState.urgency === 'high'
  const color = isAlert ? '#ff4f9d' : '#75e9ff'
  const opacity = (isActive ? 0.56 : 0.32) * visualState.ringOpacity

  // Presence lifecycle: agent idle/present → figure assembles; agent active →
  // figure dissolves into the emitter mount. Mesh opacity follows the same
  // signal so wireframe and particle cloud never pop against each other.
  const assembleTarget = useRef(1)
  assembleTarget.current = isActive ? 0 : 1

  useFrame(({ clock }) => {
    if (!groupRef.current || !motionEnabled) return
    const elapsed = clock.getElapsedTime()
    groupRef.current.rotation.y = Math.sin(elapsed * 0.42) * 0.075
    groupRef.current.position.y = Math.sin(elapsed * 0.88) * 0.012
  })

  return (
    <group ref={groupRef} name="arda-holographic-avatar-form" scale={0.62}>
      {/* 0.62: figure ~1.05u so the head clears the desk but stays below the
          upper monitor rail (WS3b live-pass scale fix — a 1.7u figure read as
          a giant behind the consoles at the seated camera). */}
      <PresenceParticleSystem assemble={assembleTarget} color={color} opacity={opacity} />
      <pointLight
        position={[0, 1.05, 0]}
        intensity={isAlert ? 0.5 : 0.28}
        distance={1.8}
        color={color}
      />
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
      <mesh rotation={[Math.PI / 4, 0, 0]}>
        <octahedronGeometry args={[0.075, 0]} />
        <meshStandardMaterial
          color={marker.color}
          emissive={marker.color}
          emissiveIntensity={marker.isFocus ? 2.8 : 2.05}
          transparent
          opacity={marker.isFocus ? 0.82 : 0.66}
          depthWrite={false}
          roughness={0.2}
          flatShading
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
      <mesh position={[0, 0.2, 0]} rotation={[0, Math.PI / 4, 0]}>
        <planeGeometry args={[0.16, 0.05]} />
        <meshBasicMaterial
          color="#ffffff"
          transparent
          opacity={0.55}
          depthWrite={false}
          side={THREE.DoubleSide}
          blending={THREE.AdditiveBlending}
        />
      </mesh>
    </group>
  )
}