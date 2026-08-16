import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
  type MouseEvent,
  type WheelEvent,
} from 'react'
import { Canvas } from '@react-three/fiber'
import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'
import { agentGetMonitorSurfaceRegistry } from '../../lib/boardroomSlotSettings'
import { coerceRuntimeMonitorRegistry } from '../../lib/monitorSurfaceRegistryBridge'
import {
  agentClickBrowserMonitorSession,
  agentGetBrowserMonitorFrame,
  agentGetBrowserMonitorSession,
  agentKeyBrowserMonitorSession,
  agentNavigateBrowserMonitorSession,
  agentScrollBrowserMonitorSession,
  agentTypeBrowserMonitorSession,
  type BrowserCaptureDescriptor,
} from '../../lib/browserMonitorSession'
import { BoardroomApertureSurface } from './BoardroomApertureSurface'
import {
  mapWorkstationPointerToCapture,
  normalizeBrowserAddress,
} from './browserMonitorWorkstationModel'

interface MonitorSessionWorkstationProps {
  sessionId: string
  record: MonitorSurfaceSessionRecord | null
  rootPath: string | null
}

function BrowserSessionWorkstation({ sessionId }: { sessionId: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const captureRef = useRef<BrowserCaptureDescriptor | null>(null)
  const inputQueueRef = useRef<Promise<void>>(Promise.resolve())
  const [capture, setCapture] = useState<BrowserCaptureDescriptor | null>(null)
  const [address, setAddress] = useState('')
  const [status, setStatus] = useState('Connecting to live browser session…')
  const [busy, setBusy] = useState(false)

  const setLiveCapture = (descriptor: BrowserCaptureDescriptor) => {
    captureRef.current = descriptor
    setCapture(descriptor)
  }

  const queueInput = (
    operation: (current: BrowserCaptureDescriptor) => Promise<BrowserCaptureDescriptor>,
    success: string,
  ) => {
    inputQueueRef.current = inputQueueRef.current
      .then(async () => {
        const current = captureRef.current
        if (!current) return
        const next = await operation(current)
        setLiveCapture(next)
        setStatus(success)
      })
      .catch((error) => setStatus(`Browser input failed: ${String(error)}`))
  }

  useEffect(() => {
    let disposed = false
    let animationFrame = 0
    let revision = 0
    const image = new Image()

    const schedule = () => {
      if (!disposed) animationFrame = requestAnimationFrame(() => void poll())
    }
    const poll = async () => {
      try {
        const frame = await agentGetBrowserMonitorFrame(sessionId, revision)
        if (disposed) return
        if (!frame) {
          schedule()
          return
        }
        revision = frame.revision
        image.onload = () => {
          if (disposed) return
          const canvas = canvasRef.current
          const context = canvas?.getContext('2d')
          if (canvas && context) {
            context.clearRect(0, 0, canvas.width, canvas.height)
            context.drawImage(image, 0, 0, canvas.width, canvas.height)
          }
          schedule()
        }
        image.onerror = () => {
          if (!disposed) setStatus('Live browser frame could not be decoded')
          schedule()
        }
        image.src = `data:image/jpeg;base64,${frame.jpegBase64}`
      } catch (error) {
        if (!disposed) setStatus(`Browser stream unavailable: ${String(error)}`)
      }
    }

    void agentGetBrowserMonitorSession(sessionId)
      .then((descriptor) => {
        if (disposed) return
        setLiveCapture(descriptor)
        setAddress(descriptor.url)
        setStatus('Live · click the page or enter an address')
        void poll()
      })
      .catch((error) => {
        if (!disposed) setStatus(`Browser session unavailable: ${String(error)}`)
      })

    return () => {
      disposed = true
      cancelAnimationFrame(animationFrame)
      image.src = ''
    }
  }, [sessionId])

  const navigate = async (event: FormEvent) => {
    event.preventDefault()
    if (!captureRef.current || busy) return
    setBusy(true)
    try {
      await inputQueueRef.current
      const current = captureRef.current
      if (!current) return
      const next = await agentNavigateBrowserMonitorSession(current, normalizeBrowserAddress(address))
      setLiveCapture(next)
      setAddress(next.url)
      setStatus(`Navigated · ${next.url}`)
    } catch (error) {
      setStatus(`Navigation failed: ${String(error)}`)
    } finally {
      setBusy(false)
    }
  }

  const clickPage = (event: MouseEvent<HTMLCanvasElement>) => {
    if (!captureRef.current || busy) return
    const position = mapWorkstationPointerToCapture({
      clientX: event.clientX,
      clientY: event.clientY,
      bounds: event.currentTarget.getBoundingClientRect(),
    })
    if (!position) return
    event.currentTarget.focus()
    queueInput(
      (current) => agentClickBrowserMonitorSession(current, position),
      `Live · clicked ${Math.round(position.x)}, ${Math.round(position.y)}`,
    )
  }

  const scrollPage = (event: WheelEvent<HTMLCanvasElement>) => {
    if (!captureRef.current || busy) return
    const position = mapWorkstationPointerToCapture({
      clientX: event.clientX,
      clientY: event.clientY,
      bounds: event.currentTarget.getBoundingClientRect(),
    })
    if (!position) return
    event.preventDefault()
    queueInput(
      (current) => agentScrollBrowserMonitorSession(current, {
        ...position,
        deltaX: event.deltaX,
        deltaY: event.deltaY,
      }),
      'Live · scrolled browser page',
    )
  }

  const keyPage = (event: KeyboardEvent<HTMLCanvasElement>) => {
    if (!captureRef.current || busy || event.ctrlKey || event.metaKey || event.altKey) return
    if (event.key.length === 1) {
      event.preventDefault()
      queueInput(
        (current) => agentTypeBrowserMonitorSession(current, event.key),
        'Live · text entered in browser page',
      )
      return
    }
    if ([
      'Enter', 'Backspace', 'Tab', 'Escape', 'ArrowUp', 'ArrowDown', 'ArrowLeft',
      'ArrowRight', 'Delete', 'Home', 'End', 'PageUp', 'PageDown',
    ].includes(event.key)) {
      event.preventDefault()
      queueInput(
        (current) => agentKeyBrowserMonitorSession(current, event.key),
        `Live · ${event.key} delivered to browser page`,
      )
    }
  }

  return (
    <div className="browser-session-workstation">
      <form className="browser-session-workstation__chrome" onSubmit={(event) => void navigate(event)}>
        <span className="browser-session-workstation__live" aria-label="Live browser status">LIVE</span>
        <label htmlFor="browser-session-address">Address</label>
        <input
          id="browser-session-address"
          value={address}
          onChange={(event) => setAddress(event.target.value)}
          spellCheck={false}
          aria-label="Browser address"
        />
        <button type="submit" disabled={!capture || busy}>Go</button>
      </form>
      <div className="browser-session-workstation__status" aria-live="polite">{status}</div>
      <div className="browser-session-workstation__viewport">
        <canvas
          ref={canvasRef}
          width={1280}
          height={720}
          tabIndex={0}
          aria-label="Interactive live browser page"
          onClick={clickPage}
          onWheel={scrollPage}
          onKeyDown={keyPage}
        />
      </div>
    </div>
  )
}

export default function MonitorSessionWorkstation({ sessionId, record, rootPath }: MonitorSessionWorkstationProps) {
  const [hydratedRecord, setHydratedRecord] = useState(record)

  useEffect(() => {
    if (record) {
      setHydratedRecord(record)
      return
    }
    let cancelled = false
    void agentGetMonitorSurfaceRegistry()
      .then((value) => {
        if (cancelled) return
        const registry = coerceRuntimeMonitorRegistry(value)
        setHydratedRecord(Object.values(registry?.sessions ?? {})
          .find((session) => session.surface_session_id === sessionId) ?? null)
      })
      .catch(() => undefined)
    return () => { cancelled = true }
  }, [record, sessionId])

  if (!hydratedRecord) {
    return (
      <main className="monitor-session-workstation monitor-session-workstation--unavailable">
        <h1>Monitor session unavailable</h1>
        <p>{sessionId}</p>
        <p>The session was released, expired, or is not present in the authoritative registry.</p>
      </main>
    )
  }

  const title = 'title' in hydratedRecord.content && typeof hydratedRecord.content.title === 'string'
    ? hydratedRecord.content.title
    : `${hydratedRecord.owner} monitor session`

  return (
    <main className="monitor-session-workstation" data-session-id={hydratedRecord.surface_session_id}>
      <header className="monitor-session-workstation__header">
        <div>
          <span className="monitor-session-workstation__eyebrow">{hydratedRecord.slot_id}</span>
          <h1>{title}</h1>
        </div>
        <div className="monitor-session-workstation__ownership" aria-label="Session ownership">
          <strong>{hydratedRecord.owner} · r{hydratedRecord.revision}</strong>
          <span>{hydratedRecord.surface_session_id}</span>
        </div>
      </header>
      <section className="monitor-session-workstation__surface" aria-label="Live monitor session content">
        {hydratedRecord.content.kind === 'remote_session' && hydratedRecord.content.transport === 'mjpeg' ? (
          <BrowserSessionWorkstation sessionId={hydratedRecord.content.sessionId} />
        ) : (
        <Canvas orthographic camera={{ position: [0, 0, 10], zoom: 70 }} dpr={[1, 1.5]}>
          <ambientLight intensity={1.2} />
          <BoardroomApertureSurface
            zoneId={hydratedRecord.slot_id}
            previewMode="monitor_surface"
            size={[12, 7.2, 1]}
            model={{
              title,
              eyebrow: `${hydratedRecord.slot_id} · ${hydratedRecord.owner}`,
              status: 'nominal',
              tone: 'cyan',
              glyph: '◇',
              preset: 'standby',
              nodes: [],
              links: [],
              rings: [],
              source: {
                freshness: 'fresh',
                sourceId: hydratedRecord.surface_session_id,
                sourceLabel: hydratedRecord.owner,
                sourcePaths: [],
                observedAtUtc: hydratedRecord.updated_at_utc,
                sourceKind: 'live',
                truthState: 'live',
              },
            }}
            descriptor={hydratedRecord.content}
            playback={hydratedRecord.playback}
            rootPath={rootPath}
            motionEnabled
            active
            onActivate={() => undefined}
          />
        </Canvas>
        )}
      </section>
    </main>
  )
}
