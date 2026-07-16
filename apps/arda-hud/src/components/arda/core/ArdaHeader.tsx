// sigil: REPAIR
import { Clock3, RefreshCw, Settings2, WandSparkles, Zap } from 'lucide-react'
import ThemeSwitcher from '../ThemeSwitcher'
import type { ThemeId, ThemeOption } from './types'

interface ArdaHeaderProps {
  themes: ThemeOption[]
  theme: ThemeId
  editMode: boolean
  viewMode: 'boardroom' | 'world' | 'panel'
  currentTime: string
  generatedAt: string
  primaryEntrypoint: string
  systemStatus: string
  powerLevel: string
  toolCount: string
  attentionCount: string
  onThemeChange: (theme: ThemeId) => void
  onToggleEditMode: () => void
  onOpenSettings: () => void
  onRefresh: () => void
}

export default function ArdaHeader({
  themes,
  theme,
  editMode,
  viewMode,
  currentTime,
  generatedAt,
  primaryEntrypoint,
  systemStatus,
  powerLevel,
  toolCount,
  attentionCount,
  onThemeChange,
  onToggleEditMode,
  onOpenSettings,
  onRefresh,
}: ArdaHeaderProps) {
  return (
    <header className="arda-header">
      <div className="arda-header__identity">
        <div className="arda-header__eyebrow">Arda HUD / Sovereign Interface</div>
        <h1 className="arda-header__title">ARDA Command Realm</h1>
        <p className="arda-header__subtitle">
          Boardroom command, world traversal, and focused panels routed from the core-state contract.
        </p>
      </div>

      <div className="arda-header__instrument-grid">
        <div className="arda-instrument arda-instrument--time">
          <span className="arda-instrument__label">
            <Clock3 size={14} />
            Local Time
          </span>
          <strong className="arda-instrument__value">{currentTime}</strong>
          <span className="arda-instrument__meta">Last refresh {generatedAt}</span>
        </div>

        <div className="arda-instrument arda-instrument--power">
          <span className="arda-instrument__label">
            <Zap size={14} />
            Reactor / Power Bus
          </span>
          <strong className="arda-instrument__value">{powerLevel}</strong>
          <span className="arda-instrument__meta">{systemStatus}</span>
        </div>

        <div className="arda-instrument arda-instrument--routing">
          <span className="arda-instrument__label">Primary Entrypoint</span>
          <strong className="arda-instrument__value arda-instrument__value--path">{primaryEntrypoint}</strong>
          <span className="arda-instrument__meta">
            View {viewMode} / {toolCount} tools / {attentionCount} flags
          </span>
        </div>
      </div>

      <div className="arda-header__controls">
        <ThemeSwitcher themes={themes} activeTheme={theme} onChange={(nextTheme) => onThemeChange(nextTheme as ThemeId)} />
        <div className="arda-header__control-row">
          <button type="button" className="refresh-button" onClick={onOpenSettings}>
            <Settings2 size={16} />
            Settings
          </button>
          <button
            type="button"
            className={editMode ? 'refresh-button refresh-button--active' : 'refresh-button'}
            onClick={onToggleEditMode}
          >
            <WandSparkles size={16} />
            {editMode ? 'Exit Edit Mode' : 'Edit Mode'}
          </button>
          <button type="button" className="refresh-button" onClick={onRefresh}>
            <RefreshCw size={16} />
            Refresh
          </button>
        </div>
      </div>
    </header>
  )
}
