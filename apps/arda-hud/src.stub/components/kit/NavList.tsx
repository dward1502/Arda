// sigil: REPAIR
import { fallbackProtocolMarker } from '../../lib/soterionRender'

interface NavListItem {
  id: string
  label: string
  meta?: string
  active?: boolean
}

interface NavListProps {
  items: NavListItem[]
  onItemClick?: (item: NavListItem) => void
  className?: string
}

export default function NavList({ items, onItemClick, className = '' }: NavListProps) {
  const reviewMarker = fallbackProtocolMarker('REVIEW')
  return (
    <div className={`flex flex-col gap-0.5 ${className}`}>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          onClick={onItemClick ? () => onItemClick(item) : undefined}
          className={`flex items-center gap-2.5 border-l-2 px-3 py-2 text-left font-['Rajdhani'] text-[13px] transition-all duration-150 ${
            item.active
              ? 'border-l-[#ff6b35] bg-[#ff6b3522] text-[#e8e8f0]'
              : 'border-l-transparent text-[#e8e8f066] hover:border-l-[#ff6b35] hover:bg-[#ff6b3522] hover:text-[#e8e8f0]'
          }`}
        >
          <span
            className={`shrink-0 text-[8px] transition-colors duration-150 ${
              item.active ? 'text-[#ff6b35]' : 'text-[#9b59b6] group-hover:text-[#ff6b35]'
            }`}
          >
            {reviewMarker}
          </span>
          <span className="flex-1">{item.label}</span>
          {item.meta && (
            <span className="ml-auto font-['Share_Tech_Mono'] text-[10px] text-[#9b59b6]">
              {item.meta}
            </span>
          )}
        </button>
      ))}
    </div>
  )
}
