import { invoke } from '@tauri-apps/api/core'
import { useCallback, useEffect, useState } from 'react'
import {
  selectableDisplays,
  type DisplayState,
} from './projectionState'

interface HermesDashboardConnection {
  url: string
  launched: boolean
}

export default function App() {
  const [displayState, setDisplayState] = useState<DisplayState | null>(null)
  const [connecting, setConnecting] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refreshDisplays = useCallback(async () => {
    const next = await invoke<DisplayState>('get_display_state')
    setDisplayState(next)
  }, [])

  const connectHermes = useCallback(async () => {
    try {
      const connection = await invoke<HermesDashboardConnection>('ensure_hermes_dashboard')
      if (!connection.url) throw new Error('Hermes did not provide a conversation surface')
      setError(null)
    } catch (cause) {
      setConnecting(false)
      setError(cause instanceof Error ? cause.message : String(cause))
    }
  }, [])

  useEffect(() => {
    void refreshDisplays().catch((cause: unknown) => setError(String(cause)))
    void connectHermes()
    const displayTimer = window.setInterval(() => {
      void refreshDisplays().catch((cause: unknown) => setError(String(cause)))
    }, 2000)
    return () => {
      window.clearInterval(displayTimer)
    }
  }, [connectHermes, refreshDisplays])

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

  const veiled = !displayState?.projected || Boolean(displayState.veil_reason)
  return <main className="mirromere-shell">
    {!veiled && <section className="connection-state">
      <h1>{connecting ? 'Opening Hermes' : 'Hermes is unavailable'}</h1>
      <p>{error ?? 'Loading the live conversation surface…'}</p>
      {!connecting && <button type="button" onClick={() => {
        setConnecting(true)
        void connectHermes()
      }}>Connect</button>}
    </section>}
    {veiled && <section className="veil" data-testid="projection-veil" aria-live="polite">
      <div className="veil-mark" aria-hidden="true">◇</div>
      <p>{displayState?.veil_reason ?? error ?? 'Choose the display where Mirromere should run.'}</p>
      <label htmlFor="display-select">Mirromere display</label>
      <select id="display-select" value="" onChange={event => void selectDisplay(event.target.value)}>
        <option value="" disabled>Select a non-primary display</option>
        {selectableDisplays(displayState).map(display => <option key={display.id} value={display.id}>{display.name} · {display.size[0]}×{display.size[1]}</option>)}
      </select>
      <button type="button" onClick={() => void refreshDisplays()}>Rescan displays</button>
    </section>}
  </main>
}
