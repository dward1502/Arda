import { useEffect, useMemo } from 'react'
import * as THREE from 'three'
import type { BoardroomVec3 } from './boardroomSpatialLayout'
import { resolveBoardroomInstrumentSurfaceGeometry } from './BoardroomInstrumentScreen'
import { shouldDrawInstrumentFrame } from './instrumentFrameCadence'
import {
  resolveUpperAmbientIdentity,
  sampleUpperAmbientField,
  UPPER_AMBIENT_IDENTITIES,
  type UpperAmbientIdentity,
} from './upperAmbientSignal'

interface UpperAmbientMonitorScreenProps {
  slotId: string
  size: BoardroomVec3
  motionEnabled: boolean
}

const withAlpha = (color: string, alpha: number): string =>
  `${color}${Math.round(Math.max(0, Math.min(1, alpha)) * 255).toString(16).padStart(2, '0')}`

function drawAmbientBackground(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  identity: UpperAmbientIdentity,
): void {
  const gradient = context.createRadialGradient(width / 2, height / 2, 20, width / 2, height / 2, width * 0.64)
  gradient.addColorStop(0, '#07131d')
  gradient.addColorStop(0.5, '#030912')
  gradient.addColorStop(1, '#010205')
  context.fillStyle = gradient
  context.fillRect(0, 0, width, height)

  context.strokeStyle = withAlpha(identity.accent, 0.035)
  context.lineWidth = 1
  for (let x = 0; x <= width; x += 32) {
    context.beginPath()
    context.moveTo(x, 0)
    context.lineTo(x, height)
    context.stroke()
  }
  for (let y = 0; y <= height; y += 32) {
    context.beginPath()
    context.moveTo(0, y)
    context.lineTo(width, y)
    context.stroke()
  }
}

function drawAuroraVeil(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  identity: UpperAmbientIdentity,
  time: number,
): void {
  context.save()
  context.globalCompositeOperation = 'lighter'
  for (let ribbon = 0; ribbon < 9; ribbon += 1) {
    const offset = ribbon / 8
    const path = new Path2D()
    path.moveTo(-80, height * (0.1 + offset * 0.82))
    for (let segment = 0; segment <= 16; segment += 1) {
      const x = -80 + segment / 16 * (width + 160)
      const y = height * (0.1 + offset * 0.82)
        + Math.sin(segment * 0.76 + time * identity.cadence * 3.2 + ribbon * 0.64) * (34 + ribbon * 3)
        + Math.cos(segment * 0.23 - time * 0.21) * 28
      path.lineTo(x, y)
    }
    context.strokeStyle = withAlpha(ribbon % 2 === 0 ? identity.accent : identity.secondary, 0.16 + (8 - Math.abs(4 - ribbon)) * 0.025)
    context.lineWidth = 10 + ribbon * 1.8
    context.shadowColor = ribbon % 2 === 0 ? identity.accent : identity.secondary
    context.shadowBlur = 20
    context.stroke(path)
  }
  context.restore()

  for (let star = 0; star < 28; star += 1) {
    const x = (star * 131 + identity.seed * 7) % width
    const y = (star * 73 + identity.seed * 3) % height
    const pulse = 0.24 + Math.max(0, Math.sin(time * 0.65 + star)) * 0.52
    context.fillStyle = withAlpha(star % 4 === 0 ? identity.secondary : identity.accent, pulse)
    context.fillRect(x, y, star % 5 === 0 ? 3 : 1.5, star % 5 === 0 ? 3 : 1.5)
  }
}

function constellationPoint(index: number, width: number, height: number, identity: UpperAmbientIdentity, time: number): [number, number] {
  const column = index % 8
  const row = Math.floor(index / 8)
  return [
    74 + column * ((width - 148) / 7) + Math.sin(time * identity.cadence + index * 1.9) * 22,
    58 + row * ((height - 116) / 4) + Math.cos(time * identity.cadence * 0.8 + index * 1.3) * 18,
  ]
}

function drawConstellationMesh(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  identity: UpperAmbientIdentity,
  time: number,
): void {
  const points = Array.from({ length: 40 }, (_, index) => constellationPoint(index, width, height, identity, time))
  context.save()
  for (let index = 0; index < points.length; index += 1) {
    const targets = [index + 1, index + 7, index + 9]
    targets.forEach((target) => {
      if (target >= points.length) return
      const [x1, y1] = points[index]
      const [x2, y2] = points[target]
      const distance = Math.hypot(x2 - x1, y2 - y1)
      if (distance > 210) return
      context.strokeStyle = withAlpha(target % 5 === 0 ? identity.secondary : identity.accent, 0.08 + (1 - distance / 210) * 0.2)
      context.lineWidth = target % 5 === 0 ? 1.8 : 1
      context.beginPath()
      context.moveTo(x1, y1)
      context.lineTo(x2, y2)
      context.stroke()
    })
  }
  points.forEach(([x, y], index) => {
    const pulse = 0.3 + Math.max(0, Math.sin(time * 0.7 + index * 1.4)) * 0.7
    context.fillStyle = withAlpha(index % 6 === 0 ? identity.secondary : identity.accent, pulse)
    context.shadowColor = index % 6 === 0 ? identity.secondary : identity.accent
    context.shadowBlur = index % 6 === 0 ? 13 : 6
    context.beginPath()
    context.arc(x, y, index % 6 === 0 ? 3.5 : 1.7, 0, Math.PI * 2)
    context.fill()
  })
  context.restore()

  const drift = (time * 18 * identity.cadence) % width
  const comet = context.createLinearGradient(drift - 120, 0, drift + 20, 0)
  comet.addColorStop(0, withAlpha(identity.secondary, 0))
  comet.addColorStop(1, withAlpha(identity.secondary, 0.72))
  context.strokeStyle = comet
  context.lineWidth = 2
  context.beginPath()
  context.moveTo(drift - 120, height * 0.28)
  context.lineTo(drift + 20, height * 0.22)
  context.stroke()
}

