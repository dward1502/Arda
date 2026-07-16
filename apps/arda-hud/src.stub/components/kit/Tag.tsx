// sigil: REPAIR
import type { ReactNode } from 'react'

type TagVariant = 'orange' | 'purple' | 'alert' | 'gold'

interface TagProps {
  variant: TagVariant
  children: ReactNode
  className?: string
}

const variantStyles: Record<TagVariant, string> = {
  orange: 'bg-[#ff6b3566] text-[#ff6b35]',
  purple: 'bg-[#9b59b666] text-[#e056fd]',
  alert: 'bg-[#ff333322] text-[#ff3333]',
  gold: 'bg-[#f9ca2422] text-[#f9ca24]',
}

const TAG_CLIP = 'polygon(6px 0%, 100% 0%, calc(100% - 6px) 100%, 0% 100%)'

export default function Tag({ variant, children, className = '' }: TagProps) {
  return (
    <span
      className={`inline-block px-2 py-1 font-['Orbitron'] text-[7px] uppercase tracking-[0.12em] ${variantStyles[variant]} ${className}`}
      style={{ clipPath: TAG_CLIP }}
    >
      {children}
    </span>
  )
}
