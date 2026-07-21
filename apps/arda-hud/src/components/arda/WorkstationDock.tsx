// sigil: REPAIR
import { X } from 'lucide-react'
import type { ReactNode } from 'react'

interface WorkstationDockProps {
  items: Array<{
    id: string
    title: string
    zIndex: number
  }>
  onFocus: (id: string) => void
  onClose: (id: string) => void
  onClear: () => void
}

export default function WorkstationDock({ items, onFocus, onClose, onClear }: WorkstationDockProps) {
  const sortedItems = [...items].sort((a, b) => b.zIndex - a.zIndex)

  return (
    <div className="workstation-dock">
      <div className="workstation-dock__header">
        <div className="workstation-dock__label">Open Workstations</div>
        <button type="button" className="workstation-dock__tile">
          Tile
        </button>
      </div>
      <div className="workstation-dock__items">
        {sortedItems.map((item) => (
          <div className="workstation-dock__item" key={`dock-${item.id}`}>
            <button type="button" className="workstation-dock__focus" onClick={() => onFocus(item.id)}>
              {item.title}
            </button>
            <button
              type="button"
              className="workstation-dock__close"
              onClick={() => onClose(item.id)}
              aria-label={`Close ${item.title}`}
            >
              <X size={12} />
            </button>
          </div>
        ))}
      </div>
      <button type="button" className="workstation-dock__clear" onClick={onClear}>
        Clear All
      </button>
    </div>
  )
}
