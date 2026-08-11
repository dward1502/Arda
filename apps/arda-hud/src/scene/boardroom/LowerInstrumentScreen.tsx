import { useEffect, useMemo, useRef, useState } from 'react'
import * as THREE from 'three'
import type { HudInstrumentModel } from './boardroomHudInstruments'
import type { BoardroomVec3 } from './boardroomSpatialLayout'
import { resolveBoardroomInstrumentSurfaceGeometry } from './BoardroomInstrumentScreen'
import { shouldDrawInstrumentFrame } from './instrumentFrameCadence'
import {
  deriveLowerInstrumentSignal,
  sampleLowerInstrumentSequence,
  type LowerInstrumentRole,
  type LowerInstrumentSignal,
} from './lowerInstrumentSignal'

interface LowerInstrumentScreenProps {
  slotId: string
  role: LowerInstrumentRole
  size: BoardroomVec3
  model: HudInstrumentModel
  motionEnabled?: boolean
  onActivate: () => void
}

function withAlpha(color: string, alpha: number): string {
  return `${color}${Math.round(Math.max(0, Math.min(1, alpha)) * 255).toString(16).padStart(2, '0')}`
}

function drawBackground(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  signal: LowerInstrumentSignal,
): void {
  const glow = context.createRadialGradient(width / 2, height / 2, 12, width / 2, height / 2, width * 0.66)
  glow.addColorStop(0, '#071720')
  glow.addColorStop(0.42, '#030b12')
  glow.addColorStop(1, '#010307')
  context.fillStyle = glow
  context.fillRect(0, 0, width, height)

  context.strokeStyle = withAlpha(signal.accent, 0.08)
  context.lineWidth = 1
  for (let x = 32; x < width; x += 32) {
    context.beginPath()
    context.moveTo(x, 0)
    context.lineTo(x, height)
    context.stroke()
  }
  for (let y = 32; y < height; y += 32) {
    context.beginPath()
    context.moveTo(0, y)
    context.lineTo(width, y)
    context.stroke()
  }
}

function drawNode(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  radius: number,
  color: string,
  pulse: number,
): void {
  context.save()
  context.strokeStyle = color
  context.fillStyle = withAlpha(color, 0.14 + pulse * 0.18)
  context.lineWidth = 2
  context.shadowColor = color
  context.shadowBlur = 7 + pulse * 12
  context.beginPath()
  context.arc(x, y, radius + pulse * 2.4, 0, Math.PI * 2)
  context.fill()
  context.stroke()
  context.restore()
}

function drawGovernance(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  signal: LowerInstrumentSignal,
  time: number,
): void {
  const centerX = width / 2
  const centerY = height / 2
  const lanes = 7
  context.save()
  context.lineWidth = 2
  for (let lane = 0; lane < lanes; lane += 1) {
    const y = 76 + lane * 60
    const bend = Math.sin(time * 0.44 + lane * 1.7) * (12 + signal.pressure * 20)
    const chosen = lane === Math.floor((time * signal.cadence * 0.35 + signal.seed) % lanes)
    context.strokeStyle = withAlpha(chosen ? signal.secondary : signal.accent, chosen ? 0.92 : 0.34)
    context.shadowColor = chosen ? signal.secondary : signal.accent
    context.shadowBlur = chosen ? 14 : 4
    context.beginPath()
    context.moveTo(42, y)
    context.bezierCurveTo(210, y + bend, 280, centerY + bend * 0.35, centerX - 44, centerY)
    context.bezierCurveTo(710, centerY - bend * 0.35, 800, y - bend, width - 42, y)
    context.stroke()
    drawNode(context, 154, y + bend * 0.52, 5, signal.accent, chosen ? 1 : 0.2)
    drawNode(context, width - 154, y - bend * 0.52, 5, chosen ? signal.secondary : signal.accent, chosen ? 1 : 0.2)
  }

  context.translate(centerX, centerY)
  context.rotate(Math.PI / 4 + Math.sin(time * 0.2) * 0.06)
  for (let layer = 0; layer < 4; layer += 1) {
    const size = 36 + layer * 29
    context.strokeStyle = withAlpha(layer === 1 ? signal.secondary : signal.accent, 0.82 - layer * 0.13)
    context.lineWidth = layer === 0 ? 4 : 2
    context.strokeRect(-size, -size, size * 2, size * 2)
  }
  context.restore()

  const quorum = 0.5 + Math.sin(time * signal.cadence * 2.1) * 0.5
  drawNode(context, centerX, centerY, 11 + signal.coherence * 7, signal.secondary, quorum)

  context.save()
  context.strokeStyle = withAlpha(signal.warning, 0.22 + signal.pressure * 0.48)
  context.lineWidth = 3
  for (let side = -1; side <= 1; side += 2) {
    context.beginPath()
    context.arc(centerX + side * 250, centerY, 64, -Math.PI * 0.72, Math.PI * 0.72)
    context.stroke()
  }
  context.restore()
}

