// sigil: REPAIR
import type { ReactNode } from 'react'

interface SceneRuntimeCardMetric {
  label: string
  value: string | number
}

interface SceneRuntimeCardAction {
  label: string
  onClick: () => void
}

interface SceneRuntimeCardProps {
  eyebrow: string
  title: string
  metrics: SceneRuntimeCardMetric[]
  actions?: SceneRuntimeCardAction[]
  variant?: 'boardroom' | 'world'
  children?: ReactNode
}

export default function SceneRuntimeCard({
  eyebrow,
  title,
  metrics,
  actions = [],
  variant = 'boardroom',
  children,
}: SceneRuntimeCardProps) {
  return (
    <div className={`scene-runtime-card scene-runtime-card--${variant}`}>
      <div className="scene-runtime-card__eyebrow">{eyebrow}</div>
      <div className="scene-runtime-card__title">{title}</div>
      <div className="scene-runtime-card__metrics">
        {metrics.map((metric) => (
          <div className="scene-runtime-card__metric" key={metric.label}>
            <span>{metric.label}</span>
            <strong>{metric.value}</strong>
          </div>
        ))}
      </div>
      {children ? <div className="scene-runtime-card__body">{children}</div> : null}
      {actions.length > 0 ? (
        <div className="scene-runtime-card__actions">
          {actions.map((action) => (
            <button
              type="button"
              className="scene-runtime-card__action"
              key={action.label}
              onClick={action.onClick}
            >
              {action.label}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  )
}
