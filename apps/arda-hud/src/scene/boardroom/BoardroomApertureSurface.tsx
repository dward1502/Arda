// sigil: REPAIR
import { useEffect, useMemo } from 'react'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import * as THREE from 'three'
import type { MonitorContentDescriptor, MonitorPlaybackState } from '../../lib/monitorSurfaceContract'
import { readFile } from '../../lib/weathertop'
import { resolveMonitorApertureDescriptorState } from './monitorApertureDescriptorState'
import type { BoardroomPreviewMode, BoardroomVec3 } from './boardroomSpatialLayout'
import { resolveMonitorApertureGeometry } from './monitorApertureGeometry'
import {
  resolveHudInstrumentTruthPresentation,
  type HudInstrumentModel,
  type HudTone,
} from './boardroomHudInstruments'
import { formatMonitorSurfaceStream } from './monitorSurfaceRuntime'
import type { MonitorSurfacePayloadEvent } from './monitorSurfaceRuntime'
import { resolveOperatorProjectionCanvasModel } from './operatorProjectionMonitorRenderer'
import {
  pumpMjpegFrames,
  parseGeneratedFrameProps,
  resolveMjpegRenderPath,
  resolveMonitorMediaUrl,
  resolveVideoPlaybackPlan,
} from './monitorMediaRuntime'
import { createMonitorMediaLifecycle } from './monitorMediaLifecycle'

const TONE_COLORS: Record<HudTone, string> = {
  cyan: '#5defff',
  violet: '#b98cff',
  gold: '#ffd37a',
  mint: '#8cffc7',
  rose: '#ffa6d9',
}

interface BoardroomApertureSurfaceProps {
  zoneId: string
  previewMode: BoardroomPreviewMode
  size: BoardroomVec3
  model: HudInstrumentModel
  onActivate: () => void
  motionEnabled?: boolean
  payload?: Pick<MonitorSurfacePayloadEvent, 'content' | 'mime'>
  descriptor?: MonitorContentDescriptor
  playback?: MonitorPlaybackState
  rootPath?: string | null
  active?: boolean
  debug?: boolean
}

function statusColor(status: string) {
  if (status === 'nominal') return '#8cffc7'
  if (status === 'watch') return '#ffd37a'
  if (status === 'external') return '#b98cff'
  return '#ff789c'
}

function drawInstrument(canvas: HTMLCanvasElement, model: HudInstrumentModel): void {
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const width = canvas.width
  const height = canvas.height
  const accent = TONE_COLORS[model.tone]
  const status = statusColor(model.status)
  const truth = model.source ? resolveHudInstrumentTruthPresentation(model.source.truthState) : null

  ctx.clearRect(0, 0, width, height)
  const background = ctx.createLinearGradient(0, 0, 0, height)
  background.addColorStop(0, '#07121c')
  background.addColorStop(1, '#02060b')
  ctx.fillStyle = background
  ctx.fillRect(0, 0, width, height)

  ctx.strokeStyle = `${accent}66`
  ctx.lineWidth = 5
  ctx.setLineDash(model.source && model.source.truthState !== 'live' ? [18, 10] : [])
  ctx.strokeRect(7, 7, width - 14, height - 14)
  ctx.setLineDash([])
  ctx.strokeStyle = 'rgba(255,255,255,0.055)'
  ctx.lineWidth = 2
  for (let x = 64; x < width; x += 64) {
    ctx.beginPath()
    ctx.moveTo(x, 120)
    ctx.lineTo(x, height - 88)
    ctx.stroke()
  }
  for (let y = 152; y < height - 88; y += 54) {
    ctx.beginPath()
    ctx.moveTo(40, y)
    ctx.lineTo(width - 40, y)
    ctx.stroke()
  }

  ctx.fillStyle = accent
  ctx.font = '800 28px IBM Plex Sans, sans-serif'
  ctx.fillText(model.eyebrow.toUpperCase(), 44, 56)

  ctx.fillStyle = '#effcff'
  ctx.font = '800 52px IBM Plex Sans, sans-serif'
  ctx.fillText(truncate(ctx, model.title, width - 220), 44, 116)

  ctx.textAlign = 'right'
  ctx.fillStyle = accent
  ctx.font = '900 30px IBM Plex Sans, sans-serif'
  ctx.fillText(model.glyph, width - 44, 60)

  ctx.textAlign = 'left'
  ctx.fillStyle = status
  ctx.font = '900 26px IBM Plex Sans, sans-serif'
  ctx.fillText(model.status.toUpperCase(), 44, height - 40)

  ctx.textAlign = 'right'
  ctx.fillStyle = 'rgba(221,248,255,0.66)'
  ctx.font = '700 22px IBM Plex Sans, sans-serif'
  const sourceCaption = truth && model.source
    ? `${truth.marker} ${truth.label} · ${model.source.sourceLabel}`
    : 'AGENT MONITOR'
  ctx.fillText(truncate(ctx, sourceCaption, width * 0.62), width - 44, height - 40)
  ctx.textAlign = 'left'
}