function drawSystems(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  signal: LowerInstrumentSignal,
  time: number,
): void {
  const centerX = width / 2
  const sequence = sampleLowerInstrumentSequence(signal, time, 48)
  context.save()
  context.strokeStyle = withAlpha(signal.accent, 0.5)
  context.lineWidth = 2
  for (let column = 0; column < 12; column += 1) {
    const x = 52 + column * 83
    const heightFactor = Math.abs(sequence[column * 3] ?? 0)
    const barHeight = 48 + heightFactor * 155 + signal.activity * 58
    context.fillStyle = withAlpha(column % 3 === 0 ? signal.secondary : signal.accent, 0.1 + heightFactor * 0.26)
    context.fillRect(x - 11, (height - barHeight) / 2, 22, barHeight)
    context.strokeRect(x - 11, (height - barHeight) / 2, 22, barHeight)
    for (let segment = 0; segment < 7; segment += 1) {
      const segmentY = height / 2 - barHeight / 2 + 12 + segment * ((barHeight - 24) / 6)
      context.fillStyle = withAlpha(signal.accent, 0.3 + Math.max(0, Math.sin(time * 2 + column + segment)) * 0.6)
      context.fillRect(x - 7, segmentY, 14, 2)
    }
  }
  context.restore()

  context.save()
  context.translate(centerX, height / 2)
  const corePulse = 0.5 + Math.sin(time * signal.cadence * 3.4) * 0.5
  for (let unit = -3; unit <= 3; unit += 1) {
    const y = unit * 57
    const radius = 18 + (unit === 0 ? corePulse * 8 : 0)
    context.strokeStyle = unit === 0 ? signal.secondary : signal.accent
    context.fillStyle = withAlpha(unit === 0 ? signal.secondary : signal.accent, unit === 0 ? 0.26 : 0.1)
    context.lineWidth = unit === 0 ? 4 : 2
    context.shadowColor = context.strokeStyle
    context.shadowBlur = unit === 0 ? 24 : 8
    context.beginPath()
    for (let point = 0; point < 6; point += 1) {
      const angle = point / 6 * Math.PI * 2 - Math.PI / 2
      const x = Math.cos(angle) * radius
      const py = y + Math.sin(angle) * radius
      if (point === 0) context.moveTo(x, py)
      else context.lineTo(x, py)
    }
    context.closePath()
    context.fill()
    context.stroke()
    if (unit < 3) {
      context.beginPath()
      context.moveTo(0, y + radius)
      context.lineTo(0, y + 57 - radius)
      context.stroke()
    }
  }
  context.restore()

  if (signal.pressure > 0.35) {
    context.fillStyle = withAlpha(signal.warning, 0.25 + signal.pressure * 0.35)
    const y = 32 + Math.abs(Math.sin(time * 1.7 + signal.seed)) * (height - 64)
    context.fillRect(28, y, width - 56, 4)
  }
}

function quadraticPoint(
  start: [number, number],
  control: [number, number],
  end: [number, number],
  progress: number,
): [number, number] {
  const inverse = 1 - progress
  return [
    inverse * inverse * start[0] + 2 * inverse * progress * control[0] + progress * progress * end[0],
    inverse * inverse * start[1] + 2 * inverse * progress * control[1] + progress * progress * end[1],
  ]
}

