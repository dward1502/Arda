// sigil: REPAIR
import { ArrowDown, ArrowUp } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import type { ModuleId } from './types'

interface LayoutEditorProps {
  moduleOrder: ModuleId[]
  titleById: Record<ModuleId, string>
  onMove: (moduleId: ModuleId, direction: 'up' | 'down') => void
}

export default function LayoutEditor({ moduleOrder, titleById, onMove }: LayoutEditorProps) {
  return (
    <ModuleCard title="Layout" eyebrow="Movable modules" accent="violet">
      <div className="layout-manager">
        {moduleOrder.map((moduleId, index) => (
          <div className="layout-manager__row" key={moduleId}>
            <span>{titleById[moduleId]}</span>
            <div className="layout-manager__actions">
              <button
                type="button"
                className="layout-manager__button"
                onClick={() => onMove(moduleId, 'up')}
                disabled={index === 0}
              >
                <ArrowUp size={14} />
              </button>
              <button
                type="button"
                className="layout-manager__button"
                onClick={() => onMove(moduleId, 'down')}
                disabled={index === moduleOrder.length - 1}
              >
                <ArrowDown size={14} />
              </button>
            </div>
          </div>
        ))}
      </div>
    </ModuleCard>
  )
}