function truncate(ctx: CanvasRenderingContext2D, value: string, maxWidth: number): string {
  if (ctx.measureText(value).width <= maxWidth) return value
  let next = value
  while (next.length > 1 && ctx.measureText(`${next}…`).width > maxWidth) next = next.slice(0, -1)
  return `${next}…`
}

function drawMessage(canvas: HTMLCanvasElement, title: string, detail: string, color = '#ffd37a'): void {
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.clearRect(0, 0, canvas.width, canvas.height)
  ctx.fillStyle = '#02060b'
  ctx.fillRect(0, 0, canvas.width, canvas.height)
  ctx.strokeStyle = `${color}66`
  ctx.lineWidth = 5
  ctx.strokeRect(7, 7, canvas.width - 14, canvas.height - 14)
  ctx.textAlign = 'center'
  ctx.fillStyle = color
  ctx.font = '900 34px IBM Plex Sans, sans-serif'
  ctx.fillText(title.toUpperCase(), canvas.width / 2, canvas.height / 2 - 18)
  ctx.fillStyle = 'rgba(221,248,255,0.75)'
  ctx.font = '700 22px IBM Plex Sans, sans-serif'
  ctx.fillText(truncate(ctx, detail, canvas.width - 96), canvas.width / 2, canvas.height / 2 + 30)
  ctx.textAlign = 'left'
}

function drawDocumentText(canvas: HTMLCanvasElement, title: string, content: string): void {
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.clearRect(0, 0, canvas.width, canvas.height)
  ctx.fillStyle = '#061019'
  ctx.fillRect(0, 0, canvas.width, canvas.height)
  ctx.fillStyle = '#8cffc7'
  ctx.font = '900 26px IBM Plex Mono, monospace'
  ctx.fillText(truncate(ctx, title, canvas.width - 72), 36, 48)
  ctx.fillStyle = '#d9f8ff'
  ctx.font = '600 20px IBM Plex Mono, monospace'
  let y = 88
  const maxWidth = canvas.width - 72
  for (const paragraph of content.slice(0, 12_000).split(/\r?\n/)) {
    const words = paragraph.split(/\s+/).filter(Boolean)
    let line = ''
    for (const word of words.length ? words : [' ']) {
      const candidate = line ? `${line} ${word}` : word
      if (ctx.measureText(candidate).width > maxWidth && line) {
        ctx.fillText(line, 36, y)
        y += 28
        line = word
      } else line = candidate
      if (y > canvas.height - 38) return
    }
    ctx.fillText(line, 36, y)
    y += 28
    if (y > canvas.height - 38) return
  }
}