function drawRouting(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  signal: LowerInstrumentSignal,
  time: number,
): void {
  const centerX = width / 2
  const centerY = height / 2
  context.save()
  context.translate(centerX, centerY)
  context.strokeStyle = withAlpha(signal.accent, 0.28)
  context.lineWidth = 2
  for (let orbit = 0; orbit < 6; orbit += 1) {
    context.beginPath()
    context.ellipse(0, 0, 94 + orbit * 52, 34 + orbit * 22, orbit * 0.18, 0, Math.PI * 2)
    context.stroke()
  }
  context.rotate(time * signal.cadence * 0.56)
  const sweep = context.createLinearGradient(0, 0, 410, 0)
  sweep.addColorStop(0, withAlpha(signal.secondary, 0.9))
  sweep.addColorStop(1, withAlpha(signal.secondary, 0))
  context.strokeStyle = sweep
  context.lineWidth = 4
  context.beginPath()
  context.moveTo(0, 0)
  context.lineTo(430, 0)
  context.stroke()
  context.restore()

  const endpoints: Array<[number, number]> = [
    [54, 92], [55, 418], [250, 54], [258, 458], [766, 52], [770, 458], [970, 100], [968, 410],
  ]
  endpoints.forEach((start, index) => {
    const end = endpoints[(index + 3) % endpoints.length]
    const lift = index % 2 === 0 ? -115 : 115
    const control: [number, number] = [centerX + Math.sin(index * 1.7) * 120, centerY + lift]
    context.strokeStyle = withAlpha(index % 3 === 0 ? signal.secondary : signal.accent, 0.32 + signal.coherence * 0.3)
    context.lineWidth = index % 3 === 0 ? 3 : 1.5
    context.beginPath()
    context.moveTo(start[0], start[1])
    context.quadraticCurveTo(control[0], control[1], end[0], end[1])
    context.stroke()
    const progress = (time * signal.cadence * (0.16 + index * 0.015) + index * 0.13) % 1
    const packet = quadraticPoint(start, control, end, progress)
    drawNode(context, packet[0], packet[1], index % 3 === 0 ? 6 : 3, index % 3 === 0 ? signal.secondary : signal.accent, 0.8)
    drawNode(context, start[0], start[1], 5, signal.accent, 0.25)
  })

  drawNode(context, centerX, centerY, 12 + signal.activity * 8, signal.secondary, 0.5 + Math.sin(time * 2.3) * 0.5)
}

function drawHuman(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  signal: LowerInstrumentSignal,
  time: number,
): void {
  const centerX = width / 2
  const centerY = height / 2
  context.save()
  context.translate(centerX, centerY)
  context.lineWidth = 2
  for (let contour = 0; contour < 12; contour += 1) {
    const baseRadiusX = 48 + contour * 27
    const baseRadiusY = 23 + contour * 13
    context.strokeStyle = withAlpha(contour % 4 === 0 ? signal.secondary : signal.accent, 0.48 - contour * 0.018)
    context.beginPath()
    const points = 96
    for (let point = 0; point <= points; point += 1) {
      const angle = point / points * Math.PI * 2
      const breathing = Math.sin(angle * 3 + time * signal.cadence + contour * 0.6) * (3 + signal.activity * 5)
      const pressure = Math.sin(angle * 7 - time * 0.8 + contour) * signal.pressure * 7
      const x = Math.cos(angle) * (baseRadiusX + breathing + pressure)
      const y = Math.sin(angle) * (baseRadiusY + breathing * 0.45 + pressure * 0.35)
      if (point === 0) context.moveTo(x, y)
      else context.lineTo(x, y)
    }
    context.closePath()
    context.stroke()
  }
  context.restore()

  const samples = sampleLowerInstrumentSequence(signal, time, 110)
  for (let strand = -1; strand <= 1; strand += 2) {
    context.save()
    context.strokeStyle = strand === -1 ? withAlpha(signal.accent, 0.82) : withAlpha(signal.secondary, 0.72)
    context.shadowColor = strand === -1 ? signal.accent : signal.secondary
    context.shadowBlur = 10
    context.lineWidth = 2.5
    context.beginPath()
    samples.forEach((sample, index) => {
      const x = 76 + index / (samples.length - 1) * (width - 152)
      const y = centerY + sample * 78 + strand * Math.sin(index * 0.18 + time) * 18
      if (index === 0) context.moveTo(x, y)
      else context.lineTo(x, y)
      if (index % 11 === 0) drawNode(context, x, y, 3, strand === -1 ? signal.accent : signal.secondary, 0.4)
    })
    context.stroke()
    context.restore()
  }

  const orbitCount = 9
  for (let index = 0; index < orbitCount; index += 1) {
    const angle = time * (0.14 + signal.activity * 0.12) + index / orbitCount * Math.PI * 2
    const radiusX = 180 + Math.sin(index * 2.1) * 74
    const radiusY = 92 + Math.cos(index * 1.4) * 38
    drawNode(
      context,
      centerX + Math.cos(angle) * radiusX,
      centerY + Math.sin(angle) * radiusY,
      3 + index % 3,
      index % 3 === 0 ? signal.secondary : signal.accent,
      0.5,
    )
  }
}

