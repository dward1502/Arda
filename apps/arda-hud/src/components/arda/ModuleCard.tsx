// sigil: REPAIR
import type { ReactNode } from 'react'

interface ModuleCardProps {
  title: string
  eyebrow?: string
  marker?: string
  accent?: 'gold' | 'cyan' | 'ember' | 'mint' | 'violet'
  tag?: string
  actions?: ReactNode
  className?: string
  children?: ReactNode
}

export default function ModuleCard({
  title,
  eyebrow,
  marker,
  accent = 'cyan',
  actions,
  className = '',
  children,
  tag,
}: ModuleCardProps) {
  return (
    <section className={`module-card module-card--${accent} ${className}`.trim()}>
      <header className="module-card__header">
        <div>
          {eyebrow ? <div className="module-card__eyebrow">{eyebrow}</div> : null}
          <div className="flex items-center gap-2">
            {marker ? (
              <span className="inline-flex h-6 w-6 items-center justify-center rounded-full border border-current/20 bg-white/5 text-sm text-current/90">
                {marker}
              </span>
            ) : null}
            <h2 className="module-card__title">{title}</h2>
          </div>
          {tag ? <span className="module-card__tag">{tag}</span> : null}
        </div>
        {actions ? <div className="module-card__actions">{actions}</div> : null}
      </header>
      <div className="module-card__body">{children}</div>
    </section>
  )
}
