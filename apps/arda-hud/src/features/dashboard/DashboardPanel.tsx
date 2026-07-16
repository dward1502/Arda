import { useAppActions } from '../../hooks/useAppActions'

const DASHBOARD_URL = 'http://127.0.0.1:9119'

export default function DashboardPanel() {
  const { backToBoardroom } = useAppActions()

  return (
    <div className="dashboard-panel">
      <div className="dashboard-panel__header">
        <div>
          <strong>Hermes Dashboard</strong>
          <p>{DASHBOARD_URL}</p>
        </div>
        <button onClick={backToBoardroom} className="boardroom-button boardroom-button--ghost">
          ← back
        </button>
      </div>
      <iframe
        title="Hermes Dashboard"
        src={DASHBOARD_URL}
        className="dashboard-panel__frame"
        allow="clipboard-read; clipboard-write"
      />
    </div>
  )
}
