import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import {
  createPersonalOpsClient,
  loadConfiguredOperatorId,
  type PersonalOpsClient,
} from '../../lib/personalOps'

interface GlobalRapidCaptureProps {
  client?: Pick<PersonalOpsClient, 'createCapture'>
  operatorId?: string
  onSaved?: (receipt: { event_id: string; capture_id: string }) => void
}

function isRapidCaptureShortcut(event: globalThis.KeyboardEvent): boolean {
  return event.ctrlKey && event.shiftKey && !event.altKey && (event.code === 'Space' || event.key === ' ')
}

export default function GlobalRapidCapture({
  client,
  operatorId,
  onSaved,
}: GlobalRapidCaptureProps) {
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const [open, setOpen] = useState(false)
  const [capture, setCapture] = useState('')
  const [busy, setBusy] = useState(false)
  const [status, setStatus] = useState('Ready to capture')
  const [error, setError] = useState<string | null>(null)
  const [ops, setOps] = useState<Pick<PersonalOpsClient, 'createCapture'> | null>(() => {
    if (client) return client
    const configuredOperatorId = operatorId?.trim()
    return configuredOperatorId ? createPersonalOpsClient(configuredOperatorId) : null
  })

  useEffect(() => {
    if (client) {
      setOps(client)
      return
    }
    const configuredOperatorId = operatorId?.trim()
    if (configuredOperatorId) {
      setOps(createPersonalOpsClient(configuredOperatorId))
      return
    }
    let cancelled = false
    void loadConfiguredOperatorId()
      .then((id) => {
        if (!cancelled) setOps(createPersonalOpsClient(id))
      })
      .catch((reason: unknown) => {
        if (!cancelled) setError(reason instanceof Error ? reason.message : 'Personal operations unavailable')
      })
    return () => {
      cancelled = true
    }
  }, [client, operatorId])

  useEffect(() => {
    const handleKeyDown = (event: globalThis.KeyboardEvent) => {
      if (isRapidCaptureShortcut(event)) {
        event.preventDefault()
        event.stopImmediatePropagation()
        setOpen(true)
        window.requestAnimationFrame(() => inputRef.current?.focus())
        return
      }
      if (open && event.key === 'Escape') {
        event.preventDefault()
        event.stopImmediatePropagation()
        setOpen(false)
      }
    }
    window.addEventListener('keydown', handleKeyDown, true)
    return () => window.removeEventListener('keydown', handleKeyDown, true)
  }, [open])

  const submitCapture = async () => {
    const text = capture.trim()
    if (!text || !ops || busy) return
    setBusy(true)
    setError(null)
    setStatus('Saving capture')
    try {
      const receipt = await ops.createCapture(text)
      setCapture('')
      setStatus(`Capture saved durably · ${receipt.capture_id}`)
      onSaved?.(receipt)
      window.dispatchEvent(new CustomEvent('personal-ops-capture-saved', { detail: receipt }))
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Capture failed')
      setStatus('Capture not saved')
    } finally {
      setBusy(false)
    }
  }

  const handleInputKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== 'Enter' || event.shiftKey) return
    event.preventDefault()
    void submitCapture()
  }

  if (!open) return null

  return (
    <div className="global-rapid-capture" role="presentation">
      <section
        className="global-rapid-capture__dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="global-rapid-capture-title"
      >
        <header className="global-rapid-capture__header">
          <div>
            <span className="global-rapid-capture__eyebrow">Personal Operations</span>
            <h2 id="global-rapid-capture-title">Rapid capture</h2>
          </div>
          <button type="button" onClick={() => setOpen(false)} aria-label="Close rapid capture">×</button>
        </header>
        <label htmlFor="global-rapid-capture-input">Capture a thought</label>
        <textarea
          ref={inputRef}
          id="global-rapid-capture-input"
          value={capture}
          disabled={busy}
          placeholder="Capture now; classify it later"
          onChange={(event) => setCapture(event.target.value)}
          onKeyDown={handleInputKeyDown}
        />
        <button
          className="global-rapid-capture__save"
          type="button"
          disabled={busy || !ops || capture.trim().length === 0}
          onClick={() => void submitCapture()}
        >
          {busy ? 'Saving…' : 'Save capture'}
        </button>
        <div className="global-rapid-capture__status" role="status" aria-live="polite">{status}</div>
        {error ? <div className="global-rapid-capture__error" role="alert">{error}</div> : null}
        <small><kbd>Enter</kbd> saves · <kbd>Shift+Enter</kbd> adds a line · <kbd>Esc</kbd> closes</small>
        <small><kbd>Ctrl+Shift+Space</kbd> opens Rapid capture from anywhere in the HUD.</small>
      </section>
    </div>
  )
}
