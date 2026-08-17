import { useCallback, useEffect, useState } from 'react'
import {
  invokeLaunchNativeHud,
  invokeLifecycleStatus,
  invokeRecoverComponent,
  invokeStartSession,
  lifecyclePrimaryAction,
  lifecycleRows,
  type LifecycleSnapshot,
} from '../lib/lifecycle'

export default function LifecyclePanel() {
  const [snapshot, setSnapshot] = useState<LifecycleSnapshot | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [closeAfterHud, setCloseAfterHud] = useState(() => localStorage.getItem('arda.closeAfterHud') === 'true')

  const refresh = useCallback(async () => {
    try { setSnapshot(await invokeLifecycleStatus()); setError(null) }
    catch (cause) { setError(`Lifecycle status unavailable: ${cause}`) }
  }, [])

  useEffect(() => { void refresh() }, [refresh])

  const act = async () => {
    if (!snapshot) return
    const action = lifecyclePrimaryAction(snapshot)
    if (action.kind === 'inspect') { setError('Inspect the required component evidence below.'); return }
    setBusy(true)
    try {
      if (action.kind === 'start') await invokeStartSession()
      if (action.kind === 'retry') {
        const recovery = snapshot.components.find(component => component.class === 'required' && component.recovery_action)?.recovery_action
        if (recovery) await invokeRecoverComponent(recovery)
      }
      if (action.kind === 'open_hud') {
        await invokeLaunchNativeHud()
        if (closeAfterHud) {
          const { getCurrentWindow } = await import('@tauri-apps/api/window')
          await getCurrentWindow().close()
          return
        }
      }
      await refresh()
    } catch (cause) { setError(`Lifecycle command failed: ${cause}`) }
    finally { setBusy(false) }
  }

  if (!snapshot) return <aside className="absolute left-6 top-6 z-40 font-mono text-xs text-white/70">{error ?? 'LIFECYCLE UNKNOWN'}</aside>
  const action = lifecyclePrimaryAction(snapshot)
  return (
    <aside aria-label="Arda lifecycle" className="absolute left-6 top-6 z-40 w-80 border-l border-white/20 pl-4 font-mono text-xs text-white/70">
      <div className="mb-3 tracking-[0.25em] text-white/90">{snapshot.aggregate_state.toUpperCase()}</div>
      {lifecycleRows(snapshot).map(row => (
        <div key={row.id} className="mb-2 grid grid-cols-[1fr_auto] gap-x-3">
          <span>{row.id}</span><span>{row.process}/{row.health}</span>
          <span className="text-white/40">{row.freshness}</span><span className="text-white/40">{row.recovery ?? 'inspect'}</span>
        </div>
      ))}
      {error && <p role="alert" className="my-2 text-amber-200">{error}</p>}
      <button disabled={busy} onClick={act} className="mt-2 border border-white/30 px-3 py-1 hover:bg-white/5 disabled:opacity-50">{busy ? 'WAIT' : action.label}</button>
      <label className="mt-3 flex gap-2 text-white/40">
        <input type="checkbox" checked={closeAfterHud} onChange={event => { setCloseAfterHud(event.target.checked); localStorage.setItem('arda.closeAfterHud', String(event.target.checked)) }} />
        close after HUD opens
      </label>
    </aside>
  )
}
