// sigil: REPAIR
import { useFrame } from '@react-three/fiber'
import { useMemo, useRef } from 'react'
import * as THREE from 'three'
import { presenceVisualState } from '../systems/presenceState'
import type { AgentPresenceState } from '../systems/presenceTypes'
import type { StatefulPersona } from '../../lib/statefulPersona'
import {
  deriveParticlePresenceModel,
  shouldPauseParticleFrame,
  stepMaterialization,
} from './particlePresence'

interface ParticleOrbProps {
  presenceState: AgentPresenceState
  count?: number
  radius?: number
  motionEnabled?: boolean
  persona?: StatefulPersona
}

function particleColor(isAlert: boolean, colorTemperature: number): [number, number, number] {
  if (isAlert) return [1, 0.31, 0.62]
  const warm = Math.max(0, colorTemperature)
  const cool = Math.max(0, -colorTemperature)
  return [
    0.46 + warm * 0.46 - cool * 0.16,
    0.91 - warm * 0.13 - cool * 0.18,
    1 - warm * 0.34,
  ]
}

export function ParticleOrb({
  presenceState,
  count = 600,
  radius = 0.55,
  motionEnabled = true,
  persona,
}: ParticleOrbProps) {
  const visualState = useMemo(
    () => presenceVisualState(presenceState, persona),
    [persona, presenceState],
  )
  const particleModel = useMemo(
    () => deriveParticlePresenceModel(presenceState, visualState),
    [presenceState, visualState],
  )
  const isAlert = presenceState.scenario === 'alert'
    || presenceState.urgency === 'high'
    || presenceState.phase === 'alert'
  const renderCount = Math.max(
    1,
    Math.floor(count * (particleModel.activeFraction > 0 ? particleModel.activeFraction : 0.42)),
  )
  const baseOpacity = Math.min(0.92, 0.38 + visualState.ringOpacity * 0.42)
  const pointSize = 0.018 + visualState.scanlineOpacity * 0.07

  const particleData = useMemo(() => {
    const basePositions = new Float32Array(renderCount * 3)
    const positions = new Float32Array(renderCount * 3)
    const colors = new Float32Array(renderCount * 3)
    const baseColor = particleColor(isAlert, visualState.colorTemperature)
    const brightness = Math.min(
      1.22,
      0.72 + visualState.bodyEmissiveIntensity * 0.12 + visualState.traitAccent * 0.04,
    )

    for (let index = 0; index < renderCount; index += 1) {
      const theta = Math.random() * Math.PI * 2
      const phi = Math.acos(2 * Math.random() - 1)
      const particleRadius = radius * (0.62 + Math.random() * 0.76)
      const offset = index * 3
      basePositions[offset] = particleRadius * Math.sin(phi) * Math.cos(theta)
      basePositions[offset + 1] = particleRadius * Math.sin(phi) * Math.sin(theta)
      basePositions[offset + 2] = particleRadius * Math.cos(phi)
      positions[offset] = basePositions[offset]
      positions[offset + 1] = basePositions[offset + 1]
      positions[offset + 2] = basePositions[offset + 2]

      const variation = 0.82 + Math.random() * 0.18
      colors[offset] = Math.min(1, baseColor[0] * variation * brightness)
      colors[offset + 1] = Math.min(1, baseColor[1] * variation * brightness)
      colors[offset + 2] = Math.min(1, baseColor[2] * variation * brightness)
    }

    return { basePositions, colors, positions }
  }, [
    isAlert,
    radius,
    renderCount,
    visualState.bodyEmissiveIntensity,
    visualState.colorTemperature,
    visualState.traitAccent,
  ])

  const pointsRef = useRef<THREE.Points>(null)
  const materialRef = useRef<THREE.PointsMaterial>(null)
  const lightRef = useRef<THREE.PointLight>(null)
  const progressRef = useRef<number>(particleModel.targetProgress)

  useFrame(({ clock }, deltaSeconds) => {
    const points = pointsRef.current
    const material = materialRef.current
    if (!points || !material || !motionEnabled) return
    if (shouldPauseParticleFrame(presenceState, progressRef.current)) return

    const progress = stepMaterialization(
      progressRef.current,
      particleModel.targetProgress,
      deltaSeconds,
      particleModel.transitionRate,
    )
    progressRef.current = progress
    points.visible = progress > 0.001
    material.opacity = baseOpacity * progress * (1 - particleModel.dissolveBias * 0.28)
    if (lightRef.current) {
      lightRef.current.intensity = visualState.lightIntensity * progress * 0.38
    }

    if (progress <= 0) return

    const elapsed = clock.getElapsedTime()
    const pulse = 1 + Math.sin(elapsed * visualState.pulseRate) * 0.035
    points.scale.setScalar((0.68 + progress * 0.32) * pulse)
    points.rotation.y = elapsed * particleModel.rotationSpeed

    const positionAttribute = points.geometry.attributes.position
    const animatedPositions = positionAttribute.array as Float32Array
    for (let index = 0; index < renderCount; index += 1) {
      const offset = index * 3
      const phase = elapsed * visualState.pulseRate + index * 0.71
      animatedPositions[offset] = particleData.basePositions[offset]
        + Math.sin(phase) * particleModel.turbulence
      animatedPositions[offset + 1] = particleData.basePositions[offset + 1]
        + Math.cos(phase * 0.83) * particleModel.turbulence
      animatedPositions[offset + 2] = particleData.basePositions[offset + 2]
        + Math.sin(phase * 0.61) * particleModel.turbulence
    }
    positionAttribute.needsUpdate = true
  })

  const renderedProgress = motionEnabled ? progressRef.current : particleModel.targetProgress
  return (
    <group name="arda-particle-orb">
      <points
        ref={pointsRef}
        frustumCulled={false}
        visible
        scale={0.68 + renderedProgress * 0.32}
      >
        <bufferGeometry>
          <bufferAttribute attach="attributes-position" args={[particleData.positions, 3]} />
          <bufferAttribute attach="attributes-color" args={[particleData.colors, 3]} />
        </bufferGeometry>
        <pointsMaterial
          ref={materialRef}
          size={pointSize}
          sizeAttenuation
          transparent
          opacity={baseOpacity * renderedProgress}
          blending={THREE.AdditiveBlending}
          depthWrite={false}
          vertexColors
        />
      </points>
      <pointLight
        ref={lightRef}
        color={isAlert ? '#ff4f9d' : '#75e9ff'}
        intensity={visualState.lightIntensity * renderedProgress * 0.38}
        distance={radius * 4}
      />
    </group>
  )
}