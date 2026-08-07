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

  useFrame(({ clock }) => {
    if (!groupRef.current || !motionEnabled) return
    const elapsed = clock.getElapsedTime()
    groupRef.current.rotation.y = Math.sin(elapsed * 0.42) * 0.075
    groupRef.current.position.y = Math.sin(elapsed * 0.88) * 0.012
  })

  return (
    <group ref={groupRef} name="arda-holographic-avatar-form" scale={0.95}>
      <mesh position={[0, 0.66, 0]} renderOrder={20}>
        <cylinderGeometry args={[0.29, 0.18, 0.56, 8, 1, true]} />
        <meshBasicMaterial color={color} transparent opacity={opacity * 0.18} depthTest={false} depthWrite={false} side={THREE.DoubleSide} blending={THREE.AdditiveBlending} />
      </mesh>
      <mesh position={[0, 0.66, 0]} renderOrder={20}>
        <cylinderGeometry args={[0.29, 0.18, 0.56, 8, 1, true]} />
        <meshBasicMaterial color={color} transparent opacity={opacity} wireframe depthTest={false} depthWrite={false} blending={THREE.AdditiveBlending} />
      </mesh>
      <mesh position={[0, 1.025, 0]} scale={[0.82, 1, 0.78]} renderOrder={20}>
        <icosahedronGeometry args={[0.165, 1]} />
        <meshBasicMaterial color={color} transparent opacity={opacity * 0.2} depthTest={false} depthWrite={false} blending={THREE.AdditiveBlending} />
      </mesh>
      <mesh position={[0, 1.025, 0]} scale={[0.82, 1, 0.78]} renderOrder={20}>
        <icosahedronGeometry args={[0.165, 1]} />
        <meshBasicMaterial color={color} transparent opacity={opacity + 0.12} wireframe depthTest={false} depthWrite={false} blending={THREE.AdditiveBlending} />
      </mesh>
      <mesh position={[0, 1.025, 0.122]} renderOrder={21}>
        <boxGeometry args={[0.14, 0.018, 0.01]} />
        <meshBasicMaterial color={isAlert ? '#fff0f7' : '#d9fbff'} transparent opacity={0.8} depthTest={false} depthWrite={false} toneMapped={false} />
      </mesh>
      <mesh position={[0, 0.9, 0]} renderOrder={20}>
        <cylinderGeometry args={[0.075, 0.095, 0.14, 8]} />
        <meshBasicMaterial color={color} transparent opacity={opacity * 0.46} wireframe depthTest={false} depthWrite={false} blending={THREE.AdditiveBlending} />
      </mesh>
      <mesh position={[0, 0.62, 0]} renderOrder={21}>
        <cylinderGeometry args={[0.008, 0.014, 0.68, 8]} />
        <meshBasicMaterial color="#f38cff" transparent opacity={isActive ? 0.52 : 0.2} depthTest={false} depthWrite={false} toneMapped={false} />
      </mesh>
      <mesh position={[0, 0.91, 0]} rotation={[Math.PI / 2, 0, 0]} renderOrder={21}>
        <torusGeometry args={[0.285, 0.008, 8, 48]} />
        <meshBasicMaterial color={color} transparent opacity={opacity * 0.72} depthTest={false} depthWrite={false} blending={THREE.AdditiveBlending} />
      </mesh>
      {[0.38, 0.62, 0.86].map((height, index) => (
        <mesh key={height} position={[0, height, 0]} rotation={[Math.PI / 2, 0, 0]} renderOrder={21}>
          <torusGeometry args={[0.25 - index * 0.035, 0.006, 8, 40]} />
          <meshBasicMaterial
            color={index === 1 ? '#f38cff' : color}
            transparent
            opacity={opacity * (0.92 - index * 0.14)}
            depthTest={false}
            depthWrite={false}
            blending={THREE.AdditiveBlending}
          />
        </mesh>
      ))}
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