function drawSignalMandala(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  identity: UpperAmbientIdentity,
  time: number,
): void {
  const centerX = width / 2
  const centerY = height / 2
  context.save()
  context.translate(centerX, centerY)
  context.globalCompositeOperation = 'lighter'
  for (let layer = 0; layer < 11; layer += 1) {
    const radius = 24 + layer * 20
    const petals = 6 + layer * 2
    const rotation = time * identity.cadence * (layer % 2 === 0 ? 0.45 : -0.32)
    context.strokeStyle = withAlpha(layer % 3 === 0 ? identity.secondary : identity.accent, 0.52 - layer * 0.026)
    context.lineWidth = layer < 3 ? 2.5 : 1.3
    context.beginPath()
    for (let point = 0; point <= petals * 8; point += 1) {
      const angle = point / (petals * 8) * Math.PI * 2
      const petal = Math.sin(angle * petals + rotation) * (7 + layer * 0.9)
      const x = Math.cos(angle + rotation * 0.12) * (radius + petal)
      const y = Math.sin(angle + rotation * 0.12) * (radius + petal) * 0.82
      if (point === 0) context.moveTo(x, y)
      else context.lineTo(x, y)
    }
    context.closePath()
    context.stroke()
  }
  context.rotate(-time * identity.cadence * 0.7)
  for (let spoke = 0; spoke < 24; spoke += 1) {
    const angle = spoke / 24 * Math.PI * 2
    context.strokeStyle = withAlpha(spoke % 4 === 0 ? identity.secondary : identity.accent, spoke % 4 === 0 ? 0.42 : 0.12)
    context.beginPath()
    context.moveTo(Math.cos(angle) * 54, Math.sin(angle) * 45)
    context.lineTo(Math.cos(angle) * 236, Math.sin(angle) * 194)
    context.stroke()
  }
  context.restore()

  const pulse = 0.5 + Math.sin(time * identity.cadence * 5) * 0.5
  context.fillStyle = identity.secondary
  context.shadowColor = identity.secondary
  context.shadowBlur = 24
  context.beginPath()
  context.arc(centerX, centerY, 5 + pulse * 6, 0, Math.PI * 2)
  context.fill()
}

function drawVectorRain(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  identity: UpperAmbientIdentity,
  time: number,
): void {
  context.save()
  context.globalCompositeOperation = 'lighter'
  for (let column = 0; column < 46; column += 1) {
    const x = 18 + column * ((width - 36) / 45)
    const speed = 34 + (column * 19 % 72)
    const head = (time * speed + column * 83 + identity.seed * 5) % (height + 160) - 80
    const length = 42 + column % 7 * 12
    const gradient = context.createLinearGradient(0, head - length, 0, head + 8)
    gradient.addColorStop(0, withAlpha(column % 7 === 0 ? identity.secondary : identity.accent, 0))
    gradient.addColorStop(1, withAlpha(column % 7 === 0 ? identity.secondary : identity.accent, 0.74))
    context.strokeStyle = gradient
    context.lineWidth = column % 7 === 0 ? 3 : 1.3
    context.beginPath()
    context.moveTo(x, head - length)
    context.lineTo(x, head)
    context.stroke()
    context.fillStyle = withAlpha(column % 7 === 0 ? identity.secondary : identity.accent, 0.85)
    context.fillRect(x - 1, head, column % 7 === 0 ? 3 : 1.5, column % 7 === 0 ? 8 : 4)
  }
  context.restore()

  const field = sampleUpperAmbientField(identity, time, 120)
  context.strokeStyle = withAlpha(identity.secondary, 0.7)
  context.shadowColor = identity.secondary
  context.shadowBlur = 11
  context.lineWidth = 2
  context.beginPath()
  field.forEach((value, index) => {
    const x = 32 + index / (field.length - 1) * (width - 64)
    const y = height * 0.72 + value * 42
    if (index === 0) context.moveTo(x, y)
    else context.lineTo(x, y)
  })
  context.stroke()
}

