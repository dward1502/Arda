// sigil: REPAIR
import { useFrame } from '@react-three/fiber'
import { useMemo, useRef } from 'react'
import * as THREE from 'three'
import { presenceSupportMarkers, presenceVisualState } from '../systems/presenceState'
import type { AgentPresenceState } from '../systems/presenceTypes'

interface AgentPresenceOrbitProps {
  presenceState: AgentPresenceState
}

function AgentMarkerLabel({ label, color }: { label: string; color: string }) {
  const canvasTexture = useMemo(() => {
    const canvas = document.createElement('canvas')
    canvas.width = 256
    canvas.height = 96
    const context = canvas.getContext('2d')
    if (context) {
      context.clearRect(0, 0, canvas.width, canvas.height)
      context.font = '700 38px Inter, system-ui, sans-serif'
      context.textAlign = 'center'
      context.textBaseline = 'middle'
      context.shadowColor = color
      context.shadowBlur = 18
      context.fillStyle = '#dffcff'
      context.fillText(label, canvas.width / 2, canvas.height / 2)
    }
    const texture = new THREE.CanvasTexture(canvas)
    texture.needsUpdate = true
    return texture
  }, [color, label])

  const material = useMemo(() => new THREE.SpriteMaterial({
    map: canvasTexture,
    color: '#ffffff',
    transparent: true,
    opacity: 0.76,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  }), [canvasTexture])

  return <sprite material={material} scale={[0.42, 0.16, 1]} position={[0, 0.22, 0]} />
}

export default function AgentPresenceOrbit({ presenceState }: AgentPresenceOrbitProps) {
  const orbitRef = useRef<THREE.Group>(null)
  const markers = useMemo(() => presenceSupportMarkers(presenceState), [presenceState])
  const visualState = useMemo(() => presenceVisualState(presenceState), [presenceState])
  const markerMaterials = useMemo(() => markers.map((marker) => ({
    agent: marker.agent,
    core: new THREE.MeshStandardMaterial({
      color: marker.color,
      emissive: marker.color,
      emissiveIntensity: marker.isFocus ? 2.8 : 2.05,
      transparent: true,
      opacity: marker.isFocus ? 0.82 : 0.66,
      depthWrite: false,
      roughness: 0.2,
      blending: THREE.AdditiveBlending,
    }),
    ring: new THREE.MeshBasicMaterial({
      color: marker.color,
      transparent: true,
      opacity: marker.isFocus ? 0.7 : 0.42,
      depthWrite: false,
      side: THREE.DoubleSide,
      blending: THREE.AdditiveBlending,
    }),
  })), [markers])

  useFrame(({ clock }) => {
    if (!orbitRef.current) return
    const elapsed = clock.getElapsedTime()
    orbitRef.current.rotation.y = elapsed * 0.12
    orbitRef.current.children.forEach((child, index) => {
      const marker = markers[index]
      if (!marker) return
      child.position.y = Math.sin(elapsed * visualState.pulseRate + marker.phaseOffset) * 0.035
    })
  })

  if (markers.length === 0) return null

  return (
    <group ref={orbitRef} position={[0, 0.94, 0]}>
      {markers.map((marker) => {
        const x = Math.cos(marker.angleRadians) * marker.radius
        const z = Math.sin(marker.angleRadians) * marker.radius
        const material = markerMaterials.find((candidate) => candidate.agent === marker.agent)
        return (
          <group key={marker.agent} position={[x, 0, z]} scale={visualState.supportMarkerScale}>
            <pointLight color={marker.color} intensity={0.18} distance={1.25} />
            <mesh material={material?.core}>
              <sphereGeometry args={[0.075, 18, 18]} />
            </mesh>
            <mesh rotation={[Math.PI / 2, 0, 0]} material={material?.ring}>
              <torusGeometry args={[0.12, 0.006, 8, 32]} />
            </mesh>
            <AgentMarkerLabel label={marker.label} color={marker.color} />
          </group>
        )
      })}
    </group>
  )
}
