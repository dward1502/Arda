// sigil: REPAIR
import { useFrame } from '@react-three/fiber'
import { useGLTF, useTexture } from '@react-three/drei'
import { useMemo, useRef } from 'react'
import * as THREE from 'three'
import projectionMaskUrl from '../../assets/scene/hologram/presence_masks/presence_projection_mask.png?url'
import scanlineMaskUrl from '../../assets/scene/hologram/presence_masks/presence_scanline_mask.png?url'
import AgentPresenceOrbit from './AgentPresenceOrbit'
import { DEFAULT_AGENT_PRESENCE_STATE, presenceVisualState } from '../systems/presenceState'
import type { AgentPresenceState } from '../systems/presenceTypes'
import { getSceneAssetByBinding } from '../systems/sceneAssets'

interface PresenceAvatarProps {
  position?: [number, number, number]
  scale?: number
  presenceState?: AgentPresenceState
}

function PresenceFallback() {
  return (
    <group>
      <mesh position={[0, 1.54, 0]}>
        <sphereGeometry args={[0.18, 32, 32]} />
        <meshStandardMaterial color="#a9fbff" emissive="#5defff" emissiveIntensity={2.2} transparent opacity={0.86} />
      </mesh>
      <mesh position={[0, 0.95, 0]}>
        <capsuleGeometry args={[0.26, 0.7, 8, 24]} />
        <meshStandardMaterial color="#7df2ff" emissive="#38dfff" emissiveIntensity={1.9} transparent opacity={0.72} />
      </mesh>
    </group>
  )
}

function LoadedPresenceAvatar({
  url,
  position = [0, 0.18, -0.04],
  scale = 1.1,
  presenceState = DEFAULT_AGENT_PRESENCE_STATE,
}: PresenceAvatarProps & { url: string }) {
  const avatarRef = useRef<THREE.Group>(null)
  const scanRef = useRef<THREE.Group>(null)
  const gltf = useGLTF(url)
  const [projectionMask, scanlineMask] = useTexture([projectionMaskUrl, scanlineMaskUrl])
  const visualState = useMemo(() => presenceVisualState(presenceState), [presenceState])

  const bodyMaterial = useMemo(() => {
    const material = new THREE.MeshStandardMaterial({
      color: '#a9fbff',
      emissive: '#45eaff',
      emissiveIntensity: visualState.bodyEmissiveIntensity,
      transparent: true,
      opacity: 0.78,
      depthWrite: false,
      roughness: 0.12,
      metalness: 0,
      side: THREE.DoubleSide,
      blending: THREE.AdditiveBlending,
    })
    material.alphaMap = projectionMask
    material.needsUpdate = true
    return material
  }, [projectionMask, visualState.bodyEmissiveIntensity])

  const ringMaterial = useMemo(() => new THREE.MeshBasicMaterial({
    color: '#5df6ff',
    transparent: true,
    opacity: visualState.ringOpacity,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    side: THREE.DoubleSide,
  }), [visualState.ringOpacity])

  const baseMaterial = useMemo(() => new THREE.MeshStandardMaterial({
    color: '#071018',
    emissive: '#12344a',
    emissiveIntensity: 0.5,
    roughness: 0.38,
    metalness: 0.45,
  }), [])

  const scene = useMemo(() => {
    const clone = gltf.scene.clone(true) as THREE.Group
    clone.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return
      const name = object.name.toLowerCase()
      object.castShadow = false
      object.receiveShadow = false
      object.frustumCulled = false
      if (name.includes('head') || name.includes('torso') || name.includes('arm') || name.includes('leg')) {
        object.material = bodyMaterial
        return
      }
      if (name.includes('ring') || name.includes('orbit')) {
        object.material = ringMaterial
        return
      }
      object.material = baseMaterial
    })
    return clone
  }, [baseMaterial, bodyMaterial, gltf.scene, ringMaterial])

  const scanlineMaterial = useMemo(() => {
    scanlineMask.wrapS = THREE.RepeatWrapping
    scanlineMask.wrapT = THREE.RepeatWrapping
    scanlineMask.repeat.set(1, 2.8)
    return new THREE.MeshBasicMaterial({
      map: scanlineMask,
      color: '#d7ffff',
      transparent: true,
      opacity: visualState.scanlineOpacity,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
      side: THREE.DoubleSide,
    })
  }, [scanlineMask, visualState.scanlineOpacity])

  useFrame(({ clock }) => {
    const elapsed = clock.getElapsedTime()
    if (avatarRef.current) {
      avatarRef.current.position.y = Math.sin(elapsed * visualState.pulseRate) * 0.025
      avatarRef.current.rotation.y = Math.sin(elapsed * 0.28) * 0.08
    }
    if (scanRef.current) {
      scanRef.current.rotation.y = elapsed * 0.18
    }
  })

  return (
    <group position={position} scale={scale}>
      <pointLight position={[0, 1.05, 0]} intensity={visualState.lightIntensity} distance={3.2} color="#5defff" />
      <group ref={avatarRef}>
        <primitive object={scene} />
        <AgentPresenceOrbit presenceState={presenceState} />
        <group ref={scanRef} position={[0, 0.96, 0]}>
          <mesh material={scanlineMaterial}>
            <planeGeometry args={[0.95, 1.85]} />
          </mesh>
          <mesh rotation={[0, Math.PI / 2, 0]} material={scanlineMaterial}>
            <planeGeometry args={[0.95, 1.85]} />
          </mesh>
        </group>
      </group>
    </group>
  )
}

export default function PresenceAvatar(props: PresenceAvatarProps) {
  const asset = getSceneAssetByBinding('hologram_anchor')
  if (!asset?.glbUrl) {
    return (
      <group position={props.position ?? [0, 0.18, -0.04]} scale={props.scale ?? 1.1}>
        <PresenceFallback />
      </group>
    )
  }
  return <LoadedPresenceAvatar url={asset.glbUrl} {...props} />
}
