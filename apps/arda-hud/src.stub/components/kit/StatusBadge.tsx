// sigil: REPAIR
type BadgeState = 'nominal' | 'warning' | 'critical' | 'info'

interface StatusBadgeProps {
  state: BadgeState
  label: string
  className?: string
}

const stateStyles: Record<BadgeState, string> = {
  nominal: 'border-[#00ff9f66] text-[#00ff9f] bg-[#00ff9f1a]',
  warning: 'border-[#f9ca2466] text-[#f9ca24] bg-[#f9ca241a]',
  critical: 'border-[#ff333366] text-[#ff3333] bg-[#ff33331a]',
  info: 'border-[#00d4ff66] text-[#00d4ff] bg-[#00d4ff1a]',
}

export default function StatusBadge({ state, label, className = '' }: StatusBadgeProps) {
  return (
    <span
      className={`inline-flex items-center rounded border px-2 py-1 font-['Share_Tech_Mono'] text-[10px] uppercase tracking-[0.14em] ${stateStyles[state]} ${className}`}
    >
      {label}
    </span>
  )
}
