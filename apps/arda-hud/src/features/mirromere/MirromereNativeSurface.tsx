import { useEffect, useRef } from 'react'
import { shouldDrawInstrumentFrame } from '../../scene/boardroom/instrumentFrameCadence'
import {
  deriveMirromereVisualModel,
  drawMirromereFrame,
  resolveMirromereMotion,
} from './MirromereAperture'
import type { MirromereSurface } from './types'

interface MirromereNativeSurfaceProps {
  surface: MirromereSurface | null
  loading: boolean
  error: string | null
  motionEnabled?: boolean
}

export default function MirromereNativeSurface({
  surface,
  loading,
  error,
  motionEnabled = true,
}: MirromereNativeSurfaceProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || !surface) return
    const prefersReduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
    const animate = resolveMirromereMotion(surface, motionEnabled, prefersReduced)
    const startedAt = performance.now()
    let frame = 0
    let disposed = false
    let lastDrawAt = Number.NEGATIVE_INFINITY
    const draw = (now: number) => {
      if (disposed) return
      const scale = Math.min(window.devicePixelRatio || 1, 2)
      const width = Math.max(1, Math.round((canvas.clientWidth || window.innerWidth) * scale))
      const height = Math.max(1, Math.round((canvas.clientHeight || window.innerHeight) * scale))
      if (canvas.width !== width) canvas.width = width
      if (canvas.height !== height) canvas.height = height
      if (!animate || shouldDrawInstrumentFrame(now, lastDrawAt)) {
        drawMirromereFrame(canvas, surface, (now - startedAt) / 1000, animate)
        lastDrawAt = now
      }
      if (animate) frame = requestAnimationFrame(draw)
    }
    draw(startedAt)
    const redraw = () => draw(performance.now())
    window.addEventListener('resize', redraw)
    return () => {
      disposed = true
      if (frame) cancelAnimationFrame(frame)
      window.removeEventListener('resize', redraw)
    }
  }, [motionEnabled, surface])

  if (!surface) {
    return (
      <main className="mirromere-native mirromere-native--unavailable" role="status">
        <strong>Mirromere unavailable</strong>
        <span>{loading ? 'Awaiting governed surface projection.' : error ?? 'Selected display unavailable.'}</span>
      </main>
    )
  }

  const model = deriveMirromereVisualModel(surface)
  return (
    <main
      className={`mirromere-native mirromere-native--${model.truthState}`}
      role="status"
      aria-label={surface.accessibility.description}
      data-scene-id={surface.scene.scene_id}
      data-truth-state={model.truthState}
    >
      <canvas ref={canvasRef} className="mirromere-native__canvas" aria-hidden="true" />
      <span className="mirromere-native__semantic">
        {model.truthState === 'veiled' ? 'Privacy veil active' : surface.accessibility.description}
      </span>
    </main>
  )
}
