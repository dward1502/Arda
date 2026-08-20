import { invoke } from '@tauri-apps/api/core'
import { useCallback, useEffect, useRef, useState } from 'react'
import {
  drawMirromereFrame,
  isMirromereInspectAllowed,
  parseMirromereSurface,
  requestMirromereInteraction,
  resolveMirromereMotion,
  type MirromereSurface,
} from '@arda/mirromere-ui'
import {
  isProjectionVeiled,
  selectableDisplays,
  type DisplayState,
} from './projectionState'

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [surface, setSurface] = useState<MirromereSurface | null>(null)
  const [displayState, setDisplayState] = useState<DisplayState | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(
    () => window.matchMedia('(prefers-reduced-motion: reduce)').matches,
  )

  const refreshDisplays = useCallback(async () => {
    const next = await invoke<DisplayState>('get_display_state')
    setDisplayState(next)
  }, [])

  const refreshSurface = useCallback(async () => {
    try {
      const payload = await invoke<unknown>('get_mirromere_surface', { displayRole: 'native_outpost' })
      setSurface(parseMirromereSurface(payload))
      setError(null)
    } catch (cause) {
      setSurface(null)
      setError(cause instanceof Error ? cause.message : String(cause))
    }
  }, [])

  useEffect(() => {
    void refreshDisplays().catch((cause: unknown) => setError(String(cause)))
    void refreshSurface()
    const surfaceTimer = window.setInterval(refreshSurface, 5000)
    const displayTimer = window.setInterval(() => {
      void refreshDisplays().catch((cause: unknown) => setError(String(cause)))
    }, 2000)
    return () => {
      window.clearInterval(surfaceTimer)
      window.clearInterval(displayTimer)
    }
  }, [refreshDisplays, refreshSurface])

  useEffect(() => {
    const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)')
    const handleMotionPreference = () => setPrefersReducedMotion(motionPreference.matches)
    motionPreference.addEventListener('change', handleMotionPreference)
    return () => motionPreference.removeEventListener('change', handleMotionPreference)
  }, [])

  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      void import('@tauri-apps/api/window')
        .then(({ getCurrentWindow }) => getCurrentWindow().close())
        .catch((cause: unknown) => setError(String(cause)))
    }
    window.addEventListener('keydown', closeOnEscape)
    return () => window.removeEventListener('keydown', closeOnEscape)
  }, [])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || !surface || !displayState?.projected) return
    let frame = 0
    let start = performance.now()
    const render = (now: number) => {
      const rect = canvas.getBoundingClientRect()
      const scale = window.devicePixelRatio || 1
      canvas.width = Math.max(1, Math.floor(rect.width * scale))
      canvas.height = Math.max(1, Math.floor(rect.height * scale))
      drawMirromereFrame(canvas, surface, (now - start) / 1000, resolveMirromereMotion(surface, true, prefersReducedMotion))
      frame = requestAnimationFrame(render)
    }
    frame = requestAnimationFrame(render)
    return () => cancelAnimationFrame(frame)
  }, [displayState?.projected, prefersReducedMotion, surface])

  const selectDisplay = async (displayId: string) => {
    try {
      const next = await invoke<DisplayState>('select_mirromere_display', { displayId })
      setDisplayState(next)
      setError(null)
    } catch (cause) {
      setError(String(cause))
      await refreshDisplays()
    }
  }

  const inspect = async () => {
    if (!surface || !isMirromereInspectAllowed(surface)) return
    try {
      await requestMirromereInteraction(surface, 'inspect_provenance', true, invoke)
    } catch (cause) {
      setError(String(cause))
    }
  }

  const veiled = isProjectionVeiled(displayState, surface)
  const projectionLabel = surface
    ? `Mirromere ${surface.scene.scene_id}; ${surface.accessibility.description}; freshness ${surface.freshness}; availability ${surface.availability}; motion ${resolveMirromereMotion(surface, true, prefersReducedMotion) ? 'animated' : 'reduced'}`
    : 'Mirromere ambient projection unavailable'
  return <main className="mirromere-shell">
    <canvas ref={canvasRef} aria-label={projectionLabel} onDoubleClick={() => void inspect()} />
    {veiled && <section className="veil" data-testid="projection-veil" aria-live="polite">
      <div className="veil-mark" aria-hidden="true">◇</div>
      <p>{displayState?.veil_reason ?? error ?? 'Awaiting runtime projection'}</p>
      <label htmlFor="display-select">Projection display</label>
      <select id="display-select" value="" onChange={event => void selectDisplay(event.target.value)}>
        <option value="" disabled>Select a non-primary display</option>
        {selectableDisplays(displayState).map(display => <option key={display.id} value={display.id}>{display.name} · {display.size[0]}×{display.size[1]}</option>)}
      </select>
      <button type="button" onClick={() => void refreshDisplays()}>Rescan displays</button>
    </section>}
    {!veiled && <aside className="source-badge">RUNTIME · {surface?.freshness}</aside>}
  </main>
}