function drawFrame(
  canvas: HTMLCanvasElement,
  signal: LowerInstrumentSignal,
  elapsedSeconds: number,
  motionEnabled: boolean,
): void {
  const context = canvas.getContext('2d')
  if (!context) return
  const width = canvas.width
  const height = canvas.height
  const time = motionEnabled ? elapsedSeconds : 1.75
  context.clearRect(0, 0, width, height)
  drawBackground(context, width, height, signal)

  if (signal.role === 'governance') drawGovernance(context, width, height, signal, time)
  else if (signal.role === 'systems') drawSystems(context, width, height, signal, time)
  else if (signal.role === 'routing') drawRouting(context, width, height, signal, time)
  else drawHuman(context, width, height, signal, time)

  context.fillStyle = 'rgba(155, 238, 255, 0.022)'
  for (let y = 1; y < height; y += 4) context.fillRect(0, y, width, 1)

  context.strokeStyle = withAlpha(signal.secondary, 0.72)
  context.lineWidth = 3
  const corner = 28
  for (const [x, y, sx, sy] of [[corner, corner, 1, 1], [width - corner, corner, -1, 1], [corner, height - corner, 1, -1], [width - corner, height - corner, -1, -1]] as const) {
    context.beginPath()
    context.moveTo(x, y + sy * 22)
    context.lineTo(x, y)
    context.lineTo(x + sx * 22, y)
    context.stroke()
  }
}

export function LowerInstrumentScreen({
  slotId,
  role,
  size,
  model,
  motionEnabled = true,
  onActivate,
}: LowerInstrumentScreenProps) {
  const geometry = useMemo(
    () => resolveBoardroomInstrumentSurfaceGeometry(slotId, 'desk_surface', size),
    [size, slotId],
  )
  const [hovered, setHovered] = useState(false)
  const signal = useMemo(() => deriveLowerInstrumentSignal(role, model), [model, role])
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
  const materialRef = useRef<THREE.MeshBasicMaterial>(null)

  useEffect(() => {
    const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const animate = motionEnabled && !reducedMotion
    const startedAt = performance.now()
    let animationFrame = 0
    let disposed = false
    let lastDrawAt = Number.NEGATIVE_INFINITY
    const draw = (now: number) => {
      if (disposed) return
      if (shouldDrawInstrumentFrame(now, lastDrawAt)) {
        drawFrame(texture.canvas, signal, (now - startedAt) / 1000, animate)
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
  }, [motionEnabled, signal, texture])

  useEffect(() => () => texture.texture.dispose(), [texture])
  useEffect(() => {
    if (materialRef.current) materialRef.current.opacity = hovered ? 1 : 0.98
  }, [hovered])

  return (
    <group position={geometry.fitPosition} rotation={geometry.fitRotation}>
      <mesh
        position={geometry.position}
        rotation={geometry.rotation}
        renderOrder={8}
        userData={{ slotId, surfaceKind: `${role}_signal_instrument` }}
        onPointerOver={(event) => {
          event.stopPropagation()
          setHovered(true)
          document.body.style.cursor = 'pointer'
        }}
        onPointerOut={(event) => {
          event.stopPropagation()
          setHovered(false)
          document.body.style.cursor = 'auto'
        }}
        onClick={(event) => {
          event.stopPropagation()
          onActivate()
        }}
      >
        <planeGeometry args={[geometry.width, geometry.height]} />
        <meshBasicMaterial ref={materialRef} map={texture.texture} transparent opacity={0.98} toneMapped={false} depthWrite />
      </mesh>
    </group>
  )
}
