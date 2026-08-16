// sigil: REPAIR
import { useEffect, useMemo, useRef, useState } from 'react'
import * as THREE from 'three'
import {
  resolveHudInstrumentTruthPresentation,
  type HudInstrumentModel,
  type HudTone,
} from './boardroomHudInstruments'
import type { BoardroomPreviewMode, BoardroomVec3 } from './boardroomSpatialLayout'

const TONE_COLORS: Record<HudTone, string> = {
  cyan: '#5defff',
  violet: '#b98cff',
  gold: '#ffd37a',
  mint: '#8cffc7',
  rose: '#ffa6d9',
}

export interface BoardroomInstrumentSurfaceGeometry {
  width: number
  height: number
  fitPosition: BoardroomVec3
  fitRotation: BoardroomVec3
  position: BoardroomVec3
  rotation: BoardroomVec3
}

const DESK_SURFACE_FITS: Record<string, { position: BoardroomVec3; rotation: BoardroomVec3 }> = {
  'boardroom.lower.left_wrap': {
    position: [0, 0.07, 0],
    rotation: [0, 0, 0.08216751443234044],
  },
  'boardroom.lower.left_inner': {
    position: [0, 0.0275, 0],
    rotation: [0.03816806069159268, 0.0007715793169580637, 0.020203110006172184],
  },
  'boardroom.control.center': {
    position: [0, 0, 0],
    rotation: [0, 0, 0],
  },
  'boardroom.lower.right_inner': {
    position: [0, 0.0275, 0],
    rotation: [0.03816806069159268, -0.0007715793169580637, -0.020203110006172184],
  },
  'boardroom.lower.right_wrap': {
    position: [0, 0.07, 0],
    rotation: [0, 0, -0.08216751443234044],
  },
}

export function resolveBoardroomInstrumentSurfaceGeometry(
  slotId: string,
  previewMode: BoardroomPreviewMode,
  size: BoardroomVec3,
): BoardroomInstrumentSurfaceGeometry {
  if (previewMode === 'monitor_surface') {
    return {
      width: size[0] * 0.9,
      height: size[1] * 0.84,
      fitPosition: [0, 0, 0],
      fitRotation: [0, 0, 0],
      position: [0, 0, size[2] / 2 + 0.085],
      rotation: [0, 0, 0],
    }
  }

  const fit = DESK_SURFACE_FITS[slotId] ?? DESK_SURFACE_FITS['boardroom.control.center']
  return {
    width: size[0] * 0.9,
    height: size[2] * 0.82,
    fitPosition: fit.position,
    fitRotation: fit.rotation,
    position: [0, 0.038, 0],
    rotation: [-Math.PI / 2, 0, 0],
  }
}

function truncate(context: CanvasRenderingContext2D, value: string, maxWidth: number): string {
  if (context.measureText(value).width <= maxWidth) return value
  let next = value
  while (next.length > 1 && context.measureText(`${next}…`).width > maxWidth) next = next.slice(0, -1)
  return `${next}…`
}

