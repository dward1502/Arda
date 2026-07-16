// sigil: REPAIR
import type { ReactNode } from 'react'

type AlertVariant = 'info' | 'warning' | 'error' | 'success'

interface AlertBannerProps {
  variant: AlertVariant
  icon?: ReactNode
  children: ReactNode
  className?: string
}

const variantStyles: Record<AlertVariant, string> = {
  info: 'border-l-[#9b59b6] bg-[#9b59b611] text-[#e056fd]',
  warning: 'border-l-[#ff6b35] bg-[#ff6b3522] text-[#ff6b35]',
  error: 'border-l-[#ff3333] bg-[#ff333322] text-[#ff3333] animate-pulse',
  success: 'border-l-[#f9ca24] bg-[#f9ca2422] text-[#f9ca24]',
}

export default function AlertBanner({
  variant,
  icon,
  children,
  className = '',
}: AlertBannerProps) {
  return (
    <div
      className={`flex items-center gap-3 border-l-[3px] px-4 py-2.5 font-['Share_Tech_Mono'] text-xs ${variantStyles[variant]} ${className}`}
    >
      {icon && <span className="shrink-0 text-base">{icon}</span>}
      <span>{children}</span>
    </div>
  )
}
