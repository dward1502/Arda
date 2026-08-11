import { useEffect, useMemo } from 'react'
import * as THREE from 'three'
import type { BoardroomAgentClaim } from '../../lib/boardroomSlotSettings'
import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'
import type { BoardroomVec3 } from './boardroomSpatialLayout'
import { resolveBoardroomInstrumentSurfaceGeometry } from './BoardroomInstrumentScreen'
import { deriveMonitorOwnershipRail, type MonitorOwnershipRailModel } from './monitorOwnershipRailModel'

interface MonitorOwnershipRailProps {
  slotId: string
  size: BoardroomVec3
  session: MonitorSurfaceSessionRecord | null
  claim: BoardroomAgentClaim | null
  motionEnabled: boolean
}

function drawOwnershipRail(
  canvas: HTMLCanvasElement,
  model: MonitorOwnershipRailModel,
  elapsedSeconds: number,
): void {
  const context = canvas.getContext('2d')
  if (!context) return
  const width = canvas.width
  const height = canvas.height
  context.clearRect(0, 0, width, height)

  const base = context.createLinearGradient(0, 0, width, 0)
  base.addColorStop(0, 'rgba(2, 7, 12, 0.1)')
  base.addColorStop(0.08, 'rgba(5, 15, 22, 0.92)')
  base.addColorStop(0.92, 'rgba(5, 15, 22, 0.92)')
  base.addColorStop(1, 'rgba(2, 7, 12, 0.1)')
  context.fillStyle = base
  context.fillRect(0, 0, width, height)

  context.strokeStyle = model.occupied ? model.color : 'rgba(71, 111, 128, 0.24)'
  context.globalAlpha = model.occupied ? 0.34 : 0.22
  context.lineWidth = 1.5
  context.beginPath()
  context.moveTo(18, height / 2)
  context.lineTo(width - 18, height / 2)
  context.stroke()
  context.globalAlpha = 1

  if (!model.occupied) {
    for (let index = 0; index < 7; index += 1) {
      context.fillStyle = 'rgba(75, 120, 138, 0.12)'
      context.fillRect(width / 2 - 56 + index * 18, height / 2 - 2, 8, 4)
    }
    return
  }

  const sourceX = 28
  context.save()
  context.translate(sourceX, height / 2)
  context.rotate(Math.PI / 4)
  context.fillStyle = model.color
  context.globalAlpha = model.source === 'session' ? 0.92 : 0.58
  context.shadowColor = model.color
  context.shadowBlur = model.source === 'session' ? 14 : 7
  context.fillRect(-5, -5, 10, 10)
  context.restore()

  const startX = 62
  model.fingerprint.forEach((active, index) => {
    const barHeight = active ? 22 : 8
    const x = startX + index * 19
    context.fillStyle = model.color
    context.globalAlpha = active ? 0.82 : 0.18
    context.fillRect(x, height / 2 - barHeight / 2, 8, barHeight)
  })
  context.globalAlpha = 1

  const leaseColor = model.leaseState === 'healthy'
    ? model.color
    : model.leaseState === 'expiring'
      ? '#ffc65c'
      : '#ff527c'
  const pulse = model.leaseState === 'healthy'
    ? 0.74
    : 0.35 + Math.max(0, Math.sin(elapsedSeconds * (model.leaseState === 'expired' ? 7 : 4))) * 0.65
  const leaseX = width - 164
  for (let index = 0; index < 7; index += 1) {
    context.fillStyle = leaseColor
    context.globalAlpha = index < 6 ? pulse * (0.42 + index * 0.08) : pulse
    context.fillRect(leaseX + index * 20, height / 2 - 3, 12, 6)
  }
  context.globalAlpha = 1

  const travelerProgress = elapsedSeconds * 0.22 % 1
  const travelerX = 20 + travelerProgress * (width - 40)
  const traveler = context.createRadialGradient(travelerX, height / 2, 0, travelerX, height / 2, 24)
  traveler.addColorStop(0, model.color)
  traveler.addColorStop(1, 'rgba(0, 0, 0, 0)')
  context.fillStyle = traveler
  context.globalAlpha = model.leaseState === 'expired' ? 0.22 : 0.56
  context.fillRect(travelerX - 24, 0, 48, height)
  context.globalAlpha = 1
}

export function MonitorOwnershipRail({
  slotId,
  size,
  session,
  claim,
  motionEnabled,
}: MonitorOwnershipRailProps) {
  const geometry = useMemo(
    () => resolveBoardroomInstrumentSurfaceGeometry(slotId, 'monitor_surface', size),
    [size, slotId],
  )
  const texture = useMemo(() => {
    const canvas = document.createElement('canvas')
    canvas.width = 512
    canvas.height = 64
    const next = new THREE.CanvasTexture(canvas)
    next.colorSpace = THREE.SRGBColorSpace
    next.minFilter = THREE.LinearFilter
    next.magFilter = THREE.LinearFilter
    next.generateMipmaps = false
    return { canvas, texture: next }
  }, [])

  useEffect(() => {
    const animate = motionEnabled
      && Boolean(session || claim)
      && !window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const startedAt = performance.now()
    let animationFrame = 0
    let disposed = false
    let lastDrawAt = Number.NEGATIVE_INFINITY
    const draw = (now: number) => {
      if (disposed) return
      if (!animate || now - lastDrawAt >= 1000 / 15) {
        const model = deriveMonitorOwnershipRail(session, claim, new Date().toISOString())
        drawOwnershipRail(texture.canvas, model, (now - startedAt) / 1000)
        texture.texture.needsUpdate = true
        lastDrawAt = now
      }
      if (animate) animationFrame = requestAnimationFrame(draw)
    }
    draw(startedAt)
    return () => {
      disposed = true
      if (animationFrame) cancelAnimationFrame(animationFrame)
    }
  }, [claim, motionEnabled, session, texture])

  useEffect(() => () => texture.texture.dispose(), [texture])

  return (
    <group position={geometry.fitPosition} rotation={geometry.fitRotation}>
      <group position={geometry.position} rotation={geometry.rotation}>
        <mesh
          position={[0, -geometry.height * 0.56, 0.008]}
          renderOrder={9}
          userData={{ slotId, surfaceKind: 'monitor_ownership_rail' }}
        >
          <planeGeometry args={[geometry.width * 0.88, Math.max(geometry.height * 0.05, 0.024)]} />
          <meshBasicMaterial map={texture.texture} transparent opacity={0.98} toneMapped={false} depthWrite={false} />
        </mesh>
      </group>
    </group>
  )
}
