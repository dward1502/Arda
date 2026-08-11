import { useEffect, useMemo, useRef, useState } from 'react'
import * as THREE from 'three'
import type { HudInstrumentModel } from './boardroomHudInstruments'
import type { BoardroomVec3 } from './boardroomSpatialLayout'
import { resolveBoardroomInstrumentSurfaceGeometry } from './BoardroomInstrumentScreen'
import { shouldDrawInstrumentFrame } from './instrumentFrameCadence'
import {
  deriveCommandCoreSignal,
  resolveCommandCoreFrameTime,
  sampleCommandCoreWave,
  type CommandCoreSignal,
} from './commandCoreSignal'

interface CommandCoreInstrumentScreenProps {
  slotId: string
  size: BoardroomVec3
  model: HudInstrumentModel
  onActivate: () => void
}

function drawPhosphorGrid(context: CanvasRenderingContext2D, width: number, height: number, signal: CommandCoreSignal): void {
  context.save()
  context.strokeStyle = `${signal.accent}16`
  context.lineWidth = 1
  for (let ring = 1; ring <= 6; ring += 1) {
    context.beginPath()
    context.ellipse(width / 2, height / 2, ring * 68, ring * 31, 0, 0, Math.PI * 2)
    context.stroke()
  }
  for (let spoke = 0; spoke < 16; spoke += 1) {
    const angle = (spoke / 16) * Math.PI * 2
    context.beginPath()
    context.moveTo(width / 2, height / 2)
    context.lineTo(width / 2 + Math.cos(angle) * width * 0.48, height / 2 + Math.sin(angle) * height * 0.47)
    context.stroke()
  }
  context.restore()
}

function drawAsciiField(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  signal: CommandCoreSignal,
  time: number,
): void {
  const glyphs = ['·', '+', '×', ':', '◇', '¦']
  context.save()
  context.font = '20px IBM Plex Mono, monospace'
  context.textAlign = 'center'
  for (let side = 0; side < 2; side += 1) {
    const originX = side === 0 ? 34 : width - 112
    for (let column = 0; column < 5; column += 1) {
      for (let row = 0; row < 13; row += 1) {
        const phase = Math.floor(time * signal.cadence * 3 + row + column * 2 + side * 5)
        const index = Math.abs((phase + Math.floor(signal.seed)) % glyphs.length)
        const pulse = 0.12 + 0.38 * Math.max(0, Math.sin(time * signal.cadence * 2 + row * 0.7 + column))
        context.fillStyle = `${column % 3 === 0 ? signal.secondary : signal.accent}${Math.round(pulse * 255).toString(16).padStart(2, '0')}`
        context.fillText(glyphs[index], originX + column * 19, 62 + row * 31)
      }
    }
  }
  context.restore()
}

function drawWave(
  context: CanvasRenderingContext2D,
  samples: number[],
  centerY: number,
  amplitude: number,
  color: string,
  width: number,
  glow: number,
): void {
  context.save()
  context.strokeStyle = color
  context.lineWidth = 2.5
  context.shadowColor = color
  context.shadowBlur = glow
  context.beginPath()
  samples.forEach((sample, index) => {
    const x = 120 + (index / (samples.length - 1)) * (width - 240)
    const y = centerY + sample * amplitude
    if (index === 0) context.moveTo(x, y)
    else context.lineTo(x, y)
  })
  context.stroke()
  context.restore()
}

