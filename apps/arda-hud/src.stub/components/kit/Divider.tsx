// sigil: REPAIR
import { fallbackProtocolMarker } from '../../lib/soterionRender'

type DividerVariant = 'default' | 'orange'

interface DividerProps {
  variant?: DividerVariant
  className?: string
}

const gradients: Record<DividerVariant, string> = {
  default: 'linear-gradient(90deg, transparent, rgba(155, 89, 182, 0.4), transparent)',
  orange: 'linear-gradient(90deg, transparent, rgba(255, 107, 53, 0.4), transparent)',
}

const diamondColor: Record<DividerVariant, string> = {
  default: 'text-[#9b59b6]',
  orange: 'text-[#ff6b35]',
}

export default function Divider({ variant = 'default', className = '' }: DividerProps) {
  const reviewMarker = fallbackProtocolMarker('REVIEW')
  return (
    <div className={`relative my-4 h-px ${className}`} style={{ background: gradients[variant] }}>
      <span
        className={`absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-[#0f0018] px-2 text-[8px] ${diamondColor[variant]}`}
      >
        {reviewMarker}
      </span>
    </div>
  )
}
