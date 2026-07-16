// sigil: REPAIR
import { ArrowLeft, MonitorUp, PanelRightOpen } from 'lucide-react'
import { useState } from 'react'
import ModuleCard from '../ModuleCard'
import type { ModuleId } from './types'

interface PanelWorkspaceProps {
  title: string
  subtitle: string
  modules: Array<{ id: ModuleId; title: string; node: React.ReactNode }>
  activeModuleId?: ModuleId | null
  onActiveModuleChange?: (moduleId: ModuleId) => void
  onOpenExternal?: () => void
  onBack?: () => void
}

export default function PanelWorkspace({
  title,
  subtitle,
  modules,
  activeModuleId,
  onActiveModuleChange,
  onOpenExternal,
  onBack,
}: PanelWorkspaceProps) {
  const [localActiveModuleId, setLocalActiveModuleId] = useState<ModuleId>(modules[0]?.id ?? 'section_focus')
  const resolvedActiveModuleId = activeModuleId ?? localActiveModuleId
  const activeModule = modules.find((module) => module.id === resolvedActiveModuleId) ?? modules[0] ?? null

  return (
    <section className="panel-workspace">
      <div className="scene-stage__header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          {onBack && (
            <button type="button" className="refresh-button" onClick={onBack}>
              <ArrowLeft size={16} />
              Back
            </button>
          )}
          <div>
            <div className="scene-stage__eyebrow">Layer 3 / Panel</div>
            <h2 className="scene-stage__title">{title}</h2>
            <p className="panel-workspace__subtitle">{subtitle}</p>
          </div>
        </div>
        {onOpenExternal ? (
          <button type="button" className="refresh-button" onClick={onOpenExternal}>
            <MonitorUp size={16} />
            Pop Out Window
          </button>
        ) : null}
      </div>

      <ModuleCard title="Workspace" eyebrow="Focused module set" accent="cyan">
        <div className="panel-workspace__tabs">
          {modules.map((module) => (
            <button
              type="button"
              className={module.id === activeModule?.id ? 'panel-workspace__tab panel-workspace__tab--active' : 'panel-workspace__tab'}
              key={module.id}
              onClick={() => {
                setLocalActiveModuleId(module.id)
                onActiveModuleChange?.(module.id)
              }}
            >
              <PanelRightOpen size={14} />
              {module.title}
            </button>
          ))}
        </div>
      </ModuleCard>

      {activeModule ? (
        <div className="panel-workspace__focus">
          <div className="panel-workspace__frame">
            <div className="panel-workspace__frame-grid" />
            <div className="panel-workspace__frame-corners panel-workspace__frame-corners--tl" />
            <div className="panel-workspace__frame-corners panel-workspace__frame-corners--br" />
            <div className="panel-workspace__frame-topbar">
              <span>ACTIVE TERMINAL</span>
              <strong>{activeModule.title.toUpperCase()}</strong>
              <span>{activeModule.id}</span>
            </div>
            <div className="panel-workspace__frame-body">
              {activeModule.node}
            </div>
          </div>
        </div>
      ) : null}

      {modules.length > 1 ? (
        <div className="panel-workspace__stack">
          {modules
            .filter((module) => module.id !== activeModule?.id)
            .map((module) => (
              <div key={module.id}>{module.node}</div>
            ))}
        </div>
      ) : null}
    </section>
  )
}