function drawOperatorProjection(canvas: HTMLCanvasElement, props: Record<string, unknown>): boolean {
  const model = resolveOperatorProjectionCanvasModel(props)
  if (!model.ok) return false
  const ctx = canvas.getContext('2d')
  if (!ctx) return false
  ctx.clearRect(0, 0, canvas.width, canvas.height)
  ctx.fillStyle = '#02070d'
  ctx.fillRect(0, 0, canvas.width, canvas.height)
  ctx.strokeStyle = '#5defff88'
  ctx.lineWidth = 5
  ctx.strokeRect(7, 7, canvas.width - 14, canvas.height - 14)
  ctx.fillStyle = '#5defff'
  ctx.font = '900 24px IBM Plex Mono, monospace'
  ctx.fillText(`OPERATOR PROJECTION · ${model.authority.toUpperCase()}`, 40, 50)
  ctx.fillStyle = '#effcff'
  ctx.font = '900 42px IBM Plex Sans, sans-serif'
  ctx.fillText(truncate(ctx, model.title, canvas.width - 80), 40, 105)
  ctx.fillStyle = '#8cffc7'
  ctx.font = '700 19px IBM Plex Mono, monospace'
  ctx.fillText(`${model.projectionId} · ${model.freshness.toUpperCase()}`, 40, 142)
  ctx.fillStyle = '#d9f8ff'
  ctx.font = '650 21px IBM Plex Mono, monospace'
  model.rows.slice(0, 8).forEach((row, index) => {
    ctx.fillText(truncate(ctx, row, canvas.width - 80), 40, 196 + index * 43)
  })
  ctx.fillStyle = 'rgba(221,248,255,0.58)'
  ctx.font = '700 18px IBM Plex Mono, monospace'
  ctx.fillText('CANONICAL · STATUS ONLY · NO LOCAL MUTATIONS', 40, canvas.height - 35)
  return true
}

type RendererKind = 'text' | 'image' | 'video' | 'canvas' | 'unknown'

interface NativeBrowserCaptureFrame {
  revision: number
  jpegBase64: string
}

interface NativePtyCaptureFrame {
  sessionId: string
  owner: string
  revision: number
  outputRevision: number
  processId: number | null
  rows: number
  cols: number
  output: string
}

function drawPtyFrame(canvas: HTMLCanvasElement, frame: NativePtyCaptureFrame): void {
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.fillStyle = '#010407'
  ctx.fillRect(0, 0, canvas.width, canvas.height)
  ctx.fillStyle = '#5defff'
  ctx.font = '800 18px IBM Plex Mono, monospace'
  ctx.fillText(`LIVE PTY · ${frame.sessionId} · REV ${frame.revision}/${frame.outputRevision}`, 30, 34)
  ctx.fillStyle = '#c9fbff'
  ctx.font = '650 17px IBM Plex Mono, monospace'
  const lines = frame.output.replace(/\r/g, '').split('\n').slice(-21)
  lines.forEach((line, index) => ctx.fillText(line.slice(0, frame.cols), 30, 70 + index * 20))
  ctx.fillStyle = 'rgba(140,255,199,0.68)'
  ctx.font = '700 15px IBM Plex Mono, monospace'
  ctx.fillText(`OWNER ${frame.owner} · PID ${frame.processId ?? 'UNKNOWN'} · ${frame.cols}×${frame.rows}`, 30, canvas.height - 20)
}

function resolveRendererFromPayload(
  payload: Pick<MonitorSurfacePayloadEvent, 'content' | 'mime'> | null | undefined,
): RendererKind {
  if (!payload?.content?.trim()) return 'text'
  const value = payload.content.trim()
  const mime = payload.mime.trim().toLowerCase()
  if (value.startsWith('{') || value.startsWith('[')) return 'canvas'
  if (mime.startsWith('image/')) return 'image'
  if (mime.startsWith('video/')) return 'video'
  return 'text'
}

function fitRect(
  containerWidth: number,
  containerHeight: number,
  imageWidth: number,
  imageHeight: number,
  fit: 'contain' | 'cover',
) {
  const containerRatio = containerWidth / containerHeight
  const imageRatio = imageWidth / imageHeight
  let width = containerWidth
  let height = containerHeight
  let x = 0
  let y = 0
  if (fit === 'contain' ? imageRatio > containerRatio : imageRatio < containerRatio) {
    height = width / imageRatio
    y = (containerHeight - height) / 2
  } else {
    width = height * imageRatio
    x = (containerWidth - width) / 2
  }
  return { width, height, x, y }
}

