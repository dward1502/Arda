// sigil: REPAIR
import { Minus, Maximize2, X } from 'lucide-react'
import type { ReactNode } from 'react'

interface WindowControlsProps {
  onMinimize: () => void
  onToggleFullscreen: () => void
  onClose: () => void
}

export default function WindowControls({ onMinimize, onToggleFullscreen, onClose }: WindowControlsProps) {
  return (
    <div className="window-controls" data-tauri-drag-region>
      <button type="button" className="window-control-btn" onClick={onMinimize} title="Minimize">
        <Minus size={14} />
      </button>
      <button type="button" className="window-control-btn" onClick={onToggleFullscreen} title="Toggle Fullscreen">
        <Maximize2 size={14} />
      </button>
      <button type="button" className="window-control-btn window-control-btn--close" onClick={onClose} title="Close">
        <X size={14} />
      </button>
    </div>
  )
}
