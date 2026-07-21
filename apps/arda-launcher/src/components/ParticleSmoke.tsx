import { useRef, useMemo } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'

const SMOKE_IMAGES = [
  '/smoke/smoke1.png',
  '/smoke/smoke2.png',
  '/smoke/smoke3.png',
  '/smoke/smoke4.png',
  '/smoke/smoke5.png',
]

const createSmokeTexture = () => {
  const size = 32
  const canvas = document.createElement('canvas')
  canvas.width = size
  canvas.height = size
  const ctx = canvas.getContext('2d')!
  const gradient = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2)
  gradient.addColorStop(0, 'rgba(241,235,221,1)')
  gradient.addColorStop(0.2, 'rgba(241,235,221,0.8)')
  gradient.addColorStop(0.5, 'rgba(241,235,221,0.4)')
  gradient.addColorStop(1, 'rgba(241,235,221,0)')
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, size, size)
  const texture = new THREE.CanvasTexture(canvas)
  texture.needsUpdate = true
  texture.minFilter = THREE.LinearFilter
  texture.magFilter = THREE.LinearFilter
  texture.colorSpace = THREE.SRGBColorSpace
  return texture
}

const variants = [
  { color: 0xf3efe6, opacity: 0.7 },
  { color: 0xe6e0d4, opacity: 0.65 },
  { color: 0xcfc8b8, opacity: 0.6 },
  { color: 0xffffff, opacity: 0.55 },
  { color: 0xaba393, opacity: 0.6 },
]

type SmokeEntry = {
  map: THREE.Texture
  material: THREE.MeshLambertMaterial
}

const buildTextures = (textures: THREE.Texture[]): SmokeEntry[] => {
  const base = createSmokeTexture()
  const out: SmokeEntry[] = []

  for (let i = 0; i < textures.length; i++) {
    const texture = textures[i]
    texture.minFilter = THREE.LinearFilter
    texture.magFilter = THREE.LinearFilter
    texture.colorSpace = THREE.SRGBColorSpace

    const material = new THREE.MeshLambertMaterial({
      color: variants[i % variants.length].color,
      emissive: 0xffffff,
      map: base,
      transparent: true,
      opacity: variants[i % variants.length].opacity,
      depthWrite: false,
      side: THREE.DoubleSide,
    })
    out.push({ map: texture, material })
  }

  return out
}

export default function ParticleSmoke({ active = true }: { active?: boolean }) {
  const groupRef = useRef<THREE.Group>(null!)
  const velocitiesRef = useRef<THREE.Vector3[]>([])
  const particlesRef = useRef<THREE.Mesh[]>([])

  const textureAssets = useMemo(() => {
    const loader = new THREE.TextureLoader()
    loader.crossOrigin = 'anonymous'
    const textures: THREE.Texture[] = []

    SMOKE_IMAGES.forEach((src) => {
      const map = loader.load(src)
      map.minFilter = THREE.LinearFilter
      map.magFilter = THREE.LinearFilter
      map.colorSpace = THREE.SRGBColorSpace
      textures.push(map)
    })

    return textures
  }, [])

  useFrame((state) => {
    if (!active || !groupRef.current) return

    const delta = Number(state.clock.getDelta())
    const elapsed = state.clock.elapsedTime
    const particles = particlesRef.current

    particles.forEach((particle, i) => {
      if (!particle.visible) return
      const material = particle.material as THREE.MeshLambertMaterial

      particle.rotation.z -= delta * 0.4
      const velocity = velocitiesRef.current[i] ?? new THREE.Vector3()
      particle.position.x += velocity.x + Math.sin(elapsed * 0.8 + i) * 0.05
      particle.position.y += velocity.y + Math.cos(elapsed * 1.1 + i) * 0.04
      particle.position.z += velocity.z + Math.sin(elapsed * 0.6 + i) * 0.03

      if (Number.isFinite(delta) && delta > 0) {
        material.opacity -= delta * 0.02
      }

      if (particle.position.y > 10 || particle.position.x > 10 || particle.position.x < -10) {
        particle.position.set(
          (Math.random() - 0.5) * 18,
          -10 + Math.random() * 4,
          (Math.random() - 0.5) * 12,
        )
        material.opacity = 0.65
      }
    })
  })

  useMemo(() => {
    const entries = buildTextures(textureAssets)
    const particles: THREE.Mesh[] = []
    velocitiesRef.current = []

    const bounds = { x: 18, y: 8, z: 12 }
    const numParticles = 24

    for (let p = 0; p < numParticles; p++) {
      const { material } = entries[p % entries.length]
      const mesh = new THREE.Mesh(new THREE.PlaneGeometry(10, 10), material)
      mesh.position.set(
        Math.random() * bounds.x - bounds.x * 0.5,
        Math.random() * bounds.y - bounds.y * 0.5,
        Math.random() * bounds.z - bounds.z * 0.3,
      )
      mesh.rotation.z = Math.random() * 360

      velocitiesRef.current.push(
        new THREE.Vector3(
          (Math.random() - 0.5) * 0.08,
          (Math.random() - 0.5) * 0.04 + 0.02,
          (Math.random() - 0.5) * 0.04,
        ),
      )

      particles.push(mesh)
    }

    particlesRef.current = particles

    if (groupRef.current) {
      while (groupRef.current.children.length) {
        groupRef.current.remove(groupRef.current.children[0])
      }
      particles.forEach((particle) => groupRef.current.add(particle))
    }
  }, [textureAssets])

  return <group ref={groupRef} />
}
