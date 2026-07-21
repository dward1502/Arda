// sigil: REPAIR
import { useCallback } from 'react'
import { safeTauriInvoke } from '../../../lib/tauriGuard'

export function useArdaWindowControls(windowLabel: string) {
  const minimizeWindow = useCallback(() => {
    void safeTauriInvoke('minimize_window', { windowLabel }).catch(console.error)
  }, [windowLabel])

  const toggleFullscreen = useCallback(() => {
    void safeTauriInvoke('toggle_fullscreen', { windowLabel }).catch(console.error)
  }, [windowLabel])

  const closeWindow = useCallback(() => {
    void safeTauriInvoke('close_window', { windowLabel }).catch(console.error)
  }, [windowLabel])

  const startDragging = useCallback(() => {
    void safeTauriInvoke('start_dragging', { windowLabel }).catch(console.error)
  }, [windowLabel])

  return {
    closeWindow,
    minimizeWindow,
    toggleFullscreen,
    startDragging,
  }
}
