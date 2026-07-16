import { useAppActions } from '../../hooks/useAppActions'

export default function TopBar() {
  const { state, openHermesDashboard, backToBoardroom } = useAppActions()
  const isDashboard = state.view === 'dashboard'

  return (
    <div className="flex items-center justify-between border-b border-white/10 pb-3">
      <div className="flex items-center gap-3">
        <h1 className="text-gold-bright text-lg tracking-widest">ARDA · HUD</h1>
        <span className="text-xs text-white/40">boardroom shell</span>
      </div>
      <div className="flex items-center gap-2">
        {isDashboard ? (
          <button
            onClick={backToBoardroom}
            className="px-3 py-1.5 text-xs border border-white/30 text-white/70 hover:bg-white/5 rounded"
          >
            ← BACK
          </button>
        ) : (
          <button
            onClick={openHermesDashboard}
            className="px-3 py-1.5 text-xs border border-gold/60 text-gold hover:bg-gold/10 rounded"
          >
            HERMES DASHBOARD
          </button>
        )}
        <span className="text-xs text-white/40">{isDashboard ? 'dashboard' : 'boardroom'}</span>
      </div>
    </div>
  )
}
