// sigil: REPAIR
import { useFrame } from '@react-three/fiber'
import { useGLTF } from '@react-three/drei'
import { useEffect, useMemo, useRef } from 'react'
import * as THREE from 'three'
import { MeshSurfaceSampler } from 'three/examples/jsm/math/MeshSurfaceSampler.js'
import { getSceneAssetByBinding } from '../systems/sceneAssets'

interface PresenceParticleSystemProps {
  /** 0 = fully dissolved into the emitter, 1 = fully assembled figure. */
  assemble: React.RefObject<number>
  color: string
  opacity: number
  count?: number
}

const ASSEMBLE_DURATION = 1.6

/**
 * Cortana-style particle presence: samples the `presence_form` GLB surface and
 * renders it as a point cloud whose particles flow between the emitter center
 * (dissolved) and their surface positions (assembled figure).
 *
 * Technique adapted from ektogamat/threejs-particle-skull (Anderson Mancini,
 * MIT): MeshSurfaceSampler + THREE.Points + per-particle shader displacement.
 *
 * The target value lives in a ref (`assemble.current`) so callers can drive it
 * from presence state without re-rendering; this component eases toward the
 * target each frame.
 */
export default function PresenceParticleSystem({
  assemble,
  color,
  opacity,
  // 6000: enough to read as a particle hologram without additive saturation
  // merging the silhouette into a white blob at the seated camera (WS3b fix).
  count = 6000,
}: PresenceParticleSystemProps) {
  const asset = getSceneAssetByBinding('presence_form')
  const gltf = useGLTF(asset?.glbUrl ?? '')
  const pointsRef = useRef<THREE.Points>(null)
  const easedRef = useRef(assemble.current ?? 1)

  const geometry = useMemo(() => {
    let mesh: THREE.Mesh | null = null
    gltf.scene.traverse((child) => {
      if (!mesh && (child as THREE.Mesh).isMesh) mesh = child as THREE.Mesh
    })
    if (!mesh) return null
    // Work in the mesh's own space so the cloud matches the rendered figure.
    const sampler = new MeshSurfaceSampler(mesh).build()
    const positions = new Float32Array(count * 3)
    const randoms = new Float32Array(count)
    const scatter = new Float32Array(count * 3)
    const temp = new THREE.Vector3()
    for (let i = 0; i < count; i += 1) {
      sampler.sample(temp)
      positions[i * 3] = temp.x
      positions[i * 3 + 1] = temp.y
      positions[i * 3 + 2] = temp.z
      randoms[i] = Math.random()
      // Dissolved rest position: a low dome around the emitter origin.
      const angle = Math.random() * Math.PI * 2
      const radius = Math.sqrt(Math.random()) * 0.22
      scatter[i * 3] = Math.cos(angle) * radius
      scatter[i * 3 + 1] = Math.random() * 0.06
      scatter[i * 3 + 2] = Math.sin(angle) * radius
    }
    const geo = new THREE.BufferGeometry()
    geo.setAttribute('position', new THREE.BufferAttribute(positions, 3))
    geo.setAttribute('aRandom', new THREE.BufferAttribute(randoms, 1))
    geo.setAttribute('aScatter', new THREE.BufferAttribute(scatter, 3))
    return geo
  }, [gltf.scene, count])

  const material = useMemo(
    () =>
      new THREE.ShaderMaterial({
        transparent: true,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
        uniforms: {
          uAssemble: { value: easedRef.current },
          uTime: { value: 0 },
          uColor: { value: new THREE.Color(color) },
          uOpacity: { value: opacity },
          uSize: { value: 0.16 },
        },
        vertexShader: /* glsl */ `
          attribute float aRandom;
          attribute vec3 aScatter;
          uniform float uAssemble;
          uniform float uTime;
          uniform float uSize;
          uniform float uOpacity;
          varying float vAlpha;

          void main() {
            // Per-particle stagger so the flow reads as a swarm, not a morph.
            float delay = aRandom * 0.55;
            float localT = clamp((uAssemble - delay * (1.0 - uAssemble)) / max(1.0 - delay * (1.0 - uAssemble), 0.001), 0.0, 1.0);
            float t = smoothstep(0.0, 1.0, clamp(uAssemble * (1.0 + delay) - delay, 0.0, 1.0));

            // Curved path: swirl offset peaks mid-transition.
            float mid = sin(t * 3.14159);
            float angle = aRandom * 6.2831 + uTime * (0.4 + aRandom * 0.5);
            vec3 swirl = vec3(cos(angle), sin(angle * 0.6), sin(angle)) * mid * (0.12 + aRandom * 0.22);

            // Idle shimmer when assembled.
            vec3 shimmer = normalize(position + 0.001) * sin(uTime * (1.2 + aRandom * 1.6) + aRandom * 6.2831) * 0.008;

            vec3 pos = mix(aScatter, position, t) + swirl * (1.0 - abs(t * 2.0 - 1.0)) + shimmer;

            vec4 mvPosition = modelViewMatrix * vec4(pos, 1.0);
            gl_Position = projectionMatrix * mvPosition;
            gl_PointSize = clamp(uSize * (1.0 + aRandom * 0.8) * (120.0 / -mvPosition.z), 1.0, 3.0);
            vAlpha = mix(0.22, 0.5, t) * (0.5 + 0.3 * aRandom) * uOpacity;
          }
        `,
        fragmentShader: /* glsl */ `
          uniform vec3 uColor;
          varying float vAlpha;

          void main() {
            vec2 uv = gl_PointCoord - 0.5;
            float dist = length(uv);
            if (dist > 0.5) discard;
            float glow = smoothstep(0.5, 0.05, dist);
            gl_FragColor = vec4(uColor, glow * vAlpha);
          }
        `,
      }),
    [color, opacity],
  )

  useFrame(({ clock }, delta) => {
    if (material.uniforms) {
      material.uniforms.uTime.value = clock.getElapsedTime()
      const target = assemble.current ?? 1
      // Ease toward the target so state flips produce flowing transitions.
      easedRef.current += (target - easedRef.current) * Math.min(delta / (ASSEMBLE_DURATION * 0.35), 1)
      material.uniforms.uAssemble.value = easedRef.current
    }
    void pointsRef
  })

  useEffect(() => () => geometry?.dispose(), [geometry])

  if (!geometry) return null

  return <points ref={pointsRef} geometry={geometry} material={material} frustumCulled={false} />
}
