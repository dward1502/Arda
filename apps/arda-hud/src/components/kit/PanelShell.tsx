// sigil: REPAIR
import type { ReactNode } from 'react'

type PanelVariant = 'default' | 'alert' | 'gold'

interface PanelShellProps {
  children: ReactNode
  title?: string
  tag?: string
  variant?: PanelVariant
  className?: string
}

const variantStyles: Record<PanelVariant, string> = {
  default: 'border-[#9b59b666] text-[#e8e8f0]',
  alert: 'border-[#ff333366] text-[#ffe5e5]',
  gold: 'border-[#f9ca2466] text-[#fff8df]',
}

export default function PanelShell({
  children,
  title,
  tag,
  variant = 'default',
  className = '',
}: PanelShellProps) {
  return (
    <section
      className={`relative overflow-hidden rounded border bg-[#0f0018]/70 p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_10px_30px_rgba(0,0,0,0.35)] backdrop-blur-md ${variantStyles[variant]} ${className}`}
    >
      <div className="pointer-events-none absolute left-0 top-0 h-4 w-4 border-l-2 border-t-2 border-[#ff6b35]" />
      <div className="pointer-events-none absolute bottom-0 right-0 h-4 w-4 border-b-2 border-r-2 border-[#ff6b35]" />

      {(title || tag) && (
        <header className="mb-3 flex items-center gap-2">
          {title && (
            <h3 className="font-['Orbitron'] text-[10px] uppercase tracking-[0.24em] text-[#ff6b35]">
              {title}
            </h3>
          )}
          {tag && (
            <span className="ml-auto font-['Share_Tech_Mono'] text-[9px] uppercase tracking-[0.18em] text-[#9b59b6]">
              {tag}
            </span>
          )}
        </header>
      )}

      {children}
    </section>
  )
}
