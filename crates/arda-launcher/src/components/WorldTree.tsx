import { useRef } from 'react'
import * as THREE from 'three'

const GOLDEN_ANGLE = 137.508 * (Math.PI / 180)

export default function WorldTree({ phase }: { phase: number }) {
  const groupRef = useRef<THREE.Group>(null!)

  // Overall growth (0 to 1)
  const growth = Math.min(phase / 10.5, 1)

  return (
    <group ref={groupRef} position={[0, -4.2, 0]}>
      {/* Roots */}
      <group scale={[1, Math.min(growth * 1.4, 1), 1]}>
        <mesh position={[0, 0.2, 0]} rotation={[0.35, 0, 0]}>
          <cylinderGeometry args={[0.14, 0.7, 2.6, 7]} />
          <meshPhongMaterial color="#1c160f" shininess={2} />
        </mesh>
      </group>

      {/* Trunk - thicker and more solid */}
      <mesh 
        position={[0, 0.6 + 2.0 * Math.min(growth, 1), 0]} 
        scale={[1, Math.min(growth, 1), 1]}
      >
        <cylinderGeometry args={[0.18, 0.28, 4.8, 9]} />
        <meshPhongMaterial color="#b38c5e" shininess={4} />
      </mesh>

      {/* Main Branches - thicker with better structure */}
      {Array.from({ length: 7 }).map((_, i) => {
        const angle = i * GOLDEN_ANGLE
        const baseHeight = 1.3 + i * 0.32
        const branchStart = baseHeight / 9.2

        const branchProgress = Math.max(0, Math.min(1, (growth - branchStart) / 0.7))
        if (branchProgress <= 0) return null

        const len = (3.8 + (i % 3) * 0.35) * branchProgress

        // Main branch (thicker)
        const curve = new THREE.CatmullRomCurve3([
          new THREE.Vector3(0, baseHeight * 0.85, 0),
          new THREE.Vector3(Math.sin(angle) * len * 0.32, baseHeight + 0.4, Math.cos(angle) * len * 0.18),
          new THREE.Vector3(Math.sin(angle) * len, baseHeight + 0.25, Math.cos(angle) * len * 0.4),
        ])

        return (
          <group key={i}>
            {/* Main branch - thicker */}
            <mesh geometry={new THREE.TubeGeometry(curve, 16, 0.065, 5, false)}>
              <meshPhongMaterial color="#a67c52" shininess={3} />
            </mesh>

            {/* Secondary branches (adds structure, reduces string cheese look) */}
            {branchProgress > 0.6 && (
              <mesh geometry={new THREE.TubeGeometry(
                new THREE.CatmullRomCurve3([
                  new THREE.Vector3(Math.sin(angle) * len * 0.55, baseHeight + 0.35, Math.cos(angle) * len * 0.25),
                  new THREE.Vector3(Math.sin(angle) * len * 0.85, baseHeight + 0.9, Math.cos(angle) * len * 0.55),
                ]), 8, 0.028, 4, false
              )}>
                <meshPhongMaterial color="#8c6642" />
              </mesh>
            )}
          </group>
        )
      })}
    </group>
  )
}