function drawCommandCoreFrame(
  canvas: HTMLCanvasElement,
  signal: CommandCoreSignal,
  elapsedSeconds: number,
  motionEnabled: boolean,
): void {
  const context = canvas.getContext('2d')
  if (!context) return
  const width = canvas.width
  const height = canvas.height
  const time = resolveCommandCoreFrameTime(elapsedSeconds, motionEnabled)

  context.clearRect(0, 0, width, height)
  const background = context.createRadialGradient(width / 2, height / 2, 20, width / 2, height / 2, width * 0.58)
  background.addColorStop(0, '#071825')
  background.addColorStop(0.48, '#030b14')
  background.addColorStop(1, '#010308')
  context.fillStyle = background
  context.fillRect(0, 0, width, height)

  drawPhosphorGrid(context, width, height, signal)
  drawAsciiField(context, width, height, signal, time)

  const centerX = width / 2
  const centerY = height / 2
  const sweep = time * signal.cadence * 0.72
  context.save()
  context.translate(centerX, centerY)
  context.rotate(sweep)
  const sweepGradient = context.createLinearGradient(0, 0, 330, 0)
  sweepGradient.addColorStop(0, `${signal.accent}cc`)
  sweepGradient.addColorStop(1, `${signal.accent}00`)
  context.strokeStyle = sweepGradient
  context.lineWidth = 4
  context.shadowColor = signal.accent
  context.shadowBlur = 16
  context.beginPath()
  context.moveTo(0, 0)
  context.lineTo(360, 0)
  context.stroke()
  context.restore()

  context.save()
  context.translate(centerX, centerY)
  for (let layer = 0; layer < 4; layer += 1) {
    const direction = layer % 2 === 0 ? 1 : -1
    const radiusX = 74 + layer * 42
    const radiusY = 34 + layer * 19
    const rotation = time * (0.2 + layer * 0.08) * direction + layer * 0.9
    context.strokeStyle = layer === 2 ? signal.secondary : `${signal.accent}${layer === 0 ? 'ee' : '99'}`
    context.lineWidth = layer === 0 ? 5 : 3
    context.shadowColor = layer === 2 ? signal.secondary : signal.accent
    context.shadowBlur = 10
    for (let segment = 0; segment < 8; segment += 1) {
      const start = rotation + segment * (Math.PI / 4)
      context.beginPath()
      context.ellipse(0, 0, radiusX, radiusY, 0, start, start + 0.34 + signal.coherence * 0.16)
      context.stroke()
    }
  }
  context.restore()

  const primary = sampleCommandCoreWave(signal, time, 160)
  const counter = sampleCommandCoreWave({ ...signal, seed: signal.seed + 71, cadence: signal.cadence * 0.73 }, -time * 0.62, 160)
  drawWave(context, primary, centerY, 66 + signal.intensity * 24, `${signal.accent}cc`, width, 14)
  drawWave(context, counter, centerY, 34 + signal.attention * 36, `${signal.secondary}9a`, width, 8)

  const heartbeat = (Math.sin(time * signal.cadence * Math.PI * 2) + 1) / 2
  context.save()
  context.fillStyle = signal.attention > 0.45 ? signal.warning : signal.accent
  context.shadowColor = context.fillStyle
  context.shadowBlur = 26
  context.beginPath()
  context.arc(centerX, centerY, 5 + heartbeat * 8 + signal.intensity * 5, 0, Math.PI * 2)
  context.fill()
  context.restore()

  const corner = 38
  context.strokeStyle = `${signal.secondary}aa`
  context.lineWidth = 3
  for (const [x, y, sx, sy] of [[corner, corner, 1, 1], [width - corner, corner, -1, 1], [corner, height - corner, 1, -1], [width - corner, height - corner, -1, -1]] as const) {
    context.beginPath()
    context.moveTo(x, y + sy * 28)
    context.lineTo(x, y)
    context.lineTo(x + sx * 28, y)
    context.stroke()
  }

  if (signal.attention > 0.35) {
    const glitchY = Math.abs(Math.sin(time * 1.93 + signal.seed)) * height
    context.fillStyle = `${signal.warning}${Math.round(signal.attention * 50).toString(16).padStart(2, '0')}`
    context.fillRect(92, glitchY, width - 184, 2 + signal.attention * 5)
  }

  context.fillStyle = 'rgba(155, 238, 255, 0.025)'
  for (let y = 1; y < height; y += 4) context.fillRect(0, y, width, 1)
}

export function CommandCoreInstrumentScreen({ slotId, size, model, onActivate }: CommandCoreInstrumentScreenProps) {
  const geometry = useMemo(
    () => resolveBoardroomInstrumentSurfaceGeometry(slotId, 'desk_surface', size),
    [size, slotId],
  )
  const [hovered, setHovered] = useState(false)
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
  const signal = useMemo(() => deriveCommandCoreSignal(model), [model])

  useEffect(() => {
    const motionEnabled = !window.matchMedia('(prefers-reduced-motion: reduce)').matches
    let disposed = false
    let animationFrame = 0
    const startedAt = performance.now()
    let lastDrawAt = Number.NEGATIVE_INFINITY
    const draw = (now: number) => {
      if (disposed) return
      if (shouldDrawInstrumentFrame(now, lastDrawAt)) {
        drawCommandCoreFrame(texture.canvas, signal, (now - startedAt) / 1000, motionEnabled)
        texture.texture.needsUpdate = true
        lastDrawAt = now
      }
      if (motionEnabled) animationFrame = requestAnimationFrame(draw)
    }
    draw(startedAt)
    return () => {
      disposed = true
      if (animationFrame) cancelAnimationFrame(animationFrame)
    }
  }, [signal, texture])

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
        userData={{ slotId, surfaceKind: 'command_core_signal_instrument' }}
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
