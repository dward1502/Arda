import { useEffect, useMemo } from 'react'
import type { ThreeEvent } from '@react-three/fiber'
import * as THREE from 'three'
import type { BoardroomVec3 } from '../../scene/boardroom/boardroomSpatialLayout'
import { resolveBoardroomInstrumentSurfaceGeometry } from '../../scene/boardroom/BoardroomInstrumentScreen'
import { shouldDrawInstrumentFrame } from '../../scene/boardroom/instrumentFrameCadence'
import type { UpperMonitorDisplayMode } from '../../scene/boardroom/upperAmbientSignal'
import {
  deriveMirromereVisualModel,
  drawMirromereFrame,
  isMirromereInspectAllowed,
  resolveMirromereMotion,
  type MirromereSurface,
} from '@arda/mirromere-ui'

export {
  deriveMirromereVisualModel,
  drawMirromereFrame,
  isMirromereInspectAllowed,
  resolveMirromereMotion,
} from '@arda/mirromere-ui'

export interface MirromereApertureProps {
  surface: MirromereSurface
  slotId: string
  size: BoardroomVec3
  motionEnabled: boolean
  onActivate?: () => void
}

export function shouldRenderMirromereAperture(
  slotId: string,
  displayMode: UpperMonitorDisplayMode,
  surface: MirromereSurface | null | undefined,
): boolean {
  return slotId === 'monitor_3' && displayMode === 'ambient' && Boolean(surface)
}

export default function MirromereAperture({
  surface,
  slotId,
  size,
  motionEnabled,
  onActivate,
}: MirromereApertureProps) {
  const geometry = useMemo(
    () => resolveBoardroomInstrumentSurfaceGeometry(slotId, 'monitor_surface', size),
    [size, slotId],
  )
  const texture = useMemo(() => {
    const canvas = document.createElement('canvas')
    canvas.width = 512; canvas.height = 256
    const value = new THREE.CanvasTexture(canvas)
    value.colorSpace = THREE.SRGBColorSpace
    value.minFilter = THREE.LinearFilter; value.magFilter = THREE.LinearFilter
    value.generateMipmaps = false
    return { canvas, value }
  }, [])

  useEffect(() => {
    const prefersReduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
    const animate = resolveMirromereMotion(surface, motionEnabled, prefersReduced)
    const startedAt = performance.now()
    let frame = 0
    let disposed = false
    let lastDrawAt = Number.NEGATIVE_INFINITY
    const draw = (now: number) => {
      if (disposed) return
      if (!animate || shouldDrawInstrumentFrame(now, lastDrawAt)) {
        drawMirromereFrame(texture.canvas, surface, (now - startedAt) / 1000, animate)
        texture.value.needsUpdate = true
        lastDrawAt = now
      }
      if (animate) frame = requestAnimationFrame(draw)
    }
    draw(startedAt)
    return () => { disposed = true; if (frame) cancelAnimationFrame(frame) }
  }, [motionEnabled, surface, texture])

  useEffect(() => () => texture.value.dispose(), [texture])
  const inspectAllowed = isMirromereInspectAllowed(surface)
  const handleClick = inspectAllowed && onActivate
    ? (event: ThreeEvent<MouseEvent>) => { event.stopPropagation(); onActivate() }
    : undefined
  const model = deriveMirromereVisualModel(surface)
  return (
    <group position={geometry.fitPosition} rotation={geometry.fitRotation}>
      <mesh
        position={geometry.position}
        rotation={geometry.rotation}
        renderOrder={8}
        onClick={handleClick}
        userData={{
          slotId,
          surfaceKind: 'mirromere',
          sceneId: surface.scene.scene_id,
          truthState: model.truthState,
          inspectAllowed,
        }}
      >
        <planeGeometry args={[geometry.width, geometry.height]} />
        <meshBasicMaterial map={texture.value} transparent opacity={0.97} toneMapped={false} depthWrite />
      </mesh>
    </group>
  )
}
