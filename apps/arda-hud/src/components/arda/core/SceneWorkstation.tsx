// sigil: REPAIR
import { ExternalLink, Grip, X, Maximize2 } from 'lucide-react'
import { useEffect, useRef, useState, type ReactNode } from 'react'
import type { ModuleId } from './types'

interface SceneWorkstationModule {
  id: ModuleId
  title: string
  node: ReactNode
}

interface SceneWorkstationProps {
  id: string
  title: string
  subtitle: string
  x: number
  y: number
  width: number
  height: number
  zIndex: number
  modules: SceneWorkstationModule[]
  activeModuleId?: ModuleId | null
  onFocus: (id: string) => void
  onClose: (id: string) => void
  onPopout?: (id: string) => void
  onMove: (id: string, x: number, y: number) => void
  onResize?: (id: string, width: number, height: number) => void
  onActiveModuleChange?: (id: string, moduleId: ModuleId) => void
}

export default function SceneWorkstation({
  id,
  title,
  subtitle,
  x,
  y,
  width,
  height,
  zIndex,
  modules,
  activeModuleId,
  onFocus,
  onClose,
  onPopout,
  onMove,
  onResize,
  onActiveModuleChange,
}: SceneWorkstationProps) {
  const [localActiveModuleId, setLocalActiveModuleId] = useState<ModuleId>(modules[0]?.id ?? 'section_focus')
  const [isDragging, setIsDragging] = useState(false)
  const [isResizing, setIsResizing] = useState(false)
  const dragRef = useRef<{ pointerId: number; offsetX: number; offsetY: number } | null>(null)
  const resizeRef = useRef<{ pointerId: number; startX: number; startY: number; startWidth: number; startHeight: number } | null>(null)
  const resolvedActiveModuleId = activeModuleId ?? localActiveModuleId
  const activeModule = modules.find((module) => module.id === resolvedActiveModuleId) ?? modules[0] ?? null

  const clearTextSelection = () => {
    const selection = window.getSelection?.()
    if (selection && selection.rangeCount > 0) {
      selection.removeAllRanges()
    }
  }

  const setDocumentInteractionMode = (active: boolean) => {
    document.documentElement.classList.toggle('arda-scene-window-interaction', active)
    document.body.classList.toggle('arda-scene-window-interaction', active)
    if (active) {
      clearTextSelection()
    }
  }

  const endDocumentInteractionMode = () => {
    dragRef.current = null
    resizeRef.current = null
    setIsDragging(false)
    setIsResizing(false)
    setDocumentInteractionMode(false)
    clearTextSelection()
  }

  const capturePointer = (target: HTMLElement, pointerId: number) => {
    try {
      target.setPointerCapture(pointerId)
    } catch {
      // Pointer capture can fail for synthetic events or if the native pointer was already released.
      // The document-level interaction guard still prevents text selection in those edge cases.
    }
  }

  const releasePointer = (target: HTMLElement, pointerId: number) => {
    try {
      if (target.hasPointerCapture(pointerId)) {
        target.releasePointerCapture(pointerId)
      }
    } catch {
      // Ignore stale pointer capture state; cleanup below is authoritative.
    }
  }

  const stopScenePointerEvent = (event: React.PointerEvent<HTMLElement>) => {
    event.stopPropagation()
    event.nativeEvent.stopImmediatePropagation?.()
  }

  useEffect(() => {
    const blockSelection = (event: Event) => {
      if (dragRef.current || resizeRef.current) {
        event.preventDefault()
        clearTextSelection()
      }
    }
    document.addEventListener('selectstart', blockSelection, true)
    document.addEventListener('dragstart', blockSelection, true)
    window.addEventListener('blur', endDocumentInteractionMode)
    return () => {
      document.removeEventListener('selectstart', blockSelection, true)
      document.removeEventListener('dragstart', blockSelection, true)
      window.removeEventListener('blur', endDocumentInteractionMode)
      setDocumentInteractionMode(false)
    }
  }, [])

  const beginDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.pointerType === 'mouse' && event.button !== 0) return
    event.preventDefault()
    stopScenePointerEvent(event)
    dragRef.current = {
      pointerId: event.pointerId,
      offsetX: event.clientX - x,
      offsetY: event.clientY - y,
    }
    setIsDragging(true)
    setDocumentInteractionMode(true)
    capturePointer(event.currentTarget, event.pointerId)
    onFocus(id)
  }

  const continueDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!dragRef.current || dragRef.current.pointerId !== event.pointerId) return
    event.preventDefault()
    stopScenePointerEvent(event)
    clearTextSelection()
    onMove(id, event.clientX - dragRef.current.offsetX, event.clientY - dragRef.current.offsetY)
  }

  const endDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!dragRef.current || dragRef.current.pointerId !== event.pointerId) return
    event.preventDefault()
    stopScenePointerEvent(event)
    dragRef.current = null
    setIsDragging(false)
    releasePointer(event.currentTarget, event.pointerId)
    setDocumentInteractionMode(false)
    clearTextSelection()
  }

  const beginResize = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.pointerType === 'mouse' && event.button !== 0) return
    event.preventDefault()
    stopScenePointerEvent(event)
    resizeRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startWidth: width,
      startHeight: height,
    }
    setIsResizing(true)
    setDocumentInteractionMode(true)
    capturePointer(event.currentTarget, event.pointerId)
    onFocus(id)
  }

  const continueResize = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!resizeRef.current || resizeRef.current.pointerId !== event.pointerId) return
    if (!onResize) return
    event.preventDefault()
    stopScenePointerEvent(event)
    clearTextSelection()
    const newWidth = Math.max(300, resizeRef.current.startWidth + (event.clientX - resizeRef.current.startX))
    const newHeight = Math.max(200, resizeRef.current.startHeight + (event.clientY - resizeRef.current.startY))
    onResize(id, newWidth, newHeight)
  }

  const endResize = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!resizeRef.current || resizeRef.current.pointerId !== event.pointerId) return
    event.preventDefault()
    stopScenePointerEvent(event)
    resizeRef.current = null
    setIsResizing(false)
    releasePointer(event.currentTarget, event.pointerId)
    setDocumentInteractionMode(false)
    clearTextSelection()
  }

  const stopTitlebarPointer = (event: React.PointerEvent<HTMLElement>) => {
    clearTextSelection()
    stopScenePointerEvent(event)
  }

  const handleWorkstationPointerDown = (event: React.PointerEvent<HTMLElement>) => {
    onFocus(id)
    clearTextSelection()
    stopScenePointerEvent(event)
    const target = event.target as HTMLElement | null
    if (target?.closest('iframe, input, textarea, select, [contenteditable="true"], [data-allow-selection="true"]')) {
      return
    }
    event.preventDefault()
  }

  const handleActionClick = (callback: () => void) => (event: React.MouseEvent<HTMLButtonElement>) => {
    clearTextSelection()
    event.stopPropagation()
    event.nativeEvent.stopImmediatePropagation?.()
    callback()
  }

  return (
    <article
      className={`scene-workstation${isDragging ? ' scene-workstation--dragging' : ''}${isResizing ? ' scene-workstation--resizing' : ''}`}
      style={{
        left: `${x}px`,
        top: `${y}px`,
        width: `${width}px`,
        height: `${height}px`,
        zIndex,
      }}
      onPointerDown={handleWorkstationPointerDown}
      onDragStart={(event) => event.preventDefault()}
    >
      <div
        className="scene-workstation__titlebar"
        onPointerDown={beginDrag}
        onPointerMove={continueDrag}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onLostPointerCapture={endDocumentInteractionMode}
      >
        <div className="scene-workstation__identity">
          <span className="scene-workstation__eyebrow">Workstation</span>
          <strong className="scene-workstation__title">{title}</strong>
          <span className="scene-workstation__subtitle">{subtitle}</span>
        </div>
        <div className="scene-workstation__actions">
          <button
            type="button"
            className="scene-workstation__icon"
            onPointerDown={stopTitlebarPointer}
            onClick={handleActionClick(() => onFocus(id))}
            aria-label="Focus workstation"
          >
            <Grip size={14} />
          </button>
          {onPopout ? (
            <button
              type="button"
              className="scene-workstation__icon"
              onPointerDown={stopTitlebarPointer}
              onClick={handleActionClick(() => onPopout(id))}
              aria-label="Pop out workstation"
            >
              <ExternalLink size={14} />
            </button>
          ) : null}
          <button
            type="button"
            className="scene-workstation__icon scene-workstation__icon--danger"
            onPointerDown={stopTitlebarPointer}
            onClick={handleActionClick(() => onClose(id))}
            aria-label="Close workstation"
          >
            <X size={14} />
          </button>
        </div>
      </div>

      <div className="scene-workstation__tabs">
        {modules.map((module) => (
          <button
            key={module.id}
            type="button"
            className={module.id === activeModule?.id ? 'scene-workstation__tab scene-workstation__tab--active' : 'scene-workstation__tab'}
            onPointerDown={stopTitlebarPointer}
            onClick={() => {
              onFocus(id)
              setLocalActiveModuleId(module.id)
              onActiveModuleChange?.(id, module.id)
            }}
          >
            {module.title}
          </button>
        ))}
      </div>

      <div className="scene-workstation__terminal-strip">
        <span className="scene-workstation__terminal-code">TERM READY</span>
        <span className="scene-workstation__terminal-hint">Open focused module and run boardroom action flow.</span>
      </div>

      <div className="scene-workstation__body">
        {activeModule?.node}
      </div>
      {onResize && (
        <div 
          className="scene-workstation__resize"
          onPointerDown={beginResize}
          onPointerMove={continueResize}
          onPointerUp={endResize}
          onPointerCancel={endResize}
          onLostPointerCapture={endDocumentInteractionMode}
        >
          <Maximize2 size={12} />
        </div>
      )}
    </article>
  )
}