function drawDreamHorizon(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  identity: UpperAmbientIdentity,
  time: number,
): void {
  const horizon = height * 0.59
  const glow = context.createRadialGradient(width / 2, horizon, 5, width / 2, horizon, 210)
  glow.addColorStop(0, withAlpha(identity.secondary, 0.28))
  glow.addColorStop(1, withAlpha(identity.secondary, 0))
  context.fillStyle = glow
  context.fillRect(0, 0, width, height)

  for (let layer = 0; layer < 14; layer += 1) {
    const baseY = horizon + layer * 11
    const field = sampleUpperAmbientField({ ...identity, seed: identity.seed + layer * 9 }, time * (0.45 + layer * 0.018), 90)
    context.strokeStyle = withAlpha(layer % 4 === 0 ? identity.secondary : identity.accent, 0.42 - layer * 0.018)
    context.lineWidth = layer % 4 === 0 ? 2 : 1
    context.beginPath()
    field.forEach((value, index) => {
      const x = index / (field.length - 1) * width
      const y = baseY + value * (24 + layer * 2)
      if (index === 0) context.moveTo(x, y)
      else context.lineTo(x, y)
    })
    context.stroke()
  }

  context.save()
  context.translate(width / 2, horizon)
  context.strokeStyle = withAlpha(identity.secondary, 0.55)
  context.lineWidth = 2
  for (let arc = 1; arc <= 7; arc += 1) {
    context.beginPath()
    context.arc(0, 0, arc * 24, Math.PI, Math.PI * 2)
    context.stroke()
  }
  context.rotate(time * identity.cadence * 0.35)
  for (let ray = 0; ray < 20; ray += 1) {
    const angle = ray / 20 * Math.PI
    context.strokeStyle = withAlpha(identity.accent, ray % 5 === 0 ? 0.38 : 0.1)
    context.beginPath()
    context.moveTo(Math.cos(angle) * 44, -Math.sin(angle) * 44)
    context.lineTo(Math.cos(angle) * 260, -Math.sin(angle) * 260)
    context.stroke()
  }
  context.restore()
}

function drawAmbientFrame(
  canvas: HTMLCanvasElement,
  identity: UpperAmbientIdentity,
  elapsedSeconds: number,
  motionEnabled: boolean,
): void {
  const context = canvas.getContext('2d')
  if (!context) return
  const width = canvas.width
  const height = canvas.height
  const time = motionEnabled ? elapsedSeconds : 2.4
  context.clearRect(0, 0, width, height)
  drawAmbientBackground(context, width, height, identity)

  if (identity.id === 'aurora_veil') drawAuroraVeil(context, width, height, identity, time)
  else if (identity.id === 'constellation_mesh') drawConstellationMesh(context, width, height, identity, time)
  else if (identity.id === 'signal_mandala') drawSignalMandala(context, width, height, identity, time)
  else if (identity.id === 'vector_rain') drawVectorRain(context, width, height, identity, time)
  else drawDreamHorizon(context, width, height, identity, time)

  context.fillStyle = 'rgba(155, 238, 255, 0.018)'
  for (let y = 1; y < height; y += 4) context.fillRect(0, y, width, 1)
}

export function UpperAmbientMonitorScreen({
  slotId,
  size,
  motionEnabled,
}: UpperAmbientMonitorScreenProps) {
  const identityId = resolveUpperAmbientIdentity(slotId) ?? 'signal_mandala'
  const identity = UPPER_AMBIENT_IDENTITIES[identityId]
  const geometry = useMemo(
    () => resolveBoardroomInstrumentSurfaceGeometry(slotId, 'monitor_surface', size),
    [size, slotId],
  )
  const texture = useMemo(() => {
    const canvas = document.createElement('canvas')
    canvas.width = 512
    canvas.height = 256
    const next = new THREE.CanvasTexture(canvas)
    next.colorSpace = THREE.SRGBColorSpace
    next.minFilter = THREE.LinearFilter
    next.magFilter = THREE.LinearFilter
    next.generateMipmaps = false
    return { canvas, texture: next }
  }, [])

  useEffect(() => {
    const animate = motionEnabled && !window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const startedAt = performance.now()
    let animationFrame = 0
    let disposed = false
    let lastDrawAt = Number.NEGATIVE_INFINITY
    const draw = (now: number) => {
      if (disposed) return
      if (!animate || shouldDrawInstrumentFrame(now, lastDrawAt)) {
        drawAmbientFrame(texture.canvas, identity, (now - startedAt) / 1000, animate)
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
  }, [identity, motionEnabled, texture])

  useEffect(() => () => texture.texture.dispose(), [texture])

  return (
    <group position={geometry.fitPosition} rotation={geometry.fitRotation}>
      <mesh
        position={geometry.position}
        rotation={geometry.rotation}
        renderOrder={8}
        userData={{ slotId, surfaceKind: `upper_ambient_${identity.id}` }}
      >
        <planeGeometry args={[geometry.width, geometry.height]} />
        <meshBasicMaterial map={texture.texture} transparent opacity={0.96} toneMapped={false} depthWrite />
      </mesh>
    </group>
  )
}