export function BoardroomApertureSurface({
  zoneId,
  previewMode,
  size,
  model,
  onActivate,
  motionEnabled,
  payload,
  descriptor,
  playback,
  rootPath,
  active = true,
  debug = false,
}: BoardroomApertureSurfaceProps) {
  const geometry = useMemo(
    () => resolveMonitorApertureGeometry(zoneId, previewMode, size),
    [zoneId, previewMode, size],
  )

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

  const renderModel = useMemo((): HudInstrumentModel => {
    const streamText = formatMonitorSurfaceStream(payload ?? null, !motionEnabled)
    const binding = model.title
    const title = streamText || binding
    const glyph = streamText ? 'LIVE' : model.glyph
    const tone = streamText ? 'cyan' : model.tone
    const status = streamText ? 'nominal' : model.status
    return { ...model, title, glyph, tone, status }
  }, [model, payload, motionEnabled])

  useEffect(() => {
    const { canvas, texture: nextTexture } = texture
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    let disposed = false
    let animationFrame = 0
    let image: HTMLImageElement | null = null
    let video: HTMLVideoElement | null = null
    const mediaLifecycle = createMonitorMediaLifecycle(cancelAnimationFrame)

    const markUpdated = () => {
      if (!disposed) nextTexture.needsUpdate = true
    }
    const showMessage = (title: string, detail: string, color?: string) => {
      drawMessage(canvas, title, detail, color)
      markUpdated()
    }
    const loadImage = (source: string, fit: 'contain' | 'cover') => {
      image = new Image()
      mediaLifecycle.registerImage(image)
      image.crossOrigin = 'anonymous'
      image.onload = () => {
        if (disposed || !image) return
        ctx.clearRect(0, 0, canvas.width, canvas.height)
        ctx.fillStyle = '#02060b'
        ctx.fillRect(0, 0, canvas.width, canvas.height)
        const view = fitRect(canvas.width, canvas.height, image.width, image.height, fit)
        ctx.drawImage(image, view.x, view.y, view.width, view.height)
        markUpdated()
      }
      image.onerror = () => showMessage('IMAGE UNAVAILABLE', source, '#ff789c')
      image.src = source
    }
    const loadMjpeg = (source: string) => {
      image = new Image()
      const streamImage = image
      mediaLifecycle.registerImage(streamImage)
      streamImage.crossOrigin = 'anonymous'
      streamImage.onerror = () => showMessage('STREAM UNAVAILABLE', source, '#ff789c')
      const drawFrame = () => {
        const view = fitRect(
          canvas.width,
          canvas.height,
          streamImage.naturalWidth,
          streamImage.naturalHeight,
          'contain',
        )
        ctx.clearRect(0, 0, canvas.width, canvas.height)
        ctx.fillStyle = '#02060b'
        ctx.fillRect(0, 0, canvas.width, canvas.height)
        ctx.drawImage(streamImage, view.x, view.y, view.width, view.height)
        markUpdated()
      }
      const scheduleFrame = (next: () => void) => {
        animationFrame = requestAnimationFrame(next)
        mediaLifecycle.scheduleAnimationFrame(animationFrame)
      }
      streamImage.src = source
      pumpMjpegFrames(streamImage, drawFrame, scheduleFrame, () => !disposed)
    }
    const loadNativeBrowserFrames = (sessionId: string) => {
      image = new Image()
      const frameImage = image
      mediaLifecycle.registerImage(frameImage)
      let revision = 0

      const scheduleNext = () => {
        if (disposed) return
        animationFrame = requestAnimationFrame(() => void poll())
        mediaLifecycle.scheduleAnimationFrame(animationFrame)
      }
      const drawFrame = () => {
        if (disposed) return
        const view = fitRect(
          canvas.width,
          canvas.height,
          frameImage.naturalWidth,
          frameImage.naturalHeight,
          'contain',
        )
        ctx.clearRect(0, 0, canvas.width, canvas.height)
        ctx.fillStyle = '#02060b'
        ctx.fillRect(0, 0, canvas.width, canvas.height)
        ctx.drawImage(frameImage, view.x, view.y, view.width, view.height)
        markUpdated()
        scheduleNext()
      }
      const poll = async () => {
        try {
          const frame = await invoke<NativeBrowserCaptureFrame | null>('get_browser_capture_frame', {
            sessionId,
            afterRevision: revision,
          })
          if (disposed) return
          if (!frame) {
            scheduleNext()
            return
          }
          revision = frame.revision
          frameImage.onload = drawFrame
          frameImage.onerror = () => showMessage('STREAM FRAME UNAVAILABLE', sessionId, '#ff789c')
          frameImage.src = `data:image/jpeg;base64,${frame.jpegBase64}`
        } catch (error) {
          if (!disposed) showMessage('STREAM UNAVAILABLE', String(error), '#ff789c')
        }
      }

      void poll()
    }
    const loadNativePtyFrames = (sessionId: string) => {
      let lastOutputRevision = -1
      const poll = async () => {
        try {
          const frame = await invoke<NativePtyCaptureFrame>('get_pty_capture_status', { sessionId })
          if (disposed) return
          if (frame.outputRevision !== lastOutputRevision) {
            drawPtyFrame(canvas, frame)
            lastOutputRevision = frame.outputRevision
            markUpdated()
          }
          window.setTimeout(() => void poll(), 120)
        } catch (error) {
          if (!disposed) {
            showMessage('PTY STREAM UNAVAILABLE', String(error), '#ff789c')
            window.setTimeout(() => void poll(), 350)
          }
        }
      }
      void poll()
    }
    const loadVideo = (source: string, fit: 'contain' | 'cover', loop: boolean, autoplay: boolean) => {
      const playbackPlan = resolveVideoPlaybackPlan(playback, autoplay)
      video = document.createElement('video')
      video.src = source
      video.crossOrigin = 'anonymous'
      video.loop = loop
      video.muted = true
      mediaLifecycle.registerVideo(video)
      video.volume = playbackPlan.volume
      video.playsInline = true
      video.preload = 'auto'
      const tick = () => {
        if (disposed || !video) return
        if (video.readyState >= 2) {
          const view = fitRect(canvas.width, canvas.height, video.videoWidth || 16, video.videoHeight || 9, fit)
          ctx.clearRect(0, 0, canvas.width, canvas.height)
          ctx.fillStyle = '#02060b'
          ctx.fillRect(0, 0, canvas.width, canvas.height)
          ctx.drawImage(video, view.x, view.y, view.width, view.height)
          markUpdated()
        }
        animationFrame = requestAnimationFrame(tick)
        mediaLifecycle.scheduleAnimationFrame(animationFrame)
      }
      video.addEventListener('loadedmetadata', () => {
        if (video && playbackPlan.seekTo != null) video.currentTime = playbackPlan.seekTo
        if (playbackPlan.playing) void video?.play().catch(() => undefined)
        else video?.pause()
        tick()
      }, { once: true })
      video.addEventListener('error', () => showMessage('VIDEO UNAVAILABLE', source, '#ff789c'), { once: true })
      video.load()
    }

    const render = async () => {
      if (descriptor) {
        const descriptorState = resolveMonitorApertureDescriptorState(descriptor)
        if (descriptorState.mode === 'message') {
          showMessage(descriptorState.title, descriptorState.detail, descriptorState.color)
          return
        }
        if (descriptor.kind === 'document') {
          const label = descriptor.source.kind === 'local' ? descriptor.source.path : descriptor.source.url
          if (descriptor.documentKind === 'pdf') {
            showMessage('PDF PREVIEW REQUIRED', label)
            return
          }
          if (descriptor.source.kind !== 'local') {
            showMessage('REMOTE DOCUMENT UNAVAILABLE', label)
            return
          }
          showMessage('LOADING DOCUMENT', label, '#8cffc7')
          const sourcePath = rootPath
            ? `${rootPath.replace(/\/$/, '')}/${descriptor.source.path}`
            : descriptor.source.path
          const result = await readFile(sourcePath)
          if (disposed) return
          if (!result.success || result.content == null) {
            showMessage('DOCUMENT UNAVAILABLE', result.error ?? label, '#ff789c')
            return
          }
          drawDocumentText(canvas, label, result.content)
          markUpdated()
          return
        }
        if (descriptor.kind === 'terminal') {
          loadNativePtyFrames(descriptor.sessionId)
          return
        }
        if (descriptor.kind === 'component' && descriptor.rendererId === 'operator_projection') {
          if (!drawOperatorProjection(canvas, descriptor.props)) {
            showMessage('PROJECTION UNAVAILABLE', 'canonical projection failed validation', '#ff789c')
            return
          }
          markUpdated()
          return
        }
        if (descriptor.kind === 'component' && descriptor.rendererId === 'generated_frame') {
          const frame = parseGeneratedFrameProps(descriptor.props)
          if (!frame) {
            showMessage('FRAME UNAVAILABLE', 'generated-frame descriptor failed validation', '#ff789c')
            return
          }
          const source = resolveMonitorMediaUrl(frame.source, rootPath, convertFileSrc)
          if (source.ok === true) loadImage(source.url, frame.fit)
          else showMessage('FRAME SOURCE UNAVAILABLE', source.reason)
          return
        }
        if (descriptor.kind === 'image') {
          const source = resolveMonitorMediaUrl(descriptor.source, rootPath, convertFileSrc)
          if (source.ok === true) loadImage(source.url, descriptor.fit)
          else showMessage('IMAGE SOURCE UNAVAILABLE', source.reason)
          return
        }
        if (descriptor.kind === 'video') {
          const source = resolveMonitorMediaUrl(descriptor.source, rootPath, convertFileSrc)
          if (source.ok === true) loadVideo(source.url, descriptor.fit, descriptor.loop ?? false, descriptor.autoplay ?? false)
          else showMessage('VIDEO SOURCE UNAVAILABLE', source.reason)
          return
        }
        if (descriptor.kind === 'remote_session') {
          if (descriptor.transport === 'mjpeg') {
            const renderPath = resolveMjpegRenderPath(descriptor.streamUrl, descriptor.sessionId)
            if (renderPath.kind === 'native-browser-frames') loadNativeBrowserFrames(renderPath.sessionId)
            else loadMjpeg(descriptor.streamUrl)
          }
          else if (descriptor.transport === 'hls') loadVideo(descriptor.streamUrl, 'contain', true, true)
          return
        }
        showMessage('CONTENT UNAVAILABLE', descriptor.kind)
        return
      }

      const renderer = resolveRendererFromPayload(payload)

      if (renderer === 'image') {
        loadImage(payload?.content ?? '', 'contain')
        return
      }

      if (renderer === 'video') {
        loadVideo(payload?.content ?? '', 'contain', true, true)
        return
      }

      if (renderer === 'canvas') {
        showMessage('CANVAS RENDERER', payload?.mime ?? 'unknown mime', '#5defff')
        return
      }

      drawInstrument(canvas, renderModel)
      markUpdated()
    }

    void render()
    return () => {
      disposed = true
      mediaLifecycle.dispose()
    }
  }, [descriptor, payload, playback, renderModel, rootPath, texture])

  useEffect(() => () => texture.texture.dispose(), [texture])

  if (!active) return null

  return (
    <group position={geometry.position}>
      {debug ? (
        <mesh rotation={geometry.rotation} renderOrder={9}>
          <planeGeometry args={[geometry.width, geometry.height]} />
          <meshBasicMaterial color="#5defff" wireframe transparent opacity={0.18} depthWrite={false} toneMapped={false} />
        </mesh>
      ) : null}
      <mesh rotation={geometry.rotation} renderOrder={8} userData={{ slotId: zoneId, surfaceKind: 'monitor_aperture_surface' }} onClick={(event) => {
        event.stopPropagation()
        onActivate()
      }}>
        <planeGeometry args={[geometry.width, geometry.height]} />
        <meshBasicMaterial map={texture.texture} transparent opacity={0.98} toneMapped={false} depthWrite />
      </mesh>
    </group>
  )
}