function drawInstrument(canvas: HTMLCanvasElement, model: HudInstrumentModel): void {
  const context = canvas.getContext('2d')
  if (!context) return

  const width = canvas.width
  const height = canvas.height
  const accent = TONE_COLORS[model.tone]
  const truth = model.source ? resolveHudInstrumentTruthPresentation(model.source.truthState) : null
  const statusColor = model.status === 'nominal'
    ? '#8cffc7'
    : model.status === 'watch'
      ? '#ffd37a'
      : model.status === 'external'
        ? '#b98cff'
        : '#ff789c'

  context.clearRect(0, 0, width, height)
  const background = context.createLinearGradient(0, 0, 0, height)
  background.addColorStop(0, '#07121c')
  background.addColorStop(1, '#02060b')
  context.fillStyle = background
  context.fillRect(0, 0, width, height)

  context.strokeStyle = `${accent}66`
  context.lineWidth = 5
  context.setLineDash(model.source && model.source.truthState !== 'live' ? [18, 10] : [])
  context.strokeRect(7, 7, width - 14, height - 14)
  context.setLineDash([])
  context.strokeStyle = 'rgba(255,255,255,0.055)'
  context.lineWidth = 2
  for (let x = 64; x < width; x += 64) {
    context.beginPath()
    context.moveTo(x, 120)
    context.lineTo(x, height - 88)
    context.stroke()
  }
  for (let y = 152; y < height - 88; y += 54) {
    context.beginPath()
    context.moveTo(40, y)
    context.lineTo(width - 40, y)
    context.stroke()
  }

  context.fillStyle = accent
  context.font = '800 28px IBM Plex Sans, sans-serif'
  context.fillText(model.eyebrow.toUpperCase(), 44, 56)

  context.fillStyle = '#effcff'
  context.font = '800 52px IBM Plex Sans, sans-serif'
  context.fillText(truncate(context, model.title, width - 220), 44, 116)

  context.textAlign = 'right'
  context.fillStyle = accent
  context.font = '900 30px IBM Plex Sans, sans-serif'
  context.fillText(model.glyph, width - 44, 60)
  context.textAlign = 'left'

  const nodes = model.nodes.length > 0 ? model.nodes : [
    { id: 'fallback-a', x: 18, y: 52, state: 'dim' as const },
    { id: 'fallback-b', x: 50, y: 28, state: 'dim' as const },
    { id: 'fallback-c', x: 82, y: 62, state: 'dim' as const },
  ]
  const plotLeft = 56
  const plotTop = 152
  const plotWidth = width - 112
  const plotHeight = height - 256

  context.strokeStyle = `${accent}99`
  context.lineWidth = 5
  context.beginPath()
  nodes.forEach((node, index) => {
    const x = plotLeft + (node.x / 100) * plotWidth
    const y = plotTop + (node.y / 100) * plotHeight
    if (index === 0) context.moveTo(x, y)
    else context.lineTo(x, y)
  })
  context.stroke()

  for (const node of nodes) {
    const x = plotLeft + (node.x / 100) * plotWidth
    const y = plotTop + (node.y / 100) * plotHeight
    context.beginPath()
    context.arc(x, y, node.state === 'alert' ? 10 : 7, 0, Math.PI * 2)
    context.fillStyle = node.state === 'alert' ? '#ff789c' : node.state === 'warn' ? '#ffd37a' : accent
    context.fill()
  }

  context.fillStyle = statusColor
  context.font = '900 26px IBM Plex Sans, sans-serif'
  context.fillText(model.status.toUpperCase(), 44, height - 40)

  context.textAlign = 'right'
  context.fillStyle = 'rgba(221,248,255,0.66)'
  context.font = '700 22px IBM Plex Sans, sans-serif'
  const sourceCaption = truth && model.source
    ? `${truth.marker} ${truth.label} · ${model.source.sourceLabel}`
    : 'LOCAL SURFACE'
  context.fillText(truncate(context, sourceCaption, width * 0.62), width - 44, height - 40)
  context.textAlign = 'left'
}

interface BoardroomInstrumentScreenProps {
  slotId: string
  previewMode: BoardroomPreviewMode
  size: BoardroomVec3
  model: HudInstrumentModel
  onActivate: () => void
}

export function BoardroomInstrumentScreen({
  slotId,
  previewMode,
  size,
  model,
  onActivate,
}: BoardroomInstrumentScreenProps) {
  const geometry = useMemo(
    () => resolveBoardroomInstrumentSurfaceGeometry(slotId, previewMode, size),
    [previewMode, size, slotId],
  )
  const [hovered, setHovered] = useState(false)
  const texture = useMemo(() => {
    const canvas = document.createElement('canvas')
    canvas.width = 1024
    canvas.height = 512
    const next = new THREE.CanvasTexture(canvas)
    next.colorSpace = THREE.SRGBColorSpace
    next.minFilter = THREE.LinearFilter
    next.magFilter = THREE.LinearFilter
    next.generateMipmaps = false
    return { canvas, texture: next }
  }, [])
  const materialRef = useRef<THREE.MeshBasicMaterial>(null)

  useEffect(() => {
    drawInstrument(texture.canvas, model)
    texture.texture.needsUpdate = true
  }, [model, texture])

  useEffect(() => () => texture.texture.dispose(), [texture])

  useEffect(() => {
    if (materialRef.current) materialRef.current.opacity = hovered ? 1 : 0.96
  }, [hovered])

  return (
    <group position={geometry.fitPosition} rotation={geometry.fitRotation}>
      <mesh
        position={geometry.position}
        rotation={geometry.rotation}
        renderOrder={8}
        userData={{ slotId, surfaceKind: 'physical_canvas_texture' }}
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
        <meshBasicMaterial
          ref={materialRef}
          map={texture.texture}
          transparent
          opacity={0.96}
          toneMapped={false}
          depthWrite
        />
      </mesh>
    </group>
  )
}
