import { useState, useEffect } from 'react'

// The HUD's only job: surface the live `manwe` gateway (port 7171, reserved
// workspace-wide) so an operator can see what models are available locally.
// It talks OpenAI-compatible REST — same contract the daemon-supervised
// `manwe` already serves.

type Model = { id: string; object?: string; owned_by?: string }

function gatewayUrl(): string {
  // Mirrors the Rust `gateway_base_url` command. Hardcoded here for the
  // standalone build; when run inside Tauri we could call the command instead.
  return 'http://127.0.0.1:7171'
}

export default function App() {
  const [models, setModels] = useState<Model[]>([])
  const [status, setStatus] = useState<'idle' | 'loading' | 'ok' | 'error'>('idle')
  const [error, setError] = useState<string>('')

  const refresh = async () => {
    setStatus('loading')
    setError('')
    try {
      const res = await fetch(`${gatewayUrl()}/v1/models`, { cache: 'no-store' })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = (await res.json()) as { data?: Model[] }
      setModels(data.data ?? [])
      setStatus('ok')
    } catch (e) {
      setStatus('error')
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  useEffect(() => {
    refresh()
  }, [])

  return (
    <div className="w-screen h-screen bg-cosmic text-white/80 font-mono p-6 flex flex-col gap-4">
      <header className="flex items-baseline justify-between border-b border-white/10 pb-3">
        <h1 className="text-gold-bright text-lg tracking-widest">ARDA · HUD</h1>
        <span className="text-xs text-white/40">manwe gateway @ :7171</span>
      </header>

      <div className="flex items-center gap-3">
        <button
          onClick={refresh}
          className="px-4 py-1.5 text-xs border border-white/30 text-white/70 hover:bg-white/5 rounded"
        >
          REFRESH
        </button>
        <span className="text-xs">
          {status === 'loading' && <span className="text-gold">probing gateway…</span>}
          {status === 'ok' && <span className="text-emerald-400">connected · {models.length} model(s)</span>}
          {status === 'error' && <span className="text-red-400">error: {error}</span>}
          {status === 'idle' && <span className="text-white/40">idle</span>}
        </span>
      </div>

      <ul className="flex-1 overflow-auto divide-y divide-white/5">
        {models.map((m) => (
          <li key={m.id} className="py-2 flex items-center justify-between">
            <span className="text-white/90">{m.id}</span>
            <span className="text-xs text-white/40">{m.owned_by ?? 'local'}</span>
          </li>
        ))}
        {status === 'ok' && models.length === 0 && (
          <li className="py-2 text-white/40 text-xs">gateway reachable, no models reported</li>
        )}
      </ul>
    </div>
  )
}
