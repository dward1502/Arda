import { useFrame } from '@react-three/fiber'
import { useRef, useMemo } from 'react'
import * as THREE from 'three'

export default function ParticleSmoke({ active = true }: { active?: boolean }) {
  const pointsRef = useRef<THREE.Points>(null!)
  const count = 860 // much denser

  const { positions, velocities } = useMemo(() => {
    const pos = new Float32Array(count * 3)
    const vel = new Float32Array(count * 3)

    for (let i = 0; i < count; i++) {
     // Keep smoke low and wide at the bottom
      pos[i * 3] = (Math.random() - 0.5) * 16
      pos[i * 3 + 1] = -2.8 + Math.random() * -4    // Lower starting position
      pos[i * 3 + 2] = (Math.random() - 0.5) * 5

      vel[i * 3] = (Math.random() - 0.5) * 0.01
      vel[i * 3 + 1] = 0.006 + Math.random() * 0.002   // Slower rise
      vel[i * 3 + 2] = (Math.random() - 0.5) * 0.008
   }
    return { positions: pos, velocities: vel }
  }, [])

  useFrame((state) => {
    if (!active || !pointsRef.current) return

    const posAttr = pointsRef.current.geometry.attributes.position as THREE.BufferAttribute
    const posArray = posAttr.array as Float32Array

    for (let i = 0; i < count; i++) {
      const idx = i * 3

      // Apply velocity
      posArray[idx] += velocities[idx] + Math.sin(state.clock.elapsedTime * 0.8 + i) * 0.0015
      posArray[idx + 1] += velocities[idx + 1]
      posArray[idx + 2] += velocities[idx + 2]

      // Slow fade + respawn at bottom when dead
      if (posArray[idx + 1] > 3.5) {
        posArray[idx] = (Math.random() - 0.5) * 9
        posArray[idx + 1] = -1.3 + Math.random() * 0.4
        posArray[idx + 2] = (Math.random() - 0.5) * 4
      }
    }

    posAttr.needsUpdate = true
  })

  return (
    <points ref={pointsRef}>
      <bufferGeometry>
        <bufferAttribute attach="attributes-position" args={[positions, 3]} />
      </bufferGeometry>
      <pointsMaterial
        size={0.18}
        color="#a8a090"
        transparent
        opacity={0.55}
        depthWrite={false}
        blending={THREE.AdditiveBlending}
        sizeAttenuation
      />
    </points>
  )
}