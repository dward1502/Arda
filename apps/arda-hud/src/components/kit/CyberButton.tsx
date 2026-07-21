// sigil: REPAIR
import type { ReactNode } from 'react'

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'alert' | 'icon'

interface CyberButtonProps {
  children: ReactNode
  variant?: ButtonVariant
  disabled?: boolean
  onClick?: () => void
  className?: string
}

const ANGLED_CLIP = 'polygon(8px 0%, 100% 0%, calc(100% - 8px) 100%, 0% 100%)'

const variantStyles: Record<ButtonVariant, string> = {
  primary:
    'bg-[#ff6b35] text-[#0a0010] hover:bg-[#e056fd] hover:shadow-[0_0_20px_rgba(224,86,253,0.27)]',
  secondary:
    'bg-transparent text-[#ff6b35] border border-[#ff6b3566] hover:bg-[#ff6b3522] hover:border-[#ff6b35] hover:shadow-[0_0_16px_rgba(255,107,53,0.4)]',
  ghost:
    'bg-transparent text-[#9b59b6] border border-[#9b59b666] hover:bg-[#9b59b611] hover:text-[#e056fd] hover:border-[#e056fd44]',
  alert:
    'bg-transparent text-[#ff3333] border border-[#ff3333] animate-pulse',
  icon:
    'bg-transparent text-[#ff6b35] border border-[#ff6b3566] hover:bg-[#ff6b3522] hover:shadow-[0_0_12px_rgba(255,107,53,0.4)]',
}

const variantClipPath: Record<ButtonVariant, string | undefined> = {
  primary: ANGLED_CLIP,
  secondary: ANGLED_CLIP,
  ghost: undefined,
  alert: undefined,
  icon: undefined,
}

export default function CyberButton({
  children,
  variant = 'primary',
  disabled = false,
  onClick,
  className = '',
}: CyberButtonProps) {
  const padding = variant === 'icon' ? 'px-3 py-2' : 'px-5 py-2.5'

  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`inline-flex items-center gap-2 font-['Orbitron'] text-[9px] font-bold uppercase tracking-[0.18em] transition-all duration-200 ${padding} ${variantStyles[variant]} disabled:opacity-30 disabled:cursor-not-allowed ${className}`}
      style={variantClipPath[variant] ? { clipPath: variantClipPath[variant] } : undefined}
    >
      {children}
    </button>
  )
